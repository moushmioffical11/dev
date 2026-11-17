use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

// --------------------------------------------------
// RESOLVE
// --------------------------------------------------

#[derive(Deserialize, IntoParams)]
pub struct ResolveParams {
    /// Listing page that shows the event cards.
    pub url: String,

    /// Full or partial event name. Case and punctuation are ignored.
    pub name: String,
}

/// Result of clicking an event card on a listing page.
#[derive(Serialize, ToSchema)]
pub struct ResolveResponse {
    /// The event name that was requested.
    pub name: String,

    /// URL the site opened after clicking the card, or `null` if no card
    /// matched or nothing opened.
    pub url: Option<String>,
}
