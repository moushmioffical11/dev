use actix_web::{HttpResponse, Responder, get, web};
use serde_json::json;
use sqlx::PgPool;

use crate::models::EventListQuery;

mod queries;

use queries::{DETAIL_SQL, LIST_SQL};

fn bad_request(message: String) -> HttpResponse {
    HttpResponse::BadRequest().json(json!({
        "success": false,
        "error": message
    }))
}

fn server_error(message: &str) -> HttpResponse {
    HttpResponse::InternalServerError().json(json!({
        "success": false,
        "error": message
    }))
}

/// Trimmed, lower-case value, or None when missing or empty.
fn clean_lower(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_lowercase)
}

/// List events (from the events table), with optional filters.
#[get("/events")]
pub async fn list_events(
    pool: web::Data<PgPool>,
    query: web::Query<EventListQuery>,
) -> impl Responder {
    let provider = clean_lower(&query.provider);

    if let Some(provider) = &provider {
        if provider != "ifinish" && provider != "mysamay" {
            return bad_request(format!(
                "Invalid provider '{}'. Allowed values are 'ifinish' and 'mysamay'.",
                provider
            ));
        }
    }

    let status = clean_lower(&query.status);

    if let Some(status) = &status {
        if status != "upcoming" && status != "past" {
            return bad_request(format!(
                "Invalid status '{}'. Allowed values are 'upcoming' and 'past'.",
                status
            ));
        }
    }

    let search = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let limit = query.limit.unwrap_or(200).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);
    let include_raw = query.include_raw.unwrap_or(false);

    let events = sqlx::query_as::<_, crate::models::EventResponse>(LIST_SQL)
        .bind(&provider)
        .bind(&status)
        .bind(&search)
        .bind(limit)
        .bind(offset)
        .bind(include_raw)
        .fetch_all(pool.get_ref())
        .await;

    match events {
        Ok(events) => {
            let total = events.first().map(|event| event.total).unwrap_or(0);

            HttpResponse::Ok().json(json!({
                "success": true,
                "total": total,
                "count": events.len(),
                "limit": limit,
                "offset": offset,
                "data": events
            }))
        }

        Err(error) => {
            eprintln!("Failed to fetch events: {}", error);
            server_error("Failed to fetch events")
        }
    }
}

/// Get one event by its id, including the full raw_data.
#[get("/events/{id}")]
pub async fn get_event(pool: web::Data<PgPool>, id: web::Path<uuid::Uuid>) -> impl Responder {
    let event = sqlx::query_as::<_, crate::models::EventResponse>(DETAIL_SQL)
        .bind(id.into_inner())
        .fetch_optional(pool.get_ref())
        .await;

    match event {
        Ok(Some(event)) => HttpResponse::Ok().json(json!({
            "success": true,
            "data": event
        })),

        Ok(None) => HttpResponse::NotFound().json(json!({
            "success": false,
            "error": "Event not found"
        })),

        Err(error) => {
            eprintln!("Failed to fetch event: {}", error);
            server_error("Failed to fetch event")
        }
    }
}
