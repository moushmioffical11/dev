use actix_web::{HttpResponse, Responder, get, web};
use chrono::Utc;
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::models::user::events_info::UserEvent;
use crate::models::user::get_events_info::{
    UserEventDetails, UserEventResponseData, UserEventsInfoGetResponse,
};

#[get("/users/{user_id}/events")]
pub async fn get_user_events(pool: web::Data<PgPool>, user_id: web::Path<Uuid>) -> impl Responder {
    let user_id = user_id.into_inner();

    // --------------------------------------------------
    // 1. Get user's stored events
    // --------------------------------------------------

    let user_events = sqlx::query(
        r#"
        SELECT user_events
        FROM user_data
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool.get_ref())
    .await;

    let user_events_json = match user_events {
        Ok(Some(row)) => row.get::<serde_json::Value, _>("user_events"),

        Ok(None) => {
            return HttpResponse::Ok().json(UserEventsInfoGetResponse {
                message: "No events found for user".to_string(),
                data: Vec::new(),
            });
        }

        Err(error) => {
            eprintln!("Failed to fetch user events: {}", error);

            return HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Failed to fetch user events"
            }));
        }
    };

    // --------------------------------------------------
    // 2. Parse stored events
    // --------------------------------------------------

    let stored_events = match serde_json::from_value::<Vec<UserEvent>>(user_events_json) {
        Ok(events) => events,

        Err(error) => {
            eprintln!("Failed to parse user events: {}", error);

            return HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Failed to parse user events"
            }));
        }
    };

    if stored_events.is_empty() {
        return HttpResponse::Ok().json(UserEventsInfoGetResponse {
            message: "No events found for user".to_string(),
            data: Vec::new(),
        });
    }

    // --------------------------------------------------
    // 3. Extract event IDs
    // --------------------------------------------------

    let event_ids: Vec<String> = stored_events
        .iter()
        .map(|event| event.event_id.clone())
        .collect();

    // --------------------------------------------------
    // 4. Fetch complete event details
    // --------------------------------------------------

    let event_rows = sqlx::query(
        r#"
        SELECT
            id,
            provider,
            event_id,
            event_code,
            event_name,
            event_type,
            event_date,
            organizer,
            dates,
            location,
            links,
            images,
            details,
            prices,
            raw_data,
            created_at,
            updated_at
        FROM events
        WHERE event_id = ANY($1)
        "#,
    )
    .bind(&event_ids)
    .fetch_all(pool.get_ref())
    .await;

    let event_rows = match event_rows {
        Ok(rows) => rows,

        Err(error) => {
            eprintln!("Failed to fetch event details: {}", error);

            return HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Failed to fetch event details"
            }));
        }
    };

    // --------------------------------------------------
    // 5. Build response
    // --------------------------------------------------

    let today = Utc::now().date_naive();

    let mut events = Vec::new();

    for stored_event in stored_events {
        let matching_event = event_rows.iter().find(|row| {
            let event_id: String = row.get("event_id");
            event_id == stored_event.event_id
        });

        let Some(event) = matching_event else {
            continue;
        };

        let event_date: Option<chrono::NaiveDate> = event.get("event_date");

        let status = if event_date.is_some_and(|date| date < today) {
            "COMPLETED".to_string()
        } else {
            stored_event.status
        };

        let event_details = UserEventDetails {
            id: event.get::<Uuid, _>("id").to_string(),
            provider: event.get("provider"),
            event_id: event.get("event_id"),
            event_code: event.get("event_code"),
            event_name: event.get("event_name"),
            event_type: event.get("event_type"),
            event_date,
            organizer: event.get("organizer"),
            dates: event.get("dates"),
            location: event.get("location"),
            links: event.get("links"),
            images: event.get("images"),
            details: event.get("details"),
            prices: event.get("prices"),
            raw_data: event.get("raw_data"),
            created_at: event.get("created_at"),
            updated_at: event.get("updated_at"),
        };

        events.push(UserEventResponseData {
            event_id: stored_event.event_id,
            status,
            event: event_details,
        });
    }

    // --------------------------------------------------
    // 6. Return response
    // --------------------------------------------------

    HttpResponse::Ok().json(UserEventsInfoGetResponse {
        message: "User events fetched successfully".to_string(),
        data: events,
    })
}
