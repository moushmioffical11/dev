use actix_web::{HttpResponse, Responder, get, web};
use url::Url;

use crate::browser::render_html;
use crate::models::{MapResponse, Params, ReaderResponse};
use crate::utils::{extract_page_data, normalize_host};

// --------------------------------------------------
// READ
// --------------------------------------------------

#[get("/read")]
pub async fn read_page(params: web::Query<Params>) -> impl Responder {
    println!("Reading URL: {}", params.url);

    let base_url = match Url::parse(&params.url) {
        Ok(url) => url,
        Err(error) => {
            return HttpResponse::BadRequest().body(format!("Invalid URL: {}", error));
        }
    };

    let allowed_host = match base_url.host_str() {
        Some(host) => normalize_host(host),
        None => {
            return HttpResponse::BadRequest().body("URL does not contain a valid host");
        }
    };

    let target = params.url.clone();

    let html = match web::block(move || render_html(&target)).await {
        Ok(Ok(html)) => html,
        Ok(Err((status, message))) => {
            return if status == 400 {
                HttpResponse::BadRequest().body(message)
            } else {
                HttpResponse::InternalServerError().body(message)
            };
        }
        Err(error) => {
            return HttpResponse::InternalServerError().body(format!("Worker error: {}", error));
        }
    };

    println!("Extracting page data...");

    let (title, content, links, images) = extract_page_data(&html, &params.url, &allowed_host);

    println!(
        "Extraction completed. Title: '{}', Content: {} chars, Links: {}, Images: {}",
        title,
        content.len(),
        links.len(),
        images.len()
    );

    let response = ReaderResponse {
        url: params.url.clone(),
        title,
        content,
        links,
        images,
    };

    HttpResponse::Ok().json(response)
}

// --------------------------------------------------
// MAP
// --------------------------------------------------

#[get("/map")]
pub async fn map_page(params: web::Query<Params>) -> impl Responder {
    println!("Mapping URL: {}", params.url);

    let base_url = match Url::parse(&params.url) {
        Ok(url) => url,
        Err(error) => {
            return HttpResponse::BadRequest().body(format!("Invalid URL: {}", error));
        }
    };

    let allowed_host = match base_url.host_str() {
        Some(host) => normalize_host(host),
        None => {
            return HttpResponse::BadRequest().body("URL does not contain a valid host");
        }
    };

    let target = params.url.clone();

    let html = match web::block(move || render_html(&target)).await {
        Ok(Ok(html)) => html,
        Ok(Err((status, message))) => {
            return if status == 400 {
                HttpResponse::BadRequest().body(message)
            } else {
                HttpResponse::InternalServerError().body(message)
            };
        }
        Err(error) => {
            return HttpResponse::InternalServerError().body(format!("Worker error: {}", error));
        }
    };

    println!("Extracting links...");

    let (_, _, links, _) = extract_page_data(&html, &params.url, &allowed_host);

    println!("Found {} links", links.len());

    let response = MapResponse {
        url: params.url.clone(),
        links,
    };

    HttpResponse::Ok().json(response)
}
