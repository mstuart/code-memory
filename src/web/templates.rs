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
