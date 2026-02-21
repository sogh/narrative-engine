/// Stochastic grammar runtime — types, parsing, loading, and expansion.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A set of named grammar rules.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GrammarSet {
    pub rules: HashMap<String, GrammarRule>,
}

/// A single grammar rule with tag preconditions and weighted alternatives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarRule {
    pub name: String,
    pub requires: Vec<String>,
    pub excludes: Vec<String>,
    pub alternatives: Vec<Alternative>,
}

/// A weighted text alternative within a grammar rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub weight: u32,
    pub template: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammar_set_default() {
        let gs = GrammarSet::default();
        assert!(gs.rules.is_empty());
    }
}
