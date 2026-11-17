use headless_chrome::{Browser, LaunchOptions};
use std::{env, ffi::OsStr, time::Duration};

pub fn create_browser() -> Result<Browser, String> {
    let chrome_path = env::var("CHROME_PATH").map_err(|_| "CHROME_PATH is not set".to_string())?;

    Browser::new(LaunchOptions {
        path: Some(chrome_path.into()),
        sandbox: false,
        idle_browser_timeout: Duration::from_secs(120),
        args: vec![
            OsStr::new("--no-sandbox"),
            OsStr::new("--disable-dev-shm-usage"),
            OsStr::new("--disable-gpu"),
            OsStr::new("--disable-software-rasterizer"),
            OsStr::new("--disable-blink-features=AutomationControlled"),
        ],
        ..Default::default()
    })
    .map_err(|error| format!("Browser error: {}", error))
}
