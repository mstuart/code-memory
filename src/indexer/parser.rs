use std::path::Path;

/// Represents a parsed code symbol
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Class,
    Struct,
    Enum,
    Trait,
    Type,
    Constant,
    Module,
}

/// Detect language from file extension
pub fn detect_language(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()? {
        "rs" => Some("rust"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" => Some("javascript"),
        "py" => Some("python"),
        "go" => Some("go"),
        "java" => Some("java"),
        "c" | "h" => Some("c"),
        "cpp" | "cc" | "cxx" | "hpp" => Some("cpp"),
        "rb" => Some("ruby"),
        "swift" => Some("swift"),
        "kt" | "kts" => Some("kotlin"),
        "zig" => Some("zig"),
        "toml" | "yaml" | "yml" | "json" => Some("config"),
        "md" => Some("markdown"),
        "sh" | "bash" | "zsh" => Some("shell"),
        _ => None,
    }
}

/// Extract symbols from source code using simple pattern matching.
/// (Tree-sitter integration will be added later for more precision.)
pub fn extract_symbols(content: &str, language: &str) -> Vec<Symbol> {
    let mut symbols = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        match language {
            "rust" => extract_rust_symbols(trimmed, line_num, &mut symbols),
            "typescript" | "javascript" => extract_ts_symbols(trimmed, line_num, &mut symbols),
            "python" => extract_python_symbols(trimmed, line_num, &mut symbols),
            _ => {}
        }
    }

    symbols
}

fn extract_rust_symbols(line: &str, line_num: usize, symbols: &mut Vec<Symbol>) {
    if let Some(name) = extract_after_keyword(line, "fn ") {
        symbols.push(Symbol { name, kind: SymbolKind::Function, line: line_num });
    } else if let Some(name) = extract_after_keyword(line, "struct ") {
        symbols.push(Symbol { name, kind: SymbolKind::Struct, line: line_num });
    } else if let Some(name) = extract_after_keyword(line, "enum ") {
        symbols.push(Symbol { name, kind: SymbolKind::Enum, line: line_num });
    } else if let Some(name) = extract_after_keyword(line, "trait ") {
        symbols.push(Symbol { name, kind: SymbolKind::Trait, line: line_num });
    } else if let Some(name) = extract_after_keyword(line, "type ") {
        symbols.push(Symbol { name, kind: SymbolKind::Type, line: line_num });
    } else if let Some(name) = extract_after_keyword(line, "mod ") {
        symbols.push(Symbol { name, kind: SymbolKind::Module, line: line_num });
    } else if let Some(name) = extract_after_keyword(line, "const ") {
        symbols.push(Symbol { name, kind: SymbolKind::Constant, line: line_num });
    }
}

fn extract_ts_symbols(line: &str, line_num: usize, symbols: &mut Vec<Symbol>) {
    if let Some(name) = extract_after_keyword(line, "function ") {
        symbols.push(Symbol { name, kind: SymbolKind::Function, line: line_num });
    } else if let Some(name) = extract_after_keyword(line, "class ") {
        symbols.push(Symbol { name, kind: SymbolKind::Class, line: line_num });
    } else if let Some(name) = extract_after_keyword(line, "interface ") {
        symbols.push(Symbol { name, kind: SymbolKind::Type, line: line_num });
    } else if let Some(name) = extract_after_keyword(line, "type ") {
        symbols.push(Symbol { name, kind: SymbolKind::Type, line: line_num });
    } else if let Some(name) = extract_after_keyword(line, "enum ") {
        symbols.push(Symbol { name, kind: SymbolKind::Enum, line: line_num });
    }
}

fn extract_python_symbols(line: &str, line_num: usize, symbols: &mut Vec<Symbol>) {
    if let Some(name) = extract_after_keyword(line, "def ") {
        symbols.push(Symbol { name, kind: SymbolKind::Function, line: line_num });
    } else if let Some(name) = extract_after_keyword(line, "class ") {
        symbols.push(Symbol { name, kind: SymbolKind::Class, line: line_num });
    }
}

/// Extract an identifier following a keyword
fn extract_after_keyword(line: &str, keyword: &str) -> Option<String> {
    let rest = line.strip_prefix("pub ")
        .or_else(|| line.strip_prefix("pub(crate) "))
        .or_else(|| line.strip_prefix("export "))
        .or_else(|| line.strip_prefix("async "))
        .unwrap_or(line);

    if let Some(after) = rest.strip_prefix(keyword) {
        let name: String = after.chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if !name.is_empty() {
            return Some(name);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("main.rs")), Some("rust"));
        assert_eq!(detect_language(Path::new("app.ts")), Some("typescript"));
        assert_eq!(detect_language(Path::new("util.py")), Some("python"));
        assert_eq!(detect_language(Path::new("no_ext")), None);
    }

    #[test]
    fn test_extract_rust_symbols() {
        let code = r#"
pub fn hello_world() {}
struct MyStruct {}
pub enum Color { Red, Blue }
trait Drawable {}
"#;
        let symbols = extract_symbols(code, "rust");
        assert_eq!(symbols.len(), 4);
        assert_eq!(symbols[0].name, "hello_world");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert_eq!(symbols[1].name, "MyStruct");
        assert_eq!(symbols[2].name, "Color");
        assert_eq!(symbols[3].name, "Drawable");
    }

    #[test]
    fn test_extract_python_symbols() {
        let code = r#"
def process_data():
    pass
class DataProcessor:
    pass
"#;
        let symbols = extract_symbols(code, "python");
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "process_data");
        assert_eq!(symbols[1].name, "DataProcessor");
    }
}
