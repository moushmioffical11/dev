use scraper::{ElementRef, Html, Node, Selector};

const SKIP_TAGS: &[&str] = &[
    "script", "style", "noscript", "template", "svg", "head", "iframe",
];

const INLINE_TAGS: &[&str] = &[
    "b", "strong", "em", "i", "u", "small", "sup", "sub", "mark", "code", "abbr", "font", "span",
];

const OLD_TAGS: &[&str] = &["h1", "h2", "h3", "h4", "p", "li"];

fn flush_line(buf: &mut String, out: &mut Vec<String>) {
    let line = buf.split_whitespace().collect::<Vec<_>>().join(" ");

    if !line.is_empty() {
        out.push(line);
    }

    buf.clear();
}

fn joined_text(el: ElementRef) -> String {
    el.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn walk_text(el: ElementRef, in_old: bool, buf: &mut String, out: &mut Vec<String>) {
    let tag = el.value().name();

    if SKIP_TAGS.contains(&tag) {
        return;
    }

    let is_old = OLD_TAGS.contains(&tag);

    if is_old {
        flush_line(buf, out);

        let line = joined_text(el);

        if !line.is_empty() {
            out.push(line);
        }
    }

    let inside_old = in_old || is_old;
    let inline = INLINE_TAGS.contains(&tag);

    if !inside_old && !inline {
        flush_line(buf, out);
    }

    for child in el.children() {
        match child.value() {
            Node::Text(t) => {
                if !inside_old {
                    buf.push_str(t);
                }
            }

            Node::Element(_) => {
                if let Some(c) = ElementRef::wrap(child) {
                    walk_text(c, inside_old, buf, out);
                }
            }

            _ => {}
        }
    }

    if !inside_old && !inline {
        flush_line(buf, out);
    }
}

pub fn extract_visible_text(document: &Html) -> String {
    let mut out = Vec::new();
    let mut buf = String::new();

    walk_text(document.root_element(), false, &mut buf, &mut out);

    flush_line(&mut buf, &mut out);

    out.dedup();

    out.join("\n")
}

pub fn extract_legacy_text(document: &Html) -> String {
    let selector = Selector::parse("h1, h2, h3, h4, p, li").unwrap();

    let mut lines = Vec::new();

    for element in document.select(&selector) {
        let text = joined_text(element);

        if !text.is_empty() {
            lines.push(text);
        }
    }

    lines.join("\n")
}
