/// Corpus Trainer — trains Markov models from text corpora.
///
/// Usage: corpus_trainer --input <file.txt> --output <model.ron> --ngram <2|3|4>
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut input = None;
    let mut output = None;
    let mut ngram = 2usize;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--input" => {
                i += 1;
                input = Some(args[i].clone());
            }
            "--output" => {
                i += 1;
                output = Some(args[i].clone());
            }
            "--ngram" => {
                i += 1;
                ngram = args[i].parse().unwrap_or_else(|_| {
                    eprintln!("Error: --ngram must be 2, 3, or 4");
                    process::exit(1);
                });
            }
            "--help" | "-h" => {
                println!(
                    "Usage: corpus_trainer --input <file.txt> --output <model.ron> --ngram <2|3|4>"
                );
                process::exit(0);
            }
            other => {
                eprintln!("Unknown argument: {}", other);
                process::exit(1);
            }
        }
        i += 1;
    }

    let input_path = input.unwrap_or_else(|| {
        eprintln!("Error: --input is required");
        eprintln!("Usage: corpus_trainer --input <file.txt> --output <model.ron> --ngram <2|3|4>");
        process::exit(1);
    });

    let output_path = output.unwrap_or_else(|| {
        eprintln!("Error: --output is required");
        eprintln!("Usage: corpus_trainer --input <file.txt> --output <model.ron> --ngram <2|3|4>");
        process::exit(1);
    });

    if !(2..=4).contains(&ngram) {
        eprintln!("Error: --ngram must be 2, 3, or 4");
        process::exit(1);
    }

    let text = std::fs::read_to_string(&input_path).unwrap_or_else(|e| {
        eprintln!("Error reading input file '{}': {}", input_path, e);
        process::exit(1);
    });

    println!("Training {}-gram model from '{}'...", ngram, input_path);
    let model = narrative_engine::core::markov::MarkovTrainer::train(&text, ngram);

    let transition_count: usize = model.transitions.values().map(|v| v.len()).sum();
    println!(
        "Model trained: {} unique prefixes, {} transitions",
        model.transitions.len(),
        transition_count
    );

    if !model.tagged_transitions.is_empty() {
        println!(
            "Tags found: {:?}",
            model.tagged_transitions.keys().collect::<Vec<_>>()
        );
    }

    narrative_engine::core::markov::save_model(&model, std::path::Path::new(&output_path))
        .unwrap_or_else(|e| {
            eprintln!("Error saving model to '{}': {}", output_path, e);
            process::exit(1);
        });

    println!("Model saved to '{}'", output_path);
}
