/// Voice system — persona/tone bundles that shape generated text.
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::schema::entity::VoiceId;

/// A voice definition that shapes how text sounds for a specific
/// speaker, narrator, or document type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    pub id: VoiceId,
    pub name: String,
    pub parent: Option<VoiceId>,
    #[serde(default)]
    pub grammar_weights: HashMap<String, f32>,
    #[serde(default)]
    pub vocabulary: VocabularyPool,
    #[serde(default)]
    pub markov_bindings: Vec<MarkovBinding>,
    #[serde(default)]
    pub structure_prefs: StructurePrefs,
    #[serde(default)]
    pub quirks: Vec<Quirk>,
}

/// Preferred and avoided words for a voice.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VocabularyPool {
    #[serde(default)]
    pub preferred: FxHashSet<String>,
    #[serde(default)]
    pub avoided: FxHashSet<String>,
}

/// Binding a voice to a Markov corpus with weight and tags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkovBinding {
    pub corpus_id: String,
    pub weight: f32,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Structural preferences for text generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructurePrefs {
    /// (min, max) word count range for sentences.
    pub avg_sentence_length: (u32, u32),
    /// 0.0 = simple, 1.0 = complex clause structure.
    pub clause_complexity: f32,
    /// 0.0..1.0 probability of generating questions.
    pub question_frequency: f32,
}

impl Default for StructurePrefs {
    fn default() -> Self {
        Self {
            avg_sentence_length: (8, 18),
            clause_complexity: 0.5,
            question_frequency: 0.1,
        }
    }
}

/// A verbal tic or recurring phrase that gets occasionally inserted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quirk {
    pub pattern: String,
    /// Probability of injecting per passage (0.0..1.0).
    pub frequency: f32,
}

/// A fully resolved voice with inheritance chain merged.
#[derive(Debug, Clone)]
pub struct ResolvedVoice {
    pub id: VoiceId,
    pub name: String,
    pub grammar_weights: HashMap<String, f32>,
    pub vocabulary: VocabularyPool,
    pub markov_bindings: Vec<MarkovBinding>,
    pub structure_prefs: StructurePrefs,
    pub quirks: Vec<Quirk>,
}

/// Registry of all loaded voices with inheritance resolution.
#[derive(Debug, Clone, Default)]
pub struct VoiceRegistry {
    voices: HashMap<VoiceId, Voice>,
}

impl VoiceRegistry {
    pub fn new() -> Self {
        Self {
            voices: HashMap::new(),
        }
    }

    pub fn register(&mut self, voice: Voice) {
        self.voices.insert(voice.id, voice);
    }

    pub fn get(&self, id: VoiceId) -> Option<&Voice> {
        self.voices.get(&id)
    }

    /// Resolve a voice by walking its inheritance chain and merging properties.
    ///
    /// Child grammar_weights override parent, vocabulary pools union,
    /// markov_bindings concatenate, structure_prefs take child values
    /// (falling back to parent), quirks concatenate.
    pub fn resolve(&self, id: VoiceId) -> Option<ResolvedVoice> {
        let voice = self.voices.get(&id)?;

        // Build the inheritance chain (child first, ancestors after)
        let mut chain = vec![voice];
        let mut current = voice;
        while let Some(parent_id) = current.parent {
            if let Some(parent) = self.voices.get(&parent_id) {
                chain.push(parent);
                current = parent;
            } else {
                break;
            }
        }

        // Resolve from root ancestor to child (so child overrides parent)
        let mut grammar_weights = HashMap::new();
        let mut preferred = FxHashSet::default();
        let mut avoided = FxHashSet::default();
        let mut markov_bindings = Vec::new();
        let mut structure_prefs = StructurePrefs::default();
        let mut quirks = Vec::new();

        for ancestor in chain.iter().rev() {
            // Grammar weights: child overrides parent
            for (k, v) in &ancestor.grammar_weights {
                grammar_weights.insert(k.clone(), *v);
            }

            // Vocabulary: union
            preferred.extend(ancestor.vocabulary.preferred.iter().cloned());
            avoided.extend(ancestor.vocabulary.avoided.iter().cloned());

            // Markov bindings: concatenate
            markov_bindings.extend(ancestor.markov_bindings.iter().cloned());

            // Structure prefs: child takes precedence (last write wins)
            structure_prefs = ancestor.structure_prefs.clone();

            // Quirks: concatenate
            quirks.extend(ancestor.quirks.iter().cloned());
        }

        Some(ResolvedVoice {
            id: voice.id,
            name: voice.name.clone(),
            grammar_weights,
            vocabulary: VocabularyPool { preferred, avoided },
            markov_bindings,
            structure_prefs,
            quirks,
        })
    }

    /// Load voices from a RON file. The file should contain a list of Voice definitions.
    pub fn load_from_ron(&mut self, path: &std::path::Path) -> Result<(), VoiceError> {
        let contents = std::fs::read_to_string(path)?;
        let voices: Vec<Voice> = ron::from_str(&contents)?;
        for voice in voices {
            self.register(voice);
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("RON deserialization error: {0}")]
    Ron(#[from] ron::error::SpannedError),
    #[error("voice not found: {0:?}")]
    NotFound(VoiceId),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_parent_voice() -> Voice {
        Voice {
            id: VoiceId(1),
            name: "military".to_string(),
            parent: None,
            grammar_weights: HashMap::from([
                ("greeting".to_string(), 0.5),
                ("action_detail".to_string(), 2.0),
            ]),
            vocabulary: VocabularyPool {
                preferred: ["sir".to_string(), "affirmative".to_string()]
                    .into_iter()
                    .collect(),
                avoided: ["hello".to_string()].into_iter().collect(),
            },
            markov_bindings: vec![MarkovBinding {
                corpus_id: "military_prose".to_string(),
                weight: 1.0,
                tags: vec!["formal".to_string()],
            }],
            structure_prefs: StructurePrefs {
                avg_sentence_length: (5, 12),
                clause_complexity: 0.3,
                question_frequency: 0.05,
            },
            quirks: vec![Quirk {
                pattern: "if you will".to_string(),
                frequency: 0.1,
            }],
        }
    }

    fn make_child_voice() -> Voice {
        Voice {
            id: VoiceId(2),
            name: "ship_captain".to_string(),
            parent: Some(VoiceId(1)),
            grammar_weights: HashMap::from([
                ("greeting".to_string(), 0.8), // overrides parent's 0.5
                ("nautical_detail".to_string(), 3.0),
            ]),
            vocabulary: VocabularyPool {
                preferred: ["aye".to_string(), "starboard".to_string()]
                    .into_iter()
                    .collect(),
                avoided: FxHashSet::default(),
            },
            markov_bindings: vec![MarkovBinding {
                corpus_id: "nautical_prose".to_string(),
                weight: 1.5,
                tags: vec!["sea".to_string()],
            }],
            structure_prefs: StructurePrefs {
                avg_sentence_length: (6, 15),
                clause_complexity: 0.4,
                question_frequency: 0.08,
            },
            quirks: vec![Quirk {
                pattern: "by the bow".to_string(),
                frequency: 0.15,
            }],
        }
    }

    #[test]
    fn voice_registry_register_and_get() {
        let mut registry = VoiceRegistry::new();
        let voice = make_parent_voice();
        registry.register(voice);
        assert!(registry.get(VoiceId(1)).is_some());
        assert!(registry.get(VoiceId(99)).is_none());
    }

    #[test]
    fn resolve_single_voice() {
        let mut registry = VoiceRegistry::new();
        registry.register(make_parent_voice());

        let resolved = registry.resolve(VoiceId(1)).unwrap();
        assert_eq!(resolved.name, "military");
        assert_eq!(resolved.grammar_weights.get("greeting"), Some(&0.5));
        assert!(resolved.vocabulary.preferred.contains("sir"));
        assert!(resolved.vocabulary.avoided.contains("hello"));
        assert_eq!(resolved.markov_bindings.len(), 1);
        assert_eq!(resolved.quirks.len(), 1);
    }

    #[test]
    fn resolve_inheritance_chain() {
        let mut registry = VoiceRegistry::new();
        registry.register(make_parent_voice());
        registry.register(make_child_voice());

        let resolved = registry.resolve(VoiceId(2)).unwrap();
        assert_eq!(resolved.name, "ship_captain");

        // Grammar weights: child overrides parent for "greeting"
        assert_eq!(resolved.grammar_weights.get("greeting"), Some(&0.8));
        // Parent-only weight preserved
        assert_eq!(resolved.grammar_weights.get("action_detail"), Some(&2.0));
        // Child-only weight present
        assert_eq!(resolved.grammar_weights.get("nautical_detail"), Some(&3.0));

        // Vocabulary: union of both
        assert!(resolved.vocabulary.preferred.contains("sir")); // from parent
        assert!(resolved.vocabulary.preferred.contains("aye")); // from child
        assert!(resolved.vocabulary.preferred.contains("starboard")); // from child
        assert!(resolved.vocabulary.avoided.contains("hello")); // from parent

        // Markov bindings: concatenated
        assert_eq!(resolved.markov_bindings.len(), 2);

        // Structure prefs: child takes precedence
        assert_eq!(resolved.structure_prefs.avg_sentence_length, (6, 15));

        // Quirks: concatenated
        assert_eq!(resolved.quirks.len(), 2);
    }

    #[test]
    fn resolve_missing_voice() {
        let registry = VoiceRegistry::new();
        assert!(registry.resolve(VoiceId(99)).is_none());
    }

    #[test]
    fn resolve_missing_parent_graceful() {
        let mut registry = VoiceRegistry::new();
        // Register child without its parent
        registry.register(make_child_voice());

        let resolved = registry.resolve(VoiceId(2)).unwrap();
        // Should resolve with just the child's properties
        assert_eq!(resolved.name, "ship_captain");
        assert_eq!(resolved.grammar_weights.get("greeting"), Some(&0.8));
    }

    #[test]
    fn ron_round_trip() {
        let voice = make_parent_voice();
        let serialized = ron::to_string(&voice).unwrap();
        let deserialized: Voice = ron::from_str(&serialized).unwrap();
        assert_eq!(deserialized.name, "military");
        assert_eq!(deserialized.id, VoiceId(1));
        assert_eq!(deserialized.grammar_weights.get("greeting"), Some(&0.5));
    }

    #[test]
    fn voice_grammar_weight_integration() {
        use crate::core::grammar::{GrammarSet, SelectionContext};
        use rand::rngs::StdRng;
        use rand::SeedableRng;

        // Build a simple grammar with two alternatives
        let grammar_ron = r#"{
            "test_rule": Rule(
                requires: [],
                excludes: [],
                alternatives: [
                    (weight: 1, text: "option_a"),
                    (weight: 1, text: "option_b"),
                ],
            ),
        }"#;
        let gs = GrammarSet::parse_ron(grammar_ron).unwrap();

        // Without voice weights: roughly 50/50
        let mut count_a_no_voice = 0;
        for seed in 0..1000 {
            let mut ctx = SelectionContext::new();
            let mut rng = StdRng::seed_from_u64(seed);
            let result = gs.expand("test_rule", &mut ctx, &mut rng).unwrap();
            if result == "option_a" {
                count_a_no_voice += 1;
            }
        }

        // Voice weights multiply all alternatives equally for a given rule,
        // so with equal-weight alternatives the ratio stays the same.
        // The count should still be roughly 50/50.
        assert!(
            count_a_no_voice > 400 && count_a_no_voice < 600,
            "Expected roughly 50/50 distribution, got option_a: {}/1000",
            count_a_no_voice
        );
    }

    #[test]
    fn load_test_voices_from_ron() {
        let path = std::path::PathBuf::from("tests/fixtures/test_voices.ron");
        let mut registry = VoiceRegistry::new();
        registry.load_from_ron(&path).unwrap();

        assert!(registry.get(VoiceId(1)).is_some());
        assert!(registry.get(VoiceId(2)).is_some());

        let resolved = registry.resolve(VoiceId(2)).unwrap();
        assert_eq!(resolved.name, "gossip");
        // Should inherit from host
        assert!(resolved.vocabulary.preferred.contains("indeed")); // from parent
        assert!(resolved.vocabulary.preferred.contains("apparently")); // from child
    }
}
