use actix_web::{HttpResponse, Responder, get, web};

use crate::models::event_result::EventResultParams;

use super::{ifinish, mysamay};

#[get("/event-result")]
pub async fn event_result(params: web::Query<EventResultParams>) -> impl Responder {
    println!(
        "Event result request: source={}, year={}, event_name={}, bib_number={} ",
        params.source,
        params.year.as_deref().unwrap_or(""),
        params.event_name.as_deref().unwrap_or(""),
        params.bib_number
    );

    match params.source.to_lowercase().as_str() {
        "ifinish" => ifinish::handle(
            params.year.as_deref().unwrap_or(""),
            params.event_name.as_deref().unwrap_or(""),
            &params.bib_number,
        ),

        "mysamay" => mysamay::handle(
            params.event_name.as_deref().unwrap_or(""),
            &params.bib_number,
        ),

        _ => HttpResponse::BadRequest().body("Invalid source. Supported sources: ifinish, mysamay"),
    }
}
