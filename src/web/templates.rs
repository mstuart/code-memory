pub fn index_page() -> String {
    r#"<!DOCTYPE html>
<html>
<head>
    <title>code-memory Web UI</title>
    <style>
        body { font-family: sans-serif; max-width: 800px; margin: 50px auto; }
        input[type="text"] { width: 100%; padding: 10px; font-size: 16px; }
        button { padding: 10px 20px; font-size: 16px; }
        .result { border: 1px solid #ddd; padding: 10px; margin: 10px 0; }
    </style>
</head>
<body>
    <h1>code-memory Web UI</h1>
    <form action="/search" method="get">
        <input type="text" name="q" placeholder="Search code..." />
        <button type="submit">Search</button>
    </form>
    <p>Pro-only feature. <a href="https://code-memory.com/pro">Upgrade to Pro</a></p>
</body>
</html>"#
        .to_string()
}

pub fn search_results(query: &str) -> String {
    let query = escape_html_text(query);

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Search: {query}</title>
    <style>
        body {{ font-family: sans-serif; max-width: 800px; margin: 50px auto; }}
        .result {{ border: 1px solid #ddd; padding: 10px; margin: 10px 0; }}
        a {{ color: #0066cc; text-decoration: none; }}
    </style>
</head>
<body>
    <h1>Search Results: {query}</h1>
    <div class="result">
        <p>No results yet (search implementation pending)</p>
    </div>
    <p><a href="/">Back to search</a></p>
</body>
</html>"#,
        query = query
    )
}

fn escape_html_text(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());

    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }

    escaped
}

#[cfg(test)]
mod tests {
    use super::search_results;

    #[test]
    fn search_results_escapes_untrusted_query_text() {
        let escaped = "&lt;/title&gt;&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;&lt;img src=x onerror=alert(1)&gt;";
        let html =
            search_results(r#"</title><script>alert('xss')</script><img src=x onerror=alert(1)>"#);

        assert!(!html.contains("<script>"));
        assert!(!html.contains("<img"));
        assert!(!html.contains("</title><script>"));
        assert_eq!(html.matches(escaped).count(), 2);
    }

    #[test]
    fn search_results_preserves_plain_text_query() {
        let html = search_results("memory search — café");

        assert!(html.contains("Search Results: memory search — café"));
    }

    #[test]
    fn search_results_does_not_reinterpret_entities_or_encoded_text() {
        let html = search_results("&lt;script&gt; %3Cscript%3E");

        assert!(html.contains("&amp;lt;script&amp;gt; %3Cscript%3E"));
        assert!(!html.contains("<script>"));
    }
}
