use actix_web::{HttpResponse, Responder, get, web};
use std::collections::{HashSet, VecDeque};
use url::Url;

use crate::browser::{create_browser, fetch_page_html};
use crate::models::{CrawlPage, CrawlParams, CrawlResponse};
use crate::utils::{extract_page_data, normalize_host};

// --------------------------------------------------
// CRAWL
// --------------------------------------------------

#[get("/crawl")]
pub async fn crawl_page(params: web::Query<CrawlParams>) -> impl Responder {
    // ----------------------------------------------
    // Crawl limits
    // ----------------------------------------------

    let max_pages = params.max_pages.unwrap_or(5).min(20);
    let max_depth = params.max_depth.unwrap_or(1).min(5);

    println!(
        "Starting crawl: {} | max_pages={} | max_depth={}",
        params.url, max_pages, max_depth
    );

    // ----------------------------------------------
    // Parse starting URL
    // ----------------------------------------------

    let base_url = match Url::parse(&params.url) {
        Ok(url) => url,
        Err(error) => {
            return HttpResponse::BadRequest().body(format!("Invalid URL: {}", error));
        }
    };

    // ----------------------------------------------
    // Dynamic allowed host
    // ----------------------------------------------

    let allowed_host = match base_url.host_str() {
        Some(host) => normalize_host(host),
        None => {
            return HttpResponse::BadRequest().body("URL does not contain a valid host");
        }
    };

    println!("Allowed host: {}", allowed_host);

    // ----------------------------------------------
    // Create browser
    // ----------------------------------------------

    let browser = match create_browser() {
        Ok(browser) => browser,
        Err(error) => {
            return HttpResponse::InternalServerError().body(error);
        }
    };

    // ----------------------------------------------
    // Queue
    // ----------------------------------------------

    let mut queue = VecDeque::new();

    // ----------------------------------------------
    // Visited URLs
    // ----------------------------------------------

    let mut visited = HashSet::new();

    // Starting URL = depth 0
    queue.push_back((params.url.clone(), 0usize));

    // ----------------------------------------------
    // Results
    // ----------------------------------------------

    let mut pages = Vec::new();

    // ----------------------------------------------
    // Crawl loop
    // ----------------------------------------------

    while let Some((url, depth)) = queue.pop_front() {
        // ------------------------------------------
        // Maximum pages
        // ------------------------------------------

        if pages.len() >= max_pages {
            break;
        }

        // ------------------------------------------
        // Already visited
        // ------------------------------------------

        if visited.contains(&url) {
            continue;
        }

        // ------------------------------------------
        // Maximum depth
        // ------------------------------------------

        if depth > max_depth {
            continue;
        }

        // ------------------------------------------
        // Mark as visited
        // ------------------------------------------

        visited.insert(url.clone());

        println!(
            "Crawling page {}/{} at depth {}: {}",
            pages.len() + 1,
            max_pages,
            depth,
            url
        );

        // ------------------------------------------
        // Fetch HTML
        // ------------------------------------------

        let html = match fetch_page_html(&browser, &url) {
            Ok(html) => html,
            Err(error) => {
                println!("Failed to crawl {}: {}", url, error);
                continue;
            }
        };

        // ------------------------------------------
        // Extract data
        // ------------------------------------------

        let (title, content, links, _images) = extract_page_data(&html, &url, &allowed_host);

        println!("Found {} links on {}", links.len(), url);

        for link in &links {
            println!("Crawl link: {}", link);
        }

        // ------------------------------------------
        // Add next-level URLs
        // ------------------------------------------

        if depth < max_depth {
            for link in &links {
                if !visited.contains(link) {
                    queue.push_back((link.clone(), depth + 1));
                }
            }
        }

        // ------------------------------------------
        // Save page
        // ------------------------------------------

        pages.push(CrawlPage {
            url,
            title,
            depth,
            content,
            links,
        });
    }

    // ----------------------------------------------
    // Response
    // ----------------------------------------------

    let response = CrawlResponse {
        start_url: params.url.clone(),
        pages_crawled: pages.len(),
        pages,
    };

    HttpResponse::Ok().json(response)
}
