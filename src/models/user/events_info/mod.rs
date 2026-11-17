use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// An event associated with a user.
#[derive(Debug, Deserialize, Serialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserEvent {
    /// Event identifier from the events table.
    pub event_id: String,

    /// User's current status for the event.
    ///
    /// Supported values:
    /// - `INTERESTED`
    /// - `REGISTERED`
    pub status: String,
}

/// Request body for saving a user's event information.
#[derive(Debug, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserEventsInfoRequest {
    /// ID of the user.
    pub user_id: Uuid,

    /// Events associated with the user.
    pub events: Vec<UserEvent>,
}

/// Response data for saved user event information.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserEventsInfoData {
    /// ID of the user.
    pub user_id: Uuid,

    /// Events associated with the user.
    pub events: Vec<UserEvent>,
}

/// Response for saving user event information.
#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserEventsInfoResponse {
    /// Result message.
    pub message: String,

    /// Saved user event information.
    pub data: UserEventsInfoData,
}
