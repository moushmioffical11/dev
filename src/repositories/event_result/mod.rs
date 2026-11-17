use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub async fn save_manual_event_result(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    event_id: &str,
    result: Value,
) -> Result<bool, sqlx::Error> {
    let existing_results: Value = sqlx::query_scalar(
        r#"
        SELECT event_results
        FROM user_data
        WHERE user_id = $1
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await?;

    let mut event_results = existing_results.as_array().cloned().unwrap_or_default();

    let new_result = serde_json::json!({
        "eventId": event_id,
        "result": result,
        "source": "MANUAL"
    });

    let mut updated = false;

    for existing_result in &mut event_results {
        if existing_result.get("eventId").and_then(Value::as_str) == Some(event_id) {
            *existing_result = new_result.clone();
            updated = true;
            break;
        }
    }

    if !updated {
        event_results.push(new_result);
    }

    let event_results = Value::Array(event_results);

    sqlx::query(
        r#"
        UPDATE user_data
        SET
            event_results = $2,
            updated_at = NOW()
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(event_results)
    .execute(&mut **tx)
    .await?;

    Ok(!updated)
}

pub async fn begin_transaction(pool: &PgPool) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
    pool.begin().await
}

pub async fn save_provider_event_result(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    event_id: &str,
    result: Value,
    source: &str,
) -> Result<bool, sqlx::Error> {
    let existing_results: Value = sqlx::query_scalar(
        r#"
        SELECT event_results
        FROM user_data
        WHERE user_id = $1
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut **tx)
    .await?;

    let mut event_results = existing_results.as_array().cloned().unwrap_or_default();

    let new_result = serde_json::json!({
        "eventId": event_id,
        "result": result,
        "source": source
    });

    let mut updated = false;

    for existing_result in &mut event_results {
        if existing_result.get("eventId").and_then(Value::as_str) == Some(event_id) {
            *existing_result = new_result.clone();
            updated = true;
            break;
        }
    }

    if !updated {
        event_results.push(new_result);
    }

    let event_results = Value::Array(event_results);

    sqlx::query(
        r#"
        UPDATE user_data
        SET
            event_results = $2,
            updated_at = NOW()
        WHERE user_id = $1
        "#,
    )
    .bind(user_id)
    .bind(event_results)
    .execute(&mut **tx)
    .await?;

    Ok(!updated)
}
