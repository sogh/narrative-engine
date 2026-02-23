/// Grammar expansion and linting integration tests.

use narrative_engine::core::grammar::GrammarSet;

#[test]
fn social_drama_grammar_loads() {
    let path = std::path::Path::new("genre_data/social_drama/grammar.ron");
    let gs = GrammarSet::load_from_ron(path).unwrap();
    assert!(!gs.rules.is_empty());

    // Check that all expected rules are present
    let expected_rules = [
        "revelation_opening",
        "revelation_body",
        "revelation_closing",
        "confrontation_opening",
        "confrontation_body",
        "confrontation_closing",
        "betrayal_opening",
        "betrayal_body",
        "betrayal_closing",
        "alliance_opening",
        "alliance_body",
        "alliance_closing",
        "comic_relief_opening",
        "comic_relief_body",
        "comic_relief_closing",
        "deliberate_action",
        "emotional_reaction",
        "body_language",
        "dialogue_tag",
        "dialogue_line",
        "social_observation",
        "room_detail",
    ];

    for rule_name in &expected_rules {
        assert!(
            gs.rules.contains_key(*rule_name),
            "Missing rule: {}",
            rule_name
        );
    }
}

#[test]
fn survival_thriller_grammar_loads() {
    let path = std::path::Path::new("genre_data/survival_thriller/grammar.ron");
    let gs = GrammarSet::load_from_ron(path).unwrap();
    assert!(!gs.rules.is_empty());

    let expected_rules = [
        "escalation_opening",
        "escalation_body",
        "escalation_closing",
        "discovery_opening",
        "discovery_body",
        "discovery_closing",
        "loss_opening",
        "loss_body",
        "loss_closing",
        "foreshadowing_opening",
        "foreshadowing_body",
        "foreshadowing_closing",
        "status_change_opening",
        "status_change_body",
        "status_change_closing",
        "environmental_detail",
        "threat_proximity",
        "resource_status",
        "sensory_detail",
        "technical_readout",
        "urgency_marker",
        "radio_chatter",
    ];

    for rule_name in &expected_rules {
        assert!(
            gs.rules.contains_key(*rule_name),
            "Missing rule: {}",
            rule_name
        );
    }
}

#[test]
fn all_genre_templates_have_minimum_alternatives() {
    let paths = [
        "genre_data/social_drama/grammar.ron",
        "genre_data/survival_thriller/grammar.ron",
    ];

    for path_str in &paths {
        let path = std::path::Path::new(path_str);
        let gs = GrammarSet::load_from_ron(path).unwrap();

        for (name, rule) in &gs.rules {
            assert!(
                rule.alternatives.len() >= 3,
                "Rule '{}' in {} has only {} alternatives (minimum 3 expected)",
                name,
                path_str,
                rule.alternatives.len()
            );
        }
    }
}

#[test]
fn no_broken_rule_references_in_templates() {
    let paths = [
        "genre_data/social_drama/grammar.ron",
        "genre_data/survival_thriller/grammar.ron",
    ];

    for path_str in &paths {
        let path = std::path::Path::new(path_str);
        let gs = GrammarSet::load_from_ron(path).unwrap();

        for (name, rule) in &gs.rules {
            for alt in &rule.alternatives {
                for segment in &alt.template.segments {
                    if let narrative_engine::core::grammar::TemplateSegment::RuleRef(ref_name) =
                        segment
                    {
                        assert!(
                            gs.rules.contains_key(ref_name.as_str()),
                            "Rule '{}' in {} references non-existent rule '{}'",
                            name,
                            path_str,
                            ref_name
                        );
                    }
                }
            }
        }
    }
}
