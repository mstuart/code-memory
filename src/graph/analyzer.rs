use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Bfs;
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::imports::{ImportParser, ImportType};

/// Metadata about a node in the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub path: String,
    pub language: String,
    pub import_count: usize,
    pub imported_by_count: usize,
}

/// Result of a dependency query.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQuery {
    pub file: String,
    pub depends_on: Vec<String>,
    pub depended_on_by: Vec<String>,
    pub transitive_deps: Vec<String>,
}

/// Normalize a path by removing `.` and `..` components without filesystem access.
fn normalize_path(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for component in path.split('/') {
        match component {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

fn source_aliases(file_path: &str) -> Vec<String> {
    let path = Path::new(file_path);
    let extension = path.extension().and_then(|extension| extension.to_str());
    let mut aliases = vec![file_path.to_string()];

    if matches!(
        extension,
        Some("ts" | "tsx" | "js" | "jsx" | "py" | "rs" | "go")
    ) {
        aliases.push(path.with_extension("").to_string_lossy().into_owned());
    }
    if matches!(extension, Some("ts" | "tsx" | "js"))
        && path.file_stem().and_then(|stem| stem.to_str()) == Some("index")
    {
        if let Some(parent) = path.parent() {
            aliases.push(parent.to_string_lossy().into_owned());
        }
    }

    aliases
}

#[derive(Debug, Clone)]
struct ImportEdge {
    source: NodeIndex,
    target: NodeIndex,
    source_candidates: Vec<String>,
}

/// Dependency graph built from import analysis.
pub struct DependencyGraph {
    graph: DiGraph<String, ()>,
    node_map: HashMap<String, NodeIndex>,
    external_nodes: HashMap<String, NodeIndex>,
    source_paths: HashSet<String>,
    relative_placeholders: HashMap<String, NodeIndex>,
    import_edges: Vec<ImportEdge>,
    resolvable_imports: HashMap<String, Vec<usize>>,
    edge_ref_counts: HashMap<(NodeIndex, NodeIndex), usize>,
    parser: ImportParser,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
            external_nodes: HashMap::new(),
            source_paths: HashSet::new(),
            relative_placeholders: HashMap::new(),
            import_edges: Vec::new(),
            resolvable_imports: HashMap::new(),
            edge_ref_counts: HashMap::new(),
            parser: ImportParser::new(),
        }
    }

    /// Add a file and its imports to the graph.
    pub fn add_file(&mut self, file_path: &str, content: &str) {
        let source_idx = self.get_or_create_source_node(file_path);
        self.rebind_resolvable_imports(file_path);
        let imports = self.parser.parse(file_path, content);

        for import in &imports {
            let resolved =
                self.resolve_import(file_path, &import.imported_path, import.is_relative);
            let source_candidates = if import.is_relative {
                vec![resolved.clone()]
            } else if matches!(
                import.import_type,
                ImportType::PythonImport | ImportType::PythonFrom
            ) {
                self.python_source_candidates(file_path, &import.imported_path)
            } else {
                Vec::new()
            };
            let target_idx = if import.is_relative {
                self.find_matching_source(&resolved)
                    .unwrap_or_else(|| self.get_or_create_relative_placeholder(&resolved))
            } else if let Some(target_idx) = self.find_best_source(&source_candidates) {
                target_idx
            } else {
                self.get_or_create_external_node(&resolved)
            };
            self.increment_edge(source_idx, target_idx);
            let edge_index = self.import_edges.len();
            for candidate in &source_candidates {
                self.resolvable_imports
                    .entry(candidate.clone())
                    .or_default()
                    .push(edge_index);
            }
            self.import_edges.push(ImportEdge {
                source: source_idx,
                target: target_idx,
                source_candidates,
            });
        }
    }

    fn get_or_create_source_node(&mut self, file_path: &str) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(file_path) {
            return idx;
        }
        let idx = self.graph.add_node(file_path.to_string());
        self.node_map.insert(file_path.to_string(), idx);
        self.source_paths.insert(file_path.to_string());
        idx
    }

    /// Find the deterministic best source for an import path. Extension files
    /// take precedence over directory indexes regardless of walk order.
    fn find_matching_source(&self, resolved: &str) -> Option<NodeIndex> {
        if self.source_paths.contains(resolved) {
            return self.node_map.get(resolved).copied();
        }
        for ext in &[".ts", ".tsx", ".js", ".jsx", ".py", ".rs", ".go"] {
            let with_ext = format!("{}{}", resolved, ext);
            if self.source_paths.contains(&with_ext) {
                return self.node_map.get(&with_ext).copied();
            }
        }
        for ext in &["/index.ts", "/index.js", "/index.tsx"] {
            let with_index = format!("{}{}", resolved, ext);
            if self.source_paths.contains(&with_index) {
                return self.node_map.get(&with_index).copied();
            }
        }
        None
    }

    fn find_best_source(&self, candidates: &[String]) -> Option<NodeIndex> {
        candidates
            .iter()
            .find_map(|candidate| self.find_matching_source(candidate))
    }

    fn python_source_candidates(&self, source_file: &str, import_path: &str) -> Vec<String> {
        let module_path = import_path.replace('.', "/");
        let source_dir = Path::new(source_file)
            .parent()
            .unwrap_or_else(|| Path::new(""));
        let sibling = normalize_path(&source_dir.join(&module_path).to_string_lossy());

        if sibling == module_path {
            vec![module_path]
        } else {
            vec![sibling, module_path]
        }
    }

    fn get_or_create_relative_placeholder(&mut self, path: &str) -> NodeIndex {
        if let Some(&idx) = self.relative_placeholders.get(path) {
            return idx;
        }
        let idx = self.graph.add_node(path.to_string());
        self.relative_placeholders.insert(path.to_string(), idx);
        idx
    }

    /// Redirect unresolved imports whenever a newly discovered source is a
    /// better match. Keeping import bindings separate from graph nodes also
    /// prevents bare packages with the same name from being claimed as files.
    fn rebind_resolvable_imports(&mut self, file_path: &str) {
        for relative_path in source_aliases(file_path) {
            let edge_indices = self
                .resolvable_imports
                .get(&relative_path)
                .cloned()
                .unwrap_or_default();
            for edge_index in edge_indices {
                let source = self.import_edges[edge_index].source;
                let old_target = self.import_edges[edge_index].target;
                let Some(new_target) =
                    self.find_best_source(&self.import_edges[edge_index].source_candidates)
                else {
                    continue;
                };
                if new_target == old_target {
                    continue;
                }

                self.decrement_edge(source, old_target);
                self.increment_edge(source, new_target);
                self.import_edges[edge_index].target = new_target;
            }
            self.relative_placeholders.remove(&relative_path);
        }
    }

    fn get_or_create_external_node(&mut self, path: &str) -> NodeIndex {
        if let Some(&idx) = self.external_nodes.get(path) {
            idx
        } else {
            let idx = self.graph.add_node(path.to_string());
            self.external_nodes.insert(path.to_string(), idx);
            idx
        }
    }

    fn increment_edge(&mut self, source: NodeIndex, target: NodeIndex) {
        let count = self.edge_ref_counts.entry((source, target)).or_default();
        if *count == 0 {
            self.graph.add_edge(source, target, ());
        }
        *count += 1;
    }

    fn decrement_edge(&mut self, source: NodeIndex, target: NodeIndex) {
        let Some(count) = self.edge_ref_counts.get_mut(&(source, target)) else {
            return;
        };
        *count -= 1;
        if *count == 0 {
            self.edge_ref_counts.remove(&(source, target));
            if let Some(edge) = self.graph.find_edge(source, target) {
                self.graph.remove_edge(edge);
            }
        }
    }

    fn lookup_node(&self, path: &str) -> Option<NodeIndex> {
        self.node_map
            .get(path)
            .or_else(|| self.relative_placeholders.get(path))
            .or_else(|| self.external_nodes.get(path))
            .copied()
    }

    fn resolve_import(&self, source_file: &str, import_path: &str, is_relative: bool) -> String {
        if !is_relative {
            return import_path.to_string();
        }

        let source_dir = Path::new(source_file)
            .parent()
            .unwrap_or_else(|| Path::new(""));

        if import_path.starts_with('.') {
            let joined = source_dir.join(import_path);
            return normalize_path(&joined.to_string_lossy());
        }

        if import_path.starts_with("crate::") {
            return import_path.replace("crate::", "src/").replace("::", "/");
        }
        if import_path.starts_with("super::") {
            let parent = source_dir.parent().unwrap_or_else(|| Path::new(""));
            let rest = import_path.strip_prefix("super::").unwrap_or(import_path);
            let joined = parent.join(rest.replace("::", "/"));
            return normalize_path(&joined.to_string_lossy());
        }
        if import_path.starts_with("self::") {
            let rest = import_path.strip_prefix("self::").unwrap_or(import_path);
            let joined = source_dir.join(rest.replace("::", "/"));
            return normalize_path(&joined.to_string_lossy());
        }

        // For mod declarations, resolve relative to the current module directory
        let source_stem = Path::new(source_file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        if source_stem == "mod" || source_stem == "lib" || source_stem == "main" {
            let joined = source_dir.join(import_path);
            return normalize_path(&joined.to_string_lossy());
        }

        import_path.to_string()
    }

    /// What does this file depend on? (direct)
    pub fn depends_on(&self, file_path: &str) -> Vec<String> {
        let Some(idx) = self.lookup_node(file_path) else {
            return Vec::new();
        };
        self.graph
            .neighbors_directed(idx, Direction::Outgoing)
            .map(|n| self.graph[n].clone())
            .collect()
    }

    /// What depends on this file? (direct)
    pub fn depended_on_by(&self, file_path: &str) -> Vec<String> {
        let Some(idx) = self.lookup_node(file_path) else {
            return Vec::new();
        };
        self.graph
            .neighbors_directed(idx, Direction::Incoming)
            .map(|n| self.graph[n].clone())
            .collect()
    }

    /// Find all transitively related code (BFS from file).
    pub fn transitive_dependencies(&self, file_path: &str) -> Vec<String> {
        let Some(idx) = self.lookup_node(file_path) else {
            return Vec::new();
        };
        let mut bfs = Bfs::new(&self.graph, idx);
        let mut result = Vec::new();
        while let Some(node) = bfs.next(&self.graph) {
            if node != idx {
                result.push(self.graph[node].clone());
            }
        }
        result
    }

    /// Full query for a file's dependency info.
    pub fn query(&self, file_path: &str) -> GraphQuery {
        GraphQuery {
            file: file_path.to_string(),
            depends_on: self.depends_on(file_path),
            depended_on_by: self.depended_on_by(file_path),
            transitive_deps: self.transitive_dependencies(file_path),
        }
    }

    /// Info about all nodes.
    pub fn all_nodes(&self) -> Vec<NodeInfo> {
        self.node_map
            .iter()
            .chain(self.external_nodes.iter())
            .chain(self.relative_placeholders.iter())
            .map(|(path, &idx)| {
                let ext = Path::new(path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                let language = match ext {
                    "rs" => "rust",
                    "ts" | "tsx" => "typescript",
                    "js" | "jsx" => "javascript",
                    "py" => "python",
                    "go" => "go",
                    other => other,
                }
                .to_string();

                NodeInfo {
                    path: path.clone(),
                    language,
                    import_count: self
                        .graph
                        .neighbors_directed(idx, Direction::Outgoing)
                        .count(),
                    imported_by_count: self
                        .graph
                        .neighbors_directed(idx, Direction::Incoming)
                        .count(),
                }
            })
            .collect()
    }

    /// Total nodes and edges.
    pub fn stats(&self) -> (usize, usize) {
        (
            self.node_map.len() + self.external_nodes.len() + self.relative_placeholders.len(),
            self.graph.edge_count(),
        )
    }

    /// Files with most incoming dependencies.
    pub fn most_depended_on(&self, limit: usize) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = self
            .node_map
            .iter()
            .chain(self.external_nodes.iter())
            .chain(self.relative_placeholders.iter())
            .map(|(path, &idx)| {
                (
                    path.clone(),
                    self.graph
                        .neighbors_directed(idx, Direction::Incoming)
                        .count(),
                )
            })
            .collect();
        counts.sort_by_key(|b| std::cmp::Reverse(b.1));
        counts.truncate(limit);
        counts
    }

    /// Files with most outgoing dependencies (most coupled).
    pub fn most_coupled(&self, limit: usize) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = self
            .node_map
            .iter()
            .chain(self.external_nodes.iter())
            .chain(self.relative_placeholders.iter())
            .map(|(path, &idx)| {
                (
                    path.clone(),
                    self.graph
                        .neighbors_directed(idx, Direction::Outgoing)
                        .count(),
                )
            })
            .collect();
        counts.sort_by_key(|b| std::cmp::Reverse(b.1));
        counts.truncate(limit);
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_files_and_query() {
        let mut graph = DependencyGraph::new();

        graph.add_file(
            "src/main.ts",
            "import { Router } from './router';\nimport { Database } from './db';\n",
        );
        graph.add_file("src/router.ts", "import { handler } from './handler';\n");
        graph.add_file("src/handler.ts", "import { Database } from './db';\n");

        let deps = graph.depends_on("src/main.ts");
        assert_eq!(deps.len(), 2);

        let dependents = graph.depended_on_by("src/db");
        assert_eq!(dependents.len(), 2);
    }

    #[test]
    fn reconciles_extensionless_imports_when_target_is_added_later() {
        let mut graph = DependencyGraph::new();

        graph.add_file("src/main.ts", "import { Router } from './router';\n");
        graph.add_file("src/router.ts", "import { handler } from './handler';\n");
        graph.add_file("src/handler.ts", "export function handler() {}\n");

        assert_eq!(
            graph.depends_on("src/main.ts"),
            vec!["src/router.ts".to_string()]
        );
        assert_eq!(
            graph.depended_on_by("src/router.ts"),
            vec!["src/main.ts".to_string()]
        );
        let transitive = graph.transitive_dependencies("src/main.ts");
        assert!(transitive.contains(&"src/router.ts".to_string()));
        assert!(transitive.contains(&"src/handler.ts".to_string()));
    }

    #[test]
    fn reconciles_directory_imports_when_index_file_is_added_later() {
        let mut graph = DependencyGraph::new();

        graph.add_file("src/main.ts", "import { api } from './api';\n");
        graph.add_file("src/api/index.ts", "export const api = {};\n");

        assert_eq!(
            graph.depends_on("src/main.ts"),
            vec!["src/api/index.ts".to_string()]
        );
    }

    #[test]
    fn reconciles_all_aliases_for_an_index_source() {
        let mut graph = DependencyGraph::new();

        graph.add_file("src/a.ts", "import { api } from './api';\n");
        graph.add_file("src/b.ts", "import { api } from './api/index';\n");
        graph.add_file("src/api/index.ts", "export const api = {};\n");

        let mut dependents = graph.depended_on_by("src/api/index.ts");
        dependents.sort();
        assert_eq!(
            dependents,
            vec!["src/a.ts".to_string(), "src/b.ts".to_string()]
        );
    }

    #[test]
    fn extension_file_wins_over_index_regardless_of_walk_order() {
        for sources in [
            ["src/api.ts", "src/api/index.ts"],
            ["src/api/index.ts", "src/api.ts"],
        ] {
            let mut graph = DependencyGraph::new();
            graph.add_file("src/main.ts", "import { api } from './api';\n");
            for source in sources {
                graph.add_file(source, "export const api = {};\n");
            }

            assert_eq!(
                graph.depends_on("src/main.ts"),
                vec!["src/api.ts".to_string()]
            );
        }
    }

    #[test]
    fn bare_packages_are_not_reconciled_with_local_sources() {
        let mut graph = DependencyGraph::new();

        graph.add_file("src/main.ts", "import express from 'express';\n");
        graph.add_file("express.js", "export default {};\n");

        assert_eq!(graph.depends_on("src/main.ts"), vec!["express".to_string()]);
        assert!(graph.depended_on_by("express.js").is_empty());
    }

    #[test]
    fn exact_bare_specifiers_are_separate_from_source_paths() {
        let mut graph = DependencyGraph::new();

        graph.add_file("src/main.ts", "import value from 'index.js';\n");
        graph.add_file("index.js", "export default {};\n");

        assert!(graph.depended_on_by("index.js").is_empty());
        assert_eq!(
            graph.depends_on("src/main.ts"),
            vec!["index.js".to_string()]
        );
    }

    #[test]
    fn unsupported_index_files_do_not_claim_module_imports() {
        let mut graph = DependencyGraph::new();

        graph.add_file("src/main.ts", "import { api } from './api';\n");
        graph.add_file("src/api/index.json", "{}");

        assert_eq!(graph.depends_on("src/main.ts"), vec!["src/api".to_string()]);
        assert!(graph.depended_on_by("src/api/index.json").is_empty());
    }

    #[test]
    fn reconciles_python_absolute_imports_with_sibling_modules() {
        for sources in [
            ["src/main.py", "src/utils.py"],
            ["src/utils.py", "src/main.py"],
        ] {
            let mut graph = DependencyGraph::new();
            for source in sources {
                let content = if source == "src/main.py" {
                    "from utils import helper\n"
                } else {
                    "def helper(): pass\n"
                };
                graph.add_file(source, content);
            }

            assert_eq!(
                graph.depends_on("src/main.py"),
                vec!["src/utils.py".to_string()]
            );
            assert_eq!(
                graph.depended_on_by("src/utils.py"),
                vec!["src/main.py".to_string()]
            );
        }
    }

    #[test]
    fn leaves_unresolved_python_imports_as_external_modules() {
        let mut graph = DependencyGraph::new();

        graph.add_file("src/main.py", "import os\n");

        assert_eq!(graph.depends_on("src/main.py"), vec!["os".to_string()]);
    }

    #[test]
    fn test_transitive_dependencies() {
        let mut graph = DependencyGraph::new();

        graph.add_file("a.ts", "import { b } from './b';");
        graph.add_file("b.ts", "import { c } from './c';");
        graph.add_file("c.ts", "// no imports");

        let transitive = graph.transitive_dependencies("a.ts");
        // a.ts -> ./b (resolved) -> and b.ts -> ./c (resolved)
        // At minimum we expect the direct dependency to be found
        assert!(!transitive.is_empty());
    }

    #[test]
    fn test_rust_imports() {
        let mut graph = DependencyGraph::new();
        graph.add_file(
            "src/main.rs",
            "use crate::git::history;\nuse crate::graph::analyzer;\n",
        );
        let deps = graph.depends_on("src/main.rs");
        assert_eq!(deps.len(), 2);
    }

    #[test]
    fn test_stats() {
        let mut graph = DependencyGraph::new();
        graph.add_file("a.ts", "import { b } from './b';");
        graph.add_file("b.ts", "// nothing");
        let (nodes, edges) = graph.stats();
        // a.ts, b.ts, and resolved "./b" may create 3 nodes
        assert!(nodes >= 2);
        assert!(edges >= 1);
    }

    #[test]
    fn test_most_depended_on() {
        let mut graph = DependencyGraph::new();
        graph.add_file("a.ts", "import { utils } from './utils';");
        graph.add_file("b.ts", "import { utils } from './utils';");
        graph.add_file("c.ts", "import { utils } from './utils';");
        let top = graph.most_depended_on(5);
        assert!(!top.is_empty());
        assert!(top[0].1 >= 3);
    }
}
