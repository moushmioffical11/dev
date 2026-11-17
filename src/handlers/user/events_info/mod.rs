use actix_web::{HttpResponse, Responder, post, web};
use sqlx::PgPool;

use crate::models::UserEventsInfoRequest;
use crate::models::user::events_info::{UserEventsInfoData, UserEventsInfoResponse};

#[post("/users/events-info")]
pub async fn events_info(
    pool: web::Data<PgPool>,
    request: web::Json<UserEventsInfoRequest>,
) -> impl Responder {
    let user_id = request.user_id;

    let user_events = match serde_json::to_value(&request.events) {
        Ok(value) => value,
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "message": "Invalid events data"
            }));
        }
    };

    let result = sqlx::query(
        r#"
        INSERT INTO user_data (user_id, user_events)
        VALUES ($1, $2)
        ON CONFLICT (user_id)
        DO UPDATE SET
            user_events = EXCLUDED.user_events,
            updated_at = now()
        "#,
    )
    .bind(user_id)
    .bind(user_events)
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(_) => {
            let data = UserEventsInfoData {
                user_id,
                events: request.events.clone(),
            };

            HttpResponse::Ok().json(UserEventsInfoResponse {
                message: "User events information saved successfully".to_string(),
                data,
            })
        }

        Err(error) => {
            eprintln!("Failed to save user events: {}", error);

            HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Failed to save user events information"
            }))
        }
    }
}
