use actix_web::{HttpResponse, Responder, get, web};
use serde_json::Value;
use std::thread::sleep;
use std::time::Duration;

use crate::browser::create_browser;
use crate::models::{IfinishEvent, IfinishEventsResponse};
use crate::utils::{escape_js_string, evaluate_string, evaluate_value, wait_for_condition};

const EVENT_RESULT_URL: &str = "https://ifinish.in/eventresult";

// True once the "Recent Events" thumbnails are in the DOM.
const THUMBNAILS_READY_SCRIPT: &str = r###"
(() => {
    return document.querySelectorAll(
        'img[src*="timing_results_assets/thumbnail_"]'
    ).length > 0;
})()
"###;

// Unique thumbnail URLs (without the changing signed query string), in page order.
const THUMBNAILS_SCRIPT: &str = r###"
(() => {
    const seen = new Set();
    const out = [];

    document
        .querySelectorAll('img[src*="timing_results_assets/thumbnail_"]')
        .forEach(img => {
            const base = (img.src || "").split("?")[0];

            if (base && !seen.has(base)) {
                seen.add(base);
                out.push(base);
            }
        });

    return JSON.stringify(out);
})()
"###;

// After clicking a card the site opens /eventresult/result-<Event Name>.
// Returns "" until that URL is open, then {"url": ..., "name": ...}.
const OPENED_EVENT_SCRIPT: &str = r###"
(() => {
    const parts = location.pathname.split("/").filter(Boolean);

    if (parts.length < 2 || parts[0].toLowerCase() !== "eventresult") {
        return "";
    }

    let slug = parts[parts.length - 1];

    try {
        slug = decodeURIComponent(slug);
    } catch (e) {}

    return JSON.stringify({
        url: location.href,
        name: slug.replace(/^result-/i, "").trim()
    });
})()
"###;

fn open_results_page(tab: &headless_chrome::Tab) -> Result<(), String> {
    tab.navigate_to(EVENT_RESULT_URL)
        .map_err(|error| format!("Failed to open iFinish: {}", error))?;

    tab.wait_until_navigated()
        .map_err(|error| format!("Navigation failed: {}", error))?;

    wait_for_condition(tab, THUMBNAILS_READY_SCRIPT, 40, 500)
        .map_err(|error| format!("Recent events were not found: {}", error))
}

fn fetch_recent_events() -> Result<Vec<IfinishEvent>, String> {
    let browser = create_browser()?;

    let tab = browser
        .new_tab()
        .map_err(|error| format!("Failed to create browser tab: {}", error))?;

    println!("Opening iFinish results page...");

    open_results_page(&tab)?;

    let thumbnails_json = evaluate_string(&tab, THUMBNAILS_SCRIPT)?.unwrap_or_default();

    let thumbnails: Vec<String> = serde_json::from_str(&thumbnails_json).unwrap_or_default();

    let total = thumbnails.len();

    println!("Found {} recent event thumbnails.", total);

    let mut events: Vec<IfinishEvent> = Vec::new();

    for (index, base) in thumbnails.iter().enumerate() {
        println!("Resolving recent event {}/{}...", index + 1, total);

        let click_script = format!(
            r###"
            (() => {{
                const base = "{base}";

                const img = Array.from(document.querySelectorAll("img")).find(
                    image => (image.src || "").split("?")[0] === base
                );

                if (!img) {{
                    return false;
                }}

                img.scrollIntoView({{ block: "center" }});

                // Click the closest clickable parent (cards are often not <a> tags)
                let node = img;

                for (let i = 0; i < 8 && node; i++, node = node.parentElement) {{
                    const clickable =
                        node.tagName === "A"
                        || node.tagName === "BUTTON"
                        || node.hasAttribute("onclick")
                        || getComputedStyle(node).cursor === "pointer";

                    if (clickable) {{
                        node.dispatchEvent(
                            new MouseEvent("click", {{
                                bubbles: true,
                                cancelable: true,
                                view: window
                            }})
                        );

                        return true;
                    }}
                }}

                img.click();
                return true;
            }})()
            "###,
            base = escape_js_string(base)
        );

        match evaluate_value(&tab, &click_script) {
            Ok(Some(Value::Bool(true))) => {}
            _ => {
                println!("Thumbnail {} could not be clicked, skipping.", index + 1);
                continue;
            }
        }

        // Wait for the site to open /eventresult/result-<name>
        let mut opened: Option<(String, String)> = None;

        for _ in 0..24 {
            sleep(Duration::from_millis(500));

            if let Ok(Some(info)) = evaluate_string(&tab, OPENED_EVENT_SCRIPT) {
                if info.trim().is_empty() {
                    continue;
                }

                if let Ok(value) = serde_json::from_str::<Value>(&info) {
                    let url = value["url"].as_str().unwrap_or("").to_string();
                    let name = value["name"].as_str().unwrap_or("").to_string();

                    if !name.is_empty() {
                        opened = Some((name, url));
                        break;
                    }
                }
            }
        }

        match opened {
            Some((name, url)) => {
                println!("Recent event {} -> {}", index + 1, name);

                if !events.iter().any(|event| event.name == name) {
                    events.push(IfinishEvent { name, url });
                }
            }
            None => println!("Thumbnail {} did not open an event page.", index + 1),
        }

        // Go back to the list for the next thumbnail
        if index + 1 < total {
            if let Err(error) = open_results_page(&tab) {
                println!("Could not reopen the results page: {}", error);
                break;
            }
        }
    }

    Ok(events)
}

#[get("/ifinish-events")]
pub async fn ifinish_events() -> impl Responder {
    println!("iFinish recent events request");

    let events = match web::block(fetch_recent_events).await {
        Ok(Ok(events)) => events,
        Ok(Err(message)) => {
            return HttpResponse::InternalServerError().body(message);
        }
        Err(error) => {
            return HttpResponse::InternalServerError().body(format!("Worker error: {}", error));
        }
    };

    if events.is_empty() {
        return HttpResponse::NotFound()
            .body("Could not resolve any recent event names from iFinish");
    }

    HttpResponse::Ok().json(IfinishEventsResponse {
        count: events.len(),
        events,
    })
}
