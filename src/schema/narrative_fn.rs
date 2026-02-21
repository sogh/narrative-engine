use serde::{Deserialize, Serialize};

/// The core narrative function taxonomy.
///
/// Narrative function is the most important abstraction in the engine.
/// It separates WHAT narratively is happening from HOW it's expressed
/// in a specific genre.
///
/// Each variant returns normalized pacing, valence, and intensity values
/// that the grammar system uses to shape output.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NarrativeFunction {
    /// Hidden information becomes known.
    Revelation,
    /// Stakes or tension increase.
    Escalation,
    /// Two entities in direct opposition.
    Confrontation,
    /// Trust is violated.
    Betrayal,
    /// Entities align interests.
    Alliance,
    /// Something new is found or understood.
    Discovery,
    /// Something valued is taken or destroyed.
    Loss,
    /// Tension is broken with humor.
    ComicRelief,
    /// Future events are hinted at.
    Foreshadowing,
    /// An entity's position shifts.
    StatusChange,
    /// Game-defined narrative function.
    Custom(String),
}

impl NarrativeFunction {
    /// Returns a normalized pacing value (0.0 = slow/deliberate, 1.0 = fast/urgent).
    pub fn pacing(&self) -> f32 {
        match self {
            Self::Revelation => 0.4,
            Self::Escalation => 0.8,
            Self::Confrontation => 0.7,
            Self::Betrayal => 0.6,
            Self::Alliance => 0.3,
            Self::Discovery => 0.5,
            Self::Loss => 0.4,
            Self::ComicRelief => 0.6,
            Self::Foreshadowing => 0.2,
            Self::StatusChange => 0.5,
            Self::Custom(_) => 0.5,
        }
    }

    /// Returns a normalized valence value (-1.0 = strongly negative, 1.0 = strongly positive).
    pub fn valence(&self) -> f32 {
        match self {
            Self::Revelation => 0.0,
            Self::Escalation => -0.3,
            Self::Confrontation => -0.5,
            Self::Betrayal => -0.8,
            Self::Alliance => 0.6,
            Self::Discovery => 0.5,
            Self::Loss => -0.7,
            Self::ComicRelief => 0.7,
            Self::Foreshadowing => -0.2,
            Self::StatusChange => 0.0,
            Self::Custom(_) => 0.0,
        }
    }

    /// Returns a normalized intensity value (0.0 = subtle/muted, 1.0 = extreme/dramatic).
    pub fn intensity(&self) -> f32 {
        match self {
            Self::Revelation => 0.7,
            Self::Escalation => 0.8,
            Self::Confrontation => 0.9,
            Self::Betrayal => 0.9,
            Self::Alliance => 0.4,
            Self::Discovery => 0.6,
            Self::Loss => 0.8,
            Self::ComicRelief => 0.3,
            Self::Foreshadowing => 0.3,
            Self::StatusChange => 0.5,
            Self::Custom(_) => 0.5,
        }
    }

    /// Returns the snake_case name of this narrative function for grammar rule lookups.
    pub fn name(&self) -> &str {
        match self {
            Self::Revelation => "revelation",
            Self::Escalation => "escalation",
            Self::Confrontation => "confrontation",
            Self::Betrayal => "betrayal",
            Self::Alliance => "alliance",
            Self::Discovery => "discovery",
            Self::Loss => "loss",
            Self::ComicRelief => "comic_relief",
            Self::Foreshadowing => "foreshadowing",
            Self::StatusChange => "status_change",
            Self::Custom(name) => name.as_str(),
        }
    }
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

    #[test]
    fn pacing_values_in_range() {
        let variants = [
            NarrativeFunction::Revelation,
            NarrativeFunction::Escalation,
            NarrativeFunction::Confrontation,
            NarrativeFunction::Betrayal,
            NarrativeFunction::Alliance,
            NarrativeFunction::Discovery,
            NarrativeFunction::Loss,
            NarrativeFunction::ComicRelief,
            NarrativeFunction::Foreshadowing,
            NarrativeFunction::StatusChange,
            NarrativeFunction::Custom("test".to_string()),
        ];
        for v in &variants {
            let p = v.pacing();
            assert!(
                (0.0..=1.0).contains(&p),
                "{:?} pacing {} out of range",
                v,
                p
            );
        }
    }

    #[test]
    fn valence_values_in_range() {
        let variants = [
            NarrativeFunction::Revelation,
            NarrativeFunction::Escalation,
            NarrativeFunction::Confrontation,
            NarrativeFunction::Betrayal,
            NarrativeFunction::Alliance,
            NarrativeFunction::Discovery,
            NarrativeFunction::Loss,
            NarrativeFunction::ComicRelief,
            NarrativeFunction::Foreshadowing,
            NarrativeFunction::StatusChange,
        ];
        for v in &variants {
            let val = v.valence();
            assert!(
                (-1.0..=1.0).contains(&val),
                "{:?} valence {} out of range",
                v,
                val
            );
        }
    }

    #[test]
    fn intensity_values_in_range() {
        let variants = [
            NarrativeFunction::Revelation,
            NarrativeFunction::Escalation,
            NarrativeFunction::Confrontation,
            NarrativeFunction::Betrayal,
            NarrativeFunction::Alliance,
            NarrativeFunction::Discovery,
            NarrativeFunction::Loss,
            NarrativeFunction::ComicRelief,
            NarrativeFunction::Foreshadowing,
            NarrativeFunction::StatusChange,
        ];
        for v in &variants {
            let i = v.intensity();
            assert!(
                (0.0..=1.0).contains(&i),
                "{:?} intensity {} out of range",
                v,
                i
            );
        }
    }

    #[test]
    fn confrontation_is_high_intensity() {
        let c = NarrativeFunction::Confrontation;
        assert!(c.intensity() >= 0.8);
        assert!(c.valence() < 0.0);
    }

    #[test]
    fn alliance_is_positive_valence() {
        let a = NarrativeFunction::Alliance;
        assert!(a.valence() > 0.0);
        assert!(a.intensity() < 0.5);
    }

    #[test]
    fn foreshadowing_is_slow_paced() {
        let f = NarrativeFunction::Foreshadowing;
        assert!(f.pacing() <= 0.3);
    }

    #[test]
    fn name_returns_snake_case() {
        assert_eq!(NarrativeFunction::ComicRelief.name(), "comic_relief");
        assert_eq!(NarrativeFunction::StatusChange.name(), "status_change");
        assert_eq!(NarrativeFunction::Revelation.name(), "revelation");
        assert_eq!(
            NarrativeFunction::Custom("my_fn".to_string()).name(),
            "my_fn"
        );
    }
}
