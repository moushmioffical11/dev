use crate::auth::password::hash_password;
use crate::models::{RegisterRequest, RegisterResponse};
use actix_web::{HttpResponse, Responder, post, web};
use sqlx::{PgPool, Row};
use std::time::Instant;

#[post("/register")]
pub async fn register(
    pool: web::Data<PgPool>,
    request: web::Json<RegisterRequest>,
) -> impl Responder {
    let start_time = Instant::now();

    println!(
        "Registering user: {} | Mobile: {} | Address: {} | Age: {}",
        request.name, request.mobile_number, request.address, request.age
    );

    // Basic validation
    if request.name.trim().is_empty()
        || request.mobile_number.trim().is_empty()
        || request.address.trim().is_empty()
        || request.password.trim().is_empty()
    {
        return HttpResponse::BadRequest().body("Required fields cannot be empty");
    }

    if request.age <= 0 {
        return HttpResponse::BadRequest().body("Age must be greater than 0");
    }

    // Hash password
    let hash_start = Instant::now();
    let password_hash = match hash_password(&request.password) {
        Ok(hash) => hash,

        Err(error) => {
            println!("Password hashing error: {}", error);

            return HttpResponse::InternalServerError().body("Failed to process password");
        }
    };

    println!("Password hashing took: {:?}", hash_start.elapsed());

    // Insert user
    let insert_start = Instant::now();

    let user = sqlx::query(
        r#"
        INSERT INTO users
            (
                name,
                mobile_number,
                address,
                password_hash,
                age
            )
        VALUES
            ($1, $2, $3, $4, $5)
        RETURNING id, name, mobile_number
        "#,
    )
    .bind(request.name.trim())
    .bind(request.mobile_number.trim())
    .bind(request.address.trim())
    .bind(password_hash)
    .bind(request.age)
    .fetch_one(pool.get_ref())
    .await;

    println!("DB insert took: {:?}", insert_start.elapsed());

    match user {
        Ok(user) => {
            println!("Total registration time: {:?}", start_time.elapsed());

            let id: uuid::Uuid = user.get("id");
            let name: String = user.get("name");
            let mobile_number: String = user.get("mobile_number");

            HttpResponse::Created().json(RegisterResponse {
                id: id.to_string(),
                name,
                mobile_number,
                message: "Registration successful".to_string(),
            })
        }

        Err(error) => {
            println!("Registration database error: {}", error);

            if let sqlx::Error::Database(db_error) = &error {
                if db_error.constraint() == Some("users_mobile_number_key") {
                    return HttpResponse::Conflict().body("Mobile number already registered");
                }
            }

            HttpResponse::InternalServerError().body("Failed to register user")
        }
    }
}
