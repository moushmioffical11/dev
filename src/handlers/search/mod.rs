use actix_web::{HttpResponse, Responder, get, web};
use std::collections::{HashSet, VecDeque};
use url::Url;

use crate::browser::{create_browser, fetch_page_html};
use crate::models::{SearchParams, SearchResponse, SearchResult};
use crate::utils::{create_search_snippet, extract_page_data, normalize_host};

// --------------------------------------------------
// SEARCH
// --------------------------------------------------

#[get("/search")]
pub async fn search_page(params: web::Query<SearchParams>) -> impl Responder {
    // ----------------------------------------------
    // Validate search query
    // ----------------------------------------------

    let query = params.q.trim().to_lowercase();

    if query.is_empty() {
        return HttpResponse::BadRequest().body("Search query cannot be empty");
    }

    // ----------------------------------------------
    // Limits
    // ----------------------------------------------

    let max_pages = params.max_pages.unwrap_or(5).min(20);
    let max_depth = params.max_depth.unwrap_or(1).min(5);

    println!(
        "Searching '{}' on {} | max_pages={} | max_depth={}",
        query, params.url, max_pages, max_depth
    );

    // ----------------------------------------------
    // Parse URL
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
    // Visited
    // ----------------------------------------------

    let mut visited = HashSet::new();

    // Starting URL = depth 0
    queue.push_back((params.url.clone(), 0usize));

    // ----------------------------------------------
    // Results
    // ----------------------------------------------

    let mut pages_searched = 0;
    let mut results = Vec::new();

    // ----------------------------------------------
    // Search loop
    // ----------------------------------------------

    while let Some((url, depth)) = queue.pop_front() {
        // ------------------------------------------
        // Maximum pages
        // ------------------------------------------

        if pages_searched >= max_pages {
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
        // Mark visited
        // ------------------------------------------

        visited.insert(url.clone());

        println!(
            "Searching page {}/{} at depth {}: {}",
            pages_searched + 1,
            max_pages,
            depth,
            url
        );

        // ------------------------------------------
        // Fetch page
        // ------------------------------------------

        let html = match fetch_page_html(&browser, &url) {
            Ok(html) => html,
            Err(error) => {
                println!("Failed to read {}: {}", url, error);
                continue;
            }
        };

        // ------------------------------------------
        // Extract page
        // ------------------------------------------

        let (title, content, links, _images) = extract_page_data(&html, &url, &allowed_host);

        pages_searched += 1;

        // ------------------------------------------
        // Search content
        // ------------------------------------------

        if let Some(snippet) = create_search_snippet(&content, &query) {
            results.push(SearchResult {
                url: url.clone(),
                title: title.clone(),
                matched_content: snippet,
            });
        }

        // ------------------------------------------
        // Add next-level URLs
        // ------------------------------------------

        if depth < max_depth {
            for link in links {
                if !visited.contains(&link) {
                    queue.push_back((link, depth + 1));
                }
            }
        }
    }

    // ----------------------------------------------
    // Response
    // ----------------------------------------------

    let response = SearchResponse {
        query: params.q.clone(),
        pages_searched,
        results,
    };

    HttpResponse::Ok().json(response)
}
