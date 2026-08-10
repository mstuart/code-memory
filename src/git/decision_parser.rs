use regex::Regex;

#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub decision_type: String,
    pub from: Option<String>,
    pub to: String,
    pub reasoning: String,
    pub commit_sha: Option<String>,
    pub author: Option<String>,
    pub timestamp: Option<u64>,
}

pub struct DecisionParser {
    decision_keywords: Vec<Regex>,
}

impl Default for DecisionParser {
    fn default() -> Self {
        Self::new()
    }
}

impl DecisionParser {
    pub fn new() -> Self {
        let keywords = [
            r"decided to",
            r"chose",
            r"migrat(?:ed|ing) from",
            r"switch(?:ed|ing) to",
            r"moving to",
        ];

        let decision_keywords = keywords.iter().map(|k| Regex::new(k).unwrap()).collect();

        Self { decision_keywords }
    }

    pub fn parse_message(&self, message: &str) -> Vec<Decision> {
        let mut decisions = Vec::new();

        // Check if message contains decision keywords
        let has_decision = self
            .decision_keywords
            .iter()
            .any(|re| re.is_match(&message.to_lowercase()));

        if !has_decision {
            return decisions;
        }

        // Extract decision type
        let decision_type = if message.contains("migrat") {
            "migration"
        } else if message.contains("arch:") || message.contains("architecture") {
            "architecture"
        } else if message.contains("refactor:") {
            "refactoring"
        } else {
            "general"
        };

        // Extract from/to for migrations
        let (from, to) = if decision_type == "migration" {
            extract_migration_pair(message)
        } else {
            (None, extract_technology(message))
        };

        // Extract reasoning
        let reasoning = extract_reasoning(message);

        decisions.push(Decision {
            decision_type: decision_type.to_string(),
            from,
            to,
            reasoning,
            commit_sha: None,
            author: None,
            timestamp: None,
        });

        decisions
    }
}

fn extract_migration_pair(message: &str) -> (Option<String>, String) {
    // Pattern: "from X to Y" or "migrate from X to Y"
    let from_to_re = Regex::new(r"from\s+(\w+)\s+to\s+(\w+)").unwrap();

    if let Some(caps) = from_to_re.captures(message) {
        let from = caps.get(1).map(|m| m.as_str().to_string());
        let to = caps
            .get(2)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default();
        (from, to)
    } else {
        (None, String::new())
    }
}

fn extract_technology(message: &str) -> String {
    // Extract technology names (capitalized words, common tech)
    let tech_re = Regex::new(r"(?i)(GraphQL|REST|microservices|monolith|PostgreSQL|MongoDB|React|Vue|Angular|TypeScript|JavaScript)").unwrap();

    if let Some(cap) = tech_re.captures(message) {
        cap.get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_default()
    } else {
        String::new()
    }
}

fn extract_reasoning(message: &str) -> String {
    // Extract lines after the decision statement
    let lines: Vec<&str> = message.lines().collect();

    if lines.len() > 1 {
        lines[1..].join("\n").trim().to_string()
    } else {
        String::new()
    }
}
