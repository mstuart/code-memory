use anyhow::Result;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, Occur, Query, QueryParser, RegexQuery, TermQuery};
use tantivy::schema::{IndexRecordOption, Value};
use tantivy::{Index, IndexReader, TantivyDocument, Term};

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
    pub path_exact: tantivy::schema::Field,
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
                path_exact: code_schema.path_exact,
                symbols: code_schema.symbols,
                language: code_schema.language,
            },
        })
    }

    /// Search the index with a query string
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.search_with_filters(query, limit, None, None)
    }

    /// Search the index while optionally restricting results to a path and language.
    pub fn search_with_filters(
        &self,
        query: &str,
        limit: usize,
        path: Option<&str>,
        language_filter: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let query = self.query_parser.parse_query(query)?;
        let normalized_path = path.map(normalize_path_filter);
        let mut clauses: Vec<(Occur, Box<dyn Query>)> = vec![(Occur::Must, query)];
        if let Some(path) = normalized_path.as_deref().filter(|path| !path.is_empty()) {
            let pattern = format!(r"{}(/.*)?", regex::escape(path));
            clauses.push((
                Occur::Must,
                Box::new(RegexQuery::from_pattern(&pattern, self.schema.path_exact)?),
            ));
        }
        if let Some(language) = language_filter {
            clauses.push((
                Occur::Must,
                Box::new(TermQuery::new(
                    Term::from_field_text(self.schema.language, &language.to_ascii_lowercase()),
                    IndexRecordOption::Basic,
                )),
            ));
        }
        let query = BooleanQuery::new(clauses);
        let searcher = self.reader.searcher();
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit).order_by_score())?;

        let mut results = Vec::with_capacity(top_docs.len());
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;

            let path = doc
                .get_first(self.schema.path)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let symbols = doc
                .get_first(self.schema.symbols)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let language = doc
                .get_first(self.schema.language)
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

fn normalize_path_filter(path: &str) -> String {
    let path = path
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_matches('/')
        .to_string();
    if path == "." {
        String::new()
    } else {
        path
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
        )
        .unwrap();
        fs::write(
            src_dir.join("database.rs"),
            "pub fn connect_database(url: &str) -> Result<(), String> {\n    Ok(())\n}\n\
             pub struct DatabasePool {\n    pub connections: Vec<String>,\n}\n",
        )
        .unwrap();
        fs::write(
            src_dir.join("api.rs"),
            "pub fn handle_request() {}\npub fn parse_json() {}\n",
        )
        .unwrap();
        fs::write(
            src_dir.join("main.ts"),
            "export function connectDatabase() { return 'database'; }\n",
        )
        .unwrap();
        let test_dir = dir.path().join("tests");
        fs::create_dir_all(&test_dir).unwrap();
        fs::write(
            test_dir.join("database.rs"),
            "pub fn test_database_connection() {}\n",
        )
        .unwrap();

        // Index the project
        let index_dir = dir.path().join("index");
        let code_index = CodeIndex::open_or_create(&index_dir).unwrap();
        let stats = walker::index_project(dir.path(), &code_index).unwrap();
        assert!(stats.files_indexed >= 3);

        // Create search engine
        let search = FullTextSearch::new(code_index.index(), code_index.schema()).unwrap();

        // Search for "authenticate"
        let results = search.search("authenticate", 10).unwrap();
        assert!(
            !results.is_empty(),
            "Should find results for 'authenticate'"
        );
        assert!(
            results[0].path.contains("auth"),
            "Top result should be auth.rs"
        );

        // Search for "database"
        let results = search.search("database", 10).unwrap();
        assert!(!results.is_empty(), "Should find results for 'database'");
        assert!(
            results[0].path.contains("database"),
            "Top result should be database.rs"
        );

        let results = search
            .search_with_filters("database", 10, None, Some("typescript"))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "src/main.ts");

        let results = search
            .search_with_filters("database", 10, Some("tests"), None)
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "tests/database.rs");

        let results = search
            .search_with_filters("database", 10, Some("./src/database.rs/"), Some("rust"))
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "src/database.rs");

        assert!(search
            .search_with_filters("database", 0, Some("src"), None)
            .unwrap()
            .is_empty());
        assert_eq!(
            search
                .search_with_filters("database", 10, Some("."), None)
                .unwrap()
                .len(),
            3
        );

        // Search for symbol
        let results = search.search("symbols:AuthToken", 10).unwrap();
        assert!(!results.is_empty(), "Should find AuthToken symbol");

        // Search with no results
        let results = search.search("nonexistent_xyz_42", 10).unwrap();
        assert!(results.is_empty(), "Should find no results for gibberish");
    }
}
