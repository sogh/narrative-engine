/// The main narrative pipeline: Event → Text orchestration.
///
/// Wires together grammar expansion, voice selection, Markov fill,
/// variety pass, and context checking.

use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

use crate::core::context::NarrativeContext;
use crate::core::grammar::{GrammarError, GrammarSet, SelectionContext};
use crate::core::markov::{MarkovError, MarkovModel};
use crate::core::variety::VarietyPass;
use crate::core::voice::{VoiceError, VoiceRegistry};
use crate::schema::entity::{Entity, EntityId, VoiceId};
use crate::schema::event::Event;
use crate::schema::narrative_fn::NarrativeFunction;

#[derive(Debug, Error)]
pub enum PipelineError {
    #[error("grammar error: {0}")]
    Grammar(#[from] GrammarError),
    #[error("voice error: {0}")]
    Voice(#[from] VoiceError),
    #[error("markov error: {0}")]
    Markov(#[from] MarkovError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("RON error: {0}")]
    Ron(#[from] ron::error::SpannedError),
    #[error("entity not found: {0:?}")]
    EntityNotFound(EntityId),
    #[error("no grammar rule found for narrative function: {0}")]
    NoRuleForFunction(String),
    #[error("generation failed after {0} retries")]
    GenerationFailed(u32),
}

/// World state passed by the game to the narration pipeline.
pub struct WorldState<'a> {
    pub entities: &'a HashMap<EntityId, Entity>,
}

/// Event-type to narrative-function mapping entry.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct EventMapping {
    pub event_type: String,
    pub narrative_fn: NarrativeFunction,
}

/// The top-level narrative engine. Built via `NarrativeEngine::builder()`.
pub struct NarrativeEngine {
    grammars: GrammarSet,
    voices: VoiceRegistry,
    markov_models: HashMap<String, MarkovModel>,
    mappings: HashMap<String, NarrativeFunction>,
    context: NarrativeContext,
    seed: u64,
    generation_count: u64,
}

/// Builder for constructing a `NarrativeEngine`.
pub struct NarrativeEngineBuilder {
    genre_templates: Vec<String>,
    grammars_dir: Option<String>,
    voices_dir: Option<String>,
    markov_models_dir: Option<String>,
    mappings_path: Option<String>,
    seed: u64,
    /// Directly provided grammars (for testing without files).
    grammars: Option<GrammarSet>,
    /// Directly provided voices (for testing without files).
    voices: Option<VoiceRegistry>,
    /// Directly provided markov models (for testing without files).
    markov_models: Option<HashMap<String, MarkovModel>>,
    /// Directly provided mappings (for testing without files).
    mappings: Option<HashMap<String, NarrativeFunction>>,
}

impl NarrativeEngine {
    pub fn builder() -> NarrativeEngineBuilder {
        NarrativeEngineBuilder {
            genre_templates: Vec::new(),
            grammars_dir: None,
            voices_dir: None,
            markov_models_dir: None,
            mappings_path: None,
            seed: 0,
            grammars: None,
            voices: None,
            markov_models: None,
            mappings: None,
        }
    }

    /// Generate narration for an event using the first participant's voice.
    pub fn narrate(
        &mut self,
        event: &Event,
        world: &WorldState<'_>,
    ) -> Result<String, PipelineError> {
        // Select voice from first participant
        let voice_id = self.resolve_voice_id(event, world);
        self.narrate_with_voice(event, voice_id, world)
    }

    /// Generate narration for an event using a specific voice.
    pub fn narrate_as(
        &mut self,
        event: &Event,
        voice_id: VoiceId,
        world: &WorldState<'_>,
    ) -> Result<String, PipelineError> {
        self.narrate_with_voice(event, Some(voice_id), world)
    }

    /// Generate multiple variants for an event.
    pub fn narrate_variants(
        &mut self,
        event: &Event,
        count: usize,
        world: &WorldState<'_>,
    ) -> Result<Vec<String>, PipelineError> {
        let mut results = Vec::with_capacity(count);
        for i in 0..count {
            // Use different seed offsets for each variant
            let saved_count = self.generation_count;
            self.generation_count = saved_count + (i as u64 * 1000);
            let result = self.narrate(event, world)?;
            self.generation_count = saved_count + 1;
            results.push(result);
        }
        Ok(results)
    }

    fn resolve_voice_id(&self, event: &Event, world: &WorldState<'_>) -> Option<VoiceId> {
        // Use first participant's voice_id
        for participant in &event.participants {
            if let Some(entity) = world.entities.get(&participant.entity_id) {
                if entity.voice_id.is_some() {
                    return entity.voice_id;
                }
            }
        }
        None
    }

    fn narrate_with_voice(
        &mut self,
        event: &Event,
        voice_id: Option<VoiceId>,
        world: &WorldState<'_>,
    ) -> Result<String, PipelineError> {
        let max_retries = 3u32;

        for retry in 0..max_retries {
            let mut rng = StdRng::seed_from_u64(
                self.seed
                    .wrapping_add(self.generation_count)
                    .wrapping_add(retry as u64 * 7919), // prime offset per retry
            );

            // 1. Resolve narrative function
            let narrative_fn = self.resolve_narrative_fn(event);

            // 2. Build SelectionContext
            let mut ctx = self.build_context(event, world, &narrative_fn);

            // 3-4. Resolve voice
            let resolved_voice = voice_id.and_then(|id| self.voices.resolve(id));
            if let Some(ref voice) = resolved_voice {
                ctx.voice_weights = Some(&voice.grammar_weights);
            }

            // Add markov model references to context
            for (corpus_id, model) in &self.markov_models {
                ctx.markov_models.insert(corpus_id.clone(), model);
            }

            // 5. Determine entry rule name
            let rule_name = format!("{}_opening", narrative_fn.name());

            // 6. Expand grammar
            let expanded = match self.grammars.expand(&rule_name, &mut ctx, &mut rng) {
                Ok(text) => text,
                Err(GrammarError::RuleNotFound(_)) => {
                    // Try without _opening suffix
                    match self.grammars.expand(narrative_fn.name(), &mut ctx, &mut rng) {
                        Ok(text) => text,
                        Err(e) => return Err(PipelineError::Grammar(e)),
                    }
                }
                Err(e) => return Err(PipelineError::Grammar(e)),
            };

            // 7. Run variety pass
            let output = if let Some(ref voice) = resolved_voice {
                VarietyPass::apply(&expanded, voice, &self.context, &mut rng)
            } else {
                expanded
            };

            // 8. Check for repetition
            let issues = self.context.check_repetition(&output);
            if issues.is_empty() || retry == max_retries - 1 {
                // 9. Record and return
                self.context.record(&output);
                self.generation_count += 1;
                return Ok(output);
            }
            // Retry with different seed offset
        }

        Err(PipelineError::GenerationFailed(max_retries))
    }

    fn resolve_narrative_fn(&self, event: &Event) -> NarrativeFunction {
        // Event can specify narrative_fn directly
        // Or look up from mappings table
        if let Some(mapped) = self.mappings.get(&event.event_type) {
            mapped.clone()
        } else {
            event.narrative_fn.clone()
        }
    }

    fn build_context<'a>(
        &'a self,
        event: &Event,
        world: &'a WorldState<'_>,
        narrative_fn: &NarrativeFunction,
    ) -> SelectionContext<'a> {
        let mut ctx = SelectionContext::new();

        // Add mood and stakes as tags
        ctx.tags.insert(event.mood.tag().to_string());
        ctx.tags.insert(event.stakes.tag().to_string());

        // Add narrative function as tag
        ctx.tags
            .insert(format!("fn:{}", narrative_fn.name()));

        // Add intensity-based tags
        let intensity = narrative_fn.intensity();
        if intensity >= 0.7 {
            ctx.tags.insert("intensity:high".to_string());
        } else if intensity <= 0.3 {
            ctx.tags.insert("intensity:low".to_string());
        }

        // Add participant entity tags and bindings
        for (i, participant) in event.participants.iter().enumerate() {
            if let Some(entity) = world.entities.get(&participant.entity_id) {
                for tag in &entity.tags {
                    ctx.tags.insert(tag.clone());
                }

                // Bind by role
                ctx.entity_bindings
                    .insert(participant.role.clone(), entity);

                // First participant is also "subject" if no explicit subject role
                if i == 0 && !ctx.entity_bindings.contains_key("subject") {
                    ctx.entity_bindings.insert("subject".to_string(), entity);
                }
            }
        }

        // Add location entity tags
        if let Some(ref location) = event.location {
            if let Some(entity) = world.entities.get(&location.entity_id) {
                for tag in &entity.tags {
                    ctx.tags.insert(tag.clone());
                }
                ctx.entity_bindings
                    .insert(location.role.clone(), entity);
            }
        }

        ctx
    }
}

impl NarrativeEngineBuilder {
    pub fn genre_templates(mut self, templates: &[&str]) -> Self {
        self.genre_templates = templates.iter().map(|s| s.to_string()).collect();
        self
    }

    pub fn grammars_dir(mut self, path: &str) -> Self {
        self.grammars_dir = Some(path.to_string());
        self
    }

    pub fn voices_dir(mut self, path: &str) -> Self {
        self.voices_dir = Some(path.to_string());
        self
    }

    pub fn markov_models_dir(mut self, path: &str) -> Self {
        self.markov_models_dir = Some(path.to_string());
        self
    }

    pub fn mappings(mut self, path: &str) -> Self {
        self.mappings_path = Some(path.to_string());
        self
    }

    pub fn seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    /// Provide grammars directly (for testing without files).
    pub fn with_grammars(mut self, grammars: GrammarSet) -> Self {
        self.grammars = Some(grammars);
        self
    }

    /// Provide voices directly (for testing without files).
    pub fn with_voices(mut self, voices: VoiceRegistry) -> Self {
        self.voices = Some(voices);
        self
    }

    /// Provide markov models directly (for testing without files).
    pub fn with_markov_models(mut self, models: HashMap<String, MarkovModel>) -> Self {
        self.markov_models = Some(models);
        self
    }

    /// Provide mappings directly (for testing without files).
    pub fn with_mappings(mut self, mappings: HashMap<String, NarrativeFunction>) -> Self {
        self.mappings = Some(mappings);
        self
    }

    pub fn build(self) -> Result<NarrativeEngine, PipelineError> {
        let mut grammars = self.grammars.unwrap_or_default();
        let mut voices = self.voices.unwrap_or_default();
        let mut markov_models = self.markov_models.unwrap_or_default();
        let mappings = self.mappings.unwrap_or_default();

        // Load genre templates
        for template_name in &self.genre_templates {
            let grammar_path = format!("genre_data/{}/grammar.ron", template_name);
            if Path::new(&grammar_path).exists() {
                let template_grammars = GrammarSet::load_from_ron(Path::new(&grammar_path))?;
                grammars.merge(template_grammars);
            }

            let voices_path = format!("genre_data/{}/voices.ron", template_name);
            if Path::new(&voices_path).exists() {
                voices.load_from_ron(Path::new(&voices_path))?;
            }
        }

        // Load game-specific grammars (override genre templates)
        if let Some(ref dir) = self.grammars_dir {
            if Path::new(dir).exists() {
                load_ron_files_from_dir(dir, |path| {
                    let gs = GrammarSet::load_from_ron(path)?;
                    grammars.merge(gs);
                    Ok(())
                })?;
            }
        }

        // Load game-specific voices
        if let Some(ref dir) = self.voices_dir {
            if Path::new(dir).exists() {
                load_ron_files_from_dir(dir, |path| {
                    voices.load_from_ron(path)?;
                    Ok(())
                })?;
            }
        }

        // Load Markov models
        if let Some(ref dir) = self.markov_models_dir {
            if Path::new(dir).exists() {
                load_ron_files_from_dir(dir, |path| {
                    let model = crate::core::markov::load_model(path)?;
                    let name = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    markov_models.insert(name, model);
                    Ok(())
                })?;
            }
        }

        // Load mappings
        let mappings = if let Some(ref path) = self.mappings_path {
            if Path::new(path).exists() {
                let contents = std::fs::read_to_string(path)?;
                let entries: Vec<EventMapping> = ron::from_str(&contents)?;
                let mut map = mappings;
                for entry in entries {
                    map.insert(entry.event_type, entry.narrative_fn);
                }
                map
            } else {
                mappings
            }
        } else {
            mappings
        };

        Ok(NarrativeEngine {
            grammars,
            voices,
            markov_models,
            mappings,
            context: NarrativeContext::default(),
            seed: self.seed,
            generation_count: 0,
        })
    }
}

/// Load all .ron files from a directory, calling `loader` for each.
fn load_ron_files_from_dir<F>(dir: &str, mut loader: F) -> Result<(), PipelineError>
where
    F: FnMut(&Path) -> Result<(), PipelineError>,
{
    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("ron") {
            loader(&path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::markov::MarkovTrainer;
    use crate::core::voice::Voice;
    use crate::schema::entity::Value;
    use crate::schema::event::{EntityRef, Mood, Stakes};

    fn build_test_engine() -> NarrativeEngine {
        // Create minimal grammar
        let grammar_ron = r#"{
            "confrontation_opening": Rule(
                requires: ["mood:tense"],
                excludes: [],
                alternatives: [
                    (weight: 3, text: "{subject} stepped forward. {tense_detail}"),
                    (weight: 2, text: "The tension was palpable. {subject} spoke first."),
                ],
            ),
            "tense_detail": Rule(
                requires: [],
                excludes: [],
                alternatives: [
                    (weight: 2, text: "The air felt heavy with unspoken words."),
                    (weight: 2, text: "No one dared to breathe."),
                    (weight: 1, text: "A silence settled over the room."),
                ],
            ),
            "revelation_opening": Rule(
                requires: [],
                excludes: [],
                alternatives: [
                    (weight: 2, text: "{subject} revealed the truth at last."),
                    (weight: 1, text: "The secret was finally out."),
                ],
            ),
        }"#;
        let grammars = GrammarSet::parse_ron(grammar_ron).unwrap();

        // Create a voice
        let mut voices = VoiceRegistry::new();
        voices.register(Voice {
            id: VoiceId(1),
            name: "narrator".to_string(),
            parent: None,
            grammar_weights: HashMap::new(),
            vocabulary: crate::core::voice::VocabularyPool::default(),
            markov_bindings: Vec::new(),
            structure_prefs: crate::core::voice::StructurePrefs::default(),
            quirks: Vec::new(),
        });

        // Train a small Markov model
        let corpus = std::fs::read_to_string("tests/fixtures/test_corpus.txt").unwrap();
        let markov_model = MarkovTrainer::train(&corpus, 2);

        let mut markov_models = HashMap::new();
        markov_models.insert("test_corpus".to_string(), markov_model);

        NarrativeEngine::builder()
            .seed(42)
            .with_grammars(grammars)
            .with_voices(voices)
            .with_markov_models(markov_models)
            .build()
            .unwrap()
    }

    fn make_test_world() -> (HashMap<EntityId, Entity>, Event) {
        let mut entities = HashMap::new();

        let margaret = Entity {
            id: EntityId(1),
            name: "Margaret".to_string(),
            pronouns: crate::schema::entity::Pronouns::SheHer,
            tags: ["host".to_string(), "formal".to_string()]
                .into_iter()
                .collect(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(1)),
            properties: HashMap::from([(
                "title".to_string(),
                Value::String("Duchess".to_string()),
            )]),
        };

        let james = Entity {
            id: EntityId(2),
            name: "James".to_string(),
            pronouns: crate::schema::entity::Pronouns::HeHim,
            tags: ["guest".to_string()].into_iter().collect(),
            relationships: Vec::new(),
            voice_id: None,
            properties: HashMap::new(),
        };

        entities.insert(EntityId(1), margaret);
        entities.insert(EntityId(2), james);

        let event = Event {
            event_type: "accusation".to_string(),
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

        (entities, event)
    }

    #[test]
    fn narrate_produces_output() {
        let mut engine = build_test_engine();
        let (entities, event) = make_test_world();
        let world = WorldState {
            entities: &entities,
        };

        let result = engine.narrate(&event, &world).unwrap();
        assert!(!result.is_empty(), "Expected non-empty narration");
        assert!(
            result.len() > 10,
            "Expected substantial text, got: {}",
            result
        );
    }

    #[test]
    fn narrate_deterministic_same_seed() {
        let (entities, event) = make_test_world();

        let mut engine1 = build_test_engine();
        let world1 = WorldState {
            entities: &entities,
        };
        let result1 = engine1.narrate(&event, &world1).unwrap();

        let mut engine2 = build_test_engine();
        let world2 = WorldState {
            entities: &entities,
        };
        let result2 = engine2.narrate(&event, &world2).unwrap();

        assert_eq!(result1, result2);
    }

    #[test]
    fn narrate_different_with_different_seed() {
        let (entities, event) = make_test_world();

        let mut found_different = false;
        let mut engine1 = NarrativeEngine::builder()
            .seed(1)
            .with_grammars(build_test_engine().grammars.clone())
            .build()
            .unwrap();
        let world = WorldState {
            entities: &entities,
        };
        let result1 = engine1.narrate(&event, &world).unwrap();

        for seed in 2..50 {
            let grammars_ron = r#"{
                "confrontation_opening": Rule(
                    requires: ["mood:tense"],
                    excludes: [],
                    alternatives: [
                        (weight: 3, text: "{subject} stepped forward. {tense_detail}"),
                        (weight: 2, text: "The tension was palpable. {subject} spoke first."),
                    ],
                ),
                "tense_detail": Rule(
                    requires: [],
                    excludes: [],
                    alternatives: [
                        (weight: 2, text: "The air felt heavy with unspoken words."),
                        (weight: 2, text: "No one dared to breathe."),
                        (weight: 1, text: "A silence settled over the room."),
                    ],
                ),
            }"#;
            let mut engine2 = NarrativeEngine::builder()
                .seed(seed)
                .with_grammars(GrammarSet::parse_ron(grammars_ron).unwrap())
                .build()
                .unwrap();
            let result2 = engine2.narrate(&event, &world).unwrap();
            if result1 != result2 {
                found_different = true;
                break;
            }
        }
        assert!(
            found_different,
            "Expected different output with different seeds"
        );
    }

    #[test]
    fn narrate_as_with_specific_voice() {
        let mut engine = build_test_engine();
        let (entities, event) = make_test_world();
        let world = WorldState {
            entities: &entities,
        };

        let result = engine.narrate_as(&event, VoiceId(1), &world).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn narrate_variants_produces_multiple() {
        let mut engine = build_test_engine();
        let (entities, event) = make_test_world();
        let world = WorldState {
            entities: &entities,
        };

        let variants = engine.narrate_variants(&event, 3, &world).unwrap();
        assert_eq!(variants.len(), 3);
        for v in &variants {
            assert!(!v.is_empty());
        }
    }

    #[test]
    fn narrate_contains_entity_name() {
        let mut engine = build_test_engine();
        let (entities, event) = make_test_world();
        let world = WorldState {
            entities: &entities,
        };

        // Run several seeds — at least one should contain Margaret
        let mut found_name = false;
        for _ in 0..10 {
            let result = engine.narrate(&event, &world).unwrap();
            if result.contains("Margaret") {
                found_name = true;
                break;
            }
        }
        assert!(
            found_name,
            "Expected entity name in at least one narration"
        );
    }

    #[test]
    fn builder_with_seed() {
        let engine = NarrativeEngine::builder().seed(12345).build().unwrap();
        assert_eq!(engine.seed, 12345);
    }
}
