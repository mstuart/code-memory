use anyhow::Result;
use std::path::{Path, PathBuf};
use tantivy::schema::*;
use tantivy::{doc, Index, IndexWriter};
use tracing::info;

/// Schema fields for the code index
pub struct CodeSchema {
    pub path: Field,
    pub content: Field,
    pub symbols: Field,
    pub language: Field,
    pub modified_time: Field,
}

impl CodeSchema {
    pub fn build() -> (Schema, Self) {
        let mut builder = Schema::builder();

        let path = builder.add_text_field("path", TEXT | STORED);
        let content = builder.add_text_field("content", TEXT);
        let symbols = builder.add_text_field("symbols", TEXT | STORED);
        let language = builder.add_text_field("language", STRING | STORED);
        let modified_time = builder.add_i64_field("modified_time", INDEXED | STORED);

        let schema = builder.build();
        let code_schema = Self {
            path,
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
            Index::open_in_dir(index_path)?
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
        let path_term = tantivy::Term::from_field_text(schema.path, path);
        writer.delete_term(path_term);

        // Add the new document
        writer.add_document(doc!(
            schema.path => path,
            schema.content => content,
            schema.symbols => symbols,
            schema.language => language,
            schema.modified_time => modified_time,
        ))?;

        Ok(())
    }
}
