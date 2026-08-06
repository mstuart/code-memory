use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of patterns learned from sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PatternType {
    NamingConvention,
    ErrorHandling,
    TestingStyle,
    CodeOrganization,
    ImportStyle,
    ConfigPreference,
}

/// A learned pattern with observation count and confidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub pattern_type: PatternType,
    pub key: String,
    pub description: String,
    pub observation_count: usize,
    pub confidence: f32,
    pub examples: Vec<String>,
}

/// Library of patterns learned from Claude Code sessions.
pub struct PatternLibrary {
    observations: HashMap<String, usize>,
    patterns: Vec<Pattern>,
}

impl Default for PatternLibrary {
    fn default() -> Self {
        Self::new()
    }
}

impl PatternLibrary {
    pub fn new() -> Self {
        Self {
            observations: HashMap::new(),
            patterns: Vec::new(),
        }
    }

    /// Record an observation of a pattern key.
    pub fn observe(&mut self, key: &str) {
        *self.observations.entry(key.to_string()).or_insert(0) += 1;
    }

    /// Record an observation with an example.
    pub fn observe_with_example(&mut self, key: &str, example: &str) {
        self.observe(key);

        let count = *self.observations.get(key).unwrap_or(&1);
        let confidence = Self::calc_confidence(count);

        if let Some(pattern) = self.patterns.iter_mut().find(|p| p.key == key) {
            if pattern.examples.len() < 10 {
                pattern.examples.push(example.to_string());
            }
            pattern.observation_count = count;
            pattern.confidence = confidence;
        } else {
            let (pattern_type, description) = self.classify_pattern(key);
            self.patterns.push(Pattern {
                pattern_type,
                key: key.to_string(),
                description,
                observation_count: 1,
                confidence: 0.1,
                examples: vec![example.to_string()],
            });
        }
    }

    fn classify_pattern(&self, key: &str) -> (PatternType, String) {
        if key.starts_with("naming:") {
            (
                PatternType::NamingConvention,
                format!("Naming convention: {}", key),
            )
        } else if key.starts_with("error:") {
            (
                PatternType::ErrorHandling,
                format!("Error handling: {}", key),
            )
        } else if key.starts_with("test:") {
            (PatternType::TestingStyle, format!("Testing style: {}", key))
        } else if key.starts_with("import:") {
            (PatternType::ImportStyle, format!("Import style: {}", key))
        } else if key.starts_with("config:") {
            (
                PatternType::ConfigPreference,
                format!("Config preference: {}", key),
            )
        } else {
            (
                PatternType::CodeOrganization,
                format!("Code pattern: {}", key),
            )
        }
    }

    fn calc_confidence(count: usize) -> f32 {
        1.0 - (1.0 / (1.0 + (count as f32 / 5.0)))
    }

    /// Get all patterns above a confidence threshold.
    pub fn confident_patterns(&self, min_confidence: f32) -> Vec<&Pattern> {
        self.patterns
            .iter()
            .filter(|p| p.confidence >= min_confidence)
            .collect()
    }

    /// Get all patterns of a specific type.
    pub fn patterns_of_type(&self, pattern_type: &PatternType) -> Vec<&Pattern> {
        self.patterns
            .iter()
            .filter(|p| &p.pattern_type == pattern_type)
            .collect()
    }

    /// Get the top N most observed patterns.
    pub fn top_patterns(&self, n: usize) -> Vec<&Pattern> {
        let mut sorted: Vec<&Pattern> = self.patterns.iter().collect();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.observation_count));
        sorted.truncate(n);
        sorted
    }

    /// Get raw observation counts.
    pub fn observations(&self) -> &HashMap<String, usize> {
        &self.observations
    }

    /// Get all patterns.
    pub fn all_patterns(&self) -> &[Pattern] {
        &self.patterns
    }

    /// Serialize the library to JSON.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "patterns": self.patterns,
            "observation_count": self.observations.len(),
            "total_observations": self.observations.values().sum::<usize>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observe_and_retrieve() {
        let mut lib = PatternLibrary::new();
        lib.observe_with_example("naming:rust:snake_case", "fn my_function()");
        lib.observe_with_example("naming:rust:snake_case", "let my_var = 1");
        assert_eq!(lib.observations().get("naming:rust:snake_case"), Some(&2));
    }

    #[test]
    fn test_confidence_increases() {
        let mut lib = PatternLibrary::new();
        for i in 0..10 {
            lib.observe_with_example("test:pattern", &format!("example_{}", i));
        }
        let patterns = lib.confident_patterns(0.5);
        assert!(!patterns.is_empty());
        assert!(patterns[0].confidence > 0.5);
    }

    #[test]
    fn test_patterns_of_type() {
        let mut lib = PatternLibrary::new();
        lib.observe_with_example("naming:rust:snake_case", "fn foo_bar()");
        lib.observe_with_example("error:result:unwrap", "x.unwrap()");

        let naming = lib.patterns_of_type(&PatternType::NamingConvention);
        assert_eq!(naming.len(), 1);

        let errors = lib.patterns_of_type(&PatternType::ErrorHandling);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_top_patterns() {
        let mut lib = PatternLibrary::new();
        for _ in 0..10 {
            lib.observe_with_example("naming:rust:snake_case", "example");
        }
        for _ in 0..5 {
            lib.observe_with_example("error:result", "example");
        }
        lib.observe_with_example("test:unit", "example");

        let top = lib.top_patterns(2);
        assert_eq!(top.len(), 2);
        assert_eq!(top[0].key, "naming:rust:snake_case");
    }

    #[test]
    fn test_to_json() {
        let mut lib = PatternLibrary::new();
        lib.observe_with_example("naming:rust:snake_case", "fn foo()");
        let json = lib.to_json();
        assert!(json.get("patterns").is_some());
        assert_eq!(json["observation_count"], 1);
    }
}
