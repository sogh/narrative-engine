use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use super::entity::EntityId;

/// A typed, directional edge between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub source: EntityId,
    pub target: EntityId,
    pub rel_type: String,
    pub intensity: f32,
    pub tags: FxHashSet<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relationship_creation() {
        let rel = Relationship {
            source: EntityId(1),
            target: EntityId(2),
            rel_type: "rival".to_string(),
            intensity: 0.8,
            tags: FxHashSet::default(),
        };
        assert_eq!(rel.rel_type, "rival");
        assert!(rel.intensity >= 0.0 && rel.intensity <= 1.0);
    }
}
