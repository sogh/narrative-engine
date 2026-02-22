/// Pipeline integration tests — end-to-end event-to-text generation.

use narrative_engine::core::grammar::GrammarSet;
use narrative_engine::core::pipeline::{NarrativeEngine, WorldState};
use narrative_engine::core::voice::VoiceRegistry;
use narrative_engine::schema::entity::{Entity, EntityId, Value, VoiceId};
use narrative_engine::schema::event::{EntityRef, Event, Mood, Stakes};
use narrative_engine::schema::narrative_fn::NarrativeFunction;
use std::collections::HashMap;

#[test]
fn genre_blending_social_drama_and_survival_thriller() {
    // Load both genre templates' grammars
    let social_path = std::path::Path::new("genre_data/social_drama/grammar.ron");
    let thriller_path = std::path::Path::new("genre_data/survival_thriller/grammar.ron");

    let mut grammars = GrammarSet::load_from_ron(social_path).unwrap();
    let thriller_grammars = GrammarSet::load_from_ron(thriller_path).unwrap();
    grammars.merge(thriller_grammars);

    // Should have rules from both templates
    assert!(grammars.rules.contains_key("confrontation_opening")); // social drama
    assert!(grammars.rules.contains_key("escalation_opening")); // survival thriller
    assert!(grammars.rules.contains_key("body_language")); // social drama supporting
    assert!(grammars.rules.contains_key("threat_proximity")); // thriller supporting

    // Load voices from both
    let mut voices = VoiceRegistry::new();
    voices
        .load_from_ron(std::path::Path::new("genre_data/social_drama/voices.ron"))
        .unwrap();
    voices
        .load_from_ron(std::path::Path::new(
            "genre_data/survival_thriller/voices.ron",
        ))
        .unwrap();

    // Build engine with merged content
    let mut engine = NarrativeEngine::builder()
        .seed(42)
        .with_grammars(grammars)
        .with_voices(voices)
        .build()
        .unwrap();

    // Create a confrontation event (social drama function with tense mood)
    let mut entities = HashMap::new();
    entities.insert(
        EntityId(1),
        Entity {
            id: EntityId(1),
            name: "Dr. Grant".to_string(),
            tags: ["scientist".to_string(), "determined".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(103)), // provocateur voice
            properties: HashMap::new(),
        },
    );
    entities.insert(
        EntityId(2),
        Entity {
            id: EntityId(2),
            name: "Hammond".to_string(),
            tags: ["host".to_string(), "wealthy".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(100)), // host voice
            properties: HashMap::new(),
        },
    );

    let world = WorldState {
        entities: &entities,
    };

    // Test a confrontation (social drama)
    let confrontation_event = Event {
        event_type: "argument".to_string(),
        participants: vec![
            EntityRef {
                entity_id: EntityId(1),
                role: "subject".to_string(),
            },
            EntityRef {
                entity_id: EntityId(2),
                role: "object".to_string(),
            },
        ],
        location: None,
        mood: Mood::Tense,
        stakes: Stakes::High,
        outcome: None,
        narrative_fn: NarrativeFunction::Confrontation,
        metadata: HashMap::new(),
    };

    let result = engine.narrate(&confrontation_event, &world).unwrap();
    assert!(!result.is_empty(), "Confrontation should produce output");

    // Test an escalation (survival thriller)
    let escalation_event = Event {
        event_type: "breach".to_string(),
        participants: vec![EntityRef {
            entity_id: EntityId(1),
            role: "subject".to_string(),
        }],
        location: None,
        mood: Mood::Dread,
        stakes: Stakes::Critical,
        outcome: None,
        narrative_fn: NarrativeFunction::Escalation,
        metadata: HashMap::new(),
    };

    let result2 = engine.narrate(&escalation_event, &world).unwrap();
    assert!(!result2.is_empty(), "Escalation should produce output");

    // The outputs should be different in character
    assert_ne!(
        result, result2,
        "Different narrative functions should produce different output"
    );
}

#[test]
fn pipeline_placeholder() {
    // Kept for backwards compatibility
    assert!(true);
}
