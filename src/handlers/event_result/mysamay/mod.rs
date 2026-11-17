use actix_web::HttpResponse;
use serde_json::Value;
use std::thread::sleep;
use std::time::Duration;

use crate::browser::create_browser;
use crate::models::ReaderResponse;
use crate::utils::{escape_js_string, evaluate_string, evaluate_value, wait_for_condition};
use crate::utils::{extract_page_data, normalize_host};

pub fn handle(event_name: &str, bib_number: &str) -> HttpResponse {
    match fetch_result(event_name, bib_number) {
        Ok(result) => HttpResponse::Ok().json(result),
        Err(response) => response,
    }
}

pub fn fetch_result(event_name: &str, bib_number: &str) -> Result<ReaderResponse, HttpResponse> {
    println!(
        "MySamay event result request: event_name={}, bib_number={} ",
        event_name, bib_number
    );

    if event_name.trim().is_empty() || bib_number.trim().is_empty() {
        return Err(
            HttpResponse::BadRequest().body("For mysamay, event_name and bib_number are required")
        );
    }

    let browser = match create_browser() {
        Ok(browser) => browser,
        Err(error) => {
            return Err(HttpResponse::InternalServerError()
                .body(format!("Failed to create browser: {}", error)));
        }
    };

    let home_url = "https://www.mysamay.in/event/results/home";

    let tab = match browser.new_tab() {
        Ok(tab) => tab,
        Err(error) => {
            return Err(HttpResponse::InternalServerError()
                .body(format!("Failed to create MySamay browser tab: {}", error)));
        }
    };

    println!("Opening MySamay results home...");

    if let Err(error) = tab.navigate_to(home_url) {
        return Err(HttpResponse::InternalServerError()
            .body(format!("Failed to open MySamay results home: {}", error)));
    }

    if let Err(error) = tab.wait_until_navigated() {
        return Err(HttpResponse::InternalServerError()
            .body(format!("MySamay navigation failed: {}", error)));
    }

    println!("Opened: {}", home_url);

    /* --------------------------------------------------------
    Initial browser state
    -------------------------------------------------------- */

    println!("Checking initial MySamay browser state...");

    if let Ok(Some(state)) = evaluate_string(
        &tab,
        r###"
        (() => JSON.stringify({
            url: location.href,
            title: document.title,
            readyState: document.readyState,
            bodyExists: !!document.body,
            documentElementExists: !!document.documentElement,
            bodyLength: document.body?.innerHTML.length || 0,
            htmlLength: document.documentElement?.outerHTML.length || 0
        }))()
        "###,
    ) {
        println!("Initial MySamay state: {}", state);
    }

    /* --------------------------------------------------------
    Wait for event list
    -------------------------------------------------------- */

    println!("Waiting for MySamay event list to render...");

    let events_loaded = wait_for_condition(
        &tab,
        r###"
        (() => {
            return document.querySelectorAll(
                ".events-grid .grid-item"
            ).length > 0;
        })()
        "###,
        30,
        500,
    );

    if let Err(error) = events_loaded {
        return Err(HttpResponse::InternalServerError()
            .body(format!("MySamay event list did not load: {}", error)));
    }

    /* --------------------------------------------------------
    Get event count
    -------------------------------------------------------- */

    let event_count = match evaluate_value(
        &tab,
        r###"
        document.querySelectorAll(
            ".events-grid .grid-item"
        ).length
        "###,
    ) {
        Ok(Some(value)) => value.as_i64().unwrap_or(0),
        _ => 0,
    };

    println!("MySamay event count: {}", event_count);

    if event_count == 0 {
        return Err(HttpResponse::NotFound().body("No MySamay result events found"));
    }

    /* --------------------------------------------------------
    Process events one by one
    -------------------------------------------------------- */

    for index in 0..event_count {
        println!("Checking MySamay event {}/{}...", index + 1, event_count);

        /*
         * Every time we return to the home page we locate
         * the event again by index.
         */

        let event_info_script = format!(
            r###"
            (() => {{
                const cards = Array.from(
                    document.querySelectorAll(
                        ".events-grid .grid-item"
                    )
                );

                const card = cards[{index}];

                if (!card) {{
                    return null;
                }}

                const text =
                    (card.innerText || "").trim();

                const name = text
                    .replace(/^\\d{{1,2}}\\s+[A-Za-z]+\\s+\\d{{4}}\\s*/i, "")
                    .split("\\n")
                    .map(x => x.trim())
                    .filter(Boolean)
                    .find(
                        x =>
                            x !== "RESULTS" &&
                            x !== "PHOTOS"
                    ) || "";

                const resultButton =
                    Array.from(
                        card.querySelectorAll("button")
                    ).find(
                        button =>
                            (button.innerText || "")
                                .trim()
                                .toUpperCase() === "RESULTS"
                    );

                // Optional event filter: compare names ignoring case/punctuation
                const norm = value =>
                    (value || "").toLowerCase().replace(/[^a-z0-9]+/g, "");

                const wanted = norm("{event_name}");

                const matchesEvent =
                    norm(text).includes(wanted);

                return JSON.stringify({{
                    name,
                    hasResultsButton: !!resultButton,
                    matchesEvent
                }});
            }})()
            "###,
            index = index,
            event_name = escape_js_string(event_name)
        );

        let event_info = match evaluate_string(&tab, &event_info_script) {
            Ok(Some(value)) => value,
            _ => {
                println!("Could not inspect MySamay event {}", index + 1);
                continue;
            }
        };

        println!("MySamay event {} info: {}", index + 1, event_info);

        // When event_name is given, only open the matching event.
        if !event_info.contains("\"matchesEvent\":true") {
            println!(
                "Skipping event {}: name does not match '{}'.",
                index + 1,
                event_name
            );
            continue;
        }

        let has_results_button = event_info.contains("\"hasResultsButton\":true");

        if !has_results_button {
            println!(
                "Skipping event {} because it has no RESULTS button.",
                index + 1
            );
            continue;
        }

        /* ----------------------------------------------------
        Click RESULTS
        ---------------------------------------------------- */

        println!("Opening RESULTS for MySamay event {}...", index + 1);

        let click_script = format!(
            r###"
            (() => {{
                const cards = Array.from(
                    document.querySelectorAll(
                        ".events-grid .grid-item"
                    )
                );

                const card = cards[{index}];

                if (!card) {{
                    return false;
                }}

                const button =
                    Array.from(
                        card.querySelectorAll("button")
                    ).find(
                        button =>
                            (button.innerText || "")
                                .trim()
                                .toUpperCase() === "RESULTS"
                    );

                if (!button) {{
                    return false;
                }}

                button.scrollIntoView({{
                    block: "center"
                }});

                button.click();

                return true;
            }})()
            "###,
            index = index
        );

        match evaluate_value(&tab, &click_script) {
            Ok(Some(Value::Bool(true))) => {}
            _ => {
                println!("Failed to click RESULTS for event {}", index + 1);
                continue;
            }
        }

        /* ----------------------------------------------------
        Wait for route change / participant page
        ---------------------------------------------------- */

        let mut result_page_loaded = false;

        for attempt in 1..=30 {
            if let Ok(Some(state)) = evaluate_string(
                &tab,
                r###"
                (() => JSON.stringify({
                    url: location.href,
                    bodyText: (document.body?.innerText || "").substring(0, 3000),
                    hasParticipantInput:
                        !!document.querySelector(
                            'input[placeholder*="BIB"]'
                        ),
                    hasFindParticipant:
                        (document.body?.innerText || "")
                            .includes("Find Participant")
                }))()
                "###,
            ) {
                println!("MySamay result page check {}: {}", attempt, state);

                if state.contains("\"hasParticipantInput\":true")
                    || state.contains("\"hasFindParticipant\":true")
                {
                    result_page_loaded = true;
                    break;
                }
            }

            sleep(Duration::from_millis(500));
        }

        if !result_page_loaded {
            println!(
                "MySamay event {} did not open a participant result page.",
                index + 1
            );

            /*
             * Make sure we are back on the home page before
             * trying the next event.
             */
            let _ = tab.navigate_to(home_url);
            let _ = tab.wait_until_navigated();

            sleep(Duration::from_millis(800));

            let _ = wait_for_condition(
                &tab,
                r###"
                (() => {
                    return document.querySelectorAll(
                        ".events-grid .grid-item"
                    ).length > 0;
                })()
                "###,
                20,
                500,
            );

            continue;
        }

        println!("MySamay participant result page loaded.");

        println!(
            "Current MySamay URL: {}",
            evaluate_string(&tab, "location.href",)
                .ok()
                .flatten()
                .unwrap_or_default()
        );

        /* ----------------------------------------------------
        Find Participant input
        ---------------------------------------------------- */

        let input_script = r###"
        (() => {
            const inputs =
                Array.from(document.querySelectorAll("input"));

            const input = inputs.find(input => {
                const placeholder =
                    (input.getAttribute("placeholder") || "")
                        .toLowerCase();

                return placeholder.includes("bib")
                    || placeholder.includes("full name");
            });

            if (!input) {
                return false;
            }

            input.focus();

            return true;
        })()
        "###;

        let has_input = matches!(
            evaluate_value(&tab, input_script),
            Ok(Some(Value::Bool(true)))
        );

        if !has_input {
            println!("Participant input not found for event {}.", index + 1);

            let _ = tab.navigate_to(home_url);
            let _ = tab.wait_until_navigated();

            sleep(Duration::from_millis(800));

            let _ = wait_for_condition(
                &tab,
                r###"
                (() => {
                    return document.querySelectorAll(
                        ".events-grid .grid-item"
                    ).length > 0;
                })()
                "###,
                20,
                500,
            );

            continue;
        }

        /* ----------------------------------------------------
        Fill BIB
        ---------------------------------------------------- */

        println!("Searching MySamay BIB: {}", bib_number);

        let fill_script = format!(
            r###"
            (() => {{
                const inputs =
                    Array.from(
                        document.querySelectorAll("input")
                    );

                const input = inputs.find(input => {{
                    const placeholder =
                        (
                            input.getAttribute("placeholder")
                            || ""
                        ).toLowerCase();

                    return placeholder.includes("bib")
                        || placeholder.includes("full name");
                }});

                if (!input) {{
                    return false;
                }}

                input.focus();

                const setter =
                    Object.getOwnPropertyDescriptor(
                        HTMLInputElement.prototype,
                        "value"
                    ).set;

                setter.call(
                    input,
                    "{bib}"
                );

                input.dispatchEvent(
                    new Event(
                        "input",
                        {{ bubbles: true }}
                    )
                );

                input.dispatchEvent(
                    new Event(
                        "change",
                        {{ bubbles: true }}
                    )
                );

                return true;
            }})()
            "###,
            bib = escape_js_string(bib_number)
        );

        match evaluate_value(&tab, &fill_script) {
            Ok(Some(Value::Bool(true))) => {}
            _ => {
                println!("Could not fill BIB on event {}.", index + 1);

                let _ = tab.navigate_to(home_url);
                let _ = tab.wait_until_navigated();

                sleep(Duration::from_millis(800));

                let _ = wait_for_condition(
                    &tab,
                    r###"
                    (() => {
                        return document.querySelectorAll(
                            ".events-grid .grid-item"
                        ).length > 0;
                    })()
                    "###,
                    20,
                    500,
                );

                continue;
            }
        }

        /* ----------------------------------------------------
        Wait for participant filtering/result
        ---------------------------------------------------- */

        // Checks (up to max_attempts) whether the exact BIB is shown on the page.
        let check_bib = |max_attempts: u32| -> bool {
            let mut empty_streak = 0;

            for attempt in 1..=max_attempts {
                let script = format!(
                    r###"
                    (() => {{
                        const body =
                            document.body?.innerText || "";

                        const exactBib = Array.from(
                            document.querySelectorAll("*")
                        ).some(element =>
                            (element.children.length === 0)
                            && (
                                (element.innerText || "")
                                    .trim() === "{bib}"
                            )
                        );

                        // "No participants / results found" style empty state
                        const emptyState =
                            /no\s+(participants?|results?|records?|data|athletes?|runners?)|not\s+found|no\s+matching/i
                                .test(body);

                        return JSON.stringify({{
                            exactBib,
                            emptyState,
                            bodyLength: body.length,
                            bodyHasBib:
                                body.includes("{bib}")
                        }});
                    }})()
                    "###,
                    bib = escape_js_string(bib_number)
                );

                if let Ok(Some(value)) = evaluate_string(&tab, &script) {
                    println!("MySamay BIB search check {}: {}", attempt, value);

                    if value.contains("\"exactBib\":true") {
                        return true;
                    }

                    // The page says "no results" and our BIB is not shown. Require a few
                    // consecutive checks so a brief flash before data loads is ignored.
                    if value.contains("\"emptyState\":true") {
                        empty_streak += 1;

                        if empty_streak >= 4 {
                            println!("MySamay event {} shows no matching participant.", index + 1);
                            return false;
                        }
                    } else {
                        empty_streak = 0;
                    }
                }

                sleep(Duration::from_millis(500));
            }

            false
        };

        // 1) Script-filled value (works when the page listens to the input event)
        let mut bib_found = check_bib(8);

        // 2) The page did not react: the box shows the BIB but the list is not
        //    filtered. Type it with real keyboard events and press Enter instead.
        if !bib_found {
            println!("MySamay: filter did not react to script input; retrying with real typing...");

            let clear_script = r###"
            (() => {
                const input = Array.from(document.querySelectorAll("input")).find(input => {
                    const placeholder =
                        (input.getAttribute("placeholder") || "").toLowerCase();

                    return placeholder.includes("bib")
                        || placeholder.includes("full name");
                });

                if (!input) {
                    return false;
                }

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

            sleep(Duration::from_millis(400));

            match tab
                .find_elements(r#"input[placeholder*="bib" i], input[placeholder*="full name" i]"#)
            {
                Ok(elements) if !elements.is_empty() => {
                    let _ = elements[0].click();
                    let _ = tab.type_str(bib_number);
                    let _ = tab.press_key("Enter");
                }
                _ => println!("MySamay: participant input not found for real typing."),
            }

            bib_found = check_bib(20);
        }

        if !bib_found {
            println!(
                "BIB {} not found in MySamay event {}.",
                bib_number,
                index + 1
            );

            if let Ok(Some(snippet)) = evaluate_string(
                &tab,
                r###"(document.body?.innerText || "").substring(0, 300)"###,
            ) {
                println!("MySamay event {} page text: {}", index + 1, snippet);
            }

            let _ = tab.navigate_to(home_url);
            let _ = tab.wait_until_navigated();

            sleep(Duration::from_millis(800));

            let _ = wait_for_condition(
                &tab,
                r###"
                (() => {
                    return document.querySelectorAll(
                        ".events-grid .grid-item"
                    ).length > 0;
                })()
                "###,
                20,
                500,
            );

            continue;
        }

        println!("BIB {} found in MySamay event {}.", bib_number, index + 1);

        /* ----------------------------------------------------
        Click exact BIB if clickable
        ---------------------------------------------------- */

        let bib_click_script = format!(
            r###"
            (() => {{
                const wanted = "{bib}";

                const elements =
                    Array.from(
                        document.querySelectorAll("*")
                    );

                const match = elements.find(element => {{
                    const text =
                        (element.innerText || "").trim();

                    if (text !== wanted) {{
                        return false;
                    }}

                    const rect =
                        element.getBoundingClientRect();

                    return rect.width > 0
                        && rect.height > 0;
                }});

                if (!match) {{
                    return false;
                }}

                match.scrollIntoView({{
                    block: "center"
                }});

                match.click();

                return true;
            }})()
            "###,
            bib = escape_js_string(bib_number)
        );

        let _ = evaluate_value(&tab, &bib_click_script);

        sleep(Duration::from_millis(1000));

        /* ----------------------------------------------------
        Extract final rendered HTML
        ---------------------------------------------------- */

        let html = match evaluate_string(&tab, "document.documentElement.outerHTML") {
            Ok(Some(html)) => html,
            _ => {
                println!("Failed to extract MySamay HTML.");

                continue;
            }
        };

        println!("MySamay final rendered HTML length: {}", html.len());

        println!("Extracting MySamay event result page...");

        let current_url = evaluate_string(&tab, "location.href")
            .ok()
            .flatten()
            .unwrap_or_else(|| home_url.to_string());

        let base_url = match url::Url::parse(&current_url) {
            Ok(url) => url,
            Err(error) => {
                return Err(HttpResponse::InternalServerError()
                    .body(format!("Failed to parse MySamay URL: {}", error)));
            }
        };

        let allowed_host = normalize_host(base_url.host_str().unwrap_or("mysamay.in"));

        let (title, content, links, images) = extract_page_data(&html, &current_url, &allowed_host);

        println!(
            "Extraction completed. Title: '{}', Content: {} chars, Links: {}, Images: {}",
            title,
            content.len(),
            links.len(),
            images.len()
        );

        return Ok(ReaderResponse {
            url: current_url,
            title,
            content,
            links,
            images,
        });
    }

    Err(HttpResponse::NotFound().body(format!(
        "BIB {} was not found in MySamay event '{}' (or the event has no results)",
        bib_number, event_name
    )))
}
