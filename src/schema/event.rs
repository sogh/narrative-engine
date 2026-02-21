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

/// The level of consequences at play.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Stakes {
    Trivial,
    Low,
    Medium,
    High,
    Critical,
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
            participants: vec![],
            location: None,
            mood: Mood::Tense,
            stakes: Stakes::High,
            outcome: None,
            narrative_fn: NarrativeFunction::Confrontation,
            metadata: HashMap::new(),
        };
        assert_eq!(event.mood, Mood::Tense);
    }
}
