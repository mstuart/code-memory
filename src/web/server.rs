use tiny_http::{Server, Response, Request};
use std::io::ErrorKind;

pub fn start_server(addr: &str) -> std::io::Result<()> {
    let server = Server::http(addr)
        .map_err(|e| std::io::Error::new(ErrorKind::Other, e))?;

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
    let response = Response::from_string(html)
        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap());
    let _ = request.respond(response);
}

fn handle_search(request: Request) {
    // Parse query parameter
    let url = request.url();
    let query = url.split('=').nth(1).unwrap_or("");

    let html = crate::web::templates::search_results(query);
    let response = Response::from_string(html)
        .with_header(tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html"[..]).unwrap());
    let _ = request.respond(response);
}
