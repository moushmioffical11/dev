use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordRequest {
    /// Registered mobile number of the account.
    pub mobile_number: String,

    /// New password. Must be at least 8 characters.
    pub new_password: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ForgotPasswordResponse {
    /// Status message, for example `Password reset successful`.
    pub message: String,
}
