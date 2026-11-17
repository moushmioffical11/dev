use serde::Serialize;
use utoipa::ToSchema;

// --------------------------------------------------
// IFINISH EVENTS
// --------------------------------------------------

/// One recent iFinish event.
#[derive(Serialize, ToSchema)]
pub struct IfinishEvent {
    /// Full event name exactly as iFinish uses it. Use it unchanged as
    /// `event_name` in `/event-result`.
    pub name: String,

    /// iFinish results page of this event.
    pub url: String,
}

/// Recent iFinish events with their full names.
#[derive(Serialize, ToSchema)]
pub struct IfinishEventsResponse {
    /// Number of events returned.
    pub count: usize,

    /// The events, in the order shown on iFinish.
    pub events: Vec<IfinishEvent>,
}
