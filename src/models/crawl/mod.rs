use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

// --------------------------------------------------
// CRAWL
// --------------------------------------------------

#[derive(Deserialize, IntoParams)]
pub struct CrawlParams {
    /// Starting URL, including `https://`.
    pub url: String,

    /// Maximum number of pages to read. Default 5, maximum 20.
    pub max_pages: Option<usize>,

    /// How many link levels to follow from the start page. Default 1, maximum 5.
    pub max_depth: Option<usize>,
}

/// One page read during a crawl.
#[derive(Serialize, ToSchema)]
pub struct CrawlPage {
    /// URL of the page.
    pub url: String,

    /// Page title, empty if the page has none.
    pub title: String,

    /// Link depth from the start page. The start page is `0`.
    pub depth: usize,

    /// Visible text of the page.
    pub content: String,

    /// Same-host links found on the page.
    pub links: Vec<String>,
}

/// Result of a crawl.
#[derive(Serialize, ToSchema)]
pub struct CrawlResponse {
    /// The URL the crawl started from.
    pub start_url: String,

    /// Number of pages that were read successfully.
    pub pages_crawled: usize,

    /// The pages, in the order they were read.
    pub pages: Vec<CrawlPage>,
}
