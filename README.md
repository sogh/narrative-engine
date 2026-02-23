# Narrative Engine

A Rust library for procedural text generation in games. Generates thematically appropriate, contextually aware narrative text at runtime without neural network inference, using a pipeline of simulation-driven events, stochastic grammars, and Markov-trained phrase generation.

## Features

- **Narrative Function abstraction** — separates *what* is happening narratively (revelation, escalation, confrontation) from *how* it's expressed, making the engine reusable across games
- **Stochastic grammar expansion** with tag-based rule matching
- **Markov phrase generation** for natural texture on top of structured grammar output
- **Voice system** — data-driven voice profiles shape vocabulary, grammar weights, and structure without code changes
- **Genre templates** — shipped defaults for social drama, survival thriller, political intrigue, and exploration
- **Fully deterministic** — seedable RNG guarantees identical output for the same inputs
- **No runtime dependencies** on game engines — pure library crate

## Quick Start

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
narrative-engine = { path = "path/to/narrative-engine" }
```

Generate a passage:

```rust
use narrative_engine::*;
use narrative_engine::core::grammar::GrammarSet;
use narrative_engine::core::pipeline::{NarrativeEngine, WorldState};
use narrative_engine::core::voice::VoiceRegistry;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let grammars = GrammarSet::load_from_ron(
        Path::new("genre_data/social_drama/grammar.ron"),
    )?;
    let mut voices = VoiceRegistry::new();
    voices.load_from_ron(Path::new("genre_data/social_drama/voices.ron"))?;

    let engine = NarrativeEngine::builder()
        .grammars(grammars)
        .voices(voices)
        .seed(42)
        .build()?;

    let world = WorldState::default();
    let event = Event::new("dinner_conversation")
        .mood(Mood::Tense)
        .stakes(Stakes::Personal);

    let passage = engine.narrate(&world, &event)?;
    println!("{}", passage.text);
    Ok(())
}
```

See the [User Guide](docs/user_guide.md) for the full API reference, content authoring guide, and end-to-end examples.

## The Pipeline

Everything flows through:

**Event → Narrative Function Mapping → Voice Selection → Grammar Expansion → Markov Phrase Fill → Variety Pass → Context Check → Output Text**

Each stage is a distinct module. Stages can be skipped or replaced.

## Tools

The crate ships three utility binaries:

```bash
# Validate grammar coverage across all rules
cargo run --bin grammar_linter -- genre_data/

# Train a Markov corpus from a text file
cargo run --bin corpus_trainer -- --input corpus.txt --output trained.bin --ngram 3

# Interactive generation shell for testing
cargo run --bin preview -- --grammars genre_data/social_drama/grammar.ron --voices genre_data/social_drama/voices.ron
```

## Examples

```bash
cargo run --example dinner_party
cargo run --example dino_park
```

## Documentation

- [User Guide](docs/user_guide.md) — full usage guide, API reference, and content authoring
- [Design Document](docs/narrative_engine_design_doc.md) — architecture and design rationale

## License

[MIT](LICENSE)
