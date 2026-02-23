/// Dinner Party example — demonstrates the Social Drama genre template.
///
/// A mini story: small talk → tension builds → accusation → revelation → aftermath.
///
/// Run with: cargo run --example dinner_party

use narrative_engine::core::grammar::GrammarSet;
use narrative_engine::core::markov::MarkovTrainer;
use narrative_engine::core::pipeline::{NarrativeEngine, WorldState};
use narrative_engine::core::voice::VoiceRegistry;
use narrative_engine::schema::entity::{Entity, EntityId, Pronouns, VoiceId};
use narrative_engine::schema::event::{EntityRef, Event, Mood, Stakes};
use narrative_engine::schema::narrative_fn::NarrativeFunction;
use std::collections::HashMap;

fn main() {
    // --- Load Social Drama genre template ---
    let grammars = GrammarSet::load_from_ron(
        std::path::Path::new("genre_data/social_drama/grammar.ron"),
    )
    .expect("Failed to load social drama grammar");

    let mut voices = VoiceRegistry::new();
    voices
        .load_from_ron(std::path::Path::new("genre_data/social_drama/voices.ron"))
        .expect("Failed to load social drama voices");

    // --- Train Markov model from social drama corpus ---
    let corpus = std::fs::read_to_string("genre_data/social_drama/corpus.txt")
        .expect("Failed to read social drama corpus");
    let markov_model = MarkovTrainer::train(&corpus, 3);

    let mut markov_models = HashMap::new();
    markov_models.insert("social_drama".to_string(), markov_model);

    let mut engine = NarrativeEngine::builder()
        .seed(2026)
        .with_grammars(grammars)
        .with_voices(voices)
        .with_markov_models(markov_models)
        .build()
        .expect("Failed to build engine");

    // --- Define entities ---
    let mut entities = HashMap::new();

    // Margaret — the anxious host
    entities.insert(
        EntityId(1),
        Entity {
            id: EntityId(1),
            name: "Margaret".to_string(),
            pronouns: Pronouns::SheHer,
            tags: ["host".to_string(), "anxious".to_string(), "wealthy".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(100)), // host voice
            properties: HashMap::from([
                ("title".to_string(), narrative_engine::schema::entity::Value::String("Lady".to_string())),
            ]),
        },
    );

    // James — her husband, harboring a secret
    entities.insert(
        EntityId(2),
        Entity {
            id: EntityId(2),
            name: "James".to_string(),
            pronouns: Pronouns::HeHim,
            tags: ["guest".to_string(), "secretive".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(103)), // provocateur voice
            properties: HashMap::new(),
        },
    );

    // Eleanor — old friend, sharp-tongued gossip
    entities.insert(
        EntityId(3),
        Entity {
            id: EntityId(3),
            name: "Eleanor".to_string(),
            pronouns: Pronouns::SheHer,
            tags: ["guest".to_string(), "perceptive".to_string(), "caustic".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(101)), // gossip voice
            properties: HashMap::new(),
        },
    );

    // Robert — the peacemaker, caught in the middle
    entities.insert(
        EntityId(4),
        Entity {
            id: EntityId(4),
            name: "Robert".to_string(),
            pronouns: Pronouns::HeHim,
            tags: ["guest".to_string(), "diplomatic".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(102)), // peacemaker voice
            properties: HashMap::new(),
        },
    );

    // The Dining Room — the setting
    entities.insert(
        EntityId(100),
        Entity {
            id: EntityId(100),
            name: "the dining room".to_string(),
            pronouns: Pronouns::ItIts,
            tags: ["location".to_string(), "formal".to_string(), "elegant".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: None,
            properties: HashMap::new(),
        },
    );

    let world = WorldState {
        entities: &entities,
    };

    // --- Title ---
    println!("========================================");
    println!("   THE DINNER PARTY");
    println!("   A Social Drama in Six Scenes");
    println!("========================================");
    println!();

    // --- Scene 1: Small Talk (Alliance — warm, low stakes) ---
    let event1 = Event {
        event_type: "small_talk".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(1), role: "subject".to_string() },
            EntityRef { entity_id: EntityId(4), role: "object".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(100), role: "location".to_string() }),
        mood: Mood::Warm,
        stakes: Stakes::Low,
        outcome: None,
        narrative_fn: NarrativeFunction::Alliance,
        metadata: HashMap::new(),
    };
    print_scene(1, "Small Talk", &["Margaret", "Robert"], &mut engine, &event1, &world);

    // --- Scene 2: A Whispered Alliance (Eleanor and Robert align) ---
    let event2 = Event {
        event_type: "whispered_aside".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(3), role: "subject".to_string() },
            EntityRef { entity_id: EntityId(4), role: "object".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(100), role: "location".to_string() }),
        mood: Mood::Neutral,
        stakes: Stakes::Medium,
        outcome: None,
        narrative_fn: NarrativeFunction::Alliance,
        metadata: HashMap::new(),
    };
    print_scene(2, "A Whispered Aside", &["Eleanor", "Robert"], &mut engine, &event2, &world);

    // --- Scene 3: Tension Builds (Confrontation — tense, rising) ---
    let event3 = Event {
        event_type: "tension_rises".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(3), role: "subject".to_string() },
            EntityRef { entity_id: EntityId(1), role: "object".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(100), role: "location".to_string() }),
        mood: Mood::Tense,
        stakes: Stakes::Medium,
        outcome: None,
        narrative_fn: NarrativeFunction::Confrontation,
        metadata: HashMap::new(),
    };
    print_scene(3, "Tension Builds", &["Eleanor", "Margaret"], &mut engine, &event3, &world);

    // --- Scene 4: The Accusation (Confrontation — tense, high stakes) ---
    let event4 = Event {
        event_type: "accusation".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(3), role: "subject".to_string() },
            EntityRef { entity_id: EntityId(2), role: "object".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(100), role: "location".to_string() }),
        mood: Mood::Tense,
        stakes: Stakes::High,
        outcome: None,
        narrative_fn: NarrativeFunction::Confrontation,
        metadata: HashMap::new(),
    };
    print_scene(4, "The Accusation", &["Eleanor", "James"], &mut engine, &event4, &world);

    // --- Scene 5: The Revelation (James's secret comes out) ---
    let event5 = Event {
        event_type: "confession".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(2), role: "subject".to_string() },
            EntityRef { entity_id: EntityId(1), role: "object".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(100), role: "location".to_string() }),
        mood: Mood::Somber,
        stakes: Stakes::Critical,
        outcome: None,
        narrative_fn: NarrativeFunction::Revelation,
        metadata: HashMap::new(),
    };
    print_scene(5, "The Revelation", &["James", "Margaret"], &mut engine, &event5, &world);

    // --- Scene 6: Comic Relief (Robert breaks the tension) ---
    let event6 = Event {
        event_type: "comic_relief".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(4), role: "subject".to_string() },
            EntityRef { entity_id: EntityId(3), role: "object".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(100), role: "location".to_string() }),
        mood: Mood::Neutral,
        stakes: Stakes::Low,
        outcome: None,
        narrative_fn: NarrativeFunction::ComicRelief,
        metadata: HashMap::new(),
    };
    print_scene(6, "The Aftermath", &["Robert", "Eleanor"], &mut engine, &event6, &world);

    // --- Scene 7: Betrayal (Margaret realizes James and Eleanor) ---
    let event7 = Event {
        event_type: "betrayal_realized".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(1), role: "subject".to_string() },
            EntityRef { entity_id: EntityId(2), role: "object".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(100), role: "location".to_string() }),
        mood: Mood::Somber,
        stakes: Stakes::Critical,
        outcome: None,
        narrative_fn: NarrativeFunction::Betrayal,
        metadata: HashMap::new(),
    };
    print_scene(7, "The Betrayal", &["Margaret", "James"], &mut engine, &event7, &world);

    println!("========================================");
    println!("   FIN");
    println!("========================================");
}

fn print_scene(
    number: u32,
    title: &str,
    participants: &[&str],
    engine: &mut NarrativeEngine,
    event: &Event,
    world: &WorldState<'_>,
) {
    println!("--- Scene {}: {} ---", number, title);
    println!("[{} | {} | {}]",
        participants.join(", "),
        event.mood.tag().strip_prefix("mood:").unwrap_or("?"),
        event.stakes.tag().strip_prefix("stakes:").unwrap_or("?"),
    );
    println!();

    match engine.narrate(event, world) {
        Ok(text) => println!("{}", text),
        Err(e) => println!("[Generation error: {}]", e),
    }

    println!();
    println!();
}
