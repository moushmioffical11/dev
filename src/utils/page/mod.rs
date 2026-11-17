use scraper::{Html, Selector};
use url::Url;

use super::text::{extract_legacy_text, extract_visible_text};
use super::url::normalize_url;

pub fn extract_page_data(
    html: &str,
    page_url: &str,
    allowed_host: &str,
) -> (String, String, Vec<String>, Vec<String>) {
    let document = Html::parse_document(html);

    let title_selector = Selector::parse("title").unwrap();

    let title = document
        .select(&title_selector)
        .next()
        .map(|element| element.text().collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
        .trim()
        .to_string();

    // Dynamic content extraction:
    // 1. Try generic visible-text extraction.
    // 2. Fall back to legacy extraction if nothing was found.
    let content = {
        let visible_text = extract_visible_text(&document);

        if visible_text.trim().is_empty() {
            extract_legacy_text(&document)
        } else {
            visible_text
        }
    };

    let base_url = match Url::parse(page_url) {
        Ok(url) => url,
        Err(_) => {
            return (title, content, Vec::new(), Vec::new());
        }
    };

    let link_selector = Selector::parse("a[href]").unwrap();

    let mut links = Vec::new();

    for element in document.select(&link_selector) {
        if let Some(href) = element.value().attr("href") {
            match normalize_url(&base_url, href, allowed_host) {
                Some(url) => {
                    if !links.contains(&url) {
                        links.push(url);
                    }
                }
                None => {}
            }
        }
    }

    let image_selector = Selector::parse("img").unwrap();

    let mut images = Vec::new();

    for element in document.select(&image_selector) {
        let v = element.value();

        let mut candidates: Vec<String> = Vec::new();

        for attr in ["src", "data-src", "data-lazy-src", "data-original"] {
            if let Some(val) = v.attr(attr) {
                candidates.push(val.trim().to_string());
            }
        }

        if let Some(srcset) = v.attr("srcset") {
            if let Some(first) = srcset.split(',').next() {
                if let Some(u) = first.split_whitespace().next() {
                    candidates.push(u.to_string());
                }
            }
        }

        for src in candidates {
            if src.is_empty() || src.starts_with("data:") {
                continue;
            }

            if let Ok(full_url) = base_url.join(&src) {
                let url = full_url.to_string();

                if !images.contains(&url) {
                    images.push(url);
                }
            }
        }
    }

    (title, content, links, images)
}
