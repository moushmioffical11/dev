use actix_web::{HttpResponse, Responder, get, web};
use chrono::{DateTime, NaiveDate};
use reqwest::Client;
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::models::EventRow;
use crate::repositories::event::save_events;

const MYSAMAY_EVENTS_URL: &str = "https://www.mysamay.in/events-srv/events/all";

fn string_value(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(|value| value.to_string())
}

fn parse_event_date(value: Option<&Value>) -> Option<NaiveDate> {
    let value = value?.as_str()?;

    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|date| date.date_naive())
}

fn build_organizer(event: &Value) -> Value {
    json!({
        "name": event.get("organiserName").and_then(Value::as_str),
        "email": event.get("organiserEmail").and_then(Value::as_str),
        "supportPhone": event.get("organiserSupportPhone").and_then(Value::as_str),
        "supportEmail": event.get("organiserSupportEmail").and_then(Value::as_str),
        "website": event.get("organiserWebsite").and_then(Value::as_str),
        "address": event.get("organiserAddress").and_then(Value::as_str),
        "registrationPlatform": event.get("organiserRegPlatform")
    })
}

fn build_dates(event: &Value) -> Value {
    json!({
        "registrationOpen": event.get("registrationOpen"),
        "registrationClose": event.get("registrationClose"),
        "registrationStartDate": event.get("registrationStartDate"),
        "registrationEndDate": event.get("registrationEnddate"),
        "eventDate": event.get("eventDate"),
        "eventStartDate": event.get("eventStartDate"),
        "eventEndDate": event.get("eventEndDate")
    })
}

fn build_location(event: &Value) -> Value {
    json!({
        "city": event.get("city"),
        "venue": event.get("venue"),
        "eventLocation": event.get("eventLocation")
    })
}

fn build_links(event: &Value) -> Value {
    json!({
        "eventURL": event.get("eventURL"),
        "eventWebsite": event.get("eventWebsite"),
        "organiserWebsite": event.get("organiserWebsite"),
        "eventPhotoURL": event.get("eventPhotoURL")
    })
}

fn build_images(event: &Value) -> Value {
    json!({
        "bannerUrl": event.get("bannerUrl"),
        "verticalBannerUrl": event.get("verticalBannerUrl"),
        "logoUrl": event.get("logoUrl"),
        "organizerLogo": event.get("organizerLogo"),
        "companyLogo": event.get("companyLogo"),
        "donationForLogo": event.get("donationForLogo")
    })
}

fn build_details(event: &Value) -> Value {
    json!({
        "description": event.get("description"),
        "guidelines": event.get("guidelines"),
        "tagLine": event.get("tagLine"),
        "eventCategory": event.get("eventCategory"),
        "attendeeLimit": event.get("attendeeLimit"),
        "dailyLimit": event.get("dailyLimit"),
        "dailyMinLimit": event.get("dailyMinLimit"),
        "manualEntry": event.get("manualEntry"),
        "promotionEnabled": event.get("promotionEnabled"),
        "virtual": event.get("virtual"),
        "private": event.get("private"),
        "freeEvent": event.get("freeEvent"),
        "resultUploaded": event.get("resultUploaded"),
        "awaitingForResults": event.get("awaitingForResults"),
        "resultFinalized": event.get("resultFinalized"),
        "provisionalResult": event.get("provisionalResult"),
        "partners": event.get("partners"),
        "registrationPrefix": event.get("registrationPrefix"),
        "regSummaryNote": event.get("regSummaryNote"),
        "customRegistration": event.get("customRegistration"),
        "sourceGroup": event.get("source_group"),
        "mainEvent": event.get("mainEvent"),
        "isTrainingRun": event.get("isTrainingRun"),
        "feedbackRequired": event.get("feedbackRequired")
    })
}

fn build_prices(event: &Value) -> Value {
    match event.get("categories") {
        Some(categories) if categories.is_array() => categories.clone(),
        _ => json!([]),
    }
}

pub fn map_event(event: &Value) -> Result<EventRow, String> {
    let event_id = string_value(event.get("_id"))
        .or_else(|| string_value(event.get("eventId")))
        .ok_or_else(|| "Missing event ID".to_string())?;

    let event_name = string_value(event.get("name"))
        .ok_or_else(|| format!("Missing event name for {}", event_id))?;

    Ok(EventRow {
        provider: "mysamay".to_string(),

        event_id,

        event_code: string_value(event.get("registrationPrefix"))
            .or_else(|| string_value(event.get("eventSlug"))),

        event_name,

        event_type: string_value(event.get("eventType")),

        event_date: parse_event_date(event.get("eventDate")),

        organizer: build_organizer(event),

        dates: build_dates(event),

        location: build_location(event),

        links: build_links(event),

        images: build_images(event),

        details: build_details(event),

        prices: build_prices(event),

        raw_data: Some(event.clone()),
    })
}

#[get("/sync/mysamay")]
pub async fn sync_mysamay(pool: web::Data<PgPool>) -> impl Responder {
    let client = match Client::builder().user_agent("Mozilla/5.0").build() {
        Ok(client) => client,

        Err(error) => {
            return HttpResponse::InternalServerError().json(json!({
                "provider": "mysamay",
                "success": false,
                "error": format!("Failed to create HTTP client: {}", error)
            }));
        }
    };

    println!("Starting MySamay event sync...");

    let response = match client
        .get(MYSAMAY_EVENTS_URL)
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(response) => response,

        Err(error) => {
            return HttpResponse::BadGateway().json(json!({
                "provider": "mysamay",
                "success": false,
                "error": format!("Failed to call MySamay API: {}", error)
            }));
        }
    };

    let status = response.status();

    if !status.is_success() {
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "Unable to read response body".to_string());

        return HttpResponse::BadGateway().json(json!({
            "provider": "mysamay",
            "success": false,
            "status": status.as_u16(),
            "error": body
        }));
    }

    let response_json: Value = match response.json().await {
        Ok(value) => value,

        Err(error) => {
            return HttpResponse::BadGateway().json(json!({
                "provider": "mysamay",
                "success": false,
                "error": format!("Invalid JSON response from MySamay: {}", error)
            }));
        }
    };

    if response_json.get("success").and_then(Value::as_bool) != Some(true) {
        return HttpResponse::BadGateway().json(json!({
            "provider": "mysamay",
            "success": false,
            "error": "MySamay API returned success=false",
            "response": response_json
        }));
    }

    let events = match response_json.get("data").and_then(Value::as_array) {
        Some(events) => events,

        None => {
            return HttpResponse::BadGateway().json(json!({
                "provider": "mysamay",
                "success": false,
                "error": "MySamay response does not contain data array"
            }));
        }
    };

    println!("MySamay returned {} events", events.len());

    let mut mapped_events = Vec::with_capacity(events.len());
    let mut mapping_warnings = Vec::new();

    for (index, event) in events.iter().enumerate() {
        match map_event(event) {
            Ok(event_row) => {
                println!(
                    "[{}/{}] Processing: {} ({})",
                    index + 1,
                    events.len(),
                    event_row.event_name,
                    event_row.event_id
                );

                mapped_events.push(event_row);
            }

            Err(error) => {
                println!(
                    "[{}/{}] Failed to map event: {}",
                    index + 1,
                    events.len(),
                    error
                );

                mapping_warnings.push(error);
            }
        }
    }

    let mut summary = save_events(&pool, &mapped_events).await;

    summary.warnings.extend(mapping_warnings);

    println!(
        "MySamay sync completed: fetched={}, inserted={}, updated={}, skipped={}, failed={}",
        summary.fetched, summary.inserted, summary.updated, summary.skipped, summary.failed
    );

    HttpResponse::Ok().json(json!({
        "provider": "mysamay",
        "success": true,
        "summary": summary
    }))
}
