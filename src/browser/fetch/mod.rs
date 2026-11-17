use headless_chrome::Browser;
use std::time::Duration;

pub fn fetch_page_html(browser: &Browser, url: &str) -> Result<String, String> {
    let tab = browser
        .new_tab()
        .map_err(|e| format!("Failed to create tab: {}", e))?;

    tab.navigate_to(url)
        .map_err(|e| format!("Failed to navigate: {}", e))?;

    tab.wait_until_navigated()
        .map_err(|e| format!("Navigation failed: {}", e))?;

    println!("Opened: {}", url);

    std::thread::sleep(Duration::from_secs(8));

    println!("Getting DOM...");

    let result = tab
        .evaluate("document.documentElement.outerHTML", false)
        .map_err(|e| format!("Failed to get DOM HTML: {}", e))?;

    println!("DOM received.");

    let html = result
        .value
        .ok_or_else(|| "DOM HTML was empty".to_string())?;

    let html = html
        .as_str()
        .ok_or_else(|| "DOM HTML was not a string".to_string())?
        .to_string();

    Ok(html)
}
