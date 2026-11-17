use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

// --------------------------------------------------
// SEARCH
// --------------------------------------------------

#[derive(Deserialize, IntoParams)]
pub struct SearchParams {
    /// Starting URL, including `https://`.
    pub url: String,

    /// Text to search for. Case-insensitive. Must not be empty.
    pub q: String,

    /// Maximum number of pages to search. Default 5, maximum 20.
    pub max_pages: Option<usize>,

    /// How many link levels to follow from the start page. Default 1, maximum 5.
    pub max_depth: Option<usize>,
}

/// A page that contains the search text.
#[derive(Serialize, ToSchema)]
pub struct SearchResult {
    /// URL of the matching page.
    pub url: String,

    /// Page title, empty if the page has none.
    pub title: String,

    /// About 100 characters before and after the first match, wrapped in `...`.
    pub matched_content: String,
}

/// Result of a site search.
#[derive(Serialize, ToSchema)]
pub struct SearchResponse {
    /// The search text that was sent.
    pub query: String,

    /// Number of pages that were read successfully (not the number of matches).
    pub pages_searched: usize,

    /// Pages that contain the text. Empty if nothing matched.
    pub results: Vec<SearchResult>,
}
