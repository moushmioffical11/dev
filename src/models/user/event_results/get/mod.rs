use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

// --------------------------------------------------
// EVENT RESULT - RESPONSE
// --------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventResultResponse {
    pub message: String,
    pub data: EventResultResponseData,
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EventResultResponseData {
    pub event_id: String,
    pub result: serde_json::Value,
    pub source: String,
}

// --------------------------------------------------
// USER EVENT RESULTS - GET RESPONSE
// --------------------------------------------------

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserEventResultsGetResponse {
    pub message: String,
    pub data: Vec<EventResultResponseData>,
}

// --------------------------------------------------
// EVENT RESULT - GET QUERY
// --------------------------------------------------

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct EventResultQuery {
    /// Whether to fetch a fresh result from the external provider.
    #[serde(default)]
    pub refresh: bool,

    /// External result provider: `ifinish` or `mysamay`.
    pub provider: Option<String>,

    /// Event year. Required for `ifinish`.
    pub year: Option<String>,

    /// Event name. Required for both `ifinish` and `mysamay`.
    pub event_name: Option<String>,

    /// Participant BIB number. Required for both `ifinish` and `mysamay`.
    pub bib_number: Option<String>,
}
