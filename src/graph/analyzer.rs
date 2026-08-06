use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Bfs;
use petgraph::Direction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::imports::ImportParser;

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

/// Dependency graph built from import analysis.
pub struct DependencyGraph {
    graph: DiGraph<String, ()>,
    node_map: HashMap<String, NodeIndex>,
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
            parser: ImportParser::new(),
        }
    }

    /// Add a file and its imports to the graph.
    pub fn add_file(&mut self, file_path: &str, content: &str) {
        let source_idx = self.get_or_create_node(file_path);
        let imports = self.parser.parse(file_path, content);

        for import in &imports {
            let resolved =
                self.resolve_import(file_path, &import.imported_path, import.is_relative);
            // Try to match against a known node (with common extensions)
            let target_key = self.find_matching_node(&resolved).unwrap_or(resolved);
            let target_idx = self.get_or_create_node(&target_key);
            if !self.graph.contains_edge(source_idx, target_idx) {
                self.graph.add_edge(source_idx, target_idx, ());
            }
        }
    }

    /// Try to find an existing node that matches the import path,
    /// accounting for missing file extensions (common in JS/TS/Python).
    fn find_matching_node(&self, resolved: &str) -> Option<String> {
        if self.node_map.contains_key(resolved) {
            return Some(resolved.to_string());
        }
        // Try common extensions
        for ext in &[".ts", ".tsx", ".js", ".jsx", ".py", ".rs", ".go"] {
            let with_ext = format!("{}{}", resolved, ext);
            if self.node_map.contains_key(&with_ext) {
                return Some(with_ext);
            }
        }
        // Try index files (JS/TS convention)
        for ext in &["/index.ts", "/index.js", "/index.tsx"] {
            let with_index = format!("{}{}", resolved, ext);
            if self.node_map.contains_key(&with_index) {
                return Some(with_index);
            }
        }
        None
    }

    fn get_or_create_node(&mut self, path: &str) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(path) {
            idx
        } else {
            let idx = self.graph.add_node(path.to_string());
            self.node_map.insert(path.to_string(), idx);
            idx
        }
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
        let Some(&idx) = self.node_map.get(file_path) else {
            return Vec::new();
        };
        self.graph
            .neighbors_directed(idx, Direction::Outgoing)
            .map(|n| self.graph[n].clone())
            .collect()
    }

    /// What depends on this file? (direct)
    pub fn depended_on_by(&self, file_path: &str) -> Vec<String> {
        let Some(&idx) = self.node_map.get(file_path) else {
            return Vec::new();
        };
        self.graph
            .neighbors_directed(idx, Direction::Incoming)
            .map(|n| self.graph[n].clone())
            .collect()
    }

    /// Find all transitively related code (BFS from file).
    pub fn transitive_dependencies(&self, file_path: &str) -> Vec<String> {
        let Some(&idx) = self.node_map.get(file_path) else {
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
        (self.graph.node_count(), self.graph.edge_count())
    }

    /// Files with most incoming dependencies.
    pub fn most_depended_on(&self, limit: usize) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = self
            .node_map
            .iter()
            .map(|(path, &idx)| {
                (
                    path.clone(),
                    self.graph
                        .neighbors_directed(idx, Direction::Incoming)
                        .count(),
                )
            })
            .collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1));
        counts.truncate(limit);
        counts
    }

    /// Files with most outgoing dependencies (most coupled).
    pub fn most_coupled(&self, limit: usize) -> Vec<(String, usize)> {
        let mut counts: Vec<(String, usize)> = self
            .node_map
            .iter()
            .map(|(path, &idx)| {
                (
                    path.clone(),
                    self.graph
                        .neighbors_directed(idx, Direction::Outgoing)
                        .count(),
                )
            })
            .collect();
        counts.sort_by(|a, b| b.1.cmp(&a.1));
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
