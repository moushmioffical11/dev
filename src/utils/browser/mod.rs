use headless_chrome::Tab;
use serde_json::Value;
use std::thread::sleep;
use std::time::Duration;

pub fn evaluate_value(tab: &Tab, script: &str) -> Result<Option<Value>, String> {
    let result = tab
        .evaluate(script, false)
        .map_err(|error| error.to_string())?;

    Ok(result.value)
}

pub fn evaluate_string(tab: &Tab, script: &str) -> Result<Option<String>, String> {
    let value = evaluate_value(tab, script)?;

    match value {
        Some(Value::String(value)) => Ok(Some(value)),
        Some(value) => Ok(Some(value.to_string())),
        None => Ok(None),
    }
}

pub fn wait_for_condition(
    tab: &Tab,
    script: &str,
    attempts: usize,
    delay_ms: u64,
) -> Result<(), String> {
    for _ in 0..attempts {
        match evaluate_value(tab, script) {
            Ok(Some(Value::Bool(true))) => {
                return Ok(());
            }

            Ok(_) => {}

            Err(error) => {
                return Err(error);
            }
        }

        sleep(Duration::from_millis(delay_ms));
    }

    Err("Condition was not satisfied within timeout".to_string())
}

pub fn escape_js_string(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
