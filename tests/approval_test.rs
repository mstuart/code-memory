use code_memory::approval::workflow::{ApprovalStatus, ApprovalWorkflow, PendingKnowledge};

fn test_database_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

#[tokio::test]
async fn test_submit_for_approval() {
    let Some(database_url) = test_database_url() else {
        eprintln!("skipping Postgres integration test: TEST_DATABASE_URL is not set");
        return;
    };
    let workflow = ApprovalWorkflow::new(&database_url).await.unwrap();

    let pending = PendingKnowledge {
        file_path: "src/auth.rs".to_string(),
        content_hash: "xyz789".to_string(),
        submitted_by: "user_123".to_string(),
        team_id: "team_456".to_string(),
    };

    let id = workflow.submit(pending).await.unwrap();
    assert!(!id.is_empty());
}

#[tokio::test]
async fn test_approve_knowledge() {
    let Some(database_url) = test_database_url() else {
        eprintln!("skipping Postgres integration test: TEST_DATABASE_URL is not set");
        return;
    };
    let workflow = ApprovalWorkflow::new(&database_url).await.unwrap();

    let pending = PendingKnowledge {
        file_path: "src/auth.rs".to_string(),
        content_hash: "xyz789".to_string(),
        submitted_by: "user_123".to_string(),
        team_id: "team_456".to_string(),
    };

    let id = workflow.submit(pending).await.unwrap();
    workflow.approve(&id, "reviewer_789").await.unwrap();

    let status = workflow.get_status(&id).await.unwrap();
    assert_eq!(status, ApprovalStatus::Approved);
}

#[tokio::test]
async fn test_reject_knowledge() {
    let Some(database_url) = test_database_url() else {
        eprintln!("skipping Postgres integration test: TEST_DATABASE_URL is not set");
        return;
    };
    let workflow = ApprovalWorkflow::new(&database_url).await.unwrap();

    let pending = PendingKnowledge {
        file_path: "secrets.env".to_string(),
        content_hash: "bad123".to_string(),
        submitted_by: "user_123".to_string(),
        team_id: "team_456".to_string(),
    };

    let id = workflow.submit(pending).await.unwrap();
    workflow
        .reject(&id, "reviewer_789", "Contains secrets")
        .await
        .unwrap();

    let status = workflow.get_status(&id).await.unwrap();
    assert_eq!(status, ApprovalStatus::Rejected);
}
