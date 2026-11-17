use actix_web::{HttpResponse, Responder, get, web};
use url::Url;

use crate::browser::resolve_event_url;
use crate::models::{ResolveParams, ResolveResponse};

// --------------------------------------------------
// RESOLVE
// --------------------------------------------------

#[get("/resolve")]
pub async fn resolve_page(params: web::Query<ResolveParams>) -> impl Responder {
    println!("Resolving '{}' on {}", params.name, params.url);

    if Url::parse(&params.url).is_err() {
        return HttpResponse::BadRequest().body("Invalid URL");
    }

    let list_url = params.url.clone();
    let name = params.name.clone();

    let found = match web::block(move || resolve_event_url(&list_url, &name)).await {
        Ok(Ok(found)) => found,

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

    HttpResponse::Ok().json(ResolveResponse {
        name: params.name.clone(),
        url: found,
    })
}
