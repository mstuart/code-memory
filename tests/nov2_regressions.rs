use code_memory::git::decision_parser::DecisionParser;
use code_memory::graph::analyzer::DependencyGraph;
use code_memory::sessions::PatternLibrary;

#[test]
fn migration_keywords_are_case_insensitive_without_losing_technology_case() {
    let decisions = DecisionParser::new()
        .parse_message("Decided to Migrate FROM REST TO GraphQL\nKeep typed queries.");
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].decision_type, "migration");
    assert_eq!(decisions[0].from.as_deref(), Some("REST"));
    assert_eq!(decisions[0].to, "GraphQL");
}

#[test]
fn plain_observations_are_visible_to_pattern_queries() {
    let mut library = PatternLibrary::new();
    for _ in 0..6 {
        library.observe("naming:rust:snake_case");
    }
    let patterns = library.confident_patterns(0.5);
    assert_eq!(patterns.len(), 1);
    assert_eq!(patterns[0].observation_count, 6);
    assert!(patterns[0].examples.is_empty());
    library.observe_with_example("naming:rust:snake_case", "fn hello_world()");
    assert_eq!(library.all_patterns()[0].observation_count, 7);
    assert_eq!(library.all_patterns()[0].examples, ["fn hello_world()"]);
}

#[test]
fn rust_child_modules_resolve_in_flat_and_directory_layouts() {
    for (parent, child) in [
        ("src/foo.rs", "src/foo/child.rs"),
        ("src/foo/mod.rs", "src/foo/child.rs"),
        ("src/lib.rs", "src/child/mod.rs"),
    ] {
        for child_first in [false, true] {
            let mut graph = DependencyGraph::new();
            if child_first {
                graph.add_file(child, "pub fn helper() {}\n");
            }
            graph.add_file(parent, "mod child;\n");
            if !child_first {
                graph.add_file(child, "pub fn helper() {}\n");
            }
            assert_eq!(graph.depends_on(parent), [child], "{parent} -> {child}");
        }
    }
}
