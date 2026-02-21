use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::relationship::Relationship;

/// Newtype wrapper for entity IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EntityId(pub u64);

/// Newtype wrapper for voice IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VoiceId(pub u64);

/// A dynamic value that can be stored in entity properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Value {
    String(String),
    Float(f64),
    Int(i64),
    Bool(bool),
}

/// An entity is anything that can participate in a narrative event:
/// a person, creature, place, object, or abstract concept.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub name: String,
    pub tags: FxHashSet<String>,
    pub relationships: Vec<Relationship>,
    pub voice_id: Option<VoiceId>,
    pub properties: HashMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_creation() {
        let entity = Entity {
            id: EntityId(1),
            name: "Margaret".to_string(),
            tags: FxHashSet::default(),
            relationships: Vec::new(),
            voice_id: None,
            properties: HashMap::new(),
        };
        assert_eq!(entity.name, "Margaret");
    }
}
