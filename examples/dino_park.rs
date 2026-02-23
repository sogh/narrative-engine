/// Dino Park example — demonstrates the Survival Thriller genre template.
///
/// A sequence: routine status → power warning → perimeter breach → escalation →
///             discovery of damage → critical failure.
///
/// Uses radio_operator and narrator_omniscient voices to alternate between
/// terse status reports and atmospheric narration.
///
/// Run with: cargo run --example dino_park

use narrative_engine::core::grammar::GrammarSet;
use narrative_engine::core::markov::MarkovTrainer;
use narrative_engine::core::pipeline::{NarrativeEngine, WorldState};
use narrative_engine::core::voice::VoiceRegistry;
use narrative_engine::schema::entity::{Entity, EntityId, Pronouns, VoiceId};
use narrative_engine::schema::event::{EntityRef, Event, Mood, Stakes};
use narrative_engine::schema::narrative_fn::NarrativeFunction;
use std::collections::HashMap;

fn main() {
    // --- Load Survival Thriller genre template ---
    let grammars = GrammarSet::load_from_ron(
        std::path::Path::new("genre_data/survival_thriller/grammar.ron"),
    )
    .expect("Failed to load survival thriller grammar");

    let mut voices = VoiceRegistry::new();
    voices
        .load_from_ron(std::path::Path::new(
            "genre_data/survival_thriller/voices.ron",
        ))
        .expect("Failed to load survival thriller voices");

    // --- Train Markov model from survival thriller corpus ---
    let corpus = std::fs::read_to_string("genre_data/survival_thriller/corpus.txt")
        .expect("Failed to read survival thriller corpus");
    let markov_model = MarkovTrainer::train(&corpus, 3);

    let mut markov_models = HashMap::new();
    markov_models.insert("survival_thriller".to_string(), markov_model);

    let mut engine = NarrativeEngine::builder()
        .seed(1993)
        .with_grammars(grammars)
        .with_voices(voices)
        .with_markov_models(markov_models)
        .build()
        .expect("Failed to build engine");

    // --- Define entities ---
    let mut entities = HashMap::new();

    // Dr. Grant — paleontologist, survivor instinct
    entities.insert(
        EntityId(1),
        Entity {
            id: EntityId(1),
            name: "Dr. Grant".to_string(),
            pronouns: Pronouns::HeHim,
            tags: ["scientist".to_string(), "determined".to_string(), "field_expert".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(202)), // scientist voice
            properties: HashMap::new(),
        },
    );

    // Dr. Malcolm — chaos theorist, always right at the worst time
    entities.insert(
        EntityId(2),
        Entity {
            id: EntityId(2),
            name: "Dr. Malcolm".to_string(),
            pronouns: Pronouns::HeHim,
            tags: ["scientist".to_string(), "skeptic".to_string(), "charismatic".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(202)), // scientist voice
            properties: HashMap::new(),
        },
    );

    // Muldoon — game warden, knows the danger
    entities.insert(
        EntityId(3),
        Entity {
            id: EntityId(3),
            name: "Muldoon".to_string(),
            pronouns: Pronouns::HeHim,
            tags: ["hunter".to_string(), "pragmatic".to_string(), "alert".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(201)), // survivor voice
            properties: HashMap::new(),
        },
    );

    // Control Room — the nerve center
    entities.insert(
        EntityId(10),
        Entity {
            id: EntityId(10),
            name: "Control Room".to_string(),
            pronouns: Pronouns::ItIts,
            tags: ["location".to_string(), "technology".to_string(), "enclosed".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: None,
            properties: HashMap::new(),
        },
    );

    // Rex Paddock — T. rex enclosure
    entities.insert(
        EntityId(11),
        Entity {
            id: EntityId(11),
            name: "Rex Paddock".to_string(),
            pronouns: Pronouns::ItIts,
            tags: ["location".to_string(), "dangerous".to_string(), "perimeter".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: None,
            properties: HashMap::new(),
        },
    );

    // Raptor Pen — velociraptors
    entities.insert(
        EntityId(12),
        Entity {
            id: EntityId(12),
            name: "Raptor Pen".to_string(),
            pronouns: Pronouns::ItIts,
            tags: ["location".to_string(), "dangerous".to_string(), "high_security".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: None,
            properties: HashMap::new(),
        },
    );

    // Security System — abstract entity
    entities.insert(
        EntityId(20),
        Entity {
            id: EntityId(20),
            name: "Security System".to_string(),
            pronouns: Pronouns::ItIts,
            tags: ["system".to_string(), "automated".to_string()]
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
    println!("   DINO PARK INCIDENT REPORT");
    println!("   [CLASSIFIED — Park Security]");
    println!("========================================");
    println!();

    // --- Scene 1: Routine Status (StatusChange — neutral, low stakes) ---
    // radio_operator voice — terse, technical
    let event1 = Event {
        event_type: "routine_check".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(3), role: "subject".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(10), role: "location".to_string() }),
        mood: Mood::Neutral,
        stakes: Stakes::Low,
        outcome: None,
        narrative_fn: NarrativeFunction::StatusChange,
        metadata: HashMap::new(),
    };
    print_scene(1, "0600 — Morning Status Report", "RADIO OPERATOR",
        &mut engine, &event1, &world, Some(VoiceId(200)));

    // --- Scene 2: Power Warning (Foreshadowing — neutral/dread, medium stakes) ---
    // narrator_omniscient voice — atmospheric
    let event2 = Event {
        event_type: "power_fluctuation".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(1), role: "subject".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(10), role: "location".to_string() }),
        mood: Mood::Neutral,
        stakes: Stakes::Medium,
        outcome: None,
        narrative_fn: NarrativeFunction::Foreshadowing,
        metadata: HashMap::new(),
    };
    print_scene(2, "1430 — Power Fluctuation Detected", "NARRATOR",
        &mut engine, &event2, &world, Some(VoiceId(203)));

    // --- Scene 3: Perimeter Breach (Escalation — dread, high stakes) ---
    // radio_operator voice
    let event3 = Event {
        event_type: "perimeter_breach".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(3), role: "subject".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(11), role: "location".to_string() }),
        mood: Mood::Dread,
        stakes: Stakes::High,
        outcome: None,
        narrative_fn: NarrativeFunction::Escalation,
        metadata: HashMap::new(),
    };
    print_scene(3, "2247 — Perimeter Breach: Rex Paddock", "RADIO OPERATOR",
        &mut engine, &event3, &world, Some(VoiceId(200)));

    // --- Scene 4: Escalation (Escalation — dread/chaotic, critical) ---
    // narrator_omniscient — full atmospheric dread
    let event4 = Event {
        event_type: "systems_failing".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(1), role: "subject".to_string() },
            EntityRef { entity_id: EntityId(2), role: "object".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(10), role: "location".to_string() }),
        mood: Mood::Chaotic,
        stakes: Stakes::Critical,
        outcome: None,
        narrative_fn: NarrativeFunction::Escalation,
        metadata: HashMap::new(),
    };
    print_scene(4, "2253 — Multiple System Failures", "NARRATOR",
        &mut engine, &event4, &world, Some(VoiceId(203)));

    // --- Scene 5: Discovery of Damage (Discovery — dread, high stakes) ---
    // Dr. Grant (scientist voice) discovers the extent
    let event5 = Event {
        event_type: "damage_assessment".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(1), role: "subject".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(12), role: "location".to_string() }),
        mood: Mood::Dread,
        stakes: Stakes::High,
        outcome: None,
        narrative_fn: NarrativeFunction::Discovery,
        metadata: HashMap::new(),
    };
    print_scene(5, "2301 — Discovery: Raptor Pen Integrity", "DR. GRANT",
        &mut engine, &event5, &world, None);

    // --- Scene 6: Loss (Loss — somber, critical) ---
    // radio_operator — the final status
    let event6 = Event {
        event_type: "critical_failure".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(3), role: "subject".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(10), role: "location".to_string() }),
        mood: Mood::Somber,
        stakes: Stakes::Critical,
        outcome: None,
        narrative_fn: NarrativeFunction::Loss,
        metadata: HashMap::new(),
    };
    print_scene(6, "2315 — Critical Failure: All Systems", "RADIO OPERATOR",
        &mut engine, &event6, &world, Some(VoiceId(200)));

    // --- Scene 7: Final atmospheric beat ---
    // narrator_omniscient — the island at night
    let event7 = Event {
        event_type: "aftermath".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(2), role: "subject".to_string() },
        ],
        location: Some(EntityRef { entity_id: EntityId(11), role: "location".to_string() }),
        mood: Mood::Dread,
        stakes: Stakes::Critical,
        outcome: None,
        narrative_fn: NarrativeFunction::Loss,
        metadata: HashMap::new(),
    };
    print_scene(7, "2330 — Final Log Entry", "NARRATOR",
        &mut engine, &event7, &world, Some(VoiceId(203)));

    println!("========================================");
    println!("   [END OF INCIDENT REPORT]");
    println!("   [STATUS: FACILITY ABANDONED]");
    println!("========================================");
}

fn print_scene(
    _number: u32,
    title: &str,
    voice_label: &str,
    engine: &mut NarrativeEngine,
    event: &Event,
    world: &WorldState<'_>,
    voice_override: Option<VoiceId>,
) {
    println!("--- {} ---", title);
    println!("[Voice: {} | {} | {}]",
        voice_label,
        event.mood.tag().strip_prefix("mood:").unwrap_or("?"),
        event.stakes.tag().strip_prefix("stakes:").unwrap_or("?"),
    );
    println!();

    let result = if let Some(vid) = voice_override {
        engine.narrate_as(event, vid, world)
    } else {
        engine.narrate(event, world)
    };

    match result {
        Ok(text) => println!("{}", text),
        Err(e) => println!("[Generation error: {}]", e),
    }

    println!();
    println!();
}
