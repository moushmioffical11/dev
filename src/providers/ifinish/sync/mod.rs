use actix_web::{HttpResponse, Responder, get, web};
use chrono::NaiveDate;
use reqwest::Client;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::env;

use crate::models::{EventRow, SyncSummary};
use crate::repositories::event::save_events;

const IFINISH_LIST_URL: &str = "https://api2.ifinish.in/api/userEventList?userId=";

const IFINISH_RACE_URL: &str = "https://api2.ifinish.in/api/userEventRaceList";

const IFINISH_SEARCH_EVENT_URL: &str = "https://api2.ifinish.in/api/searchTimingEvent";

fn get_ifinish_user_id() -> String {
    env::var("IFINISH_USER_ID").unwrap_or_else(|_| "0".to_string())
}

fn get_string(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(value) = value.get(*key) {
            if let Some(text) = value.as_str() {
                let text = text.trim();

                if !text.is_empty() {
                    return Some(text.to_string());
                }
            } else if !value.is_null() {
                let text = value.to_string();

                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
    }

    None
}

fn get_array(value: &Value, key: &str) -> Value {
    value
        .get(key)
        .filter(|v| v.is_array())
        .cloned()
        .unwrap_or_else(|| json!([]))
}

fn parse_event_date(value: &Value, keys: &[&str]) -> Option<NaiveDate> {
    let date_string = get_string(value, keys)?;

    // Example:
    // 2026-10-25
    if let Ok(date) = NaiveDate::parse_from_str(&date_string, "%Y-%m-%d") {
        return Some(date);
    }

    // Example:
    // 2026-10-25 00:00:00
    if let Ok(date_time) = chrono::NaiveDateTime::parse_from_str(&date_string, "%Y-%m-%d %H:%M:%S")
    {
        return Some(date_time.date());
    }

    None
}

fn extract_event_list(response: &Value) -> Vec<Value> {
    // Most likely:
    // {
    //   "code": "200",
    //   "status": true,
    //   "data": [...]
    // }

    if let Some(data) = response.get("data") {
        if let Some(events) = data.as_array() {
            return events.clone();
        }

        // Some APIs wrap the array again.
        if let Some(events) = data.get("data").and_then(Value::as_array) {
            return events.clone();
        }

        if let Some(events) = data.get("events").and_then(Value::as_array) {
            return events.clone();
        }
    }

    // Fallback if API directly returns an array.
    if let Some(events) = response.as_array() {
        return events.clone();
    }

    Vec::new()
}

fn extract_race_data(response: &Value) -> Value {
    response.get("data").cloned().unwrap_or_else(|| json!({}))
}

fn build_images(event: &Value, race_data: &Value) -> Value {
    json!({
        "logo": get_string(event, &["logo"])
            .or_else(|| get_string(race_data, &["logo"])),
        "banner": get_string(event, &["banner"])
            .or_else(|| get_string(race_data, &["banner"])),
        "thumbnail": get_string(event, &["thumbnail"])
            .or_else(|| get_string(race_data, &["thumbnail"])),
        "gallery": get_array(event, "gallery")
    })
}

fn build_organizer(event: &Value, race_data: &Value) -> Value {
    json!({
        "name": get_string(
            race_data,
            &["organiserName", "organizerName", "company_name"]
        )
        .or_else(|| {
            get_string(
                event,
                &["organiserName", "organizerName", "company_name"],
            )
        }),
        "companyName": get_string(race_data, &["company_name"])
            .or_else(|| get_string(event, &["company_name"])),
    })
}

fn build_dates(event: &Value, race_data: &Value) -> Value {
    json!({
        "eventStartDate": get_string(
            race_data,
            &["event_start_date", "eventStartDate"]
        )
        .or_else(|| {
            get_string(
                event,
                &["event_start_date", "eventStartDate", "date"],
            )
        }),
        "eventEndDate": get_string(
            race_data,
            &["event_end_date", "eventEndDate"]
        )
        .or_else(|| {
            get_string(
                event,
                &["event_end_date", "eventEndDate"],
            )
        }),
        "registrationStartDate": get_string(
            event,
            &["registration_start_date", "registrationStartDate"]
        ),
        "registrationEndDate": get_string(
            event,
            &["registration_end_date", "registrationEndDate"]
        )
    })
}

fn build_location(event: &Value, race_data: &Value) -> Value {
    json!({
        "location": get_string(race_data, &["location"])
            .or_else(|| get_string(event, &["location"])),
        "city": get_string(race_data, &["city"])
            .or_else(|| get_string(event, &["city"])),
        "state": get_string(race_data, &["state"])
            .or_else(|| get_string(event, &["state"])),
        "address": get_string(race_data, &["address"])
            .or_else(|| get_string(event, &["address"]))
    })
}

fn build_links(event: &Value, race_data: &Value) -> Value {
    json!({
        "eventUrl": get_string(event, &["event_url", "eventUrl"]),
        "slug": get_string(race_data, &["event_slug"])
            .or_else(|| get_string(event, &["event_slug"])),
    })
}

fn build_details(event: &Value, race_data: &Value) -> Value {
    json!({
        "description": get_string(race_data, &["description"])
            .or_else(|| get_string(event, &["description"])),

        "disclaimer": get_string(race_data, &["disclaimer"])
            .or_else(|| get_string(event, &["disclaimer"])),

        "status": get_string(race_data, &["status"])
            .or_else(|| get_string(event, &["status"])),

        "eventType": get_string(race_data, &["eventType", "event_type"])
            .or_else(|| get_string(event, &["eventType", "event_type"])),

        "eventTypeId": get_string(
            race_data,
            &["event_type_id", "eventTypeId"]
        )
        .or_else(|| {
            get_string(
                event,
                &["event_type_id", "eventTypeId"],
            )
        }),

        "companyName": get_string(race_data, &["company_name"])
            .or_else(|| get_string(event, &["company_name"])),

        "ticketCount": race_data
            .get("tickets")
            .and_then(Value::as_array)
            .map(|tickets| tickets.len())
            .unwrap_or(0)
    })
}

pub fn map_event(event: &Value, race_response: Option<&Value>) -> EventRow {
    let race_data = race_response
        .map(extract_race_data)
        .unwrap_or_else(|| json!({}));

    let event_id = get_string(&race_data, &["id", "event_id", "eventId"])
        .or_else(|| get_string(event, &["id", "event_id", "eventId"]))
        .unwrap_or_default();

    let event_name = get_string(&race_data, &["event_name", "eventName"])
        .or_else(|| get_string(event, &["event_name", "eventName", "name"]))
        .unwrap_or_default();

    let event_code = get_string(&race_data, &["event_slug", "eventSlug"])
        .or_else(|| get_string(event, &["event_slug", "eventSlug"]));

    let event_type = get_string(&race_data, &["eventType", "event_type"])
        .or_else(|| get_string(event, &["eventType", "event_type"]));

    let event_date = parse_event_date(&race_data, &["event_start_date", "eventStartDate", "date"])
        .or_else(|| {
            parse_event_date(
                event,
                &[
                    "event_start_date",
                    "eventStartDate",
                    "date",
                    "event_date",
                    "eventDate",
                ],
            )
        });

    let organizer = build_organizer(event, &race_data);
    let dates = build_dates(event, &race_data);
    let location = build_location(event, &race_data);
    let links = build_links(event, &race_data);
    let images = build_images(event, &race_data);
    let details = build_details(event, &race_data);

    let prices = race_data
        .get("tickets")
        .filter(|value| value.is_array())
        .cloned()
        .unwrap_or_else(|| json!([]));

    let raw_data = match race_response {
        Some(race_response) => json!({
            "event": event,
            "raceResponse": race_response
        }),
        None => json!({
            "event": event
        }),
    };

    EventRow {
        provider: "ifinish".to_string(),
        event_id,
        event_code,

        event_name,
        event_type,
        event_date,

        organizer,
        dates,
        location,
        links,
        images,
        details,
        prices,

        raw_data: Some(raw_data),
    }
}

pub async fn fetch_event_list(client: &Client) -> Result<Vec<Value>, String> {
    let user_id = get_ifinish_user_id();

    let url = format!("{}{}", IFINISH_LIST_URL, user_id);

    println!("Fetching iFinish event list...");
    println!("URL: {}", url);

    let response = client
        .get(&url)
        .header("Origin", "https://ifinish.in")
        .header("Referer", "https://ifinish.in/")
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
        .send()
        .await
        .map_err(|error| format!("Failed to call iFinish event list: {}", error))?;

    let status = response.status();

    let body = response
        .text()
        .await
        .map_err(|error| format!("Failed to read iFinish event list response: {}", error))?;

    if !status.is_success() {
        return Err(format!(
            "iFinish event list returned HTTP {}: {}",
            status, body
        ));
    }

    let json: Value = serde_json::from_str(&body)
        .map_err(|error| format!("Failed to parse iFinish event list JSON: {}", error))?;

    let events = extract_event_list(&json);

    println!("iFinish event list returned {} events", events.len());

    Ok(events)
}

pub async fn fetch_event_races(client: &Client, event_id: &str) -> Result<Value, String> {
    let payload = json!({
        "eventId": event_id
    });

    let response = client
        .post(IFINISH_RACE_URL)
        .header("Origin", "https://ifinish.in")
        .header("Referer", "https://ifinish.in/")
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
        .json(&payload)
        .send()
        .await
        .map_err(|error| {
            format!(
                "Failed to call userEventRaceList for {}: {}",
                event_id, error
            )
        })?;

    let status = response.status();

    let body = response
        .text()
        .await
        .map_err(|error| format!("Failed to read race response for {}: {}", event_id, error))?;

    if !status.is_success() {
        return Err(format!(
            "userEventRaceList returned HTTP {} for {}: {}",
            status, event_id, body
        ));
    }

    let json: Value = serde_json::from_str(&body)
        .map_err(|error| format!("Failed to parse race response for {}: {}", event_id, error))?;

    let success = json.get("status").and_then(Value::as_bool).unwrap_or(true);

    if !success {
        return Err(format!(
            "iFinish race API returned status=false for {}: {}",
            event_id, body
        ));
    }

    Ok(json)
}

pub async fn search_timing_event(
    client: &Client,
    event_name: &str,
    year: &str,
) -> Result<Option<Value>, String> {
    println!("Searching iFinish timing event: {}", event_name);

    let response = client
        .post(IFINISH_SEARCH_EVENT_URL)
        .header("Origin", "https://ifinish.in")
        .header("Referer", "https://ifinish.in/")
        .header(reqwest::header::USER_AGENT, "Mozilla/5.0")
        .json(&json!({
            "eventName": event_name,
            "year": year
        }))
        .send()
        .await
        .map_err(|error| {
            format!(
                "Failed to call searchTimingEvent for '{}': {}",
                event_name, error
            )
        })?;

    let status = response.status();

    let body = response.text().await.map_err(|error| {
        format!(
            "Failed to read searchTimingEvent response for '{}': {}",
            event_name, error
        )
    })?;

    println!("searchTimingEvent HTTP status: {}", status);

    println!("searchTimingEvent response: {}", body);

    if !status.is_success() {
        return Err(format!(
            "searchTimingEvent returned HTTP {} for '{}': {}",
            status, event_name, body
        ));
    }

    let json: Value = serde_json::from_str(&body).map_err(|error| {
        format!(
            "Failed to parse searchTimingEvent response for '{}': {}",
            event_name, error
        )
    })?;

    let success = json.get("status").and_then(Value::as_bool).unwrap_or(false);

    println!("searchTimingEvent status field: {}", success);

    if !success {
        return Ok(None);
    }

    let events = match json.get("data").and_then(Value::as_array) {
        Some(events) => events,

        None => {
            println!("searchTimingEvent returned no data array");
            return Ok(None);
        }
    };

    println!("searchTimingEvent returned {} events", events.len());

    for event in events {
        println!("iFinish search event: {}", event);

        let name = event.get("name").and_then(Value::as_str).unwrap_or("");

        if name.eq_ignore_ascii_case(event_name) {
            println!("Found exact iFinish event: {}", name);

            return Ok(Some(event.clone()));
        }
    }

    println!("No exact iFinish event matched: {}", event_name);

    Ok(None)
}

pub async fn sync_single_event(
    pool: &PgPool,
    client: &Client,
    event_name: &str,
    year: &str,
) -> Result<Option<String>, String> {
    // --------------------------------------------------
    // Search event using iFinish timing-event API
    // --------------------------------------------------

    let event = match search_timing_event(client, event_name, year).await? {
        Some(event) => event,

        None => {
            return Ok(None);
        }
    };

    // --------------------------------------------------
    // Get provider event ID
    // --------------------------------------------------

    let provider_event_id = get_string(&event, &["id", "event_id", "eventId"]).unwrap_or_default();

    if provider_event_id.trim().is_empty() {
        return Err(format!("iFinish event '{}' has no event_id", event_name));
    }

    println!(
        "Found iFinish event: {} ({})",
        event_name, provider_event_id
    );

    // --------------------------------------------------
    // Fetch full event/race details
    // --------------------------------------------------

    let race_response = fetch_event_races(client, &provider_event_id)
        .await
        .map_err(|error| {
            format!(
                "Failed to fetch race details for '{}': {}",
                event_name, error
            )
        })?;

    println!("Race details fetched successfully for: {}", event_name);

    // --------------------------------------------------
    // Map complete event
    // --------------------------------------------------

    let mapped_event = map_event(&event, Some(&race_response));

    if mapped_event.event_id.trim().is_empty() {
        return Err(format!(
            "Mapped iFinish event '{}' has no event_id",
            event_name
        ));
    }

    let event_id = mapped_event.event_id.clone();

    // --------------------------------------------------
    // Save complete event using existing repository
    // --------------------------------------------------

    let events = vec![mapped_event];

    let summary = save_events(pool, &events).await;

    if summary.failed > 0 {
        return Err(format!(
            "Failed to save iFinish event '{}': {:?}",
            event_name, summary.warnings
        ));
    }

    println!(
        "iFinish event saved successfully: {} ({})",
        event_name, event_id
    );

    Ok(Some(event_id))
}

#[get("/sync/ifinish")]
pub async fn sync_ifinish(pool: web::Data<PgPool>) -> impl Responder {
    let client = match Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(client) => client,

        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({
                "provider": "ifinish",
                "success": false,
                "error": format!("Failed to create HTTP client: {}", error)
            }));
        }
    };

    let events = match fetch_event_list(&client).await {
        Ok(events) => events,

        Err(error) => {
            return HttpResponse::BadGateway().json(json!({
                "provider": "ifinish",
                "success": false,
                "error": error
            }));
        }
    };

    let mut mapped_events: Vec<EventRow> = Vec::new();
    let mut summary = SyncSummary::new();

    summary.fetched = events.len();

    for (index, event) in events.iter().enumerate() {
        let event_id = get_string(event, &["id", "event_id", "eventId"]).unwrap_or_default();

        let event_name = get_string(event, &["event_name", "eventName", "name"])
            .unwrap_or_else(|| "Unknown Event".to_string());

        println!(
            "[{}/{}] Processing: {} ({})",
            index + 1,
            events.len(),
            event_name,
            event_id
        );

        if event_id.trim().is_empty() {
            summary.skipped += 1;

            summary.warnings.push(format!(
                "Skipped '{}' because event_id is empty",
                event_name
            ));

            continue;
        }

        let race_response = match fetch_event_races(&client, &event_id).await {
            Ok(response) => {
                println!("  Race details fetched successfully");

                Some(response)
            }

            Err(error) => {
                println!("  Warning: race details failed: {}", error);

                summary.warnings.push(format!(
                    "Race details failed for '{}' ({}): {}",
                    event_name, event_id, error
                ));

                None
            }
        };

        let mapped = map_event(event, race_response.as_ref());

        if mapped.event_id.trim().is_empty() {
            summary.skipped += 1;

            summary.warnings.push(format!(
                "Skipped '{}' because mapped event_id is empty",
                event_name
            ));

            continue;
        }

        mapped_events.push(mapped);

        // Small delay so we don't unnecessarily hammer the API.
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    println!(
        "Fetched {} events, mapped {} events",
        summary.fetched,
        mapped_events.len()
    );

    let db_summary = save_events(pool.get_ref(), &mapped_events).await;

    summary.inserted = db_summary.inserted;
    summary.updated = db_summary.updated;
    summary.skipped += db_summary.skipped;
    summary.failed += db_summary.failed;
    summary.warnings.extend(db_summary.warnings);

    HttpResponse::Ok().json(json!({
        "provider": "ifinish",
        "success": summary.failed == 0,
        "summary": summary
    }))
}
