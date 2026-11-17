pub mod browser;
pub mod page;
pub mod search;
pub mod text;
pub mod url;

pub use browser::{escape_js_string, evaluate_string, evaluate_value, wait_for_condition};

pub use page::extract_page_data;

pub use search::create_search_snippet;

pub use url::normalize_host;
