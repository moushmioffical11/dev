use actix_web::{HttpResponse, Responder, get, web};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::handlers::event_result::{ifinish, mysamay};
use crate::models::user::event_results::EventResultQuery;
use crate::repositories::{event as event_repository, event_result as event_result_repository};

// --------------------------------------------------
// GET USER EVENT RESULTS
// --------------------------------------------------

#[get("/users/{user_id}/event-results")]
pub async fn get_event_results(
    pool: web::Data<PgPool>,
    user_id: web::Path<uuid::Uuid>,
    query: web::Query<EventResultQuery>,
) -> impl Responder {
    let user_id = user_id.into_inner();

    // --------------------------------------------------
    // Validate refresh request
    // --------------------------------------------------

    if query.refresh {
        let provider = match query.provider.as_deref() {
            Some(provider) => provider.to_lowercase(),

            None => {
                return HttpResponse::BadRequest().json(json!({
                    "message": "provider is required when refresh=true"
                }));
            }
        };

        match provider.as_str() {
            "ifinish" => {
                if query.year.is_none() || query.event_name.is_none() || query.bib_number.is_none()
                {
                    return HttpResponse::BadRequest().json(json!({
                        "message": "year, event_name and bib_number are required for ifinish"
                    }));
                }
            }

            "mysamay" => {
                if query.event_name.is_none() || query.bib_number.is_none() {
                    return HttpResponse::BadRequest().json(json!({
                        "message": "event_name and bib_number are required for mysamay"
                    }));
                }
            }

            _ => {
                return HttpResponse::BadRequest().json(json!({
                    "message": "Unsupported provider. Supported providers are ifinish and mysamay"
                }));
            }
        }

        // --------------------------------------------------
        // Fetch fresh result from external provider
        // --------------------------------------------------

        let provider_response = match provider.as_str() {
            "ifinish" => ifinish::fetch_result(
                query.year.as_deref().unwrap_or(""),
                query.event_name.as_deref().unwrap_or(""),
                query.bib_number.as_deref().unwrap_or(""),
            ),

            "mysamay" => mysamay::fetch_result(
                query.event_name.as_deref().unwrap_or(""),
                query.bib_number.as_deref().unwrap_or(""),
            ),

            _ => unreachable!(),
        };

        return match provider_response {
            Ok(result) => {
                // --------------------------------------------------
                // Prepare data for background persistence
                // --------------------------------------------------

                let result_value = match serde_json::to_value(&result) {
                    Ok(value) => value,

                    Err(error) => {
                        eprintln!("Failed to serialize provider result: {}", error);

                        return HttpResponse::InternalServerError().json(json!({
                            "message": "Failed to process provider result"
                        }));
                    }
                };

                let pool = pool.clone();
                let provider = provider.clone();
                let event_name = query.event_name.clone();

                // --------------------------------------------------
                // Save result in background
                // --------------------------------------------------

                tokio::spawn(async move {
                    let event_name = match event_name {
                        Some(event_name) => event_name,

                        None => {
                            eprintln!("Background event result save failed: event_name is missing");
                            return;
                        }
                    };

                    // --------------------------------------------------
                    // Find event in database
                    // --------------------------------------------------

                    let event_id = match event_repository::find_event_id(
                        pool.get_ref(),
                        &provider,
                        &event_name,
                    )
                    .await
                    {
                        // Event already exists
                        Ok(Some(event_id)) => event_id,

                        // Event does not exist
                        Ok(None) => {
                            eprintln!(
                                "Background event result save skipped: event not found. \
                                 provider={}, event_name={}",
                                provider, event_name
                            );
                            return;
                        }

                        // Database lookup failed
                        Err(error) => {
                            eprintln!(
                                "Background event result save failed while finding event: {}",
                                error
                            );
                            return;
                        }
                    };

                    // --------------------------------------------------
                    // Save provider result
                    // --------------------------------------------------

                    let mut transaction =
                        match event_result_repository::begin_transaction(pool.get_ref()).await {
                            Ok(transaction) => transaction,

                            Err(error) => {
                                eprintln!(
                                    "Background event result save failed while \
                                     starting transaction: {}",
                                    error
                                );
                                return;
                            }
                        };

                    if let Err(error) = event_result_repository::save_provider_event_result(
                        &mut transaction,
                        user_id,
                        &event_id,
                        result_value,
                        &provider,
                    )
                    .await
                    {
                        eprintln!(
                            "Background event result save failed while saving result: {}",
                            error
                        );

                        return;
                    }

                    // --------------------------------------------------
                    // Commit transaction
                    // --------------------------------------------------

                    if let Err(error) = transaction.commit().await {
                        eprintln!(
                            "Background event result save failed while committing transaction: {}",
                            error
                        );

                        return;
                    }

                    println!(
                        "Provider event result saved successfully: \
                         user_id={}, provider={}, event_id={}",
                        user_id, provider, event_id
                    );
                });

                // --------------------------------------------------
                // Return external provider response immediately
                // --------------------------------------------------

                HttpResponse::Ok().json(result)
            }

            Err(response) => response,
        };
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
    // Get stored event results
    // --------------------------------------------------

    let event_results = match sqlx::query_scalar::<_, Value>(
        r#"
        SELECT event_results
        FROM user_data
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool.get_ref())
    .await
    {
        Ok(Some(results)) => results,

        Ok(None) => {
            return HttpResponse::Ok().json(json!({
                "message": "Event results fetched successfully",
                "data": []
            }));
        }

        Err(error) => {
            eprintln!("Failed to fetch event results: {}", error);

            return HttpResponse::InternalServerError().json(json!({
                "message": "Failed to fetch event results"
            }));
        }
    };

    // --------------------------------------------------
    // Response
    // --------------------------------------------------

    HttpResponse::Ok().json(json!({
        "message": "Event results fetched successfully",
        "data": event_results
    }))
}
