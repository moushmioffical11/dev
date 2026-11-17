use url::Url;

pub fn normalize_host(host: &str) -> String {
    host.strip_prefix("www.").unwrap_or(host).to_lowercase()
}

pub fn normalize_url(base_url: &Url, href: &str, allowed_host: &str) -> Option<String> {
    let mut url = base_url.join(href).ok()?;

    url.set_fragment(None);

    match url.scheme() {
        "http" | "https" => {}
        _ => return None,
    }

    let host = url.host_str()?;

    let current_host = normalize_host(host);
    let target_host = normalize_host(allowed_host);

    if current_host != target_host {
        return None;
    }

    Some(url.to_string())
}
