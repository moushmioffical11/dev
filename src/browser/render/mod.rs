use crate::browser::client::create_browser;
use headless_chrome::Tab;
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

pub fn wait_for_render(tab: &Arc<Tab>) -> Result<String, String> {
    let _ = tab.wait_until_navigated();

    let start = Instant::now();
    let mut last_len = 0usize;
    let mut stable = 0u32;
    let mut html = String::new();

    while start.elapsed() < Duration::from_secs(20) {
        let _ = tab.evaluate("window.scrollTo(0, document.body.scrollHeight);", false);
        thread::sleep(Duration::from_millis(800));

        html = tab
            .get_content()
            .map_err(|error| format!("Content error: {}", error))?;

        let len = html.len();
        if len > 5000 && len == last_len {
            stable += 1;
            if stable >= 2 && start.elapsed() >= Duration::from_secs(3) {
                break;
            }
        } else {
            stable = 0;
        }
        last_len = len;
    }

    let _ = tab.evaluate("window.scrollTo(0, 0);", false);
    println!(
        "Render finished after {:?}, {} bytes",
        start.elapsed(),
        html.len()
    );
    Ok(html)
}

// Full pipeline used by /read and /map: browser -> tab -> navigate -> wait -> HTML.
pub fn render_html(url: &str) -> Result<String, (u16, String)> {
    let browser = create_browser().map_err(|e| (500, e))?;
    let tab = browser
        .new_tab()
        .map_err(|e| (500, format!("Tab error: {}", e)))?;
    tab.navigate_to(url)
        .map_err(|e| (400, format!("Navigation error: {}", e)))?;
    wait_for_render(&tab).map_err(|e| (500, e))
}
