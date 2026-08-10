use anyhow::{Context, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use tracing::info;

/// A single entry in the vector store
#[derive(Debug, Clone)]
pub struct VectorEntry {
    pub path: String,
    pub symbols: String,
    pub language: String,
    pub embedding: Vec<f32>,
}

/// Result from a semantic search
#[derive(Debug, Clone)]
pub struct SemanticResult {
    pub path: String,
    pub symbols: String,
    pub language: String,
    pub similarity: f32,
}

/// Semantic search engine using fastembed for local embeddings
pub struct SemanticSearch {
    model: TextEmbedding,
    entries: Vec<VectorEntry>,
}

impl SemanticSearch {
    /// Create a new semantic search engine (downloads model on first use)
    pub fn new() -> Result<Self> {
        info!("Initializing fastembed model (all-MiniLM-L6-v2)...");
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(true),
        )
        .context("Failed to initialize embedding model")?;

        info!("Embedding model loaded successfully");
        Ok(Self {
            model,
            entries: Vec::new(),
        })
    }

    /// Generate embeddings for a list of code entries and add them to the store
    pub fn index_entries(
        &mut self,
        entries: Vec<(String, String, String, String)>,
    ) -> Result<usize> {
        if entries.is_empty() {
            return Ok(0);
        }

        // Prepare texts for embedding: combine path + symbols + content snippet
        let texts: Vec<String> = entries
            .iter()
            .map(|(path, symbols, _lang, content)| {
                // Use first 512 chars of content + symbols for embedding
                let snippet: String = content.chars().take(512).collect();
                format!("passage: {} {} {}", path, symbols, snippet)
            })
            .collect();

        // Generate embeddings in batches
        let batch_size = 64;
        let mut all_embeddings = Vec::with_capacity(texts.len());

        for chunk in texts.chunks(batch_size) {
            let chunk_refs: Vec<&str> = chunk.iter().map(|s| s.as_str()).collect();
            let embeddings = self
                .model
                .embed(chunk_refs, None)
                .context("Failed to generate embeddings")?;
            all_embeddings.extend(embeddings);
        }

        // Store entries with their embeddings
        let count = entries.len();
        for (i, (path, symbols, language, _content)) in entries.into_iter().enumerate() {
            self.entries.push(VectorEntry {
                path,
                symbols,
                language,
                embedding: all_embeddings[i].clone(),
            });
        }

        info!("Indexed {} entries with semantic embeddings", count);
        Ok(count)
    }

    /// Search for similar code using a natural language query
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SemanticResult>> {
        if self.entries.is_empty() {
            return Ok(Vec::new());
        }

        // Generate query embedding
        let query_text = format!("query: {}", query);
        let query_embeddings = self
            .model
            .embed(vec![query_text.as_str()], None)
            .context("Failed to embed query")?;
        let query_embedding = &query_embeddings[0];

        // Compute cosine similarity with all entries
        let mut scored: Vec<(usize, f32)> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let sim = cosine_similarity(query_embedding, &entry.embedding);
                (i, sim)
            })
            .collect();

        // Sort by similarity (descending)
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        let results = scored
            .into_iter()
            .filter(|(_, sim)| *sim > 0.0)
            .map(|(idx, sim)| {
                let entry = &self.entries[idx];
                SemanticResult {
                    path: entry.path.clone(),
                    symbols: entry.symbols.clone(),
                    language: entry.language.clone(),
                    similarity: sim,
                }
            })
            .collect();

        Ok(results)
    }

    /// Get the number of indexed entries
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for i in 0..a.len() {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c)).abs() < 0.001);

        let d = vec![-1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &d) - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_empty() {
        assert_eq!(cosine_similarity(&[], &[]), 0.0);
        assert_eq!(cosine_similarity(&[1.0], &[1.0, 2.0]), 0.0);
    }

    // Integration test for semantic search (requires model download)
    // Run with: cargo test test_semantic_search -- --ignored
    #[test]
    #[ignore]
    fn test_semantic_search_integration() {
        let mut search = SemanticSearch::new().unwrap();

        let entries = vec![
            (
                "src/auth.rs".to_string(),
                "authenticate_user verify_token".to_string(),
                "rust".to_string(),
                "pub fn authenticate_user(username: &str, password: &str) -> bool { true }"
                    .to_string(),
            ),
            (
                "src/db.rs".to_string(),
                "connect_database query_users".to_string(),
                "rust".to_string(),
                "pub fn connect_database(url: &str) -> Result<Pool> { Ok(pool) }".to_string(),
            ),
            (
                "src/api.rs".to_string(),
                "handle_request parse_json".to_string(),
                "rust".to_string(),
                "pub fn handle_request(req: Request) -> Response { todo!() }".to_string(),
            ),
        ];

        let count = search.index_entries(entries).unwrap();
        assert_eq!(count, 3);

        let results = search.search("user authentication login", 10).unwrap();
        assert!(!results.is_empty());
        // The auth file should rank highest for an auth-related query
        assert!(
            results[0].path.contains("auth"),
            "Expected auth.rs as top result, got: {}",
            results[0].path
        );
    }
}
