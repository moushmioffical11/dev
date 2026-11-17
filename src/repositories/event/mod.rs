use crate::models::{EventRow, SyncSummary};
use sqlx::PgPool;

impl SyncSummary {
    pub fn new() -> Self {
        Self {
            fetched: 0,
            inserted: 0,
            updated: 0,
            skipped: 0,
            failed: 0,
            warnings: Vec::new(),
        }
    }
}

const UPSERT_EVENT_SQL: &str = r#"
INSERT INTO events (
    provider,
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
    raw_data
)
VALUES (
    $1,
    $2,
    $3,
    $4,
    $5,
    $6,
    $7,
    $8,
    $9,
    $10,
    $11,
    $12,
    $13,
    $14
)
ON CONFLICT (provider, event_id)
DO UPDATE SET
    event_code = EXCLUDED.event_code,
    event_name = EXCLUDED.event_name,
    event_type = EXCLUDED.event_type,
    event_date = EXCLUDED.event_date,
    organizer = EXCLUDED.organizer,
    dates = EXCLUDED.dates,
    location = EXCLUDED.location,
    links = EXCLUDED.links,
    images = EXCLUDED.images,
    details = EXCLUDED.details,

    prices = CASE
        WHEN jsonb_typeof(EXCLUDED.prices) = 'array'
             AND jsonb_array_length(EXCLUDED.prices) > 0
        THEN EXCLUDED.prices
        ELSE events.prices
    END,

    raw_data = EXCLUDED.raw_data,
    updated_at = NOW()

RETURNING (xmax = 0) AS inserted
"#;

pub async fn upsert_event(pool: &PgPool, event: &EventRow) -> Result<bool, sqlx::Error> {
    let inserted: bool = sqlx::query_scalar(UPSERT_EVENT_SQL)
        .bind(&event.provider)
        .bind(&event.event_id)
        .bind(&event.event_code)
        .bind(&event.event_name)
        .bind(&event.event_type)
        .bind(event.event_date)
        .bind(&event.organizer)
        .bind(&event.dates)
        .bind(&event.location)
        .bind(&event.links)
        .bind(&event.images)
        .bind(&event.details)
        .bind(&event.prices)
        .bind(&event.raw_data)
        .fetch_one(pool)
        .await?;

    Ok(inserted)
}

pub async fn save_events(pool: &PgPool, events: &[EventRow]) -> SyncSummary {
    let mut summary = SyncSummary::new();

    summary.fetched = events.len();

    for event in events {
        if event.event_id.trim().is_empty() {
            summary.skipped += 1;

            summary.warnings.push(format!(
                "Skipped event '{}' because event_id is empty",
                event.event_name
            ));

            continue;
        }

        if event.event_name.trim().is_empty() {
            summary.skipped += 1;

            summary.warnings.push(format!(
                "Skipped event '{}' because event_name is empty",
                event.event_id
            ));

            continue;
        }

        match upsert_event(pool, event).await {
            Ok(inserted) => {
                if inserted {
                    summary.inserted += 1;
                } else {
                    summary.updated += 1;
                }
            }

            Err(error) => {
                summary.failed += 1;

                summary.warnings.push(format!(
                    "Failed to save event '{}' ({}/{}): {}",
                    event.event_name, event.provider, event.event_id, error
                ));
            }
        }
    }

    summary
}

pub async fn find_event_id(
    pool: &PgPool,
    provider: &str,
    event_name: &str,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        r#"
        SELECT event_id
        FROM events
        WHERE LOWER(provider) = LOWER($1)
          AND LOWER(event_name) = LOWER($2)
        LIMIT 1
        "#,
    )
    .bind(provider)
    .bind(event_name)
    .fetch_optional(pool)
    .await
}
