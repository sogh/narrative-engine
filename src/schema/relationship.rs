use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};

use super::entity::EntityId;

/// A typed, directional edge between two entities with a numerical
/// intensity value. The engine uses these to select appropriate language
/// without understanding the game's specific relationship semantics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub source: EntityId,
    pub target: EntityId,
    pub rel_type: String,
    /// Intensity of the relationship, clamped to 0.0..=1.0.
    pub intensity: f32,
    pub tags: FxHashSet<String>,
}

impl Relationship {
    /// Create a new relationship with intensity clamped to 0.0..=1.0.
    pub fn new(
        source: EntityId,
        target: EntityId,
        rel_type: String,
        intensity: f32,
        tags: FxHashSet<String>,
    ) -> Self {
        Self {
            source,
            target,
            rel_type,
            intensity: intensity.clamp(0.0, 1.0),
            tags,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relationship_creation() {
        let rel = Relationship::new(
            EntityId(1),
            EntityId(2),
            "rival".to_string(),
            0.8,
            FxHashSet::default(),
        );
        assert_eq!(rel.rel_type, "rival");
        assert!((rel.intensity - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn intensity_clamped_high() {
        let rel = Relationship::new(
            EntityId(1),
            EntityId(2),
            "ally".to_string(),
            1.5,
            FxHashSet::default(),
        );
        assert!((rel.intensity - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn intensity_clamped_low() {
        let rel = Relationship::new(
            EntityId(1),
            EntityId(2),
            "stranger".to_string(),
            -0.3,
            FxHashSet::default(),
        );
        assert!((rel.intensity - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn relationship_with_tags() {
        let mut tags = FxHashSet::default();
        tags.insert("secret".to_string());
        tags.insert("deteriorating".to_string());
        let rel = Relationship::new(EntityId(1), EntityId(2), "lover".to_string(), 0.9, tags);
        assert!(rel.tags.contains("secret"));
        assert!(rel.tags.contains("deteriorating"));
        assert_eq!(rel.tags.len(), 2);
    }
}
