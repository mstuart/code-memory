use std::collections::HashMap;

use super::fulltext::SearchResult;
use super::semantic::SemanticResult;

/// A result that combines full-text and semantic search scores
#[derive(Debug, Clone)]
pub struct HybridResult {
    pub path: String,
    pub symbols: String,
    pub language: String,
    pub keyword_score: f32,
    pub semantic_score: f32,
    pub combined_score: f32,
}

/// Weight configuration for hybrid ranking
pub struct HybridWeights {
    /// Weight for keyword/full-text scores (0.0 - 1.0)
    pub keyword: f32,
    /// Weight for semantic similarity scores (0.0 - 1.0)
    pub semantic: f32,
}

impl Default for HybridWeights {
    fn default() -> Self {
        Self {
            keyword: 0.5,
            semantic: 0.5,
        }
    }
}

/// Merge full-text and semantic search results using reciprocal rank fusion
/// and weighted scoring.
pub fn merge_results(
    keyword_results: &[SearchResult],
    semantic_results: &[SemanticResult],
    weights: &HybridWeights,
    limit: usize,
) -> Vec<HybridResult> {
    let mut by_path: HashMap<String, HybridResult> = HashMap::new();

    // Normalize keyword scores to 0-1 range
    let max_keyword_score = keyword_results
        .iter()
        .map(|r| r.score)
        .fold(0.0f32, f32::max);
    let norm_factor = if max_keyword_score > 0.0 { max_keyword_score } else { 1.0 };

    // Add keyword results
    for result in keyword_results {
        let normalized_score = result.score / norm_factor;
        by_path.insert(
            result.path.clone(),
            HybridResult {
                path: result.path.clone(),
                symbols: result.symbols.clone(),
                language: result.language.clone(),
                keyword_score: normalized_score,
                semantic_score: 0.0,
                combined_score: normalized_score * weights.keyword,
            },
        );
    }

    // Add or merge semantic results
    for result in semantic_results {
        match by_path.get_mut(&result.path) {
            Some(existing) => {
                existing.semantic_score = result.similarity;
                existing.combined_score =
                    existing.keyword_score * weights.keyword
                    + result.similarity * weights.semantic;
            }
            None => {
                by_path.insert(
                    result.path.clone(),
                    HybridResult {
                        path: result.path.clone(),
                        symbols: result.symbols.clone(),
                        language: result.language.clone(),
                        keyword_score: 0.0,
                        semantic_score: result.similarity,
                        combined_score: result.similarity * weights.semantic,
                    },
                );
            }
        }
    }

    // Sort by combined score descending
    let mut results: Vec<HybridResult> = by_path.into_values().collect();
    results.sort_by(|a, b| {
        b.combined_score
            .partial_cmp(&a.combined_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    results.truncate(limit);

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_results_keyword_only() {
        let keyword = vec![
            SearchResult { path: "a.rs".into(), symbols: "foo".into(), language: "rust".into(), score: 10.0 },
            SearchResult { path: "b.rs".into(), symbols: "bar".into(), language: "rust".into(), score: 5.0 },
        ];
        let semantic: Vec<SemanticResult> = vec![];

        let results = merge_results(&keyword, &semantic, &HybridWeights::default(), 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path, "a.rs");
        assert!(results[0].combined_score > results[1].combined_score);
    }

    #[test]
    fn test_merge_results_semantic_only() {
        let keyword: Vec<SearchResult> = vec![];
        let semantic = vec![
            SemanticResult { path: "a.rs".into(), symbols: "foo".into(), language: "rust".into(), similarity: 0.95 },
            SemanticResult { path: "b.rs".into(), symbols: "bar".into(), language: "rust".into(), similarity: 0.5 },
        ];

        let results = merge_results(&keyword, &semantic, &HybridWeights::default(), 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path, "a.rs");
    }

    #[test]
    fn test_merge_results_hybrid() {
        let keyword = vec![
            SearchResult { path: "a.rs".into(), symbols: "foo".into(), language: "rust".into(), score: 10.0 },
            SearchResult { path: "b.rs".into(), symbols: "bar".into(), language: "rust".into(), score: 2.0 },
        ];
        let semantic = vec![
            SemanticResult { path: "b.rs".into(), symbols: "bar".into(), language: "rust".into(), similarity: 0.99 },
            SemanticResult { path: "c.rs".into(), symbols: "baz".into(), language: "rust".into(), similarity: 0.8 },
        ];

        let results = merge_results(&keyword, &semantic, &HybridWeights::default(), 10);
        assert_eq!(results.len(), 3); // a.rs, b.rs, c.rs
        // b.rs should score high because it appears in both
        let b_result = results.iter().find(|r| r.path == "b.rs").unwrap();
        assert!(b_result.keyword_score > 0.0);
        assert!(b_result.semantic_score > 0.0);
    }

    #[test]
    fn test_merge_results_limit() {
        let keyword = vec![
            SearchResult { path: "a.rs".into(), symbols: "".into(), language: "rust".into(), score: 10.0 },
            SearchResult { path: "b.rs".into(), symbols: "".into(), language: "rust".into(), score: 5.0 },
            SearchResult { path: "c.rs".into(), symbols: "".into(), language: "rust".into(), score: 1.0 },
        ];
        let semantic: Vec<SemanticResult> = vec![];

        let results = merge_results(&keyword, &semantic, &HybridWeights::default(), 2);
        assert_eq!(results.len(), 2);
    }
}
