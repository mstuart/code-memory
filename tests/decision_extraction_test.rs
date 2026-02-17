use code_memory::git::decision_parser::DecisionParser;

#[test]
fn test_extract_decision_from_commit() {
    let parser = DecisionParser::new();

    let commit_message = r#"refactor: decided to migrate from REST to GraphQL

We chose GraphQL for the following reasons:
1. Better type safety
2. Reduced over-fetching
3. Single endpoint

Files affected:
- src/api/graphql/schema.ts
- src/api/rest/legacy.ts (deprecated)
"#;

    let decisions = parser.parse_message(commit_message);

    assert_eq!(decisions.len(), 1);

    let decision = &decisions[0];
    assert_eq!(decision.decision_type, "migration");
    assert!(decision.reasoning.contains("type safety"));
    assert!(decision.from.is_some());
    assert_eq!(decision.from.as_ref().unwrap(), "REST");
    assert_eq!(decision.to, "GraphQL");
}

#[test]
fn test_ignore_non_decision_commits() {
    let parser = DecisionParser::new();

    let commit_message = "fix: typo in README";

    let decisions = parser.parse_message(commit_message);

    assert!(decisions.is_empty());
}

#[test]
fn test_extract_architectural_decision() {
    let parser = DecisionParser::new();

    let commit_message = r#"arch: switching to microservices architecture

Decided to split monolith into services for better scalability.
"#;

    let decisions = parser.parse_message(commit_message);

    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].decision_type, "architecture");
}

#[test]
fn test_extract_reasoning_text() {
    let parser = DecisionParser::new();

    let commit_message = r#"refactor: chose TypeScript

TypeScript provides:
- Static type checking
- Better IDE support
- Improved refactoring
"#;

    let decisions = parser.parse_message(commit_message);

    assert_eq!(decisions.len(), 1);
    assert!(decisions[0].reasoning.contains("Static type checking"));
    assert!(decisions[0].reasoning.contains("Better IDE support"));
}
