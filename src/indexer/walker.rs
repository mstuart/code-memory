use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use tracing::{debug, warn};

use super::code_index::CodeIndex;
use super::parser;

/// Stats from an indexing operation
#[derive(Debug)]
pub struct IndexStats {
    pub files_indexed: usize,
    pub files_skipped: usize,
    pub symbols_found: usize,
    pub elapsed_ms: u128,
}

/// Walk a project directory and index all code files.
/// Respects .gitignore via the `ignore` crate.
pub fn index_project(project_path: &Path, code_index: &CodeIndex) -> Result<IndexStats> {
    let start = std::time::Instant::now();
    let mut writer = code_index.writer()?;
    let schema = code_index.schema();

    let mut files_indexed = 0usize;
    let mut files_skipped = 0usize;
    let mut symbols_found = 0usize;

    let walker = WalkBuilder::new(project_path)
        .hidden(true) // skip hidden files
        .git_ignore(true) // respect .gitignore
        .git_global(true) // respect global gitignore
        .git_exclude(true) // respect .git/info/exclude
        .build();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                warn!("Walk error: {}", e);
                files_skipped += 1;
                continue;
            }
        };

        // Only process files
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let path = entry.path();

        // Detect language
        let language = match parser::detect_language(path) {
            Some(lang) => lang,
            None => {
                files_skipped += 1;
                continue;
            }
        };

        // Skip binary / very large files
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => {
                files_skipped += 1;
                continue;
            }
        };

        if metadata.len() > 1_000_000 {
            // Skip files > 1MB
            files_skipped += 1;
            continue;
        }

        // Read content
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => {
                // Likely a binary file
                files_skipped += 1;
                continue;
            }
        };

        // Extract symbols
        let symbols = parser::extract_symbols(&content, language);
        symbols_found += symbols.len();

        let symbol_names: String = symbols
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        // Get modification time
        let modified_time = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // Relative path for storage
        let rel_path = path
            .strip_prefix(project_path)
            .unwrap_or(path)
            .to_string_lossy();

        debug!(
            "Indexing: {} ({}, {} symbols)",
            rel_path,
            language,
            symbols.len()
        );

        CodeIndex::index_file(
            &writer,
            schema,
            &rel_path,
            &content,
            &symbol_names,
            language,
            modified_time,
        )?;

        files_indexed += 1;
    }

    writer.commit()?;

    let elapsed_ms = start.elapsed().as_millis();

    Ok(IndexStats {
        files_indexed,
        files_skipped,
        symbols_found,
        elapsed_ms,
    })
}

/// Get the storage path for a project's index
pub fn index_storage_path(project_path: &Path) -> PathBuf {
    let home = dirs::home_dir().expect("cannot determine home directory");
    let current_dir = home.join(".code-memory").join("index");
    let legacy_dir = home.join(".claude-context").join("index");

    // Use a hash of the project path as the directory name to avoid conflicts.
    let project_hash = simple_hash(&project_path.to_string_lossy());
    let leaf = format!("{:016x}", project_hash);
    let current_path = current_dir.join(&leaf);
    let legacy_path = legacy_dir.join(&leaf);

    if current_path.exists() || !legacy_path.exists() {
        current_path
    } else {
        legacy_path
    }
}

fn simple_hash(s: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_index_project() {
        let dir = TempDir::new().unwrap();
        let src_dir = dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();

        // Create some test files
        fs::write(src_dir.join("main.rs"), "pub fn main() {}\nstruct App {}\n").unwrap();
        fs::write(
            src_dir.join("lib.rs"),
            "pub fn helper() {}\npub enum Color { Red }\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("script.py"),
            "def hello():\n    pass\nclass World:\n    pass\n",
        )
        .unwrap();

        // Create index in temp dir
        let index_dir = dir.path().join("index");
        let code_index = CodeIndex::open_or_create(&index_dir).unwrap();

        let stats = index_project(dir.path(), &code_index).unwrap();

        assert!(stats.files_indexed >= 3); // 3 code files + possibly config files
        assert!(stats.symbols_found >= 5); // main, App, helper, Color, hello, World
    }

    #[test]
    fn test_index_storage_path() {
        let path = Path::new("/Users/test/myproject");
        let storage = index_storage_path(path);
        assert!(
            storage.to_string_lossy().contains(".code-memory")
                || storage.to_string_lossy().contains(".claude-context")
        );
        assert!(storage.to_string_lossy().contains("index"));
    }
}
