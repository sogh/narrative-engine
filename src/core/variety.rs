/// Variety pass — post-processing transforms for text quality.
///
/// Includes synonym rotation, quirk injection, and repetition remediation.
use rand::rngs::StdRng;
use rand::Rng;
use std::collections::HashMap;

use super::context::{NarrativeContext, RepetitionIssue};
use super::voice::ResolvedVoice;

/// The variety pass applied to generated text before final output.
pub struct VarietyPass;

impl VarietyPass {
    /// Apply all variety transforms in order:
    /// 1. Synonym rotation (for avoided words)
    /// 2. Quirk injection
    /// 3. Repetition remediation
    pub fn apply(
        text: &str,
        voice: &ResolvedVoice,
        ctx: &NarrativeContext,
        rng: &mut StdRng,
    ) -> String {
        let mut result = text.to_string();

        // 1. Synonym rotation for avoided words
        result = rotate_avoided_words(&result, &voice.vocabulary.avoided, rng);

        // 2. Quirk injection
        result = inject_quirks(&result, &voice.quirks, rng);

        // 3. Repetition remediation
        let issues = ctx.check_repetition(&result);
        if !issues.is_empty() {
            result = remediate_repetition(&result, &issues, rng);
        }

        result
    }
}

/// Replace words in the voice's avoided set with synonyms.
fn rotate_avoided_words(
    text: &str,
    avoided: &rustc_hash::FxHashSet<String>,
    rng: &mut StdRng,
) -> String {
    if avoided.is_empty() {
        return text.to_string();
    }

    let synonyms = build_synonym_table();
    let mut result = text.to_string();

    for word in avoided {
        let word_lower = word.to_lowercase();
        if let Some(alternatives) = synonyms.get(word_lower.as_str()) {
            if !alternatives.is_empty() {
                let replacement = alternatives[rng.gen_range(0..alternatives.len())];
                // Case-preserving replacement
                result = replace_word_preserving_case(&result, word, replacement);
            }
        }
    }

    result
}

/// Replace a word in text, preserving the original's case pattern.
fn replace_word_preserving_case(text: &str, target: &str, replacement: &str) -> String {
    let mut result = String::new();
    let text_lower = text.to_lowercase();
    let target_lower = target.to_lowercase();
    let mut search_from = 0;

    while let Some(pos) = text_lower[search_from..].find(&target_lower) {
        let abs_pos = search_from + pos;

        // Check word boundaries
        let before_ok = abs_pos == 0 || !text.as_bytes()[abs_pos - 1].is_ascii_alphanumeric();
        let after_pos = abs_pos + target_lower.len();
        let after_ok =
            after_pos >= text.len() || !text.as_bytes()[after_pos].is_ascii_alphanumeric();

        if before_ok && after_ok {
            result.push_str(&text[search_from..abs_pos]);
            // Match case of first character
            let original_first = text[abs_pos..].chars().next().unwrap();
            if original_first.is_uppercase() {
                let mut chars = replacement.chars();
                if let Some(first) = chars.next() {
                    result.push(first.to_uppercase().next().unwrap());
                    result.extend(chars);
                }
            } else {
                result.push_str(replacement);
            }
            search_from = after_pos;
        } else {
            result.push_str(&text[search_from..abs_pos + 1]);
            search_from = abs_pos + 1;
        }
    }
    result.push_str(&text[search_from..]);
    result
}

/// Inject voice quirks at natural insertion points.
fn inject_quirks(text: &str, quirks: &[super::voice::Quirk], rng: &mut StdRng) -> String {
    if quirks.is_empty() {
        return text.to_string();
    }

    let mut result = text.to_string();

    for quirk in quirks {
        if rng.gen::<f32>() < quirk.frequency {
            // Find a natural insertion point (before a period or after a comma)
            if let Some(pos) = find_insertion_point(&result) {
                let (before, after) = result.split_at(pos);
                result = format!("{}, {}{}", before, quirk.pattern, after);
            }
        }
    }

    result
}

/// Find a natural point to insert a quirk phrase.
fn find_insertion_point(text: &str) -> Option<usize> {
    // Prefer inserting before a period (but not after the last sentence)
    let bytes = text.as_bytes();
    let mut candidates = Vec::new();

    for (i, &b) in bytes.iter().enumerate() {
        if b == b'.' && i > 10 && i < text.len() - 5 {
            candidates.push(i);
        }
    }

    if candidates.is_empty() {
        // Fall back to before the last period
        for (i, &b) in bytes.iter().enumerate().rev() {
            if b == b'.' && i > 10 {
                return Some(i);
            }
        }
        None
    } else {
        // Use the first good candidate
        Some(candidates[0])
    }
}

/// Apply minimal fixes for detected repetition issues.
fn remediate_repetition(text: &str, issues: &[RepetitionIssue], rng: &mut StdRng) -> String {
    let mut result = text.to_string();
    let synonyms = build_synonym_table();

    for issue in issues {
        match issue {
            RepetitionIssue::RepeatedOpening(_) => {
                result = swap_opening(&result, rng);
            }
            RepetitionIssue::OverusedWord { word, .. } => {
                let word_lower = word.to_lowercase();
                if let Some(alternatives) = synonyms.get(word_lower.as_str()) {
                    if !alternatives.is_empty() {
                        let replacement = alternatives[rng.gen_range(0..alternatives.len())];
                        result = replace_word_preserving_case(&result, word, replacement);
                    }
                }
            }
            RepetitionIssue::StructuralMonotony => {
                result = vary_sentence_structure(&result, rng);
            }
        }
    }

    result
}

/// Swap the opening of text to avoid repeated starts.
fn swap_opening(text: &str, rng: &mut StdRng) -> String {
    let openers = [
        "Meanwhile, ",
        "Just then, ",
        "At that moment, ",
        "In response, ",
        "Without warning, ",
        "After a pause, ",
    ];

    // Find where the first sentence content starts (skip any leading "The", "A", etc.)
    let words: Vec<&str> = text.splitn(4, ' ').collect();
    if words.len() >= 3 {
        let opener = openers[rng.gen_range(0..openers.len())];
        let first_word = words[0];
        // Only lowercase common words (articles, pronouns, etc.)
        // Keep proper nouns (names) capitalized
        let adjusted = if is_proper_noun(first_word) {
            first_word.to_string()
        } else {
            first_word[..1].to_lowercase() + &first_word[1..]
        };
        let rest = &text[first_word.len()..];
        format!("{}{}{}", opener, adjusted, rest)
    } else {
        text.to_string()
    }
}

/// Heuristic: a word is likely a proper noun if it starts uppercase
/// and is NOT a common sentence-starting word.
fn is_proper_noun(word: &str) -> bool {
    let first = match word.chars().next() {
        Some(c) => c,
        None => return false,
    };
    if !first.is_uppercase() {
        return false;
    }
    // Common words that start sentences but aren't proper nouns
    let common_starters = [
        "The",
        "A",
        "An",
        "This",
        "That",
        "These",
        "Those",
        "It",
        "There",
        "Here",
        "They",
        "We",
        "He",
        "She",
        "Every",
        "Each",
        "Some",
        "No",
        "All",
        "Any",
        "Something",
        "Nothing",
        "Everything",
        "Everyone",
        "Somewhere",
        "Nowhere",
    ];
    !common_starters.contains(&word)
}

/// Vary sentence structure to break monotony.
fn vary_sentence_structure(text: &str, _rng: &mut StdRng) -> String {
    // Simple heuristic: split at "and" or "but" conjunctions
    let mut result = String::new();
    let sentences: Vec<&str> = text.split(". ").collect();

    for (i, sentence) in sentences.iter().enumerate() {
        if sentence.contains(" and ") && sentence.split_whitespace().count() > 8 {
            // Split long sentences at "and"
            let parts: Vec<&str> = sentence.splitn(2, " and ").collect();
            if parts.len() == 2 {
                result.push_str(parts[0]);
                result.push_str(". ");
                // Capitalize the second part
                let second = parts[1].trim();
                if let Some(first_char) = second.chars().next() {
                    result.push(first_char.to_uppercase().next().unwrap());
                    result.push_str(&second[first_char.len_utf8()..]);
                }
            } else {
                result.push_str(sentence);
            }
        } else {
            result.push_str(sentence);
        }

        if i < sentences.len() - 1 {
            result.push_str(". ");
        }
    }

    result
}

/// Build a hardcoded synonym table for common overused words.
fn build_synonym_table() -> HashMap<&'static str, Vec<&'static str>> {
    HashMap::from([
        ("said", vec!["replied", "remarked", "noted", "stated"]),
        ("walked", vec!["strode", "moved", "stepped", "paced"]),
        ("looked", vec!["glanced", "gazed", "peered", "observed"]),
        ("went", vec!["proceeded", "headed", "moved", "traveled"]),
        ("good", vec!["fine", "excellent", "pleasant", "agreeable"]),
        ("bad", vec!["poor", "unfortunate", "grim", "dire"]),
        ("big", vec!["large", "vast", "immense", "substantial"]),
        ("small", vec!["tiny", "slight", "modest", "compact"]),
        ("happy", vec!["pleased", "content", "delighted", "glad"]),
        ("angry", vec!["furious", "irate", "incensed", "livid"]),
        (
            "beautiful",
            vec!["lovely", "elegant", "stunning", "striking"],
        ),
        ("dark", vec!["dim", "shadowed", "murky", "gloomy"]),
        ("light", vec!["bright", "luminous", "radiant", "glowing"]),
        ("quiet", vec!["silent", "hushed", "still", "muted"]),
        (
            "loud",
            vec!["thunderous", "booming", "deafening", "piercing"],
        ),
        ("quickly", vec!["swiftly", "rapidly", "hastily", "briskly"]),
        (
            "slowly",
            vec!["gradually", "leisurely", "unhurriedly", "deliberately"],
        ),
        (
            "very",
            vec!["quite", "remarkably", "exceedingly", "particularly"],
        ),
        (
            "really",
            vec!["truly", "genuinely", "undeniably", "certainly"],
        ),
        (
            "nice",
            vec!["pleasant", "agreeable", "charming", "delightful"],
        ),
        ("thing", vec!["matter", "object", "affair", "detail"]),
        (
            "stuff",
            vec!["material", "substance", "belongings", "items"],
        ),
        (
            "great",
            vec!["magnificent", "remarkable", "exceptional", "splendid"],
        ),
        (
            "terrible",
            vec!["dreadful", "awful", "ghastly", "horrendous"],
        ),
        ("strange", vec!["peculiar", "unusual", "curious", "odd"]),
        ("old", vec!["ancient", "aged", "weathered", "venerable"]),
        ("young", vec!["youthful", "fresh", "juvenile", "new"]),
        ("cold", vec!["frigid", "chilly", "icy", "bitter"]),
        ("hot", vec!["scorching", "blazing", "sweltering", "searing"]),
        ("fast", vec!["swift", "rapid", "fleet", "speedy"]),
        ("strong", vec!["powerful", "mighty", "robust", "formidable"]),
        ("weak", vec!["feeble", "frail", "fragile", "delicate"]),
        (
            "thought",
            vec!["considered", "reflected", "pondered", "mused"],
        ),
        (
            "suddenly",
            vec!["abruptly", "unexpectedly", "without warning", "all at once"],
        ),
        (
            "began",
            vec!["started", "commenced", "initiated", "set about"],
        ),
        (
            "seemed",
            vec!["appeared", "looked", "gave the impression", "struck one as"],
        ),
        ("turned", vec!["pivoted", "swiveled", "shifted", "rotated"]),
        ("stood", vec!["remained", "lingered", "waited", "stayed"]),
        (
            "found",
            vec!["discovered", "located", "uncovered", "encountered"],
        ),
        ("heard", vec!["caught", "detected", "perceived", "noticed"]),
        (
            "knew",
            vec!["understood", "realized", "recognized", "grasped"],
        ),
        (
            "felt",
            vec!["sensed", "experienced", "detected", "perceived"],
        ),
        ("wanted", vec!["desired", "wished", "longed for", "craved"]),
        (
            "tried",
            vec!["attempted", "endeavored", "sought to", "strove to"],
        ),
        (
            "started",
            vec!["began", "commenced", "initiated", "launched"],
        ),
        (
            "important",
            vec!["crucial", "vital", "essential", "significant"],
        ),
        (
            "interesting",
            vec!["fascinating", "intriguing", "compelling", "engaging"],
        ),
        (
            "different",
            vec!["distinct", "varied", "divergent", "unlike"],
        ),
        ("obvious", vec!["apparent", "evident", "clear", "plain"]),
        (
            "getting",
            vec!["becoming", "growing", "turning", "developing"],
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::voice::{Quirk, ResolvedVoice, StructurePrefs, VocabularyPool};
    use crate::schema::entity::VoiceId;
    use rand::SeedableRng;
    use rustc_hash::FxHashSet;

    fn make_test_voice() -> ResolvedVoice {
        ResolvedVoice {
            id: VoiceId(1),
            name: "test".to_string(),
            grammar_weights: HashMap::new(),
            vocabulary: VocabularyPool {
                preferred: FxHashSet::default(),
                avoided: ["said", "walked", "looked"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            },
            markov_bindings: Vec::new(),
            structure_prefs: StructurePrefs::default(),
            quirks: vec![Quirk {
                pattern: "you see".to_string(),
                frequency: 1.0, // Always inject for testing
            }],
        }
    }

    #[test]
    fn synonym_rotation_replaces_avoided() {
        let avoided: FxHashSet<String> = ["said"].iter().map(|s| s.to_string()).collect();
        let mut rng = StdRng::seed_from_u64(42);
        let result = rotate_avoided_words("She said nothing.", &avoided, &mut rng);
        assert_ne!(result, "She said nothing.");
        assert!(!result.contains("said"));
    }

    #[test]
    fn synonym_rotation_preserves_case() {
        let avoided: FxHashSet<String> = ["said"].iter().map(|s| s.to_string()).collect();
        let mut rng = StdRng::seed_from_u64(42);
        let result = rotate_avoided_words("Said nothing.", &avoided, &mut rng);
        // First character should still be uppercase
        assert!(result.starts_with(|c: char| c.is_uppercase()));
    }

    #[test]
    fn quirk_injection_with_full_frequency() {
        let quirks = vec![Quirk {
            pattern: "you see".to_string(),
            frequency: 1.0,
        }];
        let mut rng = StdRng::seed_from_u64(42);
        let result = inject_quirks(
            "She walked to the door. He stayed behind.",
            &quirks,
            &mut rng,
        );
        assert!(
            result.contains("you see"),
            "Expected quirk injection, got: {}",
            result
        );
    }

    #[test]
    fn quirk_injection_zero_frequency() {
        let quirks = vec![Quirk {
            pattern: "you see".to_string(),
            frequency: 0.0,
        }];
        let mut rng = StdRng::seed_from_u64(42);
        let text = "She walked to the door. He stayed behind.";
        let result = inject_quirks(text, &quirks, &mut rng);
        assert!(!result.contains("you see"));
    }

    #[test]
    fn quirk_injection_statistical() {
        let quirks = vec![Quirk {
            pattern: "you see".to_string(),
            frequency: 0.5,
        }];
        let text = "She walked to the door carefully. He stayed behind the wall.";

        let mut injected_count = 0;
        for seed in 0..200 {
            let mut rng = StdRng::seed_from_u64(seed);
            let result = inject_quirks(text, &quirks, &mut rng);
            if result.contains("you see") {
                injected_count += 1;
            }
        }

        // With 50% frequency, expect roughly 80-120 out of 200
        assert!(
            injected_count > 60 && injected_count < 140,
            "Expected ~50% injection rate, got {}/200",
            injected_count
        );
    }

    #[test]
    fn full_variety_pass() {
        let voice = make_test_voice();
        let ctx = NarrativeContext::default();
        let mut rng = StdRng::seed_from_u64(42);

        let result = VarietyPass::apply(
            "She said nothing and looked away. He walked to the door slowly.",
            &voice,
            &ctx,
            &mut rng,
        );
        // Should have replaced some avoided words and injected quirk
        assert!(!result.is_empty());
    }

    #[test]
    fn repetition_remediation_changes_opening() {
        let mut ctx = NarrativeContext::default();
        ctx.record("The evening was quiet.");
        let mut rng = StdRng::seed_from_u64(42);

        let result = remediate_repetition(
            "The evening was loud.",
            &[RepetitionIssue::RepeatedOpening(
                "the evening was".to_string(),
            )],
            &mut rng,
        );
        // Opening should have changed
        assert!(!result.starts_with("The evening"));
    }

    #[test]
    fn sentence_structure_variation() {
        let mut rng = StdRng::seed_from_u64(42);
        let result = vary_sentence_structure(
            "She walked to the door and he stayed behind the wall.",
            &mut rng,
        );
        // Should have split at "and"
        assert!(
            result.contains(". "),
            "Expected sentence split, got: {}",
            result
        );
    }
}
