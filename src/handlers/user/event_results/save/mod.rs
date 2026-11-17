use actix_web::{HttpResponse, Responder, post, web};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::{
    models::user::event_results::{
        EventResultResponse, EventResultResponseData, ManualEventResultRequest,
    },
    repositories::event_result as event_result_repository,
};

#[post("/users/{user_id}/event-results")]
pub async fn save_event_result(
    pool: web::Data<PgPool>,
    user_id: web::Path<uuid::Uuid>,
    request: web::Json<ManualEventResultRequest>,
) -> impl Responder {
    let user_id = user_id.into_inner();

    // --------------------------------------------------
    // Validate event ID
    // --------------------------------------------------

    if request.event_id.trim().is_empty() {
        return HttpResponse::BadRequest().json(json!({
            "message": "eventId is required"
        }));
    }

    // --------------------------------------------------
    // Validate result object
    // --------------------------------------------------

    if !request.result.is_object() {
        return HttpResponse::BadRequest().json(json!({
            "message": "result must be a JSON object"
        }));
    }

    // --------------------------------------------------
    // Validate timing
    // --------------------------------------------------

    let timing = request.result.get("timing").and_then(Value::as_str);

    let timing = match timing {
        Some(timing) if !timing.trim().is_empty() => timing,

        Some(_) => {
            return HttpResponse::BadRequest().json(json!({
                "message": "timing is required"
            }));
        }

        None => {
            return HttpResponse::BadRequest().json(json!({
                "message": "timing is required"
            }));
        }
    };

    let timing_parts: Vec<&str> = timing.split(':').collect();

    if timing_parts.len() != 3
        || timing_parts.iter().any(|part| part.len() != 2)
        || timing_parts.iter().any(|part| part.parse::<u32>().is_err())
    {
        return HttpResponse::BadRequest().json(json!({
            "message": "timing must be in HH:MM:SS format"
        }));
    }

    let timing_parts: Vec<u32> = timing_parts
        .iter()
        .filter_map(|part| part.parse::<u32>().ok())
        .collect();

    if timing_parts[0] > 99 || timing_parts[1] > 59 || timing_parts[2] > 59 {
        return HttpResponse::BadRequest().json(json!({
            "message": "timing must be in HH:MM:SS format"
        }));
    }

    // --------------------------------------------------
    // Check user exists
    // --------------------------------------------------

    let user_exists = match sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM users
            WHERE id = $1
        )
        "#,
    )
    .bind(user_id)
    .fetch_one(pool.get_ref())
    .await
    {
        Ok(exists) => exists,

        Err(error) => {
            eprintln!("Failed to check user: {}", error);

            return HttpResponse::InternalServerError().json(json!({
                "message": "Failed to validate user"
            }));
        }
    };

    if !user_exists {
        return HttpResponse::NotFound().json(json!({
            "message": "User not found"
        }));
    }

    // --------------------------------------------------
    // Check event exists and is completed
    // --------------------------------------------------

    let event_exists = match sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS(
            SELECT 1
            FROM events
            WHERE event_id = $1
              AND event_date IS NOT NULL
              AND event_date < CURRENT_DATE
        )
        "#,
    )
    .bind(&request.event_id)
    .fetch_one(pool.get_ref())
    .await
    {
        Ok(exists) => exists,

        Err(error) => {
            eprintln!("Failed to check event: {}", error);

            return HttpResponse::InternalServerError().json(json!({
                "message": "Failed to validate event"
            }));
        }
    };

    if !event_exists {
        return HttpResponse::BadRequest().json(json!({
            "message": "Event does not exist or is not completed"
        }));
    }

    // --------------------------------------------------
    // Save result transactionally
    // --------------------------------------------------

    let mut transaction = match event_result_repository::begin_transaction(pool.get_ref()).await {
        Ok(transaction) => transaction,

        Err(error) => {
            eprintln!("Failed to start transaction: {}", error);

            return HttpResponse::InternalServerError().json(json!({
                "message": "Failed to save event result"
            }));
        }
    };

    let result = match event_result_repository::save_manual_event_result(
        &mut transaction,
        user_id,
        &request.event_id,
        request.result.clone(),
    )
    .await
    {
        Ok(inserted) => inserted,

        Err(error) => {
            eprintln!("Failed to save event result: {}", error);

            return HttpResponse::InternalServerError().json(json!({
                "message": "Failed to save event result"
            }));
        }
    };

    // --------------------------------------------------
    // Commit transaction
    // --------------------------------------------------

    if let Err(error) = transaction.commit().await {
        eprintln!("Failed to commit event result: {}", error);

        return HttpResponse::InternalServerError().json(json!({
            "message": "Failed to save event result"
        }));
    }

    // --------------------------------------------------
    // Response
    // --------------------------------------------------

    let response = EventResultResponse {
        message: if result {
            "Event result created successfully".to_string()
        } else {
            "Event result updated successfully".to_string()
        },

        data: EventResultResponseData {
            event_id: request.event_id.clone(),
            result: request.result.clone(),
            source: "MANUAL".to_string(),
        },
    };

    if result {
        HttpResponse::Created().json(response)
    } else {
        HttpResponse::Ok().json(response)
    }
}
