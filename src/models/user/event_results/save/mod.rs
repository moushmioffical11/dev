use serde::Deserialize;
use utoipa::ToSchema;

// --------------------------------------------------
// EVENT RESULT - MANUAL SAVE
// --------------------------------------------------

#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ManualEventResultRequest {
    /// Event ID for which the result is being submitted.
    pub event_id: String,

    /// Flexible result details such as timing, rank, distance, price, etc.
    pub result: serde_json::Value,
}
