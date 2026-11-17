use crate::auth::password::verify_password;
use crate::models::auth::login::{LoginData, LoginEvent, LoginRequest, LoginResponse, LoginUser};
use actix_web::{HttpResponse, Responder, post, web};
use chrono::Utc;
use sqlx::{PgPool, Row};
use uuid::Uuid;

#[post("/login")]
pub async fn login(pool: web::Data<PgPool>, request: web::Json<LoginRequest>) -> impl Responder {
    if request.mobile_number.trim().is_empty() || request.password.trim().is_empty() {
        return HttpResponse::BadRequest().body("Mobile number and password are required");
    }

    // --------------------------------------------------
    // 1. Find user
    // --------------------------------------------------

    let user = sqlx::query(
        r#"
        SELECT id, name, mobile_number, password_hash
        FROM users
        WHERE mobile_number = $1
        "#,
    )
    .bind(request.mobile_number.trim())
    .fetch_optional(pool.get_ref())
    .await;

    let user = match user {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpResponse::Unauthorized().body("Invalid mobile number or password");
        }
        Err(error) => {
            println!("Login database error: {}", error);
            return HttpResponse::InternalServerError().body("Database error");
        }
    };

    // --------------------------------------------------
    // 2. Verify password
    // --------------------------------------------------

    let password_hash: String = user.get("password_hash");

    let password_valid = match verify_password(&request.password, &password_hash) {
        Ok(valid) => valid,
        Err(error) => {
            println!("Password hash verification error: {}", error);
            return HttpResponse::InternalServerError().body("Authentication error");
        }
    };

    if !password_valid {
        return HttpResponse::Unauthorized().body("Invalid mobile number or password");
    }

    // --------------------------------------------------
    // 3. Get user information
    // --------------------------------------------------

    let id: Uuid = user.get("id");
    let name: String = user.get("name");
    let mobile_number: String = user.get("mobile_number");

    // --------------------------------------------------
    // 4. Get user's stored events
    // --------------------------------------------------

    let user_events = sqlx::query(
        r#"
        SELECT user_events
        FROM user_data
        WHERE user_id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool.get_ref())
    .await;

    let stored_events = match user_events {
        Ok(Some(row)) => {
            let user_events_json: serde_json::Value = row.get("user_events");

            match serde_json::from_value::<Vec<LoginEvent>>(user_events_json) {
                Ok(events) => events,
                Err(error) => {
                    println!("User events parsing error: {}", error);

                    return HttpResponse::InternalServerError().body("Failed to load user events");
                }
            }
        }

        Ok(None) => Vec::new(),

        Err(error) => {
            println!("User events database error: {}", error);

            return HttpResponse::InternalServerError().body("Failed to load user events");
        }
    };

    // No stored events for this user.
    if stored_events.is_empty() {
        let login_user = LoginUser {
            id: id.to_string(),
            name,
            mobile_number,
        };

        return HttpResponse::Ok().json(LoginResponse {
            message: "Login successful".to_string(),
            data: LoginData {
                user: login_user,
                events: Vec::new(),
            },
        });
    }

    // --------------------------------------------------
    // 5. Extract event IDs
    // --------------------------------------------------

    let event_ids: Vec<String> = stored_events
        .iter()
        .map(|event| event.event_id.clone())
        .collect();

    // --------------------------------------------------
    // 6. Get matching events from events table
    // --------------------------------------------------

    let event_rows = sqlx::query(
        r#"
        SELECT event_id, event_date
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
            println!("Events database error: {}", error);

            return HttpResponse::InternalServerError().body("Failed to load events");
        }
    };

    // --------------------------------------------------
    // 7. Calculate effective event status
    // --------------------------------------------------

    let today = Utc::now().date_naive();

    let mut events = Vec::new();

    for stored_event in stored_events {
        let matching_event = event_rows.iter().find(|row| {
            let event_id: String = row.get("event_id");
            event_id == stored_event.event_id
        });

        let Some(event) = matching_event else {
            // Stored event no longer exists in events table.
            continue;
        };

        let event_date: Option<chrono::NaiveDate> = event.get("event_date");

        let status = if event_date.is_some_and(|date| date < today) {
            "COMPLETED".to_string()
        } else {
            stored_event.status
        };

        events.push(LoginEvent {
            event_id: stored_event.event_id,
            status,
        });
    }

    // --------------------------------------------------
    // 8. Build login response
    // --------------------------------------------------

    let login_user = LoginUser {
        id: id.to_string(),
        name,
        mobile_number,
    };

    let login_data = LoginData {
        user: login_user,
        events,
    };

    HttpResponse::Ok().json(LoginResponse {
        message: "Login successful".to_string(),
        data: login_data,
    })
}
