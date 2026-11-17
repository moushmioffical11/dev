use crate::auth::password::hash_password;
use crate::models::{ForgotPasswordRequest, ForgotPasswordResponse};
use actix_web::{HttpResponse, Responder, post, web};
use sqlx::PgPool;

#[post("/forgot-password")]
pub async fn forgot_password(
    pool: web::Data<PgPool>,
    request: web::Json<ForgotPasswordRequest>,
) -> impl Responder {
    if request.mobile_number.trim().is_empty() || request.new_password.trim().is_empty() {
        return HttpResponse::BadRequest().body("Mobile number and new password are required");
    }

    if request.new_password.len() < 8 {
        return HttpResponse::BadRequest().body("New password must be at least 8 characters");
    }

    // Hash the new password
    let password_hash = match hash_password(&request.new_password) {
        Ok(hash) => hash,

        Err(error) => {
            println!("Password hashing error: {}", error);

            return HttpResponse::InternalServerError().body("Failed to process password");
        }
    };

    // Update password
    let result = sqlx::query(
        r#"
        UPDATE users
        SET password_hash = $1
        WHERE mobile_number = $2
        "#,
    )
    .bind(password_hash)
    .bind(request.mobile_number.trim())
    .execute(pool.get_ref())
    .await;

    match result {
        Ok(result) => {
            if result.rows_affected() == 0 {
                return HttpResponse::NotFound().body("User not found");
            }

            HttpResponse::Ok().json(ForgotPasswordResponse {
                message: "Password reset successful".to_string(),
            })
        }

        Err(error) => {
            println!("Forgot password database error: {}", error);

            HttpResponse::InternalServerError().body("Failed to reset password")
        }
    }
}
