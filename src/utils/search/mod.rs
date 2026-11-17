pub fn create_search_snippet(content: &str, query: &str) -> Option<String> {
    let content_lower = content.to_lowercase();
    let query_lower = query.to_lowercase();

    let match_position = content_lower.find(&query_lower)?;

    let before_match = &content[..match_position];

    let char_position = before_match.chars().count();

    let chars: Vec<char> = content.chars().collect();

    let start = char_position.saturating_sub(100);

    let end = std::cmp::min(char_position + query.chars().count() + 100, chars.len());

    let snippet: String = chars[start..end].iter().collect();

    Some(format!("...{}...", snippet.trim()))
}
