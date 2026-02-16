use regex::Regex;
use serde::{Deserialize, Serialize};

use super::history::CommitInfo;

/// Types of decisions found in commit history.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DecisionType {
    Explicit,
    Choice,
    Rationale,
    Why,
    Architecture,
    Refactor,
    Breaking,
}

/// A decision extracted from git history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub decision_type: DecisionType,
    pub summary: String,
    pub details: String,
    pub commit_id: String,
    pub author: String,
    pub timestamp: i64,
    pub files_affected: Vec<String>,
    pub confidence: f32,
}

/// Extracts decisions from commit messages.
pub struct DecisionExtractor {
    patterns: Vec<DecisionPattern>,
}

struct DecisionPattern {
    regex: Regex,
    decision_type: DecisionType,
    confidence: f32,
}

impl DecisionExtractor {
    pub fn new() -> Self {
        let patterns = vec![
            DecisionPattern {
                regex: Regex::new(r"(?i)decision:\s*(.+)").unwrap(),
                decision_type: DecisionType::Explicit,
                confidence: 1.0,
            },
            DecisionPattern {
                regex: Regex::new(r"(?i)chose\s+(.+?)\s+over\s+(.+?)(?:\.|$|\n)").unwrap(),
                decision_type: DecisionType::Choice,
                confidence: 0.95,
            },
            DecisionPattern {
                regex: Regex::new(r"(?i)rationale:\s*(.+)").unwrap(),
                decision_type: DecisionType::Rationale,
                confidence: 0.9,
            },
            DecisionPattern {
                regex: Regex::new(r"(?i)why:\s*(.+)").unwrap(),
                decision_type: DecisionType::Why,
                confidence: 0.85,
            },
            DecisionPattern {
                regex: Regex::new(r"(?i)(?:decided|choosing|opted)\s+(?:to\s+)?(.+?)(?:\.|$|\n)").unwrap(),
                decision_type: DecisionType::Explicit,
                confidence: 0.7,
            },
            DecisionPattern {
                regex: Regex::new(r"(?i)(?:refactor|restructure|reorganize)\s+(.+?)(?:\.|$|\n)").unwrap(),
                decision_type: DecisionType::Refactor,
                confidence: 0.6,
            },
            DecisionPattern {
                regex: Regex::new(r"(?i)BREAKING(?:\s+CHANGE)?:\s*(.+)").unwrap(),
                decision_type: DecisionType::Breaking,
                confidence: 0.95,
            },
            DecisionPattern {
                regex: Regex::new(r"(?i)(?:architect|design)\s+(.+?)(?:\.|$|\n)").unwrap(),
                decision_type: DecisionType::Architecture,
                confidence: 0.6,
            },
            DecisionPattern {
                regex: Regex::new(r"(?i)because\s+(.+?)(?:\.|$|\n)").unwrap(),
                decision_type: DecisionType::Why,
                confidence: 0.5,
            },
        ];

        Self { patterns }
    }

    /// Extract decisions from a commit.
    pub fn extract(&self, commit: &CommitInfo) -> Vec<Decision> {
        let mut decisions = Vec::new();
        let message = &commit.message;

        for pattern in &self.patterns {
            for captures in pattern.regex.captures_iter(message) {
                let summary = captures
                    .get(1)
                    .map(|m| m.as_str().trim().to_string())
                    .unwrap_or_default();

                if summary.is_empty() || summary.len() < 3 {
                    continue;
                }

                let details = if let Some(alt) = captures.get(2) {
                    format!("{} (alternative: {})", summary, alt.as_str().trim())
                } else {
                    summary.clone()
                };

                decisions.push(Decision {
                    decision_type: pattern.decision_type.clone(),
                    summary,
                    details,
                    commit_id: commit.id.clone(),
                    author: commit.author.clone(),
                    timestamp: commit.timestamp,
                    files_affected: commit.files_changed.clone(),
                    confidence: pattern.confidence,
                });
            }
        }

        decisions.dedup_by(|a, b| a.summary == b.summary);
        decisions
    }

    /// Extract decisions from multiple commits and rank by confidence.
    pub fn extract_ranked(&self, commits: &[CommitInfo]) -> Vec<Decision> {
        let mut all: Vec<Decision> = commits
            .iter()
            .flat_map(|c| self.extract(c))
            .collect();

        all.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_commit(message: &str) -> CommitInfo {
        CommitInfo {
            id: "abc123".to_string(),
            message: message.to_string(),
            author: "test".to_string(),
            author_email: "test@test.com".to_string(),
            timestamp: 1700000000,
            files_changed: vec!["src/main.rs".to_string()],
            insertions: 10,
            deletions: 5,
        }
    }

    #[test]
    fn test_explicit_decision() {
        let extractor = DecisionExtractor::new();
        let commit = make_commit("decision: use SQLite for local storage");
        let decisions = extractor.extract(&commit);
        assert!(!decisions.is_empty());
        assert_eq!(decisions[0].decision_type, DecisionType::Explicit);
        assert!(decisions[0].summary.contains("SQLite"));
    }

    #[test]
    fn test_choice_decision() {
        let extractor = DecisionExtractor::new();
        let commit = make_commit("chose Rust over Go for performance reasons");
        let decisions = extractor.extract(&commit);
        assert!(!decisions.is_empty());
        assert_eq!(decisions[0].decision_type, DecisionType::Choice);
    }

    #[test]
    fn test_rationale() {
        let extractor = DecisionExtractor::new();
        let commit = make_commit("refactored auth module\n\nrationale: reduce coupling between services");
        let decisions = extractor.extract(&commit);
        let rationale = decisions.iter().find(|d| d.decision_type == DecisionType::Rationale);
        assert!(rationale.is_some());
    }

    #[test]
    fn test_breaking_change() {
        let extractor = DecisionExtractor::new();
        let commit = make_commit("BREAKING CHANGE: remove deprecated API endpoints");
        let decisions = extractor.extract(&commit);
        let breaking = decisions.iter().find(|d| d.decision_type == DecisionType::Breaking);
        assert!(breaking.is_some());
    }

    #[test]
    fn test_why_pattern() {
        let extractor = DecisionExtractor::new();
        let commit = make_commit("why: existing solution had O(n^2) complexity");
        let decisions = extractor.extract(&commit);
        assert!(!decisions.is_empty());
        assert_eq!(decisions[0].decision_type, DecisionType::Why);
    }

    #[test]
    fn test_no_decision_in_simple_commit() {
        let extractor = DecisionExtractor::new();
        let commit = make_commit("fix typo");
        let decisions = extractor.extract(&commit);
        assert!(decisions.is_empty());
    }

    #[test]
    fn test_ranked_extraction() {
        let extractor = DecisionExtractor::new();
        let commits = vec![
            make_commit("decision: use async runtime"),
            make_commit("because it was slow"),
            make_commit("chose tokio over async-std for ecosystem support"),
        ];
        let ranked = extractor.extract_ranked(&commits);
        assert!(ranked.len() >= 2);
        assert!(ranked[0].confidence >= ranked.last().unwrap().confidence);
    }
}
