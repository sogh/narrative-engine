/// Preview — interactive generation shell for testing grammars and voices.
///
/// Usage: preview --grammars <path> --voices <path> [--models <path>] [--seed <n>]
///
/// Commands:
///   event <fn> <mood> <stakes>  — generate from a synthetic event
///   voice <name>                — set active voice
///   entity <name> <tag1,tag2>   — define a named entity
///   seed <n>                    — set RNG seed
///   bulk <n>                    — generate n passages with variety stats
///   help                        — list commands
///   quit                        — exit

use narrative_engine::core::grammar::GrammarSet;
use narrative_engine::core::markov::MarkovModel;
use narrative_engine::core::pipeline::{NarrativeEngine, WorldState};
use narrative_engine::core::voice::VoiceRegistry;
use narrative_engine::schema::entity::{Entity, EntityId, VoiceId};
use narrative_engine::schema::event::{EntityRef, Event, Mood, Stakes};
use narrative_engine::schema::narrative_fn::NarrativeFunction;
use std::collections::HashMap;
use std::io::{self, BufRead, Write};
use std::path::Path;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 || args[1] == "--help" || args[1] == "-h" {
        print_usage();
        return;
    }

    let mut grammars_path = None;
    let mut voices_path = None;
    let mut models_path = None;
    let mut seed: u64 = 42;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--grammars" if i + 1 < args.len() => {
                i += 1;
                grammars_path = Some(args[i].clone());
            }
            "--voices" if i + 1 < args.len() => {
                i += 1;
                voices_path = Some(args[i].clone());
            }
            "--models" if i + 1 < args.len() => {
                i += 1;
                models_path = Some(args[i].clone());
            }
            "--seed" if i + 1 < args.len() => {
                i += 1;
                seed = args[i].parse().unwrap_or(42);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_usage();
                std::process::exit(1);
            }
        }
        i += 1;
    }

    // Load grammars
    let mut grammars = GrammarSet::default();
    if let Some(ref path) = grammars_path {
        load_grammars_from_path(path, &mut grammars);
    }

    // Load voices
    let mut voices = VoiceRegistry::new();
    if let Some(ref path) = voices_path {
        load_voices_from_path(path, &mut voices);
    }

    // Load markov models
    let mut markov_models: HashMap<String, MarkovModel> = HashMap::new();
    if let Some(ref path) = models_path {
        load_models_from_path(path, &mut markov_models);
    }

    println!("Loaded {} grammar rules", grammars.rules.len());
    println!("Seed: {}", seed);
    println!("Type 'help' for commands.\n");

    // Session state
    let mut entities: HashMap<EntityId, Entity> = HashMap::new();
    let mut next_entity_id: u64 = 1;
    let mut active_voice_id: Option<VoiceId> = None;
    let mut current_seed = seed;

    // Build engine
    let mut engine = build_engine(grammars.clone(), voices.clone(), markov_models.clone(), current_seed);

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("preview> ");
        stdout.flush().ok();

        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() || line.is_empty() {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        let cmd = parts[0].to_lowercase();

        match cmd.as_str() {
            "quit" | "exit" | "q" => {
                println!("Goodbye.");
                break;
            }
            "help" | "h" | "?" => {
                print_help();
            }
            "event" => {
                if parts.len() < 4 {
                    println!("Usage: event <fn> <mood> <stakes>");
                    println!("  fn: revelation, escalation, confrontation, betrayal, alliance,");
                    println!("      discovery, loss, comic_relief, foreshadowing, status_change");
                    println!("  mood: neutral, tense, warm, dread, euphoric, somber, chaotic, intimate");
                    println!("  stakes: trivial, low, medium, high, critical");
                    continue;
                }
                let narrative_fn = match parse_narrative_fn(parts[1]) {
                    Some(f) => f,
                    None => {
                        println!("Unknown narrative function: {}", parts[1]);
                        continue;
                    }
                };
                let mood = match parse_mood(parts[2]) {
                    Some(m) => m,
                    None => {
                        println!("Unknown mood: {}", parts[2]);
                        continue;
                    }
                };
                let stakes = match parse_stakes(parts[3]) {
                    Some(s) => s,
                    None => {
                        println!("Unknown stakes: {}", parts[3]);
                        continue;
                    }
                };

                // Build event with all defined entities
                let mut participants = Vec::new();
                let mut entity_ids: Vec<EntityId> = entities.keys().copied().collect();
                entity_ids.sort_by_key(|id| id.0);

                if let Some(&first) = entity_ids.first() {
                    participants.push(EntityRef {
                        entity_id: first,
                        role: "subject".to_string(),
                    });
                }
                if let Some(&second) = entity_ids.get(1) {
                    participants.push(EntityRef {
                        entity_id: second,
                        role: "object".to_string(),
                    });
                }

                let event = Event {
                    event_type: format!("preview_{}", narrative_fn.name()),
                    participants,
                    location: None,
                    mood,
                    stakes,
                    outcome: None,
                    narrative_fn,
                    metadata: HashMap::new(),
                };

                let world = WorldState {
                    entities: &entities,
                };

                match if let Some(vid) = active_voice_id {
                    engine.narrate_as(&event, vid, &world)
                } else {
                    engine.narrate(&event, &world)
                } {
                    Ok(text) => {
                        println!("\n--- Generated Text ---");
                        println!("{}", text);
                        println!("--- End ---\n");
                        print_expansion_trace(&event);
                    }
                    Err(e) => {
                        println!("ERROR: {}", e);
                    }
                }
            }
            "voice" => {
                if parts.len() < 2 {
                    println!("Usage: voice <name>");
                    println!("  Set 'none' to clear active voice.");
                    if let Some(vid) = active_voice_id {
                        println!("  Current: {:?}", vid);
                    } else {
                        println!("  Current: none");
                    }
                    continue;
                }
                let name = parts[1];
                if name == "none" {
                    active_voice_id = None;
                    println!("Active voice cleared.");
                    continue;
                }
                // Search voices by name — we need to check all registered voices
                // Since VoiceRegistry doesn't expose iteration, try common IDs
                let mut found = false;
                for id_val in 0..1000 {
                    let vid = VoiceId(id_val);
                    if let Some(voice) = voices.get(vid) {
                        if voice.name == name {
                            active_voice_id = Some(vid);
                            println!("Active voice set to '{}' ({:?})", name, vid);
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    println!("Voice '{}' not found. Try a voice name from the loaded voice files.", name);
                }
            }
            "entity" => {
                if parts.len() < 3 {
                    println!("Usage: entity <name> <tag1,tag2,...>");
                    println!("  Defined entities:");
                    let mut ids: Vec<EntityId> = entities.keys().copied().collect();
                    ids.sort_by_key(|id| id.0);
                    for id in ids {
                        let e = &entities[&id];
                        let tags: Vec<&String> = e.tags.iter().collect();
                        println!("    {} (id={}) tags={:?}", e.name, id.0, tags);
                    }
                    continue;
                }
                let name = parts[1].to_string();
                let tags: rustc_hash::FxHashSet<String> = parts[2]
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                let eid = EntityId(next_entity_id);
                next_entity_id += 1;

                entities.insert(
                    eid,
                    Entity {
                        id: eid,
                        name: name.clone(),
                        pronouns: narrative_engine::schema::entity::Pronouns::TheyThem,
                        tags,
                        relationships: Vec::new(),
                        voice_id: active_voice_id,
                        properties: HashMap::new(),
                    },
                );
                println!("Entity '{}' created with id={}", name, eid.0);
            }
            "seed" => {
                if parts.len() < 2 {
                    println!("Current seed: {}", current_seed);
                    continue;
                }
                match parts[1].parse::<u64>() {
                    Ok(s) => {
                        current_seed = s;
                        engine = build_engine(
                            grammars.clone(),
                            voices.clone(),
                            markov_models.clone(),
                            current_seed,
                        );
                        println!("Seed set to {}", current_seed);
                    }
                    Err(_) => {
                        println!("Invalid seed: {}", parts[1]);
                    }
                }
            }
            "bulk" => {
                if parts.len() < 2 {
                    println!("Usage: bulk <n>");
                    println!("  Requires at least one entity. Define with 'entity' first.");
                    continue;
                }
                let count: usize = match parts[1].parse() {
                    Ok(n) if n > 0 => n,
                    _ => {
                        println!("Invalid count: {}", parts[1]);
                        continue;
                    }
                };

                if entities.is_empty() {
                    println!("No entities defined. Use 'entity' to create one first.");
                    continue;
                }

                // Generate bulk passages using confrontation as default
                let mut entity_ids: Vec<EntityId> = entities.keys().copied().collect();
                entity_ids.sort_by_key(|id| id.0);

                let mut participants = Vec::new();
                if let Some(&first) = entity_ids.first() {
                    participants.push(EntityRef {
                        entity_id: first,
                        role: "subject".to_string(),
                    });
                }
                if let Some(&second) = entity_ids.get(1) {
                    participants.push(EntityRef {
                        entity_id: second,
                        role: "object".to_string(),
                    });
                }

                // Cycle through narrative functions
                let fns = [
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
                let moods = [
                    Mood::Tense,
                    Mood::Neutral,
                    Mood::Warm,
                    Mood::Dread,
                    Mood::Somber,
                ];

                // Rebuild engine for fresh context
                let mut bulk_engine = build_engine(
                    grammars.clone(),
                    voices.clone(),
                    markov_models.clone(),
                    current_seed,
                );

                let world = WorldState {
                    entities: &entities,
                };

                let mut passages = Vec::new();
                let mut errors = 0;

                for i in 0..count {
                    let narrative_fn = fns[i % fns.len()].clone();
                    let mood = moods[i % moods.len()];

                    let event = Event {
                        event_type: format!("bulk_{}", narrative_fn.name()),
                        participants: participants.clone(),
                        location: None,
                        mood,
                        stakes: Stakes::High,
                        outcome: None,
                        narrative_fn,
                        metadata: HashMap::new(),
                    };

                    match if let Some(vid) = active_voice_id {
                        bulk_engine.narrate_as(&event, vid, &world)
                    } else {
                        bulk_engine.narrate(&event, &world)
                    } {
                        Ok(text) => passages.push(text),
                        Err(_) => errors += 1,
                    }
                }

                // Print statistics
                println!("\n=== Bulk Generation: {} passages ({} errors) ===\n", passages.len(), errors);

                // Unique openings
                let openings: Vec<String> = passages
                    .iter()
                    .map(|p| {
                        p.split('.')
                            .next()
                            .unwrap_or("")
                            .trim()
                            .to_string()
                    })
                    .collect();
                let unique_openings: std::collections::HashSet<&String> = openings.iter().collect();
                println!("Unique openings: {} / {}", unique_openings.len(), passages.len());

                // Average length
                let avg_len: f64 = if passages.is_empty() {
                    0.0
                } else {
                    passages.iter().map(|p| p.len() as f64).sum::<f64>() / passages.len() as f64
                };
                println!("Average length: {:.0} chars", avg_len);

                // Word frequency distribution (top 10)
                let mut word_counts: HashMap<String, u32> = HashMap::new();
                for passage in &passages {
                    for word in passage.split_whitespace() {
                        let clean = word
                            .trim_matches(|c: char| !c.is_alphanumeric())
                            .to_lowercase();
                        if clean.len() > 3 {
                            *word_counts.entry(clean).or_insert(0) += 1;
                        }
                    }
                }
                let mut word_freq: Vec<(String, u32)> = word_counts.into_iter().collect();
                word_freq.sort_by(|a, b| b.1.cmp(&a.1));
                println!("\nTop 10 words:");
                for (word, count) in word_freq.iter().take(10) {
                    println!("  {}: {}", word, count);
                }

                // Print a sample
                if let Some(first) = passages.first() {
                    println!("\nSample passage:");
                    println!("  {}", first);
                }
                println!();
            }
            _ => {
                println!("Unknown command: '{}'. Type 'help' for available commands.", cmd);
            }
        }
    }
}

fn print_usage() {
    println!("Preview — interactive generation shell for testing grammars and voices.");
    println!();
    println!("Usage: preview --grammars <path> --voices <path> [--models <path>] [--seed <n>]");
    println!();
    println!("  --grammars <path>  Path to grammar file or directory");
    println!("  --voices <path>    Path to voices file or directory");
    println!("  --models <path>    Path to Markov model directory (optional)");
    println!("  --seed <n>         Initial RNG seed (default: 42)");
}

fn print_help() {
    println!("Commands:");
    println!("  event <fn> <mood> <stakes>  Generate from a synthetic event");
    println!("  voice <name>                Set active voice (or 'none' to clear)");
    println!("  entity <name> <tags>        Define a named entity (tags comma-separated)");
    println!("  seed <n>                    Set RNG seed");
    println!("  bulk <n>                    Generate n passages with variety statistics");
    println!("  help                        Show this help");
    println!("  quit                        Exit");
    println!();
    println!("Narrative functions:");
    println!("  revelation, escalation, confrontation, betrayal, alliance,");
    println!("  discovery, loss, comic_relief, foreshadowing, status_change");
    println!();
    println!("Moods: neutral, tense, warm, dread, euphoric, somber, chaotic, intimate");
    println!("Stakes: trivial, low, medium, high, critical");
}

fn parse_narrative_fn(s: &str) -> Option<NarrativeFunction> {
    match s.to_lowercase().as_str() {
        "revelation" => Some(NarrativeFunction::Revelation),
        "escalation" => Some(NarrativeFunction::Escalation),
        "confrontation" => Some(NarrativeFunction::Confrontation),
        "betrayal" => Some(NarrativeFunction::Betrayal),
        "alliance" => Some(NarrativeFunction::Alliance),
        "discovery" => Some(NarrativeFunction::Discovery),
        "loss" => Some(NarrativeFunction::Loss),
        "comic_relief" => Some(NarrativeFunction::ComicRelief),
        "foreshadowing" => Some(NarrativeFunction::Foreshadowing),
        "status_change" => Some(NarrativeFunction::StatusChange),
        _ => None,
    }
}

fn parse_mood(s: &str) -> Option<Mood> {
    match s.to_lowercase().as_str() {
        "neutral" => Some(Mood::Neutral),
        "tense" => Some(Mood::Tense),
        "warm" => Some(Mood::Warm),
        "dread" => Some(Mood::Dread),
        "euphoric" => Some(Mood::Euphoric),
        "somber" => Some(Mood::Somber),
        "chaotic" => Some(Mood::Chaotic),
        "intimate" => Some(Mood::Intimate),
        _ => None,
    }
}

fn parse_stakes(s: &str) -> Option<Stakes> {
    match s.to_lowercase().as_str() {
        "trivial" => Some(Stakes::Trivial),
        "low" => Some(Stakes::Low),
        "medium" => Some(Stakes::Medium),
        "high" => Some(Stakes::High),
        "critical" => Some(Stakes::Critical),
        _ => None,
    }
}

fn print_expansion_trace(event: &Event) {
    println!("[Trace] fn={} mood={} stakes={}",
        event.narrative_fn.name(),
        event.mood.tag(),
        event.stakes.tag(),
    );
    println!("[Trace] Entry rule: {}_opening", event.narrative_fn.name());
    println!("[Trace] Participants: {}", event.participants.len());
}

fn build_engine(
    grammars: GrammarSet,
    voices: VoiceRegistry,
    markov_models: HashMap<String, MarkovModel>,
    seed: u64,
) -> NarrativeEngine {
    NarrativeEngine::builder()
        .seed(seed)
        .with_grammars(grammars)
        .with_voices(voices)
        .with_markov_models(markov_models)
        .build()
        .unwrap()
}

fn load_grammars_from_path(path: &str, grammars: &mut GrammarSet) {
    let p = Path::new(path);
    if p.is_file() {
        match GrammarSet::load_from_ron(p) {
            Ok(gs) => {
                println!("Loaded grammar: {}", path);
                grammars.merge(gs);
            }
            Err(e) => eprintln!("ERROR loading grammar {}: {}", path, e),
        }
    } else if p.is_dir() {
        load_grammars_recursive(p, grammars);
    } else {
        eprintln!("Grammar path not found: {}", path);
    }
}

fn load_grammars_recursive(dir: &Path, grammars: &mut GrammarSet) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                load_grammars_recursive(&path, grammars);
            } else if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if stem == "grammar" || stem.ends_with("_grammar") {
                        match GrammarSet::load_from_ron(&path) {
                            Ok(gs) => {
                                println!("Loaded grammar: {}", path.display());
                                grammars.merge(gs);
                            }
                            Err(e) => eprintln!("ERROR loading {}: {}", path.display(), e),
                        }
                    }
                }
            }
        }
    }
}

fn load_voices_from_path(path: &str, voices: &mut VoiceRegistry) {
    let p = Path::new(path);
    if p.is_file() {
        match voices.load_from_ron(p) {
            Ok(()) => println!("Loaded voices: {}", path),
            Err(e) => eprintln!("ERROR loading voices {}: {}", path, e),
        }
    } else if p.is_dir() {
        load_voices_recursive(p, voices);
    } else {
        eprintln!("Voices path not found: {}", path);
    }
}

fn load_voices_recursive(dir: &Path, voices: &mut VoiceRegistry) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                load_voices_recursive(&path, voices);
            } else if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    if stem == "voices" || stem.ends_with("_voices") {
                        match voices.load_from_ron(&path) {
                            Ok(()) => println!("Loaded voices: {}", path.display()),
                            Err(e) => eprintln!("ERROR loading {}: {}", path.display(), e),
                        }
                    }
                }
            }
        }
    }
}

fn load_models_from_path(path: &str, models: &mut HashMap<String, MarkovModel>) {
    let p = Path::new(path);
    if p.is_dir() {
        if let Ok(entries) = std::fs::read_dir(p) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("ron") {
                    match narrative_engine::core::markov::load_model(&path) {
                        Ok(model) => {
                            let name = path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("unknown")
                                .to_string();
                            println!("Loaded model: {}", name);
                            models.insert(name, model);
                        }
                        Err(e) => eprintln!("ERROR loading model {}: {}", path.display(), e),
                    }
                }
            }
        }
    } else {
        eprintln!("Models path is not a directory: {}", path);
    }
}
