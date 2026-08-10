use std::thread;
use std::time::Duration;

#[test]
fn test_web_server_starts() {
    use code_memory::web::server::start_server;

    // Start server in background thread
    let handle = thread::spawn(|| {
        start_server("127.0.0.1:8081").unwrap();
    });

    // Wait for server to start
    thread::sleep(Duration::from_millis(500));

    // Test connection
    let response =
        reqwest::blocking::get("http://127.0.0.1:8081").expect("Failed to connect to server");

    assert_eq!(response.status(), 200);

    // Cleanup
    drop(handle);
}

#[test]
fn test_search_endpoint() {
    // Start server
    let handle = thread::spawn(|| {
        code_memory::web::server::start_server("127.0.0.1:8082").unwrap();
    });

    thread::sleep(Duration::from_millis(500));

    let response =
        reqwest::blocking::get("http://127.0.0.1:8082/search?q=test").expect("Failed to connect");

    assert_eq!(response.status(), 200);

    drop(handle);
}
