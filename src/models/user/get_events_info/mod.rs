use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserEventDetails {
    pub id: String,
    pub provider: String,
    pub event_id: String,
    pub event_code: Option<String>,
    pub event_name: String,
    pub event_type: Option<String>,
    pub event_date: Option<chrono::NaiveDate>,
    pub organizer: serde_json::Value,
    pub dates: serde_json::Value,
    pub location: serde_json::Value,
    pub links: serde_json::Value,
    pub images: serde_json::Value,
    pub details: serde_json::Value,
    pub prices: serde_json::Value,
    pub raw_data: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserEventResponseData {
    pub event_id: String,
    pub status: String,
    pub event: UserEventDetails,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserEventsInfoGetResponse {
    pub message: String,
    pub data: Vec<UserEventResponseData>,
}
