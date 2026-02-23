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

/// Pronoun set for an entity, used by the grammar expansion system
/// to resolve `{possessive}` and other pronoun template references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Pronouns {
    /// she/her/her/hers/herself
    SheHer,
    /// he/him/his/his/himself
    HeHim,
    /// they/them/their/theirs/themselves
    TheyThem,
    /// it/its/its/its/itself
    ItIts,
}

impl Default for Pronouns {
    fn default() -> Self {
        Self::TheyThem
    }
}

impl Pronouns {
    /// Nominative/subject form: "she", "he", "they", "it".
    pub fn subject(&self) -> &'static str {
        match self {
            Self::SheHer => "she",
            Self::HeHim => "he",
            Self::TheyThem => "they",
            Self::ItIts => "it",
        }
    }

    /// Accusative/object form: "her", "him", "them", "it".
    pub fn object(&self) -> &'static str {
        match self {
            Self::SheHer => "her",
            Self::HeHim => "him",
            Self::TheyThem => "them",
            Self::ItIts => "it",
        }
    }

    /// Possessive determiner: "her", "his", "their", "its".
    pub fn possessive(&self) -> &'static str {
        match self {
            Self::SheHer => "her",
            Self::HeHim => "his",
            Self::TheyThem => "their",
            Self::ItIts => "its",
        }
    }

    /// Possessive standalone: "hers", "his", "theirs", "its".
    pub fn possessive_standalone(&self) -> &'static str {
        match self {
            Self::SheHer => "hers",
            Self::HeHim => "his",
            Self::TheyThem => "theirs",
            Self::ItIts => "its",
        }
    }

    /// Reflexive: "herself", "himself", "themselves", "itself".
    pub fn reflexive(&self) -> &'static str {
        match self {
            Self::SheHer => "herself",
            Self::HeHim => "himself",
            Self::TheyThem => "themselves",
            Self::ItIts => "itself",
        }
    }
}

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
///
/// The engine does not interpret tag semantics — it uses tags solely
/// for grammar rule matching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: EntityId,
    pub name: String,
    pub pronouns: Pronouns,
    pub tags: FxHashSet<String>,
    pub relationships: Vec<Relationship>,
    pub voice_id: Option<VoiceId>,
    pub properties: HashMap<String, Value>,
}

impl Entity {
    /// Returns true if this entity has the given tag.
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    /// Returns true if this entity has ALL of the given tags.
    pub fn has_all_tags(&self, tags: &[&str]) -> bool {
        tags.iter().all(|tag| self.tags.contains(*tag))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(tags: &[&str]) -> Entity {
        let mut tag_set = FxHashSet::default();
        for t in tags {
            tag_set.insert(t.to_string());
        }
        Entity {
            id: EntityId(1),
            name: "Margaret".to_string(),
            pronouns: Pronouns::SheHer,
            tags: tag_set,
            relationships: Vec::new(),
            voice_id: Some(VoiceId(10)),
            properties: HashMap::from([
                ("title".to_string(), Value::String("Duchess".to_string())),
                ("age".to_string(), Value::Int(45)),
                ("composure".to_string(), Value::Float(0.85)),
                ("is_host".to_string(), Value::Bool(true)),
            ]),
        }
    }

    #[test]
    fn entity_creation() {
        let entity = make_entity(&["host", "anxious", "wealthy"]);
        assert_eq!(entity.name, "Margaret");
        assert_eq!(entity.id, EntityId(1));
        assert_eq!(entity.voice_id, Some(VoiceId(10)));
    }

    #[test]
    fn has_tag_positive() {
        let entity = make_entity(&["host", "anxious", "wealthy"]);
        assert!(entity.has_tag("host"));
        assert!(entity.has_tag("anxious"));
        assert!(entity.has_tag("wealthy"));
    }

    #[test]
    fn has_tag_negative() {
        let entity = make_entity(&["host", "anxious"]);
        assert!(!entity.has_tag("calm"));
        assert!(!entity.has_tag(""));
    }

    #[test]
    fn has_all_tags_positive() {
        let entity = make_entity(&["host", "anxious", "wealthy"]);
        assert!(entity.has_all_tags(&["host", "anxious"]));
        assert!(entity.has_all_tags(&["host", "anxious", "wealthy"]));
        assert!(entity.has_all_tags(&[]));
    }

    #[test]
    fn has_all_tags_negative() {
        let entity = make_entity(&["host", "anxious"]);
        assert!(!entity.has_all_tags(&["host", "calm"]));
        assert!(!entity.has_all_tags(&["missing"]));
    }

    #[test]
    fn entity_properties() {
        let entity = make_entity(&[]);
        assert!(matches!(entity.properties.get("title"), Some(Value::String(s)) if s == "Duchess"));
        assert!(matches!(entity.properties.get("age"), Some(Value::Int(45))));
        assert!(matches!(entity.properties.get("composure"), Some(Value::Float(f)) if (*f - 0.85).abs() < f64::EPSILON));
        assert!(matches!(entity.properties.get("is_host"), Some(Value::Bool(true))));
    }
}
