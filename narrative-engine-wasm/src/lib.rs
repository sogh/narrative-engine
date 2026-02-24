//! WASM bindings for narrative-engine — powers the interactive web demo.

use std::collections::HashMap;
use wasm_bindgen::prelude::*;

use narrative_engine::core::grammar::GrammarSet;
use narrative_engine::core::markov::MarkovTrainer;
use narrative_engine::core::pipeline::{NarrativeEngine, WorldState};
use narrative_engine::core::voice::VoiceRegistry;
use narrative_engine::schema::entity::{Entity, EntityId, Pronouns, VoiceId};
use narrative_engine::schema::event::{EntityRef, Event, Mood, Stakes};
use narrative_engine::schema::narrative_fn::NarrativeFunction;

// ---------------------------------------------------------------------------
// Embedded genre data — compiled into the WASM binary
// ---------------------------------------------------------------------------
mod data {
    pub const SOCIAL_DRAMA_GRAMMAR: &str =
        include_str!("../../genre_data/social_drama/grammar.ron");
    pub const SOCIAL_DRAMA_VOICES: &str =
        include_str!("../../genre_data/social_drama/voices.ron");
    pub const SOCIAL_DRAMA_CORPUS: &str =
        include_str!("../../genre_data/social_drama/corpus.txt");

    pub const SURVIVAL_THRILLER_GRAMMAR: &str =
        include_str!("../../genre_data/survival_thriller/grammar.ron");
    pub const SURVIVAL_THRILLER_VOICES: &str =
        include_str!("../../genre_data/survival_thriller/voices.ron");
    pub const SURVIVAL_THRILLER_CORPUS: &str =
        include_str!("../../genre_data/survival_thriller/corpus.txt");
}

// ---------------------------------------------------------------------------
// JSON helper types for communication across the WASM boundary
// ---------------------------------------------------------------------------
#[derive(serde::Deserialize)]
struct EventInput {
    subject_id: u64,
    object_id: Option<u64>,
    mood: String,
    stakes: String,
    narrative_fn: String,
    event_type: Option<String>,
}

#[derive(serde::Serialize)]
struct EntityInfo {
    id: u64,
    name: String,
    pronouns: String,
    tags: Vec<String>,
    voice_id: Option<u64>,
}

#[derive(serde::Serialize)]
struct ScenarioInfo {
    genre: String,
    entities: Vec<EntityInfo>,
}

// ---------------------------------------------------------------------------
// Conversion helpers
// ---------------------------------------------------------------------------
fn parse_mood(s: &str) -> Mood {
    match s.to_lowercase().as_str() {
        "neutral" => Mood::Neutral,
        "tense" => Mood::Tense,
        "warm" => Mood::Warm,
        "dread" => Mood::Dread,
        "euphoric" => Mood::Euphoric,
        "somber" => Mood::Somber,
        "chaotic" => Mood::Chaotic,
        "intimate" => Mood::Intimate,
        _ => Mood::Neutral,
    }
}

fn parse_stakes(s: &str) -> Stakes {
    match s.to_lowercase().as_str() {
        "trivial" => Stakes::Trivial,
        "low" => Stakes::Low,
        "medium" => Stakes::Medium,
        "high" => Stakes::High,
        "critical" => Stakes::Critical,
        _ => Stakes::Medium,
    }
}

fn parse_narrative_fn(s: &str) -> NarrativeFunction {
    match s.to_lowercase().as_str() {
        "revelation" => NarrativeFunction::Revelation,
        "escalation" => NarrativeFunction::Escalation,
        "confrontation" => NarrativeFunction::Confrontation,
        "betrayal" => NarrativeFunction::Betrayal,
        "alliance" => NarrativeFunction::Alliance,
        "discovery" => NarrativeFunction::Discovery,
        "loss" => NarrativeFunction::Loss,
        "comic_relief" => NarrativeFunction::ComicRelief,
        "foreshadowing" => NarrativeFunction::Foreshadowing,
        "status_change" => NarrativeFunction::StatusChange,
        other => NarrativeFunction::Custom(other.to_string()),
    }
}

fn pronouns_label(p: &Pronouns) -> &'static str {
    match p {
        Pronouns::SheHer => "she/her",
        Pronouns::HeHim => "he/him",
        Pronouns::TheyThem => "they/them",
        Pronouns::ItIts => "it/its",
    }
}

// ---------------------------------------------------------------------------
// Preset entities per genre
// ---------------------------------------------------------------------------
fn social_drama_entities() -> HashMap<EntityId, Entity> {
    let mut entities = HashMap::new();

    entities.insert(
        EntityId(1),
        Entity {
            id: EntityId(1),
            name: "Margaret".to_string(),
            pronouns: Pronouns::SheHer,
            tags: ["host", "anxious", "wealthy"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(100)),
            properties: HashMap::from([(
                "title".to_string(),
                narrative_engine::schema::entity::Value::String("Lady".to_string()),
            )]),
        },
    );

    entities.insert(
        EntityId(2),
        Entity {
            id: EntityId(2),
            name: "James".to_string(),
            pronouns: Pronouns::HeHim,
            tags: ["guest", "secretive"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(103)),
            properties: HashMap::new(),
        },
    );

    entities.insert(
        EntityId(3),
        Entity {
            id: EntityId(3),
            name: "Eleanor".to_string(),
            pronouns: Pronouns::SheHer,
            tags: ["guest", "perceptive", "caustic"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(101)),
            properties: HashMap::new(),
        },
    );

    entities.insert(
        EntityId(4),
        Entity {
            id: EntityId(4),
            name: "Robert".to_string(),
            pronouns: Pronouns::HeHim,
            tags: ["guest", "diplomatic"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(102)),
            properties: HashMap::new(),
        },
    );

    entities
}

fn survival_thriller_entities() -> HashMap<EntityId, Entity> {
    let mut entities = HashMap::new();

    entities.insert(
        EntityId(1),
        Entity {
            id: EntityId(1),
            name: "Dr. Grant".to_string(),
            pronouns: Pronouns::HeHim,
            tags: ["scientist", "determined", "field_expert"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(202)),
            properties: HashMap::new(),
        },
    );

    entities.insert(
        EntityId(2),
        Entity {
            id: EntityId(2),
            name: "Dr. Malcolm".to_string(),
            pronouns: Pronouns::HeHim,
            tags: ["scientist", "skeptic", "charismatic"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(202)),
            properties: HashMap::new(),
        },
    );

    entities.insert(
        EntityId(3),
        Entity {
            id: EntityId(3),
            name: "Muldoon".to_string(),
            pronouns: Pronouns::HeHim,
            tags: ["hunter", "pragmatic", "alert"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(201)),
            properties: HashMap::new(),
        },
    );

    entities
}

// ---------------------------------------------------------------------------
// NarrativeDemo — the main exported struct
// ---------------------------------------------------------------------------
#[wasm_bindgen]
pub struct NarrativeDemo {
    engine: NarrativeEngine,
    entities: HashMap<EntityId, Entity>,
    genre: String,
}

#[wasm_bindgen]
impl NarrativeDemo {
    /// Create a new demo instance for the given genre and seed.
    #[wasm_bindgen(constructor)]
    pub fn new(genre: &str, seed: u64) -> Result<NarrativeDemo, JsError> {
        let (grammar_src, voices_src, corpus_src, corpus_id, entities) = match genre {
            "social_drama" => (
                data::SOCIAL_DRAMA_GRAMMAR,
                data::SOCIAL_DRAMA_VOICES,
                data::SOCIAL_DRAMA_CORPUS,
                "social_drama",
                social_drama_entities(),
            ),
            "survival_thriller" => (
                data::SURVIVAL_THRILLER_GRAMMAR,
                data::SURVIVAL_THRILLER_VOICES,
                data::SURVIVAL_THRILLER_CORPUS,
                "survival_thriller",
                survival_thriller_entities(),
            ),
            _ => return Err(JsError::new(&format!("Unknown genre: {genre}"))),
        };

        let grammars = GrammarSet::parse_ron(grammar_src)
            .map_err(|e| JsError::new(&format!("Grammar parse error: {e}")))?;

        let mut voices = VoiceRegistry::new();
        voices
            .parse_from_ron(voices_src)
            .map_err(|e| JsError::new(&format!("Voice parse error: {e}")))?;

        let markov_model = MarkovTrainer::train(corpus_src, 3);
        let mut markov_models = HashMap::new();
        markov_models.insert(corpus_id.to_string(), markov_model);

        let engine = NarrativeEngine::builder()
            .seed(seed)
            .with_grammars(grammars)
            .with_voices(voices)
            .with_markov_models(markov_models)
            .build()
            .map_err(|e| JsError::new(&format!("Engine build error: {e}")))?;

        Ok(NarrativeDemo {
            engine,
            entities,
            genre: genre.to_string(),
        })
    }

    /// Generate narration for an event described by a JSON string.
    ///
    /// Expected JSON shape:
    /// ```json
    /// {
    ///   "subject_id": 1,
    ///   "object_id": 2,
    ///   "mood": "tense",
    ///   "stakes": "high",
    ///   "narrative_fn": "confrontation",
    ///   "event_type": "accusation"
    /// }
    /// ```
    pub fn narrate(&mut self, event_json: &str) -> Result<String, JsError> {
        let input: EventInput = serde_json::from_str(event_json)
            .map_err(|e| JsError::new(&format!("Invalid event JSON: {e}")))?;
        let event = self.build_event(&input);
        let world = WorldState {
            entities: &self.entities,
        };
        self.engine
            .narrate(&event, &world)
            .map_err(|e| JsError::new(&format!("Narration error: {e}")))
    }

    /// Generate multiple variants for the same event. Returns a JSON array of strings.
    pub fn narrate_variants(
        &mut self,
        event_json: &str,
        count: usize,
    ) -> Result<String, JsError> {
        let input: EventInput = serde_json::from_str(event_json)
            .map_err(|e| JsError::new(&format!("Invalid event JSON: {e}")))?;
        let event = self.build_event(&input);
        let world = WorldState {
            entities: &self.entities,
        };
        let variants = self
            .engine
            .narrate_variants(&event, count, &world)
            .map_err(|e| JsError::new(&format!("Narration error: {e}")))?;
        serde_json::to_string(&variants)
            .map_err(|e| JsError::new(&format!("Serialization error: {e}")))
    }

    /// Return a JSON description of the current scenario (genre + entities).
    pub fn get_scenario(&self) -> Result<String, JsError> {
        let entities: Vec<EntityInfo> = self
            .entities
            .values()
            .map(|e| EntityInfo {
                id: e.id.0,
                name: e.name.clone(),
                pronouns: pronouns_label(&e.pronouns).to_string(),
                tags: e.tags.iter().cloned().collect(),
                voice_id: e.voice_id.map(|v| v.0),
            })
            .collect();

        let info = ScenarioInfo {
            genre: self.genre.clone(),
            entities,
        };
        serde_json::to_string(&info)
            .map_err(|e| JsError::new(&format!("Serialization error: {e}")))
    }

    /// Return JSON array of available genre identifiers.
    pub fn available_genres() -> String {
        serde_json::to_string(&["social_drama", "survival_thriller"])
            .unwrap_or_else(|_| "[]".to_string())
    }

    /// Return JSON array of mood names.
    pub fn moods() -> String {
        serde_json::to_string(&[
            "neutral", "tense", "warm", "dread", "euphoric", "somber", "chaotic", "intimate",
        ])
        .unwrap_or_else(|_| "[]".to_string())
    }

    /// Return JSON array of stakes levels.
    pub fn stakes() -> String {
        serde_json::to_string(&["trivial", "low", "medium", "high", "critical"])
            .unwrap_or_else(|_| "[]".to_string())
    }

    /// Return JSON array of all built-in narrative function names.
    pub fn narrative_functions() -> String {
        serde_json::to_string(&[
            "revelation",
            "escalation",
            "confrontation",
            "betrayal",
            "alliance",
            "discovery",
            "loss",
            "comic_relief",
            "foreshadowing",
            "status_change",
        ])
        .unwrap_or_else(|_| "[]".to_string())
    }

    /// Return JSON array of narrative functions that have grammar rules
    /// in the current genre. Only these will produce output without error.
    pub fn supported_functions(&self) -> String {
        let fns: &[&str] = match self.genre.as_str() {
            "social_drama" => &[
                "revelation",
                "confrontation",
                "betrayal",
                "alliance",
                "comic_relief",
            ],
            "survival_thriller" => &[
                "escalation",
                "discovery",
                "loss",
                "foreshadowing",
                "status_change",
            ],
            _ => &[],
        };
        serde_json::to_string(fns).unwrap_or_else(|_| "[]".to_string())
    }

    /// Reset the engine with a new seed (same genre).
    pub fn reset(&mut self, seed: u64) -> Result<(), JsError> {
        let new_demo = NarrativeDemo::new(&self.genre.clone(), seed)?;
        self.engine = new_demo.engine;
        self.entities = new_demo.entities;
        Ok(())
    }
}

// Private helpers
impl NarrativeDemo {
    fn build_event(&self, input: &EventInput) -> Event {
        let mut participants = vec![EntityRef {
            entity_id: EntityId(input.subject_id),
            role: "subject".to_string(),
        }];
        if let Some(obj_id) = input.object_id {
            participants.push(EntityRef {
                entity_id: EntityId(obj_id),
                role: "object".to_string(),
            });
        }

        Event {
            event_type: input
                .event_type
                .clone()
                .unwrap_or_else(|| input.narrative_fn.clone()),
            participants,
            location: None,
            mood: parse_mood(&input.mood),
            stakes: parse_stakes(&input.stakes),
            outcome: None,
            narrative_fn: parse_narrative_fn(&input.narrative_fn),
            metadata: HashMap::new(),
        }
    }
}
