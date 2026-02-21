/// Markov chain phrase generator — training, serialization, and generation.

use serde::{Deserialize, Serialize};

/// A trained Markov model storing n-gram probability tables.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarkovModel {
    // Will be populated in Prompt 5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markov_model_default() {
        let model = MarkovModel::default();
        assert!(format!("{:?}", model).contains("MarkovModel"));
    }
}
