use actix_web::HttpResponse;

use serde_json::Value;
use std::thread::sleep;
use std::time::Duration;

use crate::browser::create_browser;
use crate::models::ReaderResponse;
use crate::utils::{escape_js_string, evaluate_string, evaluate_value, wait_for_condition};
use crate::utils::{extract_page_data, normalize_host};

// --------------------------------------------------
// EXISTING EVENT RESULT API
// --------------------------------------------------

pub fn handle(year: &str, event_name: &str, bib_number: &str) -> HttpResponse {
    match fetch_result(year, event_name, bib_number) {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(response) => response,
    }
}

// --------------------------------------------------
// FETCH IFINISH RESULT
// --------------------------------------------------

pub fn fetch_result(
    year: &str,
    event_name: &str,
    bib_number: &str,
) -> Result<ReaderResponse, HttpResponse> {
    println!(
        "Event result request: source=ifinish, year={}, event_name={}, bib_number={}",
        year, event_name, bib_number
    );

    // --------------------------------------------------
    // Validate parameters
    // --------------------------------------------------

    if year.trim().is_empty() || event_name.trim().is_empty() || bib_number.trim().is_empty() {
        return Err(HttpResponse::BadRequest()
            .body("For ifinish, year, event_name and bib_number are required"));
    }

    // --------------------------------------------------
    // Create browser
    // --------------------------------------------------

    let browser = match create_browser() {
        Ok(browser) => browser,
        Err(error) => {
            return Err(HttpResponse::InternalServerError()
                .body(format!("Failed to create browser: {}", error)));
        }
    };

    // --------------------------------------------------
    // Create tab
    // --------------------------------------------------

    let tab = match browser.new_tab() {
        Ok(tab) => tab,
        Err(error) => {
            return Err(HttpResponse::InternalServerError()
                .body(format!("Failed to create browser tab: {}", error)));
        }
    };

    println!("Opening iFinish event result page...");

    // --------------------------------------------------
    // Open iFinish
    // --------------------------------------------------

    if let Err(error) = tab.navigate_to("https://ifinish.in/eventresult") {
        return Err(HttpResponse::InternalServerError().body(format!(
            "Failed to open iFinish event result page: {}",
            error
        )));
    }

    if let Err(error) = tab.wait_until_navigated() {
        return Err(
            HttpResponse::InternalServerError().body(format!("Navigation failed: {}", error))
        );
    }

    println!("Opened: https://ifinish.in/eventresult");

    // --------------------------------------------------------
    // Wait for year selector
    // --------------------------------------------------------

    println!("Waiting for year selector...");

    if let Err(error) = wait_for_condition(
        &tab,
        r###"
        (() => {
            return !!document.querySelector("#year");
        })()
        "###,
        30,
        500,
    ) {
        return Err(HttpResponse::InternalServerError()
            .body(format!("Year selector was not found: {}", error)));
    }

    println!("Year selector found.");

    // --------------------------------------------------------
    // Wait for event inputs
    // --------------------------------------------------------

    println!("Waiting for event inputs...");

    if let Err(error) = wait_for_condition(
        &tab,
        r###"
        (() => {
            return document.querySelectorAll("input.rbt-input-main").length >= 2;
        })()
        "###,
        30,
        500,
    ) {
        return Err(HttpResponse::InternalServerError()
            .body(format!("Event inputs were not found: {}", error)));
    }

    let input_count = match evaluate_value(
        &tab,
        r###"
        document.querySelectorAll("input.rbt-input-main").length
        "###,
    ) {
        Ok(Some(value)) => value.as_i64().unwrap_or(0),
        _ => 0,
    };

    println!("Event input count: {}", input_count);

    // --------------------------------------------------------
    // Select year
    // --------------------------------------------------------

    println!("Selecting year: {}", year);

    let year_script = format!(
        r###"
        (() => {{
            const select = document.querySelector("#year");

            if (!select) {{
                return false;
            }}

            const option = Array.from(select.options).find(
                option => option.value === "{year}"
                    || option.textContent.trim() === "{year}"
            );

            if (!option) {{
                return false;
            }}

            select.value = option.value;

            select.dispatchEvent(
                new Event("change", {{ bubbles: true }})
            );

            return true;
        }})()
        "###,
        year = escape_js_string(year)
    );

    match evaluate_value(&tab, &year_script) {
        Ok(Some(Value::Bool(true))) => {}

        Ok(_) => {
            return Err(
                HttpResponse::BadRequest().body(format!("Year {} was not found on iFinish", year))
            );
        }

        Err(error) => {
            return Err(HttpResponse::InternalServerError()
                .body(format!("Failed to select year: {}", error)));
        }
    }

    sleep(Duration::from_millis(1000));

    println!("Year selection completed.");

    // --------------------------------------------------------
    // Fill event name
    // --------------------------------------------------------

    println!("Filling event name: {}", event_name);

    let event_script = format!(
        r###"
        (() => {{
            const inputs =
                document.querySelectorAll("input.rbt-input-main");

            if (inputs.length < 2) {{
                return false;
            }}

            const input = inputs[0];

            input.focus();

            const setter =
                Object.getOwnPropertyDescriptor(
                    HTMLInputElement.prototype,
                    "value"
                ).set;

            setter.call(input, "{event_name}");

            input.dispatchEvent(
                new Event("input", {{ bubbles: true }})
            );

            input.dispatchEvent(
                new Event("change", {{ bubbles: true }})
            );

            return true;
        }})()
        "###,
        event_name = escape_js_string(event_name)
    );

    match evaluate_value(&tab, &event_script) {
        Ok(Some(Value::Bool(true))) => {}

        Ok(_) => {
            return Err(
                HttpResponse::InternalServerError().body("Failed to fill iFinish event name")
            );
        }

        Err(error) => {
            return Err(HttpResponse::InternalServerError()
                .body(format!("Failed to fill event name: {}", error)));
        }
    }

    println!("Event name typed.");

    // --------------------------------------------------------
    // Select autocomplete event
    // --------------------------------------------------------

    println!("Waiting for event autocomplete...");

    let mut selected_event = false;

    for attempt in 1..=30 {
        println!("Checking autocomplete options: attempt {}", attempt);

        let script = format!(
            r###"
            (() => {{
                const wanted =
                    "{event_name}".trim().toLowerCase();

                const options = Array.from(
                    document.querySelectorAll('[role="option"]')
                );

                const match = options.find(option =>
                    (option.innerText || "")
                        .trim()
                        .toLowerCase() === wanted
                );

                if (!match) {{
                    return false;
                }}

                match.click();
                return true;
            }})()
            "###,
            event_name = escape_js_string(event_name)
        );

        match evaluate_value(&tab, &script) {
            Ok(Some(Value::Bool(true))) => {
                selected_event = true;
                break;
            }

            _ => {
                sleep(Duration::from_millis(500));
            }
        }
    }

    if !selected_event {
        return Err(HttpResponse::BadRequest().body(format!(
            "Event '{}' was not found in iFinish autocomplete",
            event_name
        )));
    }

    println!("Matching event selected successfully.");
    println!("Event selected.");

    // --------------------------------------------------------
    // Fill BIB
    // --------------------------------------------------------

    println!("Filling BIB: {}", bib_number);

    let bib_script = format!(
        r###"
        (() => {{
            const inputs =
                document.querySelectorAll("input.rbt-input-main");

            if (inputs.length < 2) {{
                return false;
            }}

            const input = inputs[1];

            input.focus();

            const setter =
                Object.getOwnPropertyDescriptor(
                    HTMLInputElement.prototype,
                    "value"
                ).set;

            setter.call(input, "{bib}");

            input.dispatchEvent(
                new Event("input", {{ bubbles: true }})
            );

            input.dispatchEvent(
                new Event("change", {{ bubbles: true }})
            );

            return true;
        }})()
        "###,
        bib = escape_js_string(bib_number)
    );

    match evaluate_value(&tab, &bib_script) {
        Ok(Some(Value::Bool(true))) => {}

        Ok(_) => {
            return Err(HttpResponse::InternalServerError().body("Failed to fill iFinish BIB"));
        }

        Err(error) => {
            return Err(
                HttpResponse::InternalServerError().body(format!("Failed to fill BIB: {}", error))
            );
        }
    }

    println!("BIB typed.");

    // --------------------------------------------------------
    // Select BIB from autocomplete dropdown
    // --------------------------------------------------------

    println!("Waiting for BIB autocomplete...");

    let bib_option_script = format!(
        r###"
        (() => {{
            const wanted = "{bib}".trim();
            const placeholder = /no matches|searching|loading/i;

            const options = Array.from(
                document.querySelectorAll('[role="option"]')
            ).filter(option => {{
                const r = option.getBoundingClientRect();
                return r.width > 0 && r.height > 0;
            }});

            if (options.length === 0) {{
                return "none";
            }}

            // Ignore placeholder rows such as "No matches found." / "Searching..."
            const real = options.filter(option =>
                !placeholder.test((option.innerText || "").trim())
            );

            if (real.length === 0) {{
                return "wait:" + options
                    .map(option => (option.innerText || "").trim())
                    .join(" | ");
            }}

            // Prefer the option containing the exact BIB as a whole token
            let match = real.find(option =>
                (option.innerText || "")
                    .split(/\D+/)
                    .includes(wanted)
            );

            // If there is only one real suggestion, use it
            if (!match && real.length === 1) {{
                match = real[0];
            }}

            if (!match) {{
                return "nomatch:" + real
                    .map(option => (option.innerText || "").trim())
                    .join(" | ");
            }}

            match.click();
            return "clicked:" + (match.innerText || "").trim();
        }})()
        "###,
        bib = escape_js_string(bib_number)
    );

    let poll_bib_option = |max_attempts: u32| -> bool {
        // Consecutive checks where the dropdown only shows "No matches found"
        // (or suggestions that don't contain our BIB). Real suggestions arrive
        // within ~1-2s, so a long streak means the BIB does not exist.
        let mut no_match_streak = 0;

        for attempt in 1..=max_attempts {
            if let Ok(Some(result)) = evaluate_string(&tab, &bib_option_script) {
                println!("BIB autocomplete check {}: {}", attempt, result);

                if result.starts_with("clicked:") {
                    return true;
                }

                if result.starts_with("wait:") || result.starts_with("nomatch:") {
                    no_match_streak += 1;

                    if no_match_streak >= 10 {
                        println!("Autocomplete reports no matching BIB; stopping search.");
                        return false;
                    }
                } else {
                    no_match_streak = 0;
                }
            }

            sleep(Duration::from_millis(500));
        }

        false
    };

    // Suggestions are loaded asynchronously, so give them time to arrive.
    let mut bib_option_selected = poll_bib_option(24);

    if !bib_option_selected {
        println!("No BIB suggestion yet; retrying with real keyboard typing...");

        let clear_script = r###"
        (() => {
            const inputs = document.querySelectorAll("input.rbt-input-main");

            if (inputs.length < 2) {
                return false;
            }

            const input = inputs[1];

            input.focus();

            const setter = Object.getOwnPropertyDescriptor(
                HTMLInputElement.prototype,
                "value"
            ).set;

            setter.call(input, "");

            input.dispatchEvent(new Event("input", { bubbles: true }));

            return true;
        })()
        "###;

        let _ = evaluate_value(&tab, clear_script);

        sleep(Duration::from_millis(500));

        match tab.find_elements("input.rbt-input-main") {
            Ok(elements) if elements.len() >= 2 => {
                let _ = elements[1].click();
                let _ = tab.type_str(bib_number);
            }

            _ => println!("Could not find BIB input for real typing."),
        }

        bib_option_selected = poll_bib_option(24);
    }

    if bib_option_selected {
        println!("BIB suggestion selected.");
        sleep(Duration::from_millis(500));
    } else {
        println!("No BIB suggestion found for {}; stopping.", bib_number);

        return Err(HttpResponse::NotFound().body(format!(
            "BIB {} was not found for event '{}' on iFinish",
            bib_number, event_name
        )));
    }

    // --------------------------------------------------------
    // Click Search
    // --------------------------------------------------------

    println!("Clicking Search...");

    let search_script = r###"
    (() => {
        const buttons = Array.from(
            document.querySelectorAll("button[type='submit']")
        );

        const button = buttons.find(
            button =>
                (button.innerText || "")
                    .trim()
                    .toLowerCase() === "search"
        );

        if (!button) {
            return false;
        }

        button.click();
        return true;
    })()
    "###;

    match evaluate_value(&tab, search_script) {
        Ok(Some(Value::Bool(true))) => {}

        Ok(_) => {
            return Err(
                HttpResponse::InternalServerError().body("Search button was not found on iFinish")
            );
        }

        Err(error) => {
            return Err(HttpResponse::InternalServerError()
                .body(format!("Failed to click Search: {}", error)));
        }
    }

    println!("Search clicked.");

    // --------------------------------------------------------
    // Wait for summary results
    // --------------------------------------------------------

    println!("Waiting for search results...");

    let mut summary_loaded = false;

    for attempt in 1..=30 {
        let script = format!(
            r###"
            (() => {{
                const body = document.body?.innerText || "";
                const upper = body.toUpperCase();

                return JSON.stringify({{
                    bodyLength: body.length,
                    hasBib: body.includes("{bib}"),
                    hasChipTime: upper.includes("CHIP TIME"),
                    hasRank: upper.includes("RANK"),
                    hasGenderTabs: upper.includes("FEMALE") && upper.includes("MALE")
                }});
            }})()
            "###,
            bib = escape_js_string(bib_number)
        );

        if let Ok(Some(value)) = evaluate_string(&tab, &script) {
            println!("Search result check {}: {}", attempt, value);

            // The BIB text must be visible in the results before we continue.
            // (CHIP TIME / RANK / Male / Female text can exist before results load.)
            if value.contains("\"hasBib\":true") {
                summary_loaded = true;
                break;
            }
        }

        sleep(Duration::from_millis(500));
    }

    if !summary_loaded {
        println!(
            "BIB {} did not appear in the search results; stopping.",
            bib_number
        );

        return Err(HttpResponse::NotFound().body(format!(
            "No iFinish search results found for BIB {}",
            bib_number
        )));
    }

    println!("Search results loaded.");

    // --------------------------------------------------------
    // Click exact BIB (current tab -> Male -> Female)
    // --------------------------------------------------------

    println!("Clicking BIB result...");

    let bib_click_script = format!(
        r###"
        (() => {{
            const wanted = "{bib}";

            // 1) Normal results table
            let match = Array.from(
                document.querySelectorAll("td span.cursor-pointer")
            ).find(el => (el.innerText || "").trim() === wanted);

            // 2) Fallback: any visible leaf element with the exact BIB text
            if (!match) {{
                match = Array.from(document.querySelectorAll("body *")).find(el => {{
                    if (el.children.length !== 0) return false;
                    if (["INPUT", "SCRIPT", "STYLE"].includes(el.tagName)) return false;
                    if ((el.innerText || "").trim() !== wanted) return false;

                    const r = el.getBoundingClientRect();

                    return r.width > 0 && r.height > 0;
                }});
            }}

            if (!match) {{
                return false;
            }}

            match.scrollIntoView({{ block: "center" }});
            match.click();

            return true;
        }})()
        "###,
        bib = escape_js_string(bib_number)
    );

    // None = whatever tab is currently open
    let gender_tabs: [Option<&str>; 3] = [None, Some("Male"), Some("Female")];

    let mut bib_clicked = false;

    'gender_loop: for gender in gender_tabs.iter() {
        if let Some(gender_name) = gender {
            println!("BIB not found yet, switching to {} tab...", gender_name);

            let gender_script = format!(
                r###"
                (() => {{
                    const wanted = "{gender}".toLowerCase();

                    const tab = Array.from(
                        document.querySelectorAll("button, a, label, div, span")
                    ).find(el => {{
                        const text = (el.innerText || "").trim().toLowerCase();

                        if (text !== wanted) return false;

                        const r = el.getBoundingClientRect();

                        return r.width > 0 && r.height > 0;
                    }});

                    if (!tab) {{
                        return false;
                    }}

                    tab.scrollIntoView({{ block: "center" }});
                    tab.click();

                    return true;
                }})()
                "###,
                gender = gender_name
            );

            match evaluate_value(&tab, &gender_script) {
                Ok(Some(Value::Bool(true))) => {}

                _ => {
                    println!("{} tab not found, skipping.", gender_name);
                    continue;
                }
            }

            // let the list re-render after switching tab
            sleep(Duration::from_millis(1500));
        }

        // Current tab gets more time (results may still be rendering);
        // gender tabs only need a short wait after the switch.
        let max_attempts = if gender.is_none() { 20 } else { 6 };

        for attempt in 1..=max_attempts {
            match evaluate_value(&tab, &bib_click_script) {
                Ok(Some(Value::Bool(true))) => {
                    println!(
                        "BIB clicked (tab: {}, attempt {}).",
                        gender.unwrap_or("current"),
                        attempt
                    );

                    bib_clicked = true;
                    break 'gender_loop;
                }

                Ok(_) => sleep(Duration::from_millis(500)),

                Err(error) => {
                    return Err(HttpResponse::InternalServerError()
                        .body(format!("Failed to click BIB result: {}", error)));
                }
            }
        }
    }

    if !bib_clicked {
        if let Ok(Some(snippet)) = evaluate_string(
            &tab,
            r###"(document.body?.innerText || "").substring(0, 800)"###,
        ) {
            println!("Page text when BIB was not found: {}", snippet);
        }

        return Err(HttpResponse::NotFound().body(format!(
            "BIB {} was not found in iFinish results (Male/Female)",
            bib_number
        )));
    }

    println!("BIB click completed.");

    // --------------------------------------------------------
    // Wait for detailed participant result
    // (event-independent: works for any iFinish layout)
    // --------------------------------------------------------

    println!("Waiting for detailed participant result...");

    let detail_script = format!(
        r###"
        (() => {{
            const body = document.body?.innerText || "";
            const upper = body.toUpperCase();

            // Detail card shows "BIB# <number>" (results table only shows "BIB")
            const hasBibLabel = /BIB\s*#/i.test(body);

            // Generic markers that exist on the detail view of different event types
            const hasMarkers =
                upper.includes("SPLITS")
                || upper.includes("OVERALL RANK")
                || upper.includes("GENDER RANK")
                || upper.includes("CATEGORY RANK")
                || upper.includes("AGE CATEGORY");

            return JSON.stringify({{
                hasBib: body.includes("{bib}"),
                hasBibLabel,
                hasMarkers,
                textLength: body.length
            }});
        }})()
        "###,
        bib = escape_js_string(bib_number)
    );

    let mut detail_loaded = false;

    for attempt in 1..=30 {
        if let Ok(Some(value)) = evaluate_string(&tab, &detail_script) {
            println!("Detailed result check {}: {}", attempt, value);

            if value.contains("\"hasBib\":true")
                && (value.contains("\"hasBibLabel\":true") || value.contains("\"hasMarkers\":true"))
            {
                detail_loaded = true;
                break;
            }
        }

        sleep(Duration::from_millis(500));
    }

    if !detail_loaded {
        // Last resort: if the BIB is on the page, still return what is rendered
        // instead of failing, so unusual layouts keep working.
        let bib_on_page = matches!(
            evaluate_string(&tab, &detail_script),
            Ok(Some(ref value)) if value.contains("\"hasBib\":true")
        );

        if bib_on_page {
            println!("Detail markers not found, but BIB is on the page. Extracting anyway.");
        } else {
            return Err(HttpResponse::NotFound().body(format!(
                "Detailed iFinish result was not loaded for BIB {}",
                bib_number
            )));
        }
    } else {
        println!("Detailed participant result loaded.");
    }

    // --------------------------------------------------------
    // Extract rendered page
    // --------------------------------------------------------

    let html = match evaluate_string(&tab, "document.documentElement.outerHTML") {
        Ok(Some(html)) => html,

        Ok(None) => {
            return Err(
                HttpResponse::InternalServerError().body("Failed to get rendered iFinish HTML")
            );
        }

        Err(error) => {
            return Err(HttpResponse::InternalServerError()
                .body(format!("Failed to extract iFinish HTML: {}", error)));
        }
    };

    println!("Final rendered HTML length: {}", html.len());

    println!("Extracting event result page...");

    let base_url = match url::Url::parse("https://ifinish.in/eventresult") {
        Ok(url) => url,

        Err(error) => {
            return Err(HttpResponse::InternalServerError()
                .body(format!("Failed to parse iFinish URL: {}", error)));
        }
    };

    let allowed_host = normalize_host(base_url.host_str().unwrap_or("ifinish.in"));

    let (title, content, links, images) =
        extract_page_data(&html, "https://ifinish.in/eventresult", &allowed_host);

    println!(
        "Extraction completed. Title: '{}', Content: {} chars, Links: {}, Images: {}",
        title,
        content.len(),
        links.len(),
        images.len()
    );

    // --------------------------------------------------------
    // Return extracted result
    // --------------------------------------------------------

    Ok(ReaderResponse {
        url: "https://ifinish.in/eventresult".to_string(),
        title,
        content,
        links,
        images,
    })
}
