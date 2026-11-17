use std::{thread, time::Duration};

use crate::browser::{create_browser, wait_for_render};

pub fn resolve_event_url(list_url: &str, name: &str) -> Result<Option<String>, (u16, String)> {
    let browser = create_browser().map_err(|e| (500, e))?;
    let tab = browser
        .new_tab()
        .map_err(|e| (500, format!("Tab error: {}", e)))?;

    tab.navigate_to(list_url)
        .map_err(|e| (400, format!("Navigation error: {}", e)))?;

    wait_for_render(&tab).map_err(|e| (500, e))?;

    let start_url = tab.get_url();

    let escaped = name
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('\n', " ")
        .replace('\r', " ");

    let script = format!(
        r#"(function() {{
  const norm = s => (s || '').toLowerCase().replace(/[^a-z0-9]/g, '');
  const target = norm('{}');

  if (!target) return 'empty';

  let best = null, bestLen = Infinity;

  for (const el of document.querySelectorAll('body *')) {{
    const t = norm(el.innerText);

    if (!t || !t.includes(target)) continue;

    if (t.length < bestLen) {{
      best = el;
      bestLen = t.length;
    }}
  }}

  if (!best) return 'notfound';

  best.scrollIntoView({{block: 'center'}});

  let node = best;

  for (let i = 0; i < 10 && node; i++, node = node.parentElement) {{
    const clickable =
      node.tagName === 'A' ||
      node.hasAttribute('onclick') ||
      node.tagName === 'BUTTON' ||
      getComputedStyle(node).cursor === 'pointer';

    if (clickable) {{
      node.dispatchEvent(
        new MouseEvent('click', {{
          bubbles: true,
          cancelable: true,
          view: window
        }})
      );

      return 'clicked';
    }}
  }}

  best.click();

  return 'clicked-self';
}})()"#,
        escaped
    );

    let result = tab
        .evaluate(&script, false)
        .map_err(|e| (500, format!("Script error: {}", e)))?;

    println!("Resolve click result for '{}': {:?}", name, result.value);

    let is_event_url = |u: &str| -> bool {
        if u == start_url {
            return false;
        }

        match url::Url::parse(u) {
            Ok(parsed) => parsed
                .path_segments()
                .map_or(false, |segs| segs.filter(|s| !s.is_empty()).count() >= 2),
            Err(_) => false,
        }
    };

    for _ in 0..24 {
        thread::sleep(Duration::from_millis(500));

        let current = tab.get_url();

        if is_event_url(&current) {
            return Ok(Some(current));
        }

        if let Ok(tabs) = browser.get_tabs().lock() {
            for t in tabs.iter() {
                let u = t.get_url();

                if is_event_url(&u) {
                    return Ok(Some(u));
                }
            }
        }
    }

    Ok(None)
}
