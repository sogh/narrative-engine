/// Markov chain phrase generator — training, serialization, and generation.

use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MarkovError {
    #[error("no data for generation (model is empty or tag has no data)")]
    NoData,
    #[error("no sentence start found")]
    NoSentenceStart,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("RON deserialization error: {0}")]
    Ron(#[from] ron::error::SpannedError),
}

/// Special token marking sentence start.
const SENTENCE_START: &str = "<S>";
/// Special token marking sentence end.
const SENTENCE_END: &str = "</S>";

/// Punctuation characters that are tokenized as separate tokens.
const SENTENCE_ENDERS: &[char] = &['.', '!', '?'];
const PUNCTUATION: &[char] = &['.', '!', '?', ',', ';', ':', '"', '\''];

/// A trained Markov model storing n-gram probability tables.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MarkovModel {
    /// N-gram depth (e.g., 2 for bigrams, 3 for trigrams).
    pub n: usize,
    /// Transition table: n-gram prefix → [(next_token, count)].
    pub transitions: HashMap<Vec<String>, Vec<(String, u32)>>,
    /// Tag-specific transition tables.
    pub tagged_transitions: HashMap<String, HashMap<Vec<String>, Vec<(String, u32)>>>,
}

impl MarkovModel {
    /// Generate text from this model.
    ///
    /// Starts from a sentence-start state, walks the chain selecting next
    /// tokens by weighted probability, and stops at a sentence boundary
    /// within the word count range.
    pub fn generate(
        &self,
        rng: &mut StdRng,
        tag: Option<&str>,
        min_words: usize,
        max_words: usize,
    ) -> Result<String, MarkovError> {
        let transitions = if let Some(tag) = tag {
            self.tagged_transitions.get(tag).ok_or(MarkovError::NoData)?
        } else {
            &self.transitions
        };

        if transitions.is_empty() {
            return Err(MarkovError::NoData);
        }

        let mut result_tokens: Vec<String> = Vec::new();
        let mut state: Vec<String> = vec![SENTENCE_START.to_string(); self.n - 1];
        let mut word_count = 0;
        let mut last_sentence_end = 0;

        for _ in 0..(max_words * 3) {
            // safety limit on iterations
            let next = match pick_next(transitions, &state, rng) {
                Some(tok) => tok,
                None => break,
            };

            if next == SENTENCE_END {
                // Record sentence boundary position
                last_sentence_end = result_tokens.len();

                if word_count >= min_words {
                    break;
                }

                // Start a new sentence
                state = vec![SENTENCE_START.to_string(); self.n - 1];
                continue;
            }

            // Count actual words (not punctuation)
            if !PUNCTUATION.contains(&next.chars().next().unwrap_or(' ')) {
                word_count += 1;
            }

            result_tokens.push(next.clone());

            // Slide state window
            state.push(next);
            if state.len() > self.n - 1 {
                state.remove(0);
            }

            if word_count >= max_words {
                // Truncate at last complete sentence
                if last_sentence_end > 0 {
                    result_tokens.truncate(last_sentence_end);
                }
                break;
            }
        }

        if result_tokens.is_empty() {
            return Err(MarkovError::NoSentenceStart);
        }

        Ok(reassemble_tokens(&result_tokens))
    }
}

/// Pick the next token from transitions given a state prefix.
fn pick_next(
    transitions: &HashMap<Vec<String>, Vec<(String, u32)>>,
    state: &[String],
    rng: &mut StdRng,
) -> Option<String> {
    let options = transitions.get(state)?;
    if options.is_empty() {
        return None;
    }

    let weights: Vec<u32> = options.iter().map(|(_, count)| *count).collect();
    let dist = WeightedIndex::new(&weights).ok()?;
    Some(options[dist.sample(rng)].0.clone())
}

/// Reassemble tokens into natural text (attach punctuation to previous word).
fn reassemble_tokens(tokens: &[String]) -> String {
    let mut result = String::new();
    for (i, tok) in tokens.iter().enumerate() {
        let is_punct = tok.len() == 1 && PUNCTUATION.contains(&tok.chars().next().unwrap());
        if i > 0 && !is_punct {
            result.push(' ');
        }
        result.push_str(tok);
    }
    result
}

/// Trains Markov models from raw text.
pub struct MarkovTrainer;

impl MarkovTrainer {
    /// Train a Markov model from raw text with the given n-gram depth.
    ///
    /// Supports tagged regions: lines prefixed with `[tag]` apply that tag
    /// to subsequent text until the next tag or end of file.
    pub fn train(text: &str, n: usize) -> MarkovModel {
        assert!((2..=4).contains(&n), "n-gram depth must be 2-4");

        let mut transitions: HashMap<Vec<String>, Vec<(String, u32)>> = HashMap::new();
        let mut tagged_transitions: HashMap<
            String,
            HashMap<Vec<String>, Vec<(String, u32)>>,
        > = HashMap::new();

        let mut current_tag: Option<String> = None;

        for line in text.lines() {
            let trimmed = line.trim();

            // Check for tag markers: [tagname]
            if trimmed.starts_with('[') && trimmed.ends_with(']') && trimmed.len() > 2 {
                let tag = &trimmed[1..trimmed.len() - 1];
                current_tag = Some(tag.to_string());
                continue;
            }

            if trimmed.is_empty() {
                continue;
            }

            let tokens = tokenize(trimmed);
            let sentences = split_into_sentences(&tokens);

            for sentence in &sentences {
                // Build n-gram chain for this sentence
                let mut padded = vec![SENTENCE_START.to_string(); n - 1];
                padded.extend(sentence.iter().cloned());
                padded.push(SENTENCE_END.to_string());

                for window in padded.windows(n) {
                    let prefix: Vec<String> = window[..n - 1].to_vec();
                    let next = window[n - 1].clone();

                    // Add to global transitions
                    add_transition(&mut transitions, prefix.clone(), next.clone());

                    // Add to tagged transitions if we have a tag
                    if let Some(ref tag) = current_tag {
                        let tag_table = tagged_transitions
                            .entry(tag.clone())
                            .or_default();
                        add_transition(tag_table, prefix, next);
                    }
                }
            }
        }

        MarkovModel {
            n,
            transitions,
            tagged_transitions,
        }
    }
}

/// Add a transition to a transition table, incrementing the count.
fn add_transition(
    table: &mut HashMap<Vec<String>, Vec<(String, u32)>>,
    prefix: Vec<String>,
    next: String,
) {
    let entries = table.entry(prefix).or_default();
    if let Some(entry) = entries.iter_mut().find(|(tok, _)| tok == &next) {
        entry.1 += 1;
    } else {
        entries.push((next, 1));
    }
}

/// Tokenize text: split on whitespace, separate punctuation as individual tokens.
fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    for word in text.split_whitespace() {
        let mut remaining = word;
        while !remaining.is_empty() {
            // Check if starts with punctuation
            let first = remaining.chars().next().unwrap();
            if PUNCTUATION.contains(&first) {
                tokens.push(first.to_string());
                remaining = &remaining[first.len_utf8()..];
                continue;
            }

            // Find end of word (before punctuation)
            if let Some(pos) = remaining.find(|c: char| PUNCTUATION.contains(&c)) {
                tokens.push(remaining[..pos].to_string());
                remaining = &remaining[pos..];
            } else {
                tokens.push(remaining.to_string());
                break;
            }
        }
    }
    tokens
}

/// Split a token sequence into sentences at sentence-ending punctuation.
fn split_into_sentences(tokens: &[String]) -> Vec<Vec<String>> {
    let mut sentences = Vec::new();
    let mut current = Vec::new();

    for tok in tokens {
        current.push(tok.clone());
        if tok.len() == 1 && SENTENCE_ENDERS.contains(&tok.chars().next().unwrap()) {
            if !current.is_empty() {
                sentences.push(current.clone());
                current.clear();
            }
        }
    }

    // Don't discard trailing tokens without sentence ender
    if !current.is_empty() {
        sentences.push(current);
    }

    sentences
}

/// Blends output from multiple Markov models with configurable weights.
pub struct MarkovBlender;

impl MarkovBlender {
    /// Generate text by blending multiple models at each step.
    pub fn generate(
        models: &[(&MarkovModel, f32)],
        rng: &mut StdRng,
        tag: Option<&str>,
        min_words: usize,
        max_words: usize,
    ) -> Result<String, MarkovError> {
        if models.is_empty() {
            return Err(MarkovError::NoData);
        }

        // All models must have the same n
        let n = models[0].0.n;

        let mut result_tokens: Vec<String> = Vec::new();
        let mut state: Vec<String> = vec![SENTENCE_START.to_string(); n - 1];
        let mut word_count = 0;
        let mut last_sentence_end = 0;

        for _ in 0..(max_words * 3) {
            // Blend transition probabilities from all models
            let next = match pick_next_blended(models, &state, tag, rng) {
                Some(tok) => tok,
                None => break,
            };

            if next == SENTENCE_END {
                last_sentence_end = result_tokens.len();
                if word_count >= min_words {
                    break;
                }
                state = vec![SENTENCE_START.to_string(); n - 1];
                continue;
            }

            if !PUNCTUATION.contains(&next.chars().next().unwrap_or(' ')) {
                word_count += 1;
            }

            result_tokens.push(next.clone());
            state.push(next);
            if state.len() > n - 1 {
                state.remove(0);
            }

            if word_count >= max_words {
                if last_sentence_end > 0 {
                    result_tokens.truncate(last_sentence_end);
                }
                break;
            }
        }

        if result_tokens.is_empty() {
            return Err(MarkovError::NoSentenceStart);
        }

        Ok(reassemble_tokens(&result_tokens))
    }
}

/// Pick next token by blending transition probabilities from multiple models.
fn pick_next_blended(
    models: &[(&MarkovModel, f32)],
    state: &[String],
    tag: Option<&str>,
    rng: &mut StdRng,
) -> Option<String> {
    let mut combined: HashMap<String, f64> = HashMap::new();

    for (model, blend_weight) in models {
        let transitions = if let Some(tag) = tag {
            model
                .tagged_transitions
                .get(tag)
                .unwrap_or(&model.transitions)
        } else {
            &model.transitions
        };

        if let Some(options) = transitions.get(state) {
            let total: u32 = options.iter().map(|(_, c)| c).sum();
            if total == 0 {
                continue;
            }
            for (tok, count) in options {
                let prob = (*count as f64) / (total as f64) * (*blend_weight as f64);
                *combined.entry(tok.clone()).or_default() += prob;
            }
        }
    }

    if combined.is_empty() {
        return None;
    }

    let tokens: Vec<String> = combined.keys().cloned().collect();
    let weights: Vec<f64> = tokens.iter().map(|t| combined[t]).collect();
    let dist = WeightedIndex::new(&weights).ok()?;
    Some(tokens[dist.sample(rng)].clone())
}

/// Save a MarkovModel to a RON file.
pub fn save_model(model: &MarkovModel, path: &std::path::Path) -> Result<(), MarkovError> {
    let serialized = ron::ser::to_string_pretty(model, ron::ser::PrettyConfig::default())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
    std::fs::write(path, serialized)?;
    Ok(())
}

/// Load a MarkovModel from a RON file.
pub fn load_model(path: &std::path::Path) -> Result<MarkovModel, MarkovError> {
    let contents = std::fs::read_to_string(path)?;
    let model: MarkovModel = ron::from_str(&contents)?;
    Ok(model)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn train_test_corpus() -> MarkovModel {
        let corpus = std::fs::read_to_string("tests/fixtures/test_corpus.txt").unwrap();
        MarkovTrainer::train(&corpus, 2)
    }

    #[test]
    fn tokenize_basic() {
        let tokens = tokenize("Hello, world.");
        assert_eq!(tokens, vec!["Hello", ",", "world", "."]);
    }

    #[test]
    fn tokenize_complex() {
        let tokens = tokenize("She said, \"What?\" He replied.");
        assert!(tokens.contains(&"She".to_string()));
        assert!(tokens.contains(&",".to_string()));
        assert!(tokens.contains(&"?".to_string()));
        assert!(tokens.contains(&".".to_string()));
    }

    #[test]
    fn train_creates_transitions() {
        let model = train_test_corpus();
        assert_eq!(model.n, 2);
        assert!(!model.transitions.is_empty());
    }

    #[test]
    fn train_creates_tagged_transitions() {
        let model = train_test_corpus();
        assert!(model.tagged_transitions.contains_key("neutral"));
        assert!(model.tagged_transitions.contains_key("tense"));
        assert!(model.tagged_transitions.contains_key("warm"));
    }

    #[test]
    fn generate_deterministic() {
        let model = train_test_corpus();
        let mut rng1 = StdRng::seed_from_u64(42);
        let mut rng2 = StdRng::seed_from_u64(42);

        let result1 = model.generate(&mut rng1, None, 3, 20).unwrap();
        let result2 = model.generate(&mut rng2, None, 3, 20).unwrap();
        assert_eq!(result1, result2);
    }

    #[test]
    fn generate_produces_output() {
        let model = train_test_corpus();
        let mut rng = StdRng::seed_from_u64(42);

        let result = model.generate(&mut rng, None, 3, 20).unwrap();
        assert!(!result.is_empty());
        let word_count = result.split_whitespace().count();
        assert!(word_count >= 3, "Expected at least 3 words, got: {}", word_count);
    }

    #[test]
    fn generate_respects_sentence_boundaries() {
        let model = train_test_corpus();
        let mut rng = StdRng::seed_from_u64(42);

        let result = model.generate(&mut rng, None, 3, 20).unwrap();
        // Result should end with sentence-ending punctuation or the last token
        let trimmed = result.trim();
        let last_char = trimmed.chars().last().unwrap();
        assert!(
            SENTENCE_ENDERS.contains(&last_char) || last_char.is_alphanumeric(),
            "Expected sentence boundary or word end, got: '{}'",
            last_char
        );
    }

    #[test]
    fn generate_with_tag() {
        let model = train_test_corpus();
        let mut rng = StdRng::seed_from_u64(42);

        let result = model.generate(&mut rng, Some("tense"), 3, 20).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn tag_filtering_changes_output() {
        let model = train_test_corpus();

        // Generate multiple outputs with different tags and check they differ
        let mut found_different = false;
        for seed in 0..50 {
            let mut rng1 = StdRng::seed_from_u64(seed);
            let mut rng2 = StdRng::seed_from_u64(seed);

            let neutral = model.generate(&mut rng1, Some("neutral"), 3, 15);
            let tense = model.generate(&mut rng2, Some("tense"), 3, 15);

            if let (Ok(n), Ok(t)) = (neutral, tense) {
                if n != t {
                    found_different = true;
                    break;
                }
            }
        }
        assert!(found_different, "Tagged generation should produce different output");
    }

    #[test]
    fn generate_invalid_tag_returns_error() {
        let model = train_test_corpus();
        let mut rng = StdRng::seed_from_u64(42);

        let result = model.generate(&mut rng, Some("nonexistent_tag"), 3, 20);
        assert!(result.is_err());
    }

    #[test]
    fn ron_round_trip() {
        let model = train_test_corpus();

        let serialized = ron::to_string(&model).unwrap();
        let deserialized: MarkovModel = ron::from_str(&serialized).unwrap();

        assert_eq!(deserialized.n, model.n);
        assert_eq!(deserialized.transitions.len(), model.transitions.len());
    }

    #[test]
    fn save_and_load_model() {
        let model = train_test_corpus();
        let path = std::path::PathBuf::from("target/test_markov_model.ron");

        save_model(&model, &path).unwrap();
        let loaded = load_model(&path).unwrap();

        assert_eq!(loaded.n, model.n);
        assert_eq!(loaded.transitions.len(), model.transitions.len());

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn blending_produces_output() {
        let model = train_test_corpus();
        let mut rng = StdRng::seed_from_u64(42);

        let result = MarkovBlender::generate(
            &[(&model, 1.0)],
            &mut rng,
            None,
            3,
            20,
        )
        .unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn trigram_model() {
        let corpus = std::fs::read_to_string("tests/fixtures/test_corpus.txt").unwrap();
        let model = MarkovTrainer::train(&corpus, 3);
        assert_eq!(model.n, 3);

        let mut rng = StdRng::seed_from_u64(42);
        let result = model.generate(&mut rng, None, 3, 20).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn reassemble_attaches_punctuation() {
        let tokens = vec![
            "Hello".to_string(),
            ",".to_string(),
            "world".to_string(),
            ".".to_string(),
        ];
        let result = reassemble_tokens(&tokens);
        assert_eq!(result, "Hello, world.");
    }
}
