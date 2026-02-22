/// Grammar Linter — validates grammar rule coverage and quality.
///
/// Usage: grammar_linter <grammar_dir> [--models-dir <dir>]

use narrative_engine::core::grammar::GrammarSet;
use std::collections::HashSet;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        println!("Usage: grammar_linter <grammar_dir> [--models-dir <dir>]");
        process::exit(0);
    }

    let grammar_dir = &args[1];
    let mut models_dir = None;

    let mut i = 2;
    while i < args.len() {
        if args[i] == "--models-dir" && i + 1 < args.len() {
            i += 1;
            models_dir = Some(args[i].clone());
        }
        i += 1;
    }

    // Load all grammar files from directory
    let mut grammars = GrammarSet::default();
    let grammar_path = Path::new(grammar_dir);

    if grammar_path.is_file() {
        match GrammarSet::load_from_ron(grammar_path) {
            Ok(gs) => grammars.merge(gs),
            Err(e) => {
                eprintln!("ERROR: Failed to load grammar file: {}", e);
                process::exit(1);
            }
        }
    } else if grammar_path.is_dir() {
        load_grammars_recursive(grammar_path, &mut grammars);
    } else {
        eprintln!("ERROR: Path '{}' does not exist", grammar_dir);
        process::exit(1);
    }

    println!("Loaded {} grammar rules", grammars.rules.len());

    // Load model corpus IDs if provided
    let model_ids: HashSet<String> = if let Some(ref dir) = models_dir {
        load_model_ids(dir)
    } else {
        HashSet::new()
    };

    // Run linting
    let (errors, warnings) = lint_grammars(&grammars, &model_ids);

    // Print report
    println!("\n=== Grammar Lint Report ===\n");

    if errors.is_empty() && warnings.is_empty() {
        println!("All checks passed!");
    }

    for warning in &warnings {
        println!("WARNING: {}", warning);
    }

    for error in &errors {
        println!("ERROR: {}", error);
    }

    println!(
        "\nSummary: {} errors, {} warnings",
        errors.len(),
        warnings.len()
    );

    if errors.is_empty() {
        process::exit(0);
    } else {
        process::exit(1);
    }
}

fn load_grammars_recursive(dir: &Path, grammars: &mut GrammarSet) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                load_grammars_recursive(&path, grammars);
            } else if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                match GrammarSet::load_from_ron(&path) {
                    Ok(gs) => {
                        println!("  Loaded: {}", path.display());
                        grammars.merge(gs);
                    }
                    Err(e) => {
                        eprintln!("  ERROR loading {}: {}", path.display(), e);
                    }
                }
            }
        }
    }
}

fn load_model_ids(dir: &str) -> HashSet<String> {
    let mut ids = HashSet::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Some(stem) = entry.path().file_stem() {
                ids.insert(stem.to_string_lossy().to_string());
            }
        }
    }
    ids
}

fn lint_grammars(grammars: &GrammarSet, model_ids: &HashSet<String>) -> (Vec<String>, Vec<String>) {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // Narrative function names (the 10 core ones)
    let fn_names = [
        "revelation",
        "escalation",
        "confrontation",
        "betrayal",
        "alliance",
        "discovery",
        "loss",
        "comic_relief",
        "foreshadowing",
        "status_change",
    ];

    let _moods = [
        "mood:neutral",
        "mood:tense",
        "mood:warm",
        "mood:dread",
        "mood:euphoric",
        "mood:somber",
        "mood:chaotic",
        "mood:intimate",
    ];

    let _stakes = [
        "stakes:trivial",
        "stakes:low",
        "stakes:medium",
        "stakes:high",
        "stakes:critical",
    ];

    // Coverage analysis: check that each fn has at least _opening rule
    for fn_name in &fn_names {
        let opening_rule = format!("{}_opening", fn_name);
        if !grammars.rules.contains_key(&opening_rule) {
            // Not an error if the grammars don't cover this function
            // (partial templates are fine)
            warnings.push(format!(
                "No '{}' rule found for narrative function '{}'",
                opening_rule, fn_name
            ));
        }
    }

    // Rule quality checks
    for (name, rule) in &grammars.rules {
        // Low variety warning
        if rule.alternatives.len() < 3 {
            warnings.push(format!(
                "Rule '{}' has only {} alternatives (minimum 3 recommended)",
                name,
                rule.alternatives.len()
            ));
        }

        // Check for MarkovRef to nonexistent corpus IDs
        if !model_ids.is_empty() {
            for alt in &rule.alternatives {
                for segment in &alt.template.segments {
                    if let narrative_engine::core::grammar::TemplateSegment::MarkovRef {
                        corpus,
                        ..
                    } = segment
                    {
                        if !model_ids.contains(corpus.as_str()) {
                            warnings.push(format!(
                                "Rule '{}' references Markov corpus '{}' which is not in loaded models",
                                name, corpus
                            ));
                        }
                    }
                }
            }
        }

        // Check for rule references that don't exist
        for alt in &rule.alternatives {
            for segment in &alt.template.segments {
                if let narrative_engine::core::grammar::TemplateSegment::RuleRef(ref_name) = segment
                {
                    if !grammars.rules.contains_key(ref_name.as_str()) {
                        errors.push(format!(
                            "Rule '{}' references non-existent rule '{}'",
                            name, ref_name
                        ));
                    }
                }
            }
        }
    }

    // Check for direct self-referencing cycles (simple cycle detection)
    for (name, rule) in &grammars.rules {
        for alt in &rule.alternatives {
            for segment in &alt.template.segments {
                if let narrative_engine::core::grammar::TemplateSegment::RuleRef(ref_name) = segment
                {
                    if ref_name == name {
                        // Direct self-reference — check if ALL alternatives are self-referencing
                        let all_self_ref = rule.alternatives.iter().all(|a| {
                            a.template.segments.iter().any(|s| {
                                matches!(s, narrative_engine::core::grammar::TemplateSegment::RuleRef(r) if r == name)
                            })
                        });
                        if all_self_ref {
                            errors.push(format!(
                                "Rule '{}' has no non-recursive alternative (infinite recursion)",
                                name
                            ));
                        }
                    }
                }
            }
        }
    }

    (errors, warnings)
}
