use tiny_http::{Request, Response, Server};

pub fn start_server(addr: &str) -> std::io::Result<()> {
    let server = Server::http(addr).map_err(std::io::Error::other)?;

    println!("Web UI running at http://{}", addr);
    println!("Press Ctrl+C to stop");

    for request in server.incoming_requests() {
        handle_request(request);
    }

    Ok(())
}

fn handle_request(request: Request) {
    let url = request.url().to_string();

    if url.starts_with("/search") {
        handle_search(request);
    } else if url == "/" {
        handle_index(request);
    } else {
        let response = Response::from_string("404 Not Found").with_status_code(404);
        let _ = request.respond(response);
    }
}

fn handle_index(request: Request) {
    let html = crate::web::templates::index_page();
    let response = Response::from_string(html).with_header(
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap(),
    );
    let _ = request.respond(response);
}

fn handle_search(request: Request) {
    let query = search_query(request.url());

    let html = crate::web::templates::search_results(&query);
    let response = Response::from_string(html).with_header(
        tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap(),
    );
    let _ = request.respond(response);
}

fn search_query(url: &str) -> String {
    let Some((_, query_string)) = url.split_once('?') else {
        return String::new();
    };

    form_urlencoded::parse(query_string.as_bytes())
        .find_map(|(name, value)| (name == "q").then(|| value.into_owned()))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::search_query;

    #[test]
    fn decodes_form_encoded_search_query() {
        assert_eq!(
            search_query("/search?q=memory+search+%E2%80%94+caf%C3%A9"),
            "memory search — café"
        );
    }

    #[test]
    fn selects_q_regardless_of_parameter_order() {
        assert_eq!(search_query("/search?scope=all&q=needle&page=2"), "needle");
    }

    #[test]
    fn returns_empty_query_when_q_is_missing() {
        assert_eq!(search_query("/search?scope=all"), "");
        assert_eq!(search_query("/search"), "");
    }
}
