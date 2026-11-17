use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// --------------------------------------------------
// EVENT
// --------------------------------------------------

#[derive(Debug, Clone)]
pub struct EventRow {
    pub provider: String,
    pub event_id: String,
    pub event_code: Option<String>,
    pub event_name: String,
    pub event_type: Option<String>,
    pub event_date: Option<NaiveDate>,
    pub organizer: Value,
    pub dates: Value,
    pub location: Value,
    pub links: Value,
    pub images: Value,
    pub details: Value,
    pub prices: Value,
    pub raw_data: Option<Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SyncSummary {
    pub fetched: usize,
    pub inserted: usize,
    pub updated: usize,
    pub skipped: usize,
    pub failed: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct EventResponse {
    pub id: Uuid,
    pub provider: String,
    pub event_id: String,
    pub event_code: Option<String>,
    pub event_name: String,
    pub event_type: Option<String>,
    pub event_date: Option<NaiveDate>,
    pub organizer: Value,
    pub dates: Value,
    pub location: Value,
    pub links: Value,
    pub images: Value,
    pub details: Value,
    pub prices: Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_data: Option<Value>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    #[serde(skip)]
    pub total: i64,
}

#[allow(dead_code)]
#[derive(Serialize, ToSchema)]
pub struct EventListResponse {
    pub success: bool,
    pub total: i64,
    pub count: usize,
    pub limit: i64,
    pub offset: i64,
    pub data: Vec<EventResponse>,
}

#[allow(dead_code)]
#[derive(Serialize, ToSchema)]
pub struct EventDetailResponse {
    pub success: bool,
    pub data: EventResponse,
}

#[allow(dead_code)]
#[derive(Serialize, ToSchema)]
pub struct EventErrorResponse {
    pub success: bool,
    pub error: String,
}

// --------------------------------------------------
// EVENT LIST QUERY
// --------------------------------------------------

#[derive(Debug, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct EventListQuery {
    /// Filter by provider: `ifinish` or `mysamay`.
    pub provider: Option<String>,

    /// Filter by event status: `upcoming` or `past`.
    pub status: Option<String>,

    /// Search event name.
    pub q: Option<String>,

    /// Number of records to return. Default 200, maximum 200.
    pub limit: Option<i64>,

    /// Number of records to skip. Default 0.
    pub offset: Option<i64>,

    /// Include the original provider response in `raw_data`.
    pub include_raw: Option<bool>,
}
