use crate::git::decision_parser::Decision;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DriftAlert {
    pub message: String,
    pub severity: Severity,
    pub decision_sha: Option<String>,
    pub conflicting_files: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Low,
    Medium,
    High,
}

pub struct DriftDetector {
    decisions: Vec<Decision>,
    alerts: Vec<DriftAlert>,
}

impl Default for DriftDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl DriftDetector {
    pub fn new() -> Self {
        Self {
            decisions: Vec::new(),
            alerts: Vec::new(),
        }
    }

    pub fn add_decision(&mut self, decision: Decision) {
        self.decisions.push(decision);
    }

    pub fn scan_files(&mut self, files: &[PathBuf]) {
        self.alerts.clear();

        for decision in &self.decisions {
            if decision.decision_type != "architecture" && decision.decision_type != "migration" {
                continue;
            }

            let technology = &decision.to;

            // Check for conflicting technologies
            let (conflicting, conflict_tech) = find_conflicting_tech(technology, files);

            if !conflicting.is_empty() {
                let message = format!(
                    "Architectural drift detected: Decision was to use {}, but {} {} files found",
                    technology,
                    conflict_tech,
                    conflicting.len()
                );

                self.alerts.push(DriftAlert {
                    message,
                    severity: Severity::Medium,
                    decision_sha: decision.commit_sha.clone(),
                    conflicting_files: conflicting,
                });
            }
        }
    }

    pub fn get_alerts(&self) -> Vec<DriftAlert> {
        self.alerts.clone()
    }
}

fn find_conflicting_tech(chosen_tech: &str, files: &[PathBuf]) -> (Vec<PathBuf>, String) {
    let mut conflicting = Vec::new();
    let mut conflict_tech = String::new();

    // Define technology patterns and their conflicts with proper capitalization
    let conflicts: Vec<(&str, &str)> = match chosen_tech.to_lowercase().as_str() {
        "rest" => vec![("graphql", "GraphQL")],
        "graphql" => vec![("rest", "REST")],
        "monolith" => vec![("microservice", "microservices")],
        "microservices" => vec![("monolith", "monolith")],
        _ => vec![],
    };

    for file in files {
        let path_str = file.to_string_lossy().to_lowercase();

        for (pattern, display_name) in &conflicts {
            if path_str.contains(pattern) {
                conflicting.push(file.clone());
                if conflict_tech.is_empty() {
                    conflict_tech = display_name.to_string();
                }
                break;
            }
        }
    }

    (conflicting, conflict_tech)
}
