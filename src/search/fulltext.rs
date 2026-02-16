use anyhow::Result;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Value;
use tantivy::{Index, IndexReader, TantivyDocument};

use crate::indexer::code_index::CodeSchema;

/// Result from a full-text search
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: String,
    pub symbols: String,
    pub language: String,
    pub score: f32,
}

/// Full-text search engine backed by tantivy
pub struct FullTextSearch {
    reader: IndexReader,
    query_parser: QueryParser,
    schema: SchemaRef,
}

/// Reference to schema fields for result extraction
pub struct SchemaRef {
    pub path: tantivy::schema::Field,
    pub symbols: tantivy::schema::Field,
    pub language: tantivy::schema::Field,
}

impl FullTextSearch {
    pub fn new(index: &Index, code_schema: &CodeSchema) -> Result<Self> {
        let reader = index.reader()?;
        let query_parser = QueryParser::for_index(
            index,
            vec![code_schema.content, code_schema.symbols, code_schema.path],
        );

        Ok(Self {
            reader,
            query_parser,
            schema: SchemaRef {
                path: code_schema.path,
                symbols: code_schema.symbols,
                language: code_schema.language,
            },
        })
    }

    /// Search the index with a query string
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let query = self.query_parser.parse_query(query)?;
        let searcher = self.reader.searcher();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::with_capacity(top_docs.len());
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;

            let path = doc.get_first(self.schema.path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let symbols = doc.get_first(self.schema.symbols)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let language = doc.get_first(self.schema.language)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            results.push(SearchResult {
                path,
                symbols,
                language,
                score,
            });
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::indexer::code_index::CodeIndex;
    use crate::indexer::walker;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_fulltext_search_end_to_end() {
        let dir = TempDir::new().unwrap();
        let src_dir = dir.path().join("src");
        fs::create_dir_all(&src_dir).unwrap();

        fs::write(
            src_dir.join("auth.rs"),
            "pub fn authenticate_user(username: &str, password: &str) -> bool {\n    true\n}\n\
             pub struct AuthToken {\n    pub token: String,\n}\n",
        ).unwrap();
        fs::write(
            src_dir.join("database.rs"),
            "pub fn connect_database(url: &str) -> Result<(), String> {\n    Ok(())\n}\n\
             pub struct DatabasePool {\n    pub connections: Vec<String>,\n}\n",
        ).unwrap();
        fs::write(
            src_dir.join("api.rs"),
            "pub fn handle_request() {}\npub fn parse_json() {}\n",
        ).unwrap();

        // Index the project
        let index_dir = dir.path().join("index");
        let code_index = CodeIndex::open_or_create(&index_dir).unwrap();
        let stats = walker::index_project(dir.path(), &code_index).unwrap();
        assert!(stats.files_indexed >= 3);

        // Create search engine
        let search = FullTextSearch::new(code_index.index(), code_index.schema()).unwrap();

        // Search for "authenticate"
        let results = search.search("authenticate", 10).unwrap();
        assert!(!results.is_empty(), "Should find results for 'authenticate'");
        assert!(results[0].path.contains("auth"), "Top result should be auth.rs");

        // Search for "database"
        let results = search.search("database", 10).unwrap();
        assert!(!results.is_empty(), "Should find results for 'database'");
        assert!(results[0].path.contains("database"), "Top result should be database.rs");

        // Search for symbol
        let results = search.search("symbols:AuthToken", 10).unwrap();
        assert!(!results.is_empty(), "Should find AuthToken symbol");

        // Search with no results
        let results = search.search("nonexistent_xyz_42", 10).unwrap();
        assert!(results.is_empty(), "Should find no results for gibberish");
    }
}
