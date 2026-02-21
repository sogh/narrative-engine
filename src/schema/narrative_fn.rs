use serde::{Deserialize, Serialize};

/// The core narrative function taxonomy.
///
/// Narrative function is the most important abstraction in the engine.
/// It separates WHAT narratively is happening from HOW it's expressed
/// in a specific genre.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NarrativeFunction {
    Revelation,
    Escalation,
    Confrontation,
    Betrayal,
    Alliance,
    Discovery,
    Loss,
    ComicRelief,
    Foreshadowing,
    StatusChange,
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrative_fn_variants() {
        let f = NarrativeFunction::Revelation;
        assert!(matches!(f, NarrativeFunction::Revelation));

        let custom = NarrativeFunction::Custom("trade".to_string());
        assert!(matches!(custom, NarrativeFunction::Custom(_)));
    }
}
