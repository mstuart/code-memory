use code_memory::indexer::watcher::FileWatcher;
use std::fs;
use std::time::Duration;

#[test]
fn test_file_change_detection() {
    let temp_dir = std::env::temp_dir().join("code-memory-test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let test_file = temp_dir.join("test.rs");
    fs::write(&test_file, "fn main() {}").unwrap();

    // Create watcher after file exists
    let mut watcher = FileWatcher::new(temp_dir.clone()).unwrap();

    // Give watcher time to initialize
    std::thread::sleep(Duration::from_millis(100));

    // Modify file
    fs::write(&test_file, "fn main() { println!(\"hello\"); }").unwrap();

    // Poll for changes with debouncing
    // First poll: collect events but don't return yet (debounce period not elapsed)
    std::thread::sleep(Duration::from_millis(100));
    let changes = watcher.get_changes();
    assert!(changes.is_empty(), "Should not return changes during debounce period");

    // Second poll: after debounce period, should return changes
    std::thread::sleep(Duration::from_millis(500));
    let changes = watcher.get_changes();

    assert!(!changes.is_empty(), "No changes detected");
    // On macOS, paths might be canonicalized (/var -> /private/var)
    let test_file_canonical = test_file.canonicalize().unwrap_or(test_file.clone());
    let changes_canonical: Vec<_> = changes.iter()
        .map(|p| p.canonicalize().unwrap_or_else(|_| p.clone()))
        .collect();
    assert!(
        changes_canonical.contains(&test_file_canonical),
        "Test file {:?} not in changes {:?}", test_file_canonical, changes_canonical
    );

    // Cleanup
    fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_debouncing() {
    let temp_dir = std::env::temp_dir().join("code-memory-test-debounce");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let test_file = temp_dir.join("test.rs");
    fs::write(&test_file, "fn main() {}").unwrap();

    // Create watcher after file exists
    let mut watcher = FileWatcher::new(temp_dir.clone()).unwrap();

    // Give watcher time to initialize
    std::thread::sleep(Duration::from_millis(100));

    // Multiple rapid changes
    for i in 0..10 {
        fs::write(&test_file, format!("fn main() {{ println!(\"{}\"); }}", i)).unwrap();
        std::thread::sleep(Duration::from_millis(50));
    }

    // Poll immediately - should collect events but not return (still debouncing)
    let changes = watcher.get_changes();
    assert!(changes.is_empty(), "Should not return changes during debounce period");

    // Wait for debounce period and poll again
    std::thread::sleep(Duration::from_millis(600));
    let changes = watcher.get_changes();

    // Should only register once due to debouncing
    assert_eq!(changes.len(), 1, "Expected 1 change, got {}", changes.len());

    // Cleanup
    fs::remove_dir_all(&temp_dir).unwrap();
}
