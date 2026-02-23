/// Stochastic grammar runtime — types, parsing, loading, and expansion.
use rand::distributions::WeightedIndex;
use rand::prelude::Distribution;
use rand::rngs::StdRng;
use rustc_hash::FxHashSet;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

use crate::core::markov::MarkovModel;
use crate::schema::entity::{Entity, Value};

const MAX_EXPANSION_DEPTH: u32 = 20;

#[derive(Debug, Error)]
pub enum GrammarError {
    #[error("template parse error: {0}")]
    TemplateParse(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("RON deserialization error: {0}")]
    Ron(#[from] ron::error::SpannedError),
    #[error("rule not found: {0}")]
    RuleNotFound(String),
    #[error("max expansion depth ({0}) exceeded")]
    MaxDepthExceeded(u32),
    #[error("no matching alternatives for rule '{0}'")]
    NoAlternatives(String),
    #[error("entity binding not found for role: {0}")]
    EntityBindingNotFound(String),
    #[error("entity field not found: {0}")]
    EntityFieldNotFound(String),
    #[error("markov generation error: {0}")]
    MarkovError(String),
}

/// Accumulated state during grammar expansion.
pub struct SelectionContext<'a> {
    pub tags: FxHashSet<String>,
    pub entity_bindings: HashMap<String, &'a Entity>,
    pub depth: u32,
    /// Optional voice grammar weight overrides (rule_name → multiplier).
    pub voice_weights: Option<&'a HashMap<String, f32>>,
    /// Loaded Markov models keyed by corpus_id.
    pub markov_models: HashMap<String, &'a MarkovModel>,
}

impl<'a> Default for SelectionContext<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> SelectionContext<'a> {
    pub fn new() -> Self {
        Self {
            tags: FxHashSet::default(),
            entity_bindings: HashMap::new(),
            depth: 0,
            voice_weights: None,
            markov_models: HashMap::new(),
        }
    }

    pub fn with_tags(mut self, tags: impl IntoIterator<Item = String>) -> Self {
        self.tags.extend(tags);
        self
    }

    pub fn with_entity(mut self, role: &str, entity: &'a Entity) -> Self {
        self.entity_bindings.insert(role.to_string(), entity);
        self
    }

    pub fn with_markov(mut self, corpus_id: &str, model: &'a MarkovModel) -> Self {
        self.markov_models.insert(corpus_id.to_string(), model);
        self
    }
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
    /// Pronoun-aware entity reference: `{subject}`, `{object}`, `{possessive}`,
    /// `{possessive_standalone}`, `{reflexive}`.
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
    /// - `{subject}` / `{object}` / `{possessive}` / `{possessive_standalone}` / `{reflexive}` → `PronounRef`
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
                    return Err(GrammarError::TemplateParse("unclosed brace".to_string()));
                }

                let content: String = chars[start..end].iter().collect();
                if content.is_empty() {
                    return Err(GrammarError::TemplateParse("empty braces".to_string()));
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
            "subject" | "object" | "possessive" | "possessive_standalone" | "reflexive" => {
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

    /// Find all rules whose `requires` tags are a subset of the context's
    /// active tags, and whose `excludes` tags have no intersection.
    pub fn find_matching_rules<'a, 'b>(
        &'a self,
        ctx: &SelectionContext<'b>,
    ) -> Vec<&'a GrammarRule> {
        self.rules
            .values()
            .filter(|rule| {
                // All requires must be present in ctx tags
                let requires_met = rule.requires.iter().all(|tag| ctx.tags.contains(tag));
                // No excludes may be present in ctx tags
                let excludes_clear = !rule.excludes.iter().any(|tag| ctx.tags.contains(tag));
                requires_met && excludes_clear
            })
            .collect()
    }

    /// Expand a named rule into text using the given context and RNG.
    pub fn expand(
        &self,
        rule_name: &str,
        ctx: &mut SelectionContext<'_>,
        rng: &mut StdRng,
    ) -> Result<String, GrammarError> {
        if ctx.depth >= MAX_EXPANSION_DEPTH {
            return Err(GrammarError::MaxDepthExceeded(MAX_EXPANSION_DEPTH));
        }

        let rule = self
            .rules
            .get(rule_name)
            .ok_or_else(|| GrammarError::RuleNotFound(rule_name.to_string()))?;

        if rule.alternatives.is_empty() {
            return Err(GrammarError::NoAlternatives(rule_name.to_string()));
        }

        // Propagate this rule's requires tags into context for child expansions
        for tag in &rule.requires {
            ctx.tags.insert(tag.clone());
        }

        // Select alternative by weighted random, with voice weight multipliers
        let alt = select_alternative(&rule.alternatives, rule_name, ctx.voice_weights, rng)?;

        // Expand template segments
        ctx.depth += 1;
        let mut output = String::new();

        for segment in &alt.template.segments {
            match segment {
                TemplateSegment::Literal(text) => {
                    output.push_str(text);
                }
                TemplateSegment::RuleRef(name) => {
                    let expanded = self.expand(name, ctx, rng)?;
                    output.push_str(&expanded);
                }
                TemplateSegment::MarkovRef { corpus, tag } => {
                    if let Some(model) = ctx.markov_models.get(corpus.as_str()) {
                        match model.generate(rng, Some(tag), 5, 15) {
                            Ok(text) => output.push_str(&text),
                            Err(e) => {
                                // Fall back to untagged generation
                                match model.generate(rng, None, 5, 15) {
                                    Ok(text) => output.push_str(&text),
                                    Err(_) => {
                                        return Err(GrammarError::MarkovError(format!(
                                            "markov generation failed for {}:{}: {}",
                                            corpus, tag, e
                                        )));
                                    }
                                }
                            }
                        }
                    } else {
                        // No model loaded — emit placeholder
                        output.push_str(&format!("[markov:{}:{}]", corpus, tag));
                    }
                }
                TemplateSegment::EntityField { field } => {
                    output.push_str(&resolve_entity_field(ctx, field)?);
                }
                TemplateSegment::PronounRef { role } => {
                    output.push_str(&resolve_pronoun(ctx, role)?);
                }
            }
        }

        ctx.depth -= 1;
        Ok(output)
    }
}

/// Select a weighted alternative, optionally applying voice weight multipliers.
fn select_alternative<'a>(
    alts: &'a [Alternative],
    rule_name: &str,
    voice_weights: Option<&HashMap<String, f32>>,
    rng: &mut StdRng,
) -> Result<&'a Alternative, GrammarError> {
    let weights: Vec<f64> = alts
        .iter()
        .map(|alt| {
            let base = alt.weight as f64;
            let multiplier = voice_weights
                .and_then(|vw| vw.get(rule_name))
                .copied()
                .unwrap_or(1.0) as f64;
            (base * multiplier).max(0.0)
        })
        .collect();

    let dist = WeightedIndex::new(&weights)
        .map_err(|_| GrammarError::NoAlternatives(rule_name.to_string()))?;
    Ok(&alts[dist.sample(rng)])
}

/// Look up an entity field from context bindings.
fn resolve_entity_field(ctx: &SelectionContext<'_>, field: &str) -> Result<String, GrammarError> {
    // Try to find the field in any bound entity's properties, or check name
    // First check the "subject" binding, then any binding
    let entity = ctx
        .entity_bindings
        .get("subject")
        .or_else(|| ctx.entity_bindings.values().next())
        .ok_or_else(|| GrammarError::EntityBindingNotFound("subject".to_string()))?;

    if field == "name" {
        return Ok(entity.name.clone());
    }

    match entity.properties.get(field) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(Value::Float(f)) => Ok(format!("{}", f)),
        Some(Value::Int(i)) => Ok(format!("{}", i)),
        Some(Value::Bool(b)) => Ok(format!("{}", b)),
        None => Err(GrammarError::EntityFieldNotFound(field.to_string())),
    }
}

/// Resolve a pronoun reference using the entity's pronoun set.
///
/// - `{subject}` → entity name (templates expect the name here)
/// - `{object}` → entity name for the "object" role
/// - `{possessive}` → possessive determiner (her, his, their, its)
/// - `{possessive_standalone}` → independent possessive (hers, his, theirs, its)
/// - `{reflexive}` → reflexive pronoun (herself, himself, themselves, itself)
fn resolve_pronoun(ctx: &SelectionContext<'_>, role: &str) -> Result<String, GrammarError> {
    // Map pronoun role to entity binding
    let binding_key = match role {
        "subject" => "subject",
        "object" => "object",
        "possessive" | "possessive_standalone" | "reflexive" => "subject",
        other => other,
    };

    let entity = ctx
        .entity_bindings
        .get(binding_key)
        // Fall back to subject for object/possessive if not separately bound
        .or_else(|| ctx.entity_bindings.get("subject"))
        .ok_or_else(|| GrammarError::EntityBindingNotFound(role.to_string()))?;

    match role {
        "possessive" => Ok(entity.pronouns.possessive().to_string()),
        "possessive_standalone" => Ok(entity.pronouns.possessive_standalone().to_string()),
        "reflexive" => Ok(entity.pronouns.reflexive().to_string()),
        _ => Ok(entity.name.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::entity::{Entity, EntityId, VoiceId};
    use rand::SeedableRng;

    fn make_test_entity(name: &str) -> Entity {
        Entity {
            id: EntityId(1),
            name: name.to_string(),
            pronouns: crate::schema::entity::Pronouns::SheHer,
            tags: FxHashSet::default(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(1)),
            properties: HashMap::from([(
                "held_item".to_string(),
                Value::String("wine glass".to_string()),
            )]),
        }
    }

    fn load_test_grammar() -> GrammarSet {
        GrammarSet::load_from_ron(std::path::Path::new("tests/fixtures/test_grammar.ron")).unwrap()
    }

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
        let t = Template::parse(
            "{subject} set down {possessive} {entity.held_item} and said {markov:dialogue:tense}.",
        )
        .unwrap();
        assert_eq!(t.segments.len(), 8);
        assert!(
            matches!(&t.segments[0], TemplateSegment::PronounRef { role } if role == "subject")
        );
        assert!(
            matches!(&t.segments[2], TemplateSegment::PronounRef { role } if role == "possessive")
        );
        assert!(
            matches!(&t.segments[4], TemplateSegment::EntityField { field } if field == "held_item")
        );
        assert!(
            matches!(&t.segments[6], TemplateSegment::MarkovRef { corpus, tag } if corpus == "dialogue" && tag == "tense")
        );
    }

    #[test]
    fn load_test_grammar_from_ron() {
        let gs = load_test_grammar();
        assert_eq!(gs.rules.len(), 9);
        assert!(gs.rules.contains_key("greeting"));
        assert!(gs.rules.contains_key("tense_observation"));
        assert!(gs.rules.contains_key("action_detail"));
        assert!(gs.rules.contains_key("confrontation_opening"));
        assert!(gs.rules.contains_key("calm_greeting"));
        assert!(gs.rules.contains_key("recursive_bomb"));
        assert!(gs.rules.contains_key("markov_test"));
        assert!(gs.rules.contains_key("possessive_standalone_test"));
        assert!(gs.rules.contains_key("reflexive_test"));

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

    // --- Expansion tests ---

    #[test]
    fn expand_literal_rule() {
        let gs = load_test_grammar();
        let entity = make_test_entity("Margaret");
        let mut ctx = SelectionContext::new()
            .with_tags(["mood:tense".to_string()])
            .with_entity("subject", &entity);
        let mut rng = StdRng::seed_from_u64(42);

        let result = gs.expand("tense_observation", &mut ctx, &mut rng).unwrap();
        assert!(!result.is_empty());
        // All alternatives are pure literals
        let valid = [
            "The air felt heavy with unspoken words.",
            "A silence settled over the room.",
            "No one dared to speak first.",
        ];
        assert!(
            valid.contains(&result.as_str()),
            "Unexpected output: {}",
            result
        );
    }

    #[test]
    fn expand_three_levels_deep() {
        let gs = load_test_grammar();
        let entity = make_test_entity("Margaret");
        let mut ctx = SelectionContext::new()
            .with_tags(["mood:tense".to_string()])
            .with_entity("subject", &entity);
        let mut rng = StdRng::seed_from_u64(42);

        // confrontation_opening references action_detail and tense_observation
        let result = gs
            .expand("confrontation_opening", &mut ctx, &mut rng)
            .unwrap();
        assert!(!result.is_empty());
        // Should contain text from child rules
        assert!(
            result.len() > 20,
            "Expected multi-rule expansion, got: {}",
            result
        );
    }

    #[test]
    fn deterministic_with_same_seed() {
        let gs = load_test_grammar();
        let entity = make_test_entity("Margaret");

        let mut ctx1 = SelectionContext::new()
            .with_tags(["mood:tense".to_string()])
            .with_entity("subject", &entity);
        let mut rng1 = StdRng::seed_from_u64(99);
        let result1 = gs
            .expand("confrontation_opening", &mut ctx1, &mut rng1)
            .unwrap();

        let mut ctx2 = SelectionContext::new()
            .with_tags(["mood:tense".to_string()])
            .with_entity("subject", &entity);
        let mut rng2 = StdRng::seed_from_u64(99);
        let result2 = gs
            .expand("confrontation_opening", &mut ctx2, &mut rng2)
            .unwrap();

        assert_eq!(result1, result2);
    }

    #[test]
    fn different_seed_different_output() {
        let gs = load_test_grammar();
        let entity = make_test_entity("Margaret");

        let mut ctx1 = SelectionContext::new()
            .with_tags(["mood:tense".to_string()])
            .with_entity("subject", &entity);
        let mut rng1 = StdRng::seed_from_u64(1);
        let result1 = gs
            .expand("confrontation_opening", &mut ctx1, &mut rng1)
            .unwrap();

        let mut found_different = false;
        for seed in 2..50 {
            let mut ctx2 = SelectionContext::new()
                .with_tags(["mood:tense".to_string()])
                .with_entity("subject", &entity);
            let mut rng2 = StdRng::seed_from_u64(seed);
            let result2 = gs
                .expand("confrontation_opening", &mut ctx2, &mut rng2)
                .unwrap();
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
    fn max_depth_error() {
        let gs = load_test_grammar();
        let mut ctx = SelectionContext::new();
        let mut rng = StdRng::seed_from_u64(42);

        let result = gs.expand("recursive_bomb", &mut ctx, &mut rng);
        assert!(result.is_err());
        assert!(
            matches!(result, Err(GrammarError::MaxDepthExceeded(_))),
            "Expected MaxDepthExceeded error"
        );
    }

    #[test]
    fn tag_propagation_affects_selection() {
        let gs = load_test_grammar();

        // Without mood:tense, calm_greeting should match but tense_observation should not
        let ctx = SelectionContext::new();
        let matching = gs.find_matching_rules(&ctx);
        let names: Vec<&str> = matching.iter().map(|r| r.name.as_str()).collect();
        assert!(
            names.contains(&"calm_greeting"),
            "calm_greeting should match without tense tag"
        );
        assert!(
            !names.contains(&"tense_observation"),
            "tense_observation should not match without tense tag"
        );

        // With mood:tense, tense_observation should match but calm_greeting should not
        let ctx_tense = SelectionContext::new().with_tags(["mood:tense".to_string()]);
        let matching_tense = gs.find_matching_rules(&ctx_tense);
        let names_tense: Vec<&str> = matching_tense.iter().map(|r| r.name.as_str()).collect();
        assert!(
            names_tense.contains(&"tense_observation"),
            "tense_observation should match with tense tag"
        );
        assert!(
            !names_tense.contains(&"calm_greeting"),
            "calm_greeting should be excluded by tense tag"
        );
    }

    #[test]
    fn entity_field_expansion() {
        let gs = load_test_grammar();
        let entity = make_test_entity("Margaret");

        // greeting rule uses {entity.name}
        // Run multiple seeds to hit a variant with entity.name
        let mut found_name = false;
        for seed in 0..20 {
            let mut ctx = SelectionContext::new().with_entity("subject", &entity);
            let mut rng = StdRng::seed_from_u64(seed);
            let result = gs.expand("greeting", &mut ctx, &mut rng).unwrap();
            if result.contains("Margaret") {
                found_name = true;
                break;
            }
        }
        assert!(
            found_name,
            "Expected entity name expansion in at least one seed"
        );
    }

    #[test]
    fn markov_placeholder_expansion() {
        let gs = load_test_grammar();
        let mut ctx = SelectionContext::new();
        let mut rng = StdRng::seed_from_u64(42);

        let result = gs.expand("markov_test", &mut ctx, &mut rng).unwrap();
        assert!(
            result.contains("[markov:dialogue:accusatory]"),
            "Expected markov placeholder, got: {}",
            result
        );
    }

    #[test]
    fn rule_not_found_error() {
        let gs = load_test_grammar();
        let mut ctx = SelectionContext::new();
        let mut rng = StdRng::seed_from_u64(42);

        let result = gs.expand("nonexistent_rule", &mut ctx, &mut rng);
        assert!(matches!(result, Err(GrammarError::RuleNotFound(_))));
    }

    #[test]
    fn parse_possessive_standalone_ref() {
        let t = Template::parse("The secret was no longer {possessive_standalone} alone.").unwrap();
        assert_eq!(
            t.segments[1],
            TemplateSegment::PronounRef {
                role: "possessive_standalone".to_string()
            }
        );
    }

    #[test]
    fn parse_reflexive_ref() {
        let t = Template::parse("{subject} reminded {reflexive} to stay calm.").unwrap();
        assert_eq!(
            t.segments[2],
            TemplateSegment::PronounRef {
                role: "reflexive".to_string()
            }
        );
    }

    #[test]
    fn expand_possessive_standalone() {
        let gs = load_test_grammar();
        let entity = make_test_entity("Margaret");
        let mut ctx = SelectionContext::new().with_entity("subject", &entity);
        let mut rng = StdRng::seed_from_u64(42);

        let result = gs
            .expand("possessive_standalone_test", &mut ctx, &mut rng)
            .unwrap();
        assert_eq!(result, "The secret was no longer hers alone.");
    }

    #[test]
    fn expand_reflexive() {
        let gs = load_test_grammar();
        let entity = make_test_entity("Margaret");
        let mut ctx = SelectionContext::new().with_entity("subject", &entity);
        let mut rng = StdRng::seed_from_u64(42);

        let result = gs
            .expand("reflexive_test", &mut ctx, &mut rng)
            .unwrap();
        assert_eq!(result, "Margaret reminded herself to stay calm.");
    }

    #[test]
    fn possessive_standalone_he_him() {
        let entity = Entity {
            id: EntityId(2),
            name: "James".to_string(),
            pronouns: crate::schema::entity::Pronouns::HeHim,
            tags: FxHashSet::default(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(1)),
            properties: HashMap::new(),
        };
        let gs = load_test_grammar();
        let mut ctx = SelectionContext::new().with_entity("subject", &entity);
        let mut rng = StdRng::seed_from_u64(42);

        let result = gs
            .expand("possessive_standalone_test", &mut ctx, &mut rng)
            .unwrap();
        assert_eq!(result, "The secret was no longer his alone.");
    }

    #[test]
    fn possessive_standalone_they_them() {
        let entity = Entity {
            id: EntityId(3),
            name: "Alex".to_string(),
            pronouns: crate::schema::entity::Pronouns::TheyThem,
            tags: FxHashSet::default(),
            relationships: Vec::new(),
            voice_id: Some(VoiceId(1)),
            properties: HashMap::new(),
        };
        let gs = load_test_grammar();
        let mut ctx = SelectionContext::new().with_entity("subject", &entity);
        let mut rng = StdRng::seed_from_u64(42);

        let result = gs
            .expand("possessive_standalone_test", &mut ctx, &mut rng)
            .unwrap();
        assert_eq!(result, "The secret was no longer theirs alone.");
    }
}
