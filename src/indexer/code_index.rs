use anyhow::Result;
use std::path::{Path, PathBuf};
use tantivy::schema::*;
use tantivy::{doc, Index, IndexWriter};
use tracing::info;

/// Schema fields for the code index
pub struct CodeSchema {
    pub path: Field,
    pub path_exact: Field,
    pub content: Field,
    pub symbols: Field,
    pub language: Field,
    pub modified_time: Field,
}

impl CodeSchema {
    pub fn build() -> (Schema, Self) {
        let mut builder = Schema::builder();

        let path = builder.add_text_field("path", TEXT | STORED);
        let path_exact = builder.add_text_field("path_exact", STRING);
        let content = builder.add_text_field("content", TEXT);
        let symbols = builder.add_text_field("symbols", TEXT | STORED);
        let language = builder.add_text_field("language", STRING | STORED);
        let modified_time = builder.add_i64_field("modified_time", INDEXED | STORED);

        let schema = builder.build();
        let code_schema = Self {
            path,
            path_exact,
            content,
            symbols,
            language,
            modified_time,
        };

        (schema, code_schema)
    }
}

/// Manages the tantivy full-text code index
pub struct CodeIndex {
    index: Index,
    schema: CodeSchema,
    /// Retained for diagnostics; not read on any current code path.
    #[allow(dead_code)]
    index_path: PathBuf,
}

impl CodeIndex {
    /// Create or open a code index at the given path
    pub fn open_or_create(index_path: &Path) -> Result<Self> {
        std::fs::create_dir_all(index_path)?;

        let (schema, code_schema) = CodeSchema::build();

        let index = if index_path.join("meta.json").exists() {
            info!("Opening existing index at {:?}", index_path);
            let existing = Index::open_in_dir(index_path)?;
            if existing.schema().get_field("path_exact").is_ok() {
                existing
            } else {
                info!(
                    "Rebuilding index at {:?} for the path-filter schema",
                    index_path
                );
                drop(existing);
                std::fs::remove_dir_all(index_path)?;
                std::fs::create_dir_all(index_path)?;
                Index::create_in_dir(index_path, schema.clone())?
            }
        } else {
            info!("Creating new index at {:?}", index_path);
            Index::create_in_dir(index_path, schema.clone())?
        };

        Ok(Self {
            index,
            schema: code_schema,
            index_path: index_path.to_path_buf(),
        })
    }

    /// Get an index writer with 50MB heap
    pub fn writer(&self) -> Result<IndexWriter> {
        Ok(self.index.writer(50_000_000)?)
    }

    /// Get a reader for searching
    pub fn reader(&self) -> Result<tantivy::IndexReader> {
        Ok(self.index.reader()?)
    }

    /// Get the schema fields
    pub fn schema(&self) -> &CodeSchema {
        &self.schema
    }

    /// Get the tantivy index
    pub fn index(&self) -> &Index {
        &self.index
    }

    /// Index a single file
    pub fn index_file(
        writer: &IndexWriter,
        schema: &CodeSchema,
        path: &str,
        content: &str,
        symbols: &str,
        language: &str,
        modified_time: i64,
    ) -> Result<()> {
        // Delete existing document for this path
        let path_term = tantivy::Term::from_field_text(schema.path_exact, path);
        writer.delete_term(path_term);

        // Add the new document
        writer.add_document(doc!(
            schema.path => path,
            schema.path_exact => path,
            schema.content => content,
            schema.symbols => symbols,
            schema.language => language,
            schema.modified_time => modified_time,
        ))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn rebuilds_an_index_without_the_exact_path_field() {
        let dir = TempDir::new().unwrap();
        let mut builder = Schema::builder();
        builder.add_text_field("path", TEXT | STORED);
        builder.add_text_field("content", TEXT);
        builder.add_text_field("symbols", TEXT | STORED);
        builder.add_text_field("language", STRING | STORED);
        builder.add_i64_field("modified_time", INDEXED | STORED);
        Index::create_in_dir(dir.path(), builder.build()).unwrap();

        let index = CodeIndex::open_or_create(dir.path()).unwrap();

        assert!(index.index().schema().get_field("path_exact").is_ok());
    }
}
