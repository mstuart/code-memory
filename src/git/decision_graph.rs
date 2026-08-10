use super::decision_parser::Decision;

/// DecisionGraph - represents relationships between architectural decisions
/// This will be implemented in a future task
pub struct DecisionGraph {
    // Future implementation will track:
    // - Decision dependencies
    // - Migration chains
    // - Reversal detection
}

impl Default for DecisionGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DecisionGraph {
    pub fn new() -> Self {
        Self {}
    }

    pub fn add_decision(&mut self, _decision: Decision) {
        // Placeholder for future implementation
    }
}
