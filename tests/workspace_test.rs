use code_memory::workspace::team_workspace::{TeamWorkspace, WorkspaceConfig};

#[tokio::test]
async fn test_create_workspace() {
    let config = WorkspaceConfig {
        team_id: "team_123".to_string(),
        name: "Engineering".to_string(),
        database_url: "postgresql://localhost/test_db".to_string(),
    };

    let workspace = TeamWorkspace::new(config).await.unwrap();
    assert_eq!(workspace.team_id(), "team_123");
}

#[tokio::test]
async fn test_add_member() {
    let config = WorkspaceConfig {
        team_id: "team_123".to_string(),
        name: "Engineering".to_string(),
        database_url: "postgresql://localhost/test_db".to_string(),
    };

    let workspace = TeamWorkspace::new(config).await.unwrap();
    workspace.add_member("user_456", "developer").await.unwrap();

    let members = workspace.list_members().await.unwrap();
    assert_eq!(members.len(), 1);
    assert_eq!(members[0].user_id, "user_456");
}

#[tokio::test]
async fn test_shared_knowledge() {
    let config = WorkspaceConfig {
        team_id: "team_123".to_string(),
        name: "Engineering".to_string(),
        database_url: "postgresql://localhost/test_db".to_string(),
    };

    let workspace = TeamWorkspace::new(config).await.unwrap();

    workspace.add_knowledge(
        "src/main.rs",
        "abc123",
        None,
        Some(serde_json::json!({"language": "rust"}))
    ).await.unwrap();

    let knowledge = workspace.search_knowledge("main.rs").await.unwrap();
    assert_eq!(knowledge.len(), 1);
}
