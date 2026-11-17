use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

// --------------------------------------------------
// READ / MAP
// --------------------------------------------------

#[derive(Deserialize, IntoParams)]
pub struct Params {
    /// Full URL of the page, including `https://`.
    pub url: String,
}

/// Rendered content of a web page.
#[derive(Serialize, ToSchema)]
pub struct ReaderResponse {
    /// The URL that was read.
    pub url: String,

    /// Page title (the HTML `<title>`), empty if the page has none.
    pub title: String,

    /// Visible text of the rendered page, one block per line. Scripts and
    /// styles are removed.
    pub content: String,

    /// Absolute links found on the page that point to the same host.
    pub links: Vec<String>,

    /// Absolute image URLs found on the page. Can point to any domain (CDN).
    pub images: Vec<String>,
}

/// Links found on a page.
#[derive(Serialize, ToSchema)]
pub struct MapResponse {
    /// The URL that was mapped.
    pub url: String,

    /// Absolute links found on the page that point to the same host.
    pub links: Vec<String>,
}
