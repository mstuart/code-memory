use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Information about a single import statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportInfo {
    pub source_file: String,
    pub imported_path: String,
    pub is_relative: bool,
    pub line_number: usize,
    pub import_type: ImportType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImportType {
    RustUse,
    RustMod,
    TsImport,
    JsRequire,
    PythonImport,
    PythonFrom,
    GoImport,
}

/// Parses import statements from source files to build dependency edges.
pub struct ImportParser {
    rust_use: Regex,
    rust_mod: Regex,
    ts_import: Regex,
    js_require: Regex,
    python_import: Regex,
    python_from: Regex,
    go_import: Regex,
}

impl Default for ImportParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ImportParser {
    pub fn new() -> Self {
        Self {
            rust_use: Regex::new(r"^\s*use\s+((?:crate|super|self)::[\w:]+|[\w:]+)").unwrap(),
            rust_mod: Regex::new(r"^\s*(?:pub\s+)?mod\s+(\w+)\s*;").unwrap(),
            ts_import: Regex::new(r#"^\s*import\s+.*?from\s+['"]([^'"]+)['"]"#).unwrap(),
            js_require: Regex::new(
                r#"(?:const|let|var)\s+.*?=\s*require\s*\(\s*['"]([^'"]+)['"]\s*\)"#,
            )
            .unwrap(),
            python_import: Regex::new(r"^\s*import\s+([\w.]+)").unwrap(),
            python_from: Regex::new(r"^\s*from\s+([\w.]+)\s+import").unwrap(),
            go_import: Regex::new(r#"^\s*"([^"]+)""#).unwrap(),
        }
    }

    /// Parse imports from a file's content based on its extension.
    pub fn parse(&self, file_path: &str, content: &str) -> Vec<ImportInfo> {
        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        match ext {
            "rs" => self.parse_rust(file_path, content),
            "ts" | "tsx" | "mts" | "cts" => self.parse_typescript(file_path, content),
            "js" | "jsx" | "mjs" | "cjs" => self.parse_javascript(file_path, content),
            "py" => self.parse_python(file_path, content),
            "go" => self.parse_go(file_path, content),
            _ => Vec::new(),
        }
    }

    fn parse_rust(&self, file_path: &str, content: &str) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            if let Some(cap) = self.rust_use.captures(line) {
                let path = cap[1].to_string();
                let is_relative = path.starts_with("crate::")
                    || path.starts_with("super::")
                    || path.starts_with("self::");
                imports.push(ImportInfo {
                    source_file: file_path.to_string(),
                    imported_path: path,
                    is_relative,
                    line_number: line_num + 1,
                    import_type: ImportType::RustUse,
                });
            }
            if let Some(cap) = self.rust_mod.captures(line) {
                imports.push(ImportInfo {
                    source_file: file_path.to_string(),
                    imported_path: cap[1].to_string(),
                    is_relative: true,
                    line_number: line_num + 1,
                    import_type: ImportType::RustMod,
                });
            }
        }
        imports
    }

    fn parse_typescript(&self, file_path: &str, content: &str) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            if let Some(cap) = self.ts_import.captures(line) {
                let path = cap[1].to_string();
                let is_relative = path.starts_with('.') || path.starts_with('/');
                imports.push(ImportInfo {
                    source_file: file_path.to_string(),
                    imported_path: path,
                    is_relative,
                    line_number: line_num + 1,
                    import_type: ImportType::TsImport,
                });
            }
        }
        imports
    }

    fn parse_javascript(&self, file_path: &str, content: &str) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            if let Some(cap) = self.ts_import.captures(line) {
                let path = cap[1].to_string();
                let is_relative = path.starts_with('.') || path.starts_with('/');
                imports.push(ImportInfo {
                    source_file: file_path.to_string(),
                    imported_path: path,
                    is_relative,
                    line_number: line_num + 1,
                    import_type: ImportType::TsImport,
                });
            }
            if let Some(cap) = self.js_require.captures(line) {
                let path = cap[1].to_string();
                let is_relative = path.starts_with('.') || path.starts_with('/');
                imports.push(ImportInfo {
                    source_file: file_path.to_string(),
                    imported_path: path,
                    is_relative,
                    line_number: line_num + 1,
                    import_type: ImportType::JsRequire,
                });
            }
        }
        imports
    }

    fn parse_python(&self, file_path: &str, content: &str) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        for (line_num, line) in content.lines().enumerate() {
            if let Some(cap) = self.python_from.captures(line) {
                let path = cap[1].to_string();
                let is_relative = path.starts_with('.');
                imports.push(ImportInfo {
                    source_file: file_path.to_string(),
                    imported_path: path,
                    is_relative,
                    line_number: line_num + 1,
                    import_type: ImportType::PythonFrom,
                });
            } else if let Some(cap) = self.python_import.captures(line) {
                let path = cap[1].to_string();
                imports.push(ImportInfo {
                    source_file: file_path.to_string(),
                    imported_path: path,
                    is_relative: false,
                    line_number: line_num + 1,
                    import_type: ImportType::PythonImport,
                });
            }
        }
        imports
    }

    fn parse_go(&self, file_path: &str, content: &str) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        let mut in_import_block = false;
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("import (") {
                in_import_block = true;
                continue;
            }
            if in_import_block && trimmed == ")" {
                in_import_block = false;
                continue;
            }
            if in_import_block || trimmed.starts_with("import \"") {
                if let Some(cap) = self.go_import.captures(line) {
                    let path = cap[1].to_string();
                    imports.push(ImportInfo {
                        source_file: file_path.to_string(),
                        imported_path: path,
                        is_relative: false,
                        line_number: line_num + 1,
                        import_type: ImportType::GoImport,
                    });
                }
            }
        }
        imports
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rust_use() {
        let parser = ImportParser::new();
        let content =
            "use crate::git::history;\nuse super::decisions;\nuse std::collections::HashMap;\n";
        let imports = parser.parse("src/main.rs", content);
        assert_eq!(imports.len(), 3);
        assert!(imports[0].is_relative);
        assert!(imports[1].is_relative);
        assert!(!imports[2].is_relative);
    }

    #[test]
    fn test_parse_rust_mod() {
        let parser = ImportParser::new();
        let content = "pub mod history;\nmod decisions;";
        let imports = parser.parse("src/git/mod.rs", content);
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].import_type, ImportType::RustMod);
    }

    #[test]
    fn test_parse_typescript() {
        let parser = ImportParser::new();
        let content = "import { Router } from 'express';\nimport { handler } from './handler';\nimport type { Config } from '../config';\n";
        let imports = parser.parse("src/index.ts", content);
        assert_eq!(imports.len(), 3);
        assert!(!imports[0].is_relative);
        assert!(imports[1].is_relative);
        assert!(imports[2].is_relative);
    }

    #[test]
    fn test_parse_javascript_require() {
        let parser = ImportParser::new();
        let content =
            "const express = require('express');\nconst handler = require('./handler');\n";
        let imports = parser.parse("src/index.js", content);
        assert_eq!(imports.len(), 2);
        assert!(!imports[0].is_relative);
        assert!(imports[1].is_relative);
    }

    #[test]
    fn test_parse_python() {
        let parser = ImportParser::new();
        let content = "import os\nfrom pathlib import Path\nfrom .utils import helper\n";
        let imports = parser.parse("src/main.py", content);
        assert_eq!(imports.len(), 3);
        assert_eq!(imports[0].import_type, ImportType::PythonImport);
        assert_eq!(imports[1].import_type, ImportType::PythonFrom);
        assert!(imports[2].is_relative);
    }

    #[test]
    fn test_parse_go() {
        let parser = ImportParser::new();
        let content = "package main\n\nimport (\n\t\"fmt\"\n\t\"net/http\"\n\t\"github.com/gin-gonic/gin\"\n)\n";
        let imports = parser.parse("main.go", content);
        assert_eq!(imports.len(), 3);
    }
}
