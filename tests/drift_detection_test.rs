use code_memory::drift::detector::{DriftAlert, DriftDetector};
use code_memory::git::decision_parser::Decision;
use std::path::PathBuf;

#[test]
fn test_detect_architectural_drift() {
    let mut detector = DriftDetector::new();

    // Historical decision: Use REST API
    let decision = Decision {
        decision_type: "architecture".to_string(),
        from: None,
        to: "REST".to_string(),
        reasoning: "Simple, well-understood".to_string(),
        commit_sha: Some("abc123".to_string()),
        author: Some("alice".to_string()),
        timestamp: Some(1000000),
    };

    detector.add_decision(decision);

    // Current codebase: GraphQL file exists
    let current_files = vec![
        PathBuf::from("src/api/graphql/schema.ts"),
        PathBuf::from("src/api/rest/endpoints.ts"),
    ];

    detector.scan_files(&current_files);

    let alerts = detector.get_alerts();

    assert!(!alerts.is_empty());
    assert!(alerts[0].message.contains("REST"));
    assert!(alerts[0].message.contains("GraphQL"));
}

#[test]
fn test_no_drift_when_consistent() {
    let mut detector = DriftDetector::new();

    let decision = Decision {
        decision_type: "architecture".to_string(),
        from: None,
        to: "REST".to_string(),
        reasoning: "".to_string(),
        commit_sha: None,
        author: None,
        timestamp: None,
    };

    detector.add_decision(decision);

    let current_files = vec![PathBuf::from("src/api/rest/endpoints.ts")];

    detector.scan_files(&current_files);

    let alerts = detector.get_alerts();

    assert!(alerts.is_empty());
}
