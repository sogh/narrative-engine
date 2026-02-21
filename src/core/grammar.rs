/// Stochastic grammar runtime — types, parsing, loading, and expansion.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GrammarError {
    #[error("template parse error: {0}")]
    TemplateParse(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("RON deserialization error: {0}")]
    Ron(#[from] ron::error::SpannedError),
}

/// A segment of a parsed template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TemplateSegment {
    /// Literal text, emitted as-is.
    Literal(String),
    /// Reference to another grammar rule: `{rule_name}`.
    RuleRef(String),
    /// Reference to a Markov generator: `{markov:corpus:tag}`.
    MarkovRef { corpus: String, tag: String },
    /// Entity field interpolation: `{entity.field}`.
    EntityField { field: String },
    /// Pronoun-aware entity reference: `{subject}`, `{object}`, `{possessive}`.
    PronounRef { role: String },
}

/// A parsed template — a sequence of segments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Template {
    pub segments: Vec<TemplateSegment>,
}

impl Template {
    /// Parse a template string into a sequence of segments.
    ///
    /// Syntax:
    /// - `{rule_name}` → `RuleRef`
    /// - `{markov:corpus:tag}` → `MarkovRef`
    /// - `{entity.field}` → `EntityField`
    /// - `{subject}` / `{object}` / `{possessive}` → `PronounRef`
    /// - `{{` → literal `{`
    /// - Everything else → `Literal`
    pub fn parse(input: &str) -> Result<Template, GrammarError> {
        let mut segments = Vec::new();
        let mut literal_buf = String::new();
        let chars: Vec<char> = input.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            if chars[i] == '{' {
                // Escaped brace
                if i + 1 < len && chars[i + 1] == '{' {
                    literal_buf.push('{');
                    i += 2;
                    continue;
                }

                // Flush any accumulated literal
                if !literal_buf.is_empty() {
                    segments.push(TemplateSegment::Literal(literal_buf.clone()));
                    literal_buf.clear();
                }

                // Find the closing brace
                let start = i + 1;
                let mut depth = 1;
                let mut end = start;
                while end < len {
                    if chars[end] == '{' {
                        return Err(GrammarError::TemplateParse(
                            "nested braces are not allowed".to_string(),
                        ));
                    }
                    if chars[end] == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    end += 1;
                }

                if depth != 0 {
                    return Err(GrammarError::TemplateParse(
                        "unclosed brace".to_string(),
                    ));
                }

                let content: String = chars[start..end].iter().collect();
                if content.is_empty() {
                    return Err(GrammarError::TemplateParse(
                        "empty braces".to_string(),
                    ));
                }

                segments.push(Self::parse_segment(&content)?);
                i = end + 1;
            } else if chars[i] == '}' {
                // Escaped closing brace
                if i + 1 < len && chars[i + 1] == '}' {
                    literal_buf.push('}');
                    i += 2;
                    continue;
                }
                return Err(GrammarError::TemplateParse(
                    "unmatched closing brace".to_string(),
                ));
            } else {
                literal_buf.push(chars[i]);
                i += 1;
            }
        }

        if !literal_buf.is_empty() {
            segments.push(TemplateSegment::Literal(literal_buf));
        }

        Ok(Template { segments })
    }

    fn parse_segment(content: &str) -> Result<TemplateSegment, GrammarError> {
        // Check for pronoun refs
        match content {
            "subject" | "object" | "possessive" => {
                return Ok(TemplateSegment::PronounRef {
                    role: content.to_string(),
                });
            }
            _ => {}
        }

        // Check for markov ref: markov:corpus:tag
        if let Some(rest) = content.strip_prefix("markov:") {
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() == 2 {
                return Ok(TemplateSegment::MarkovRef {
                    corpus: parts[0].to_string(),
                    tag: parts[1].to_string(),
                });
            }
            return Err(GrammarError::TemplateParse(format!(
                "invalid markov ref '{}': expected markov:corpus:tag",
                content
            )));
        }

        // Check for entity field: entity.field
        if let Some(field) = content.strip_prefix("entity.") {
            if field.is_empty() {
                return Err(GrammarError::TemplateParse(
                    "empty entity field name".to_string(),
                ));
            }
            return Ok(TemplateSegment::EntityField {
                field: field.to_string(),
            });
        }

        // Default: rule reference
        Ok(TemplateSegment::RuleRef(content.to_string()))
    }
}

/// A weighted text alternative within a grammar rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub weight: u32,
    pub template: Template,
}

/// A single grammar rule with tag preconditions and weighted alternatives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarRule {
    pub name: String,
    pub requires: Vec<String>,
    pub excludes: Vec<String>,
    pub alternatives: Vec<Alternative>,
}

/// A set of named grammar rules.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GrammarSet {
    pub rules: HashMap<String, GrammarRule>,
}

// RON deserialization helpers — the RON format uses a different shape
// than the internal types, so we need intermediate structs.

#[derive(Debug, Deserialize)]
struct RonAlternative {
    weight: u32,
    text: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename = "Rule")]
struct RonRule {
    requires: Vec<String>,
    #[serde(default)]
    excludes: Vec<String>,
    alternatives: Vec<RonAlternative>,
}

impl GrammarSet {
    /// Load a grammar set from a RON file.
    pub fn load_from_ron(path: &Path) -> Result<GrammarSet, GrammarError> {
        let contents = std::fs::read_to_string(path)?;
        Self::parse_ron(&contents)
    }

    /// Parse a grammar set from a RON string.
    pub fn parse_ron(input: &str) -> Result<GrammarSet, GrammarError> {
        let raw: HashMap<String, RonRule> = ron::from_str(input)?;
        let mut rules = HashMap::new();

        for (name, ron_rule) in raw {
            let mut alternatives = Vec::new();
            for alt in ron_rule.alternatives {
                let template = Template::parse(&alt.text)?;
                alternatives.push(Alternative {
                    weight: alt.weight,
                    template,
                });
            }
            rules.insert(
                name.clone(),
                GrammarRule {
                    name,
                    requires: ron_rule.requires,
                    excludes: ron_rule.excludes,
                    alternatives,
                },
            );
        }

        Ok(GrammarSet { rules })
    }

    /// Merge another grammar set into this one. Rules from `other`
    /// override rules in `self` with the same name.
    pub fn merge(&mut self, other: GrammarSet) {
        for (name, rule) in other.rules {
            self.rules.insert(name, rule);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_literal_only() {
        let t = Template::parse("Hello, world.").unwrap();
        assert_eq!(
            t.segments,
            vec![TemplateSegment::Literal("Hello, world.".to_string())]
        );
    }

    #[test]
    fn parse_rule_ref() {
        let t = Template::parse("Start {action_detail} end").unwrap();
        assert_eq!(t.segments.len(), 3);
        assert_eq!(
            t.segments[1],
            TemplateSegment::RuleRef("action_detail".to_string())
        );
    }

    #[test]
    fn parse_markov_ref() {
        let t = Template::parse("She said {markov:dialogue:accusatory} quietly.").unwrap();
        assert_eq!(t.segments.len(), 3);
        assert_eq!(
            t.segments[1],
            TemplateSegment::MarkovRef {
                corpus: "dialogue".to_string(),
                tag: "accusatory".to_string(),
            }
        );
    }

    #[test]
    fn parse_entity_field() {
        let t = Template::parse("Hello, {entity.name}.").unwrap();
        assert_eq!(t.segments.len(), 3);
        assert_eq!(
            t.segments[1],
            TemplateSegment::EntityField {
                field: "name".to_string()
            }
        );
    }

    #[test]
    fn parse_pronoun_refs() {
        let t = Template::parse("{subject} looked at {object} with {possessive} eyes.").unwrap();
        assert_eq!(
            t.segments[0],
            TemplateSegment::PronounRef {
                role: "subject".to_string()
            }
        );
        assert_eq!(
            t.segments[2],
            TemplateSegment::PronounRef {
                role: "object".to_string()
            }
        );
        assert_eq!(
            t.segments[4],
            TemplateSegment::PronounRef {
                role: "possessive".to_string()
            }
        );
    }

    #[test]
    fn parse_escaped_braces() {
        let t = Template::parse("Use {{braces}} here.").unwrap();
        assert_eq!(
            t.segments,
            vec![TemplateSegment::Literal("Use {braces} here.".to_string())]
        );
    }

    #[test]
    fn parse_empty_braces_error() {
        assert!(Template::parse("Bad {} here").is_err());
    }

    #[test]
    fn parse_nested_braces_error() {
        assert!(Template::parse("Bad {outer{inner}} here").is_err());
    }

    #[test]
    fn parse_unclosed_brace_error() {
        assert!(Template::parse("Bad {unclosed here").is_err());
    }

    #[test]
    fn parse_unmatched_close_error() {
        assert!(Template::parse("Bad } here").is_err());
    }

    #[test]
    fn parse_mixed_segments() {
        let t =
            Template::parse("{subject} set down {possessive} {entity.held_item} and said {markov:dialogue:tense}.")
                .unwrap();
        assert_eq!(t.segments.len(), 8);
        assert!(matches!(&t.segments[0], TemplateSegment::PronounRef { role } if role == "subject"));
        assert!(matches!(&t.segments[2], TemplateSegment::PronounRef { role } if role == "possessive"));
        assert!(matches!(&t.segments[4], TemplateSegment::EntityField { field } if field == "held_item"));
        assert!(matches!(&t.segments[6], TemplateSegment::MarkovRef { corpus, tag } if corpus == "dialogue" && tag == "tense"));
    }

    #[test]
    fn load_test_grammar_from_ron() {
        let path = std::path::PathBuf::from("tests/fixtures/test_grammar.ron");
        let gs = GrammarSet::load_from_ron(&path).unwrap();
        assert_eq!(gs.rules.len(), 3);
        assert!(gs.rules.contains_key("greeting"));
        assert!(gs.rules.contains_key("tense_observation"));
        assert!(gs.rules.contains_key("action_detail"));

        let greeting = &gs.rules["greeting"];
        assert_eq!(greeting.alternatives.len(), 3);
        assert!(greeting.requires.is_empty());
    }

    #[test]
    fn ron_round_trip() {
        let mut gs = GrammarSet::default();
        gs.rules.insert(
            "test_rule".to_string(),
            GrammarRule {
                name: "test_rule".to_string(),
                requires: vec!["mood:tense".to_string()],
                excludes: vec![],
                alternatives: vec![Alternative {
                    weight: 1,
                    template: Template::parse("Hello {entity.name}.").unwrap(),
                }],
            },
        );

        let serialized = ron::to_string(&gs).unwrap();
        let deserialized: GrammarSet = ron::from_str(&serialized).unwrap();
        assert_eq!(deserialized.rules.len(), 1);
        assert!(deserialized.rules.contains_key("test_rule"));
    }

    #[test]
    fn merge_precedence() {
        let mut base = GrammarSet::default();
        base.rules.insert(
            "shared".to_string(),
            GrammarRule {
                name: "shared".to_string(),
                requires: vec![],
                excludes: vec![],
                alternatives: vec![Alternative {
                    weight: 1,
                    template: Template::parse("base version").unwrap(),
                }],
            },
        );
        base.rules.insert(
            "base_only".to_string(),
            GrammarRule {
                name: "base_only".to_string(),
                requires: vec![],
                excludes: vec![],
                alternatives: vec![Alternative {
                    weight: 1,
                    template: Template::parse("only in base").unwrap(),
                }],
            },
        );

        let mut override_set = GrammarSet::default();
        override_set.rules.insert(
            "shared".to_string(),
            GrammarRule {
                name: "shared".to_string(),
                requires: vec!["mood:tense".to_string()],
                excludes: vec![],
                alternatives: vec![Alternative {
                    weight: 2,
                    template: Template::parse("override version").unwrap(),
                }],
            },
        );

        base.merge(override_set);

        // Override took precedence
        assert_eq!(base.rules["shared"].alternatives[0].weight, 2);
        assert_eq!(
            base.rules["shared"].requires,
            vec!["mood:tense".to_string()]
        );
        // Base-only rule still present
        assert!(base.rules.contains_key("base_only"));
    }

    #[test]
    fn grammar_set_default() {
        let gs = GrammarSet::default();
        assert!(gs.rules.is_empty());
    }

    #[test]
    fn template_requires_tags_loaded() {
        let path = std::path::PathBuf::from("tests/fixtures/test_grammar.ron");
        let gs = GrammarSet::load_from_ron(&path).unwrap();
        let tense = &gs.rules["tense_observation"];
        assert_eq!(tense.requires, vec!["mood:tense".to_string()]);
    }
}
