use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::entity::{EntityId, Value};
use super::narrative_fn::NarrativeFunction;

/// The emotional tone of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mood {
    Neutral,
    Tense,
    Warm,
    Dread,
    Euphoric,
    Somber,
    Chaotic,
    Intimate,
}

impl Mood {
    /// Returns the tag string for this mood (e.g., "mood:tense").
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Neutral => "mood:neutral",
            Self::Tense => "mood:tense",
            Self::Warm => "mood:warm",
            Self::Dread => "mood:dread",
            Self::Euphoric => "mood:euphoric",
            Self::Somber => "mood:somber",
            Self::Chaotic => "mood:chaotic",
            Self::Intimate => "mood:intimate",
        }
    }
}

/// The level of consequences at play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stakes {
    Trivial,
    Low,
    Medium,
    High,
    Critical,
}

impl Stakes {
    /// Returns the tag string for this stakes level (e.g., "stakes:high").
    pub fn tag(&self) -> &'static str {
        match self {
            Self::Trivial => "stakes:trivial",
            Self::Low => "stakes:low",
            Self::Medium => "stakes:medium",
            Self::High => "stakes:high",
            Self::Critical => "stakes:critical",
        }
    }
}

/// The result of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Outcome {
    Success,
    Failure,
    Partial,
    Ambiguous,
}

/// A lightweight reference to an entity participating in an event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRef {
    pub entity_id: EntityId,
    pub role: String,
}

/// A structured record of something that happened in the game simulation.
/// Events are the sole input to the narrative pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub event_type: String,
    pub participants: Vec<EntityRef>,
    pub location: Option<EntityRef>,
    pub mood: Mood,
    pub stakes: Stakes,
    pub outcome: Option<Outcome>,
    pub narrative_fn: NarrativeFunction,
    pub metadata: HashMap<String, Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_creation() {
        let event = Event {
            event_type: "accusation".to_string(),
            participants: vec![
                EntityRef {
                    entity_id: EntityId(1),
                    role: "subject".to_string(),
                },
                EntityRef {
                    entity_id: EntityId(2),
                    role: "target".to_string(),
                },
            ],
            location: Some(EntityRef {
                entity_id: EntityId(100),
                role: "location".to_string(),
            }),
            mood: Mood::Tense,
            stakes: Stakes::High,
            outcome: None,
            narrative_fn: NarrativeFunction::Confrontation,
            metadata: HashMap::from([(
                "held_item".to_string(),
                Value::String("wine glass".to_string()),
            )]),
        };
        assert_eq!(event.mood, Mood::Tense);
        assert_eq!(event.stakes, Stakes::High);
        assert_eq!(event.participants.len(), 2);
        assert_eq!(event.participants[0].role, "subject");
        assert!(event.location.is_some());
    }

    #[test]
    fn mood_tags() {
        assert_eq!(Mood::Tense.tag(), "mood:tense");
        assert_eq!(Mood::Neutral.tag(), "mood:neutral");
        assert_eq!(Mood::Dread.tag(), "mood:dread");
        assert_eq!(Mood::Intimate.tag(), "mood:intimate");
    }

    #[test]
    fn stakes_tags() {
        assert_eq!(Stakes::Trivial.tag(), "stakes:trivial");
        assert_eq!(Stakes::Critical.tag(), "stakes:critical");
        assert_eq!(Stakes::High.tag(), "stakes:high");
    }

    #[test]
    fn outcome_variants() {
        assert_eq!(Outcome::Success, Outcome::Success);
        assert_ne!(Outcome::Success, Outcome::Failure);
    }

    #[test]
    fn entity_ref_roles() {
        let witness = EntityRef {
            entity_id: EntityId(3),
            role: "witness".to_string(),
        };
        assert_eq!(witness.role, "witness");
    }
}
