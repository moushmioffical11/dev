use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    /// User's registered mobile number.
    pub mobile_number: String,

    /// User's password.
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginUser {
    /// Unique ID of the user.
    pub id: String,

    /// Name of the user.
    pub name: String,

    /// Registered mobile number of the user.
    pub mobile_number: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginEvent {
    /// Unique identifier of the event.
    pub event_id: String,

    /// Current effective status of the user's event.
    ///
    /// Examples:
    /// - `INTERESTED`
    /// - `REGISTERED`
    /// - `COMPLETED`
    pub status: String,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginData {
    /// Logged-in user's profile information.
    pub user: LoginUser,

    /// Events associated with the logged-in user.
    pub events: Vec<LoginEvent>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LoginResponse {
    /// Login operation result message.
    pub message: String,

    /// User profile and associated events.
    pub data: LoginData,
}
