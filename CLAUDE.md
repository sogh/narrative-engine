# CLAUDE.md

## Project Overview

Narrative Engine is a Rust crate (`narrative-engine`) for procedural text generation in games. It generates thematically appropriate, contextually aware text at runtime without neural network inference, using a pipeline of simulation-driven events, stochastic grammars, and Markov-trained phrase generation.

The design doc is at `docs/narrative_engine_design_doc.md`.

## Tech Stack

- **Language:** Rust (latest stable)
- **Serialization:** `serde` + `ron` for all data formats (grammars, voices, corpora configs)
- **Randomness:** `rand` crate with `StdRng` (seedable)
- **Hashing:** `rustc-hash` (`FxHashMap`/`FxHashSet`) for hot-path lookups
- **Testing:** built-in `#[cfg(test)]` modules, plus integration tests in `tests/`
- **No runtime dependencies** on game engines — this is a pure library crate. Bevy integration (if needed) lives in a separate `narrative-engine-bevy` crate.

## Project Structure

```
narrative-engine/
├── Cargo.toml
├── CLAUDE.md
├── docs/
│   └── design.md
├── src/
│   ├── lib.rs              # Public API re-exports
│   ├── schema/
│   │   ├── mod.rs
│   │   ├── entity.rs
│   │   ├── event.rs
│   │   ├── narrative_fn.rs
│   │   └── relationship.rs
│   ├── core/
│   │   ├── mod.rs
│   │   ├── pipeline.rs
│   │   ├── grammar.rs
│   │   ├── markov.rs
│   │   ├── voice.rs
│   │   ├── context.rs
│   │   └── variety.rs
│   └── genre_templates/
│       ├── mod.rs
│       ├── social_drama.rs
│       ├── survival_thriller.rs
│       ├── political_intrigue.rs
│       └── exploration.rs
├── tools/
│   ├── corpus_trainer.rs    # Binary: train Markov models offline
│   ├── grammar_linter.rs   # Binary: validate grammar coverage
│   └── preview.rs          # Binary: interactive generation shell
├── genre_data/              # Shipped RON files for genre templates
│   ├── social_drama/
│   ├── survival_thriller/
│   ├── political_intrigue/
│   └── exploration/
├── tests/
│   ├── grammar_tests.rs
│   ├── markov_tests.rs
│   ├── pipeline_tests.rs
│   └── fixtures/            # Test grammars, corpora, voices
└── examples/
    ├── dinner_party.rs
    └── jurassic_park.rs
```

## Architecture Principles

### The Pipeline

Everything flows through: **Event → Narrative Function Mapping → Voice Selection → Grammar Expansion → Markov Phrase Fill → Variety Pass → Context Check → Output Text**

Each stage is a distinct module. Stages can be skipped or replaced.

### Key Abstractions

- **Narrative Function** is the most important abstraction. It separates WHAT is happening narratively (revelation, escalation, confrontation) from HOW it's expressed. This is what makes the engine reusable across games. When in doubt about where something belongs, ask: "Is this about narrative function or genre-specific rendering?"
- **Voice** is a data bundle, not code. Voices shape output through grammar weight overrides, vocabulary pools, Markov corpus bindings, and structural preferences. Voices can inherit from other voices.
- **Tags** are the universal coupling mechanism. Entities have tags, events have mood/stakes, grammar rules have preconditions — matching happens through tag intersection. The engine never interprets tag *meaning*.

### Data vs. Code Boundary

The engine (code) defines the *process*. Games define the *content* (grammars, corpora, voices, entity tags, event mappings). If you're writing game-specific logic inside `src/`, something is wrong. Genre templates sit at the boundary — they're shipped data with sensible defaults.

## Coding Conventions

### Rust Style

- Use `thiserror` for error types. Every public function returns `Result<T, NarrativeError>`.
- Prefer `&str` over `String` in function signatures where possible.
- Use newtypes for IDs: `EntityId(u64)`, `VoiceId(u64)`, etc. No raw integer IDs in public APIs.
- Builder pattern for complex construction (`NarrativeEngine::builder()`).
- All public types implement `Debug`, `Clone`, and `Serialize`/`Deserialize`.
- No `unwrap()` or `expect()` outside of tests.

### Determinism

All randomness goes through a single seedable `StdRng` passed via the pipeline context. Never use `thread_rng()` or any non-deterministic source. Given the same seed, world state, and event, output must be identical.

### Performance Priorities

1. Grammar rule matching is the hot path — optimize tag intersection lookups.
2. Markov models should be memory-mapped or pre-loaded, not parsed at generation time.
3. Allocations during generation should be minimized. Pre-allocate buffers where possible.
4. The engine should comfortably generate a passage in <1ms on commodity hardware.

### Testing

- Every grammar rule pattern needs a unit test proving it expands without error.
- Markov tests use small fixed corpora in `tests/fixtures/`.
- Pipeline integration tests use deterministic seeds and assert exact output.
- The grammar linter (`tools/grammar_linter.rs`) is both a shipped tool and a test harness — `cargo test` runs linting against all shipped genre templates.

## RON Data Format Conventions

Grammar files use RON (Rusty Object Notation). Key conventions:

```ron
// Rules are keyed by name
"rule_name": Rule(
    requires: ["tag:value", "mood:tense"],   // ALL must match (AND logic)
    excludes: ["voice:informal"],             // NONE may match
    alternatives: [
        (weight: 3, text: "literal text with {rule_ref} expansions"),
        (weight: 1, text: "another option with {markov:corpus:tag}"),
    ],
)
```

- `{rule_name}` expands another grammar rule
- `{markov:corpus_id:tag}` delegates to Markov generator
- `{entity.name}`, `{entity.property_key}` interpolates entity data
- `{subject}`, `{object}`, `{possessive}` are pronoun-aware entity refs

## Common Tasks

```bash
# Build
cargo build

# Run all tests
cargo test

# Run the grammar linter against genre templates
cargo run --bin grammar_linter -- genre_data/

# Train a Markov corpus
cargo run --bin corpus_trainer -- --input corpus.txt --output trained.bin --ngram 3

# Interactive preview shell
cargo run --bin preview -- --grammars path/to/grammars --voices path/to/voices

# Run a specific example
cargo run --example dinner_party
```

## What NOT to Do

- Don't add game-specific logic to the core engine. If a rule only makes sense for one game, it belongs in a grammar file, not in Rust code.
- Don't use `HashMap<String, String>` as a catch-all. Define proper types.
- Don't generate text by string concatenation with `format!`. The pipeline should assemble text through the grammar expansion system.
- Don't make the Markov layer mandatory. The grammar engine alone should produce usable output; Markov adds texture on top.
- Don't over-engineer the narrative function taxonomy up front. Start with the 10 core functions from the design doc and extend only when a real game needs it.

## Prompt Log

Manage a prompt log folder (`prompt-log/`) where you store markdown files that summarize our work:

- **Before executing any work**: Make an entry with the date/time and the user's verbatim prompt
- **After completing work**: Summarize what was done in the markdown doc
- This is an **append-only log** - never delete or modify previous entries
- Start a new file when beginning a new conversation or after a long gap since the last work
- Name files using the date of file creation (e.g., `2026-02-02.md`)

## TODO System

Check the folder `to-do/` for markdown documents that give you prompts to ingest and work on. If you have completed the work for a document there, move it to a folder called `done-work/`. If there is nothing in the folder, check the prompt log to see if there is an unfinished prompt in the latest document. If neither of these has unfinished work, then ask the user what is next.