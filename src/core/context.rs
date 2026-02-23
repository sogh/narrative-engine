/// Narrative context — anti-repetition tracking and pronoun management.
use std::collections::HashMap;

/// A sliding window of recently generated passages for repetition detection.
#[derive(Debug, Clone)]
pub struct NarrativeContext {
    /// Recent passages (most recent last).
    passages: Vec<String>,
    /// Maximum number of passages to track.
    window_size: usize,
    /// Recent sentence openings (first 3 words, lowercased).
    recent_openings: Vec<String>,
    /// Word frequency counts across the window.
    word_counts: HashMap<String, usize>,
    /// Entity mention counts for pronoun decisions.
    pub entity_mentions: HashMap<String, usize>,
}

impl Default for NarrativeContext {
    fn default() -> Self {
        Self::new(10)
    }
}

/// An issue detected by repetition checking.
#[derive(Debug, Clone, PartialEq)]
pub enum RepetitionIssue {
    /// The candidate's opening words match a recent passage.
    RepeatedOpening(String),
    /// A significant word appears too many times across recent context.
    OverusedWord { word: String, count: usize },
    /// Sentence lengths are too uniform across recent context.
    StructuralMonotony,
}

/// Stopwords that don't count as "significant" for repetition tracking.
const STOPWORDS: &[&str] = &[
    "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
    "from", "is", "it", "as", "was", "are", "be", "been", "had", "has", "have", "that", "this",
    "not", "her", "hers", "him", "his", "she", "he", "they", "them", "their", "theirs", "its",
    "herself", "himself", "themselves", "itself", "into", "than", "then",
    "were", "will", "would", "could", "should", "did", "does", "do", "all", "each", "every",
    "both", "few", "more", "most", "other", "some", "such", "only", "own", "same", "so", "just",
    "very",
];

impl NarrativeContext {
    pub fn new(window_size: usize) -> Self {
        Self {
            passages: Vec::new(),
            window_size,
            recent_openings: Vec::new(),
            word_counts: HashMap::new(),
            entity_mentions: HashMap::new(),
        }
    }

    /// Record a generated passage into the sliding window.
    pub fn record(&mut self, text: &str) {
        // Add to passages
        self.passages.push(text.to_string());
        if self.passages.len() > self.window_size {
            self.passages.remove(0);
        }

        // Track opening words
        let opening = extract_opening(text);
        if !opening.is_empty() {
            self.recent_openings.push(opening);
            if self.recent_openings.len() > self.window_size {
                self.recent_openings.remove(0);
            }
        }

        // Rebuild word counts from current window
        self.rebuild_word_counts();
    }

    /// Check a candidate passage for repetition issues.
    pub fn check_repetition(&self, candidate: &str) -> Vec<RepetitionIssue> {
        let mut issues = Vec::new();

        // Check repeated openings
        let opening = extract_opening(candidate);
        if !opening.is_empty() && self.recent_openings.contains(&opening) {
            issues.push(RepetitionIssue::RepeatedOpening(opening));
        }

        // Check overused words (combining existing counts with candidate)
        let candidate_words = extract_significant_words(candidate);
        for word in &candidate_words {
            let existing = self.word_counts.get(word.as_str()).copied().unwrap_or(0);
            let total = existing + 1;
            if total >= 4 {
                issues.push(RepetitionIssue::OverusedWord {
                    word: word.clone(),
                    count: total,
                });
            }
        }

        // Check structural monotony
        if self.passages.len() >= 3 {
            let mut lengths: Vec<f64> = self
                .passages
                .iter()
                .flat_map(|p| sentence_lengths(p))
                .collect();
            lengths.extend(sentence_lengths(candidate));

            if lengths.len() >= 4 {
                let mean: f64 = lengths.iter().sum::<f64>() / lengths.len() as f64;
                let variance: f64 =
                    lengths.iter().map(|l| (l - mean).powi(2)).sum::<f64>() / lengths.len() as f64;
                let stddev = variance.sqrt();

                // If standard deviation is very low, sentences are monotonously uniform
                if stddev < 2.0 && mean > 3.0 {
                    issues.push(RepetitionIssue::StructuralMonotony);
                }
            }
        }

        issues
    }

    fn rebuild_word_counts(&mut self) {
        self.word_counts.clear();
        for passage in &self.passages {
            for word in extract_significant_words(passage) {
                *self.word_counts.entry(word).or_default() += 1;
            }
        }
    }
}

/// Extract the first 3 words of text, lowercased, as the "opening".
fn extract_opening(text: &str) -> String {
    text.split_whitespace()
        .take(3)
        .map(|w| w.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ")
}

/// Extract "significant" words: length > 4, not a stopword.
fn extract_significant_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| w.len() > 4 && !STOPWORDS.contains(&w.as_str()))
        .collect()
}

/// Get sentence lengths (word count per sentence) from text.
fn sentence_lengths(text: &str) -> Vec<f64> {
    text.split(['.', '!', '?'])
        .map(|s| s.split_whitespace().count() as f64)
        .filter(|&len| len > 0.0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_default() {
        let ctx = NarrativeContext::default();
        assert!(ctx.passages.is_empty());
    }

    #[test]
    fn record_and_window() {
        let mut ctx = NarrativeContext::new(3);
        ctx.record("First passage.");
        ctx.record("Second passage.");
        ctx.record("Third passage.");
        ctx.record("Fourth passage.");
        // Window should only keep last 3
        assert_eq!(ctx.passages.len(), 3);
        assert_eq!(ctx.passages[0], "Second passage.");
    }

    #[test]
    fn repeated_opening_detected() {
        let mut ctx = NarrativeContext::default();
        ctx.record("The evening was quiet and still.");
        let issues = ctx.check_repetition("The evening was loud and chaotic.");
        assert!(issues
            .iter()
            .any(|i| matches!(i, RepetitionIssue::RepeatedOpening(_))));
    }

    #[test]
    fn no_repeated_opening_for_different_starts() {
        let mut ctx = NarrativeContext::default();
        ctx.record("The evening was quiet.");
        let issues = ctx.check_repetition("A silence settled over the room.");
        assert!(!issues
            .iter()
            .any(|i| matches!(i, RepetitionIssue::RepeatedOpening(_))));
    }

    #[test]
    fn overused_word_detected() {
        let mut ctx = NarrativeContext::default();
        ctx.record("The silence was deafening in the silence.");
        ctx.record("A terrible silence filled the room.");
        ctx.record("There was nothing but silence.");
        let issues = ctx.check_repetition("The silence continued.");
        assert!(issues.iter().any(|i| matches!(
            i,
            RepetitionIssue::OverusedWord { word, .. } if word == "silence"
        )));
    }

    #[test]
    fn structural_monotony_detected() {
        let mut ctx = NarrativeContext::default();
        // All sentences of very similar length (5 words)
        ctx.record("She looked at the door.");
        ctx.record("He turned to the wall.");
        ctx.record("They walked to the car.");
        let issues = ctx.check_repetition("She moved to the room.");
        assert!(issues
            .iter()
            .any(|i| matches!(i, RepetitionIssue::StructuralMonotony)));
    }

    #[test]
    fn no_monotony_with_varied_lengths() {
        let mut ctx = NarrativeContext::default();
        ctx.record("She looked at the door with a growing sense of unease.");
        ctx.record("He turned.");
        ctx.record("They walked to the car and drove away into the night, headlights cutting through the fog.");
        let issues = ctx.check_repetition("Nothing happened.");
        assert!(!issues
            .iter()
            .any(|i| matches!(i, RepetitionIssue::StructuralMonotony)));
    }

    #[test]
    fn extract_opening_works() {
        assert_eq!(extract_opening("The evening was quiet."), "the evening was");
        assert_eq!(extract_opening("Hello."), "hello.");
        assert_eq!(extract_opening(""), "");
    }

    #[test]
    fn significant_words_filter() {
        let words = extract_significant_words("The quick brown silence filled the empty room.");
        assert!(words.contains(&"quick".to_string()));
        assert!(words.contains(&"brown".to_string()));
        assert!(words.contains(&"silence".to_string()));
        assert!(words.contains(&"filled".to_string()));
        assert!(words.contains(&"empty".to_string()));
        assert!(!words.contains(&"the".to_string()));
        assert!(!words.contains(&"room".to_string())); // only 4 chars
    }
}
