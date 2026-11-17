use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    /// Full name of the user.
    pub name: String,

    /// Mobile number. Used as the unique login ID, so it can be registered
    /// only once.
    pub mobile_number: String,

    /// Postal address of the user.
    pub address: String,

    /// Password in plain text. It is hashed with Argon2 before it is saved.
    pub password: String,

    /// Age in years. Must be greater than 0.
    pub age: i32,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisterResponse {
    /// Unique user ID (UUID).
    pub id: String,

    /// Full name of the user.
    pub name: String,

    /// Registered mobile number.
    pub mobile_number: String,

    /// Status message, for example `Registration successful`.
    pub message: String,
}
