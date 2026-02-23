# Narrative Engine User Guide

Narrative Engine is a Rust library for procedural text generation in games. It generates thematically appropriate, contextually aware narrative text at runtime without neural network inference, using a pipeline of simulation-driven events, stochastic grammars, and Markov-trained phrase generation.

This guide covers everything you need to integrate the engine into your game, author content for it, and use its tooling.

---

## Table of Contents

- [Quick Start](#quick-start)
- [Core Concepts](#core-concepts)
  - [The Pipeline](#the-pipeline)
  - [Narrative Functions](#narrative-functions)
  - [Events](#events)
  - [Entities](#entities)
  - [Voices](#voices)
  - [Grammars](#grammars)
  - [Markov Models](#markov-models)
  - [Context and Variety](#context-and-variety)
- [API Reference](#api-reference)
  - [NarrativeEngine Builder](#narrativeengine-builder)
  - [Narration Methods](#narration-methods)
  - [WorldState](#worldstate)
  - [Error Handling](#error-handling)
- [Content Authoring](#content-authoring)
  - [Grammar Files (RON)](#grammar-files-ron)
  - [Template Syntax](#template-syntax)
  - [Voice Files (RON)](#voice-files-ron)
  - [Markov Corpora](#markov-corpora)
  - [Event Mappings](#event-mappings)
- [Genre Templates](#genre-templates)
  - [Social Drama](#social-drama)
  - [Survival Thriller](#survival-thriller)
  - [Political Intrigue](#political-intrigue)
  - [Exploration](#exploration)
  - [Blending Templates](#blending-templates)
- [Tools](#tools)
  - [Grammar Linter](#grammar-linter)
  - [Corpus Trainer](#corpus-trainer)
  - [Preview Shell](#preview-shell)
- [End-to-End Examples](#end-to-end-examples)
  - [Dinner Party (Social Drama)](#dinner-party-social-drama)
  - [Jurassic Park (Survival Thriller)](#jurassic-park-survival-thriller)
- [Determinism and Seeding](#determinism-and-seeding)
- [Performance](#performance)
- [Extending the Engine](#extending-the-engine)
  - [Custom Narrative Functions](#custom-narrative-functions)
  - [Custom Genre Templates](#custom-genre-templates)
  - [Voice Inheritance](#voice-inheritance)
  - [Grammar Override Layering](#grammar-override-layering)

---

## Quick Start

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
narrative-engine = { path = "path/to/narrative-engine" }
```

Generate your first passage:

```rust
use narrative_engine::*;
use narrative_engine::core::grammar::GrammarSet;
use narrative_engine::core::pipeline::{NarrativeEngine, WorldState};
use narrative_engine::core::voice::VoiceRegistry;
use std::collections::HashMap;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Load a genre template
    let grammars = GrammarSet::load_from_ron(
        Path::new("genre_data/social_drama/grammar.ron"),
    )?;
    let mut voices = VoiceRegistry::new();
    voices.load_from_ron(Path::new("genre_data/social_drama/voices.ron"))?;

    // 2. Build the engine
    let mut engine = NarrativeEngine::builder()
        .seed(42)
        .with_grammars(grammars)
        .with_voices(voices)
        .build()?;

    // 3. Define entities
    let mut entities = HashMap::new();
    entities.insert(EntityId(1), Entity {
        id: EntityId(1),
        name: "Alice".to_string(),
        tags: ["host".to_string(), "anxious".to_string()].into_iter().collect(),
        relationships: Vec::new(),
        voice_id: Some(VoiceId(100)),
        properties: HashMap::new(),
    });
    entities.insert(EntityId(2), Entity {
        id: EntityId(2),
        name: "Bob".to_string(),
        tags: ["guest".to_string()].into_iter().collect(),
        relationships: Vec::new(),
        voice_id: None,
        properties: HashMap::new(),
    });

    let world = WorldState { entities: &entities };

    // 4. Create an event and generate text
    let event = Event {
        event_type: "argument".to_string(),
        participants: vec![
            EntityRef { entity_id: EntityId(1), role: "subject".to_string() },
            EntityRef { entity_id: EntityId(2), role: "object".to_string() },
        ],
        location: None,
        mood: Mood::Tense,
        stakes: Stakes::High,
        outcome: None,
        narrative_fn: NarrativeFunction::Confrontation,
        metadata: HashMap::new(),
    };

    let text = engine.narrate(&event, &world)?;
    println!("{}", text);

    Ok(())
}
```

---

## Core Concepts

### The Pipeline

Everything flows through a fixed pipeline of stages. Each stage is a distinct module. Stages can be skipped or replaced.

```
Game Simulation
    |
    v
Structured Event
    |
    v
Narrative Function Mapping .... What is happening narratively?
    |
    v
Voice Selection ............... Who is telling this story?
    |
    v
Grammar Expansion ............. Generate text from weighted rules
    |
    v
Markov Phrase Fill ............ Add texture from trained models
    |
    v
Variety Pass .................. Synonym rotation, quirk injection
    |
    v
Context Check ................. Anti-repetition validation
    |
    v
Output Text
```

Your game produces **events**. The engine produces **text**. The bridge between them is the **narrative function** — an abstract label describing what is happening in story terms.

### Narrative Functions

A narrative function is the most important abstraction in the system. It separates *what* is happening narratively from *how* it is expressed.

The engine ships with 10 core functions:

| Function | Description | Pacing | Valence | Intensity |
|---|---|---|---|---|
| `Revelation` | Hidden information becomes known | 0.5 | -0.3 | 0.7 |
| `Escalation` | Stakes or tension increase | 0.8 | -0.6 | 0.8 |
| `Confrontation` | Two entities in direct opposition | 0.7 | -0.5 | 0.9 |
| `Betrayal` | Trust is violated | 0.6 | -0.9 | 0.9 |
| `Alliance` | Entities align interests | 0.3 | 0.6 | 0.4 |
| `Discovery` | Something new is found or understood | 0.5 | 0.3 | 0.6 |
| `Loss` | Something valued is taken or destroyed | 0.4 | -0.8 | 0.8 |
| `ComicRelief` | Tension broken with humor | 0.6 | 0.7 | 0.3 |
| `Foreshadowing` | Future events are hinted at | 0.2 | -0.2 | 0.3 |
| `StatusChange` | An entity's position shifts | 0.4 | 0.0 | 0.5 |

You can also create custom functions with `NarrativeFunction::Custom("your_function".to_string())`.

Each function has three numeric properties:

- **Pacing** (0.0 to 1.0): How fast/urgent the beat feels. High for Escalation, low for Foreshadowing.
- **Valence** (-1.0 to 1.0): Emotional direction. Negative for Betrayal/Loss, positive for Alliance/ComicRelief.
- **Intensity** (0.0 to 1.0): How dramatic. High for Confrontation, low for ComicRelief.

These properties are injected as tags during grammar expansion (e.g., `intensity:high`), allowing grammar rules to adapt their output.

### Events

An `Event` is a structured record of something that happened in your game simulation. The engine never generates events — your game does. The engine turns events into text.

```rust
pub struct Event {
    pub event_type: String,           // Game-defined label, e.g. "accusation"
    pub participants: Vec<EntityRef>, // Who's involved and their roles
    pub location: Option<EntityRef>,  // Where it happens
    pub mood: Mood,                   // Emotional tone
    pub stakes: Stakes,               // How consequential
    pub outcome: Option<Outcome>,     // Did it succeed/fail?
    pub narrative_fn: NarrativeFunction, // What narratively happened
    pub metadata: HashMap<String, Value>, // Arbitrary extra data
}
```

**Mood** sets the emotional tone and is injected as a tag (e.g., `mood:tense`):

| Variant | Tag | Use for |
|---|---|---|
| `Neutral` | `mood:neutral` | Default, unremarkable moments |
| `Tense` | `mood:tense` | Conflict, confrontation |
| `Warm` | `mood:warm` | Friendship, comfort |
| `Dread` | `mood:dread` | Horror, foreboding |
| `Euphoric` | `mood:euphoric` | Triumph, joy |
| `Somber` | `mood:somber` | Grief, reflection |
| `Chaotic` | `mood:chaotic` | Panic, disorder |
| `Intimate` | `mood:intimate` | Private, personal |

**Stakes** set the consequence level:

| Variant | Tag |
|---|---|
| `Trivial` | `stakes:trivial` |
| `Low` | `stakes:low` |
| `Medium` | `stakes:medium` |
| `High` | `stakes:high` |
| `Critical` | `stakes:critical` |

**Participant roles** determine which entity fills `{subject}`, `{object}`, `{possessive}`, and `{entity.field}` template slots:

```rust
EntityRef {
    entity_id: EntityId(1),
    role: "subject".to_string(), // "subject", "object", "witness", etc.
}
```

### Entities

An `Entity` is anything that participates in narrative events — characters, locations, objects, abstract concepts.

```rust
pub struct Entity {
    pub id: EntityId,
    pub name: String,
    pub tags: FxHashSet<String>,
    pub relationships: Vec<Relationship>,
    pub voice_id: Option<VoiceId>,
    pub properties: HashMap<String, Value>,
}
```

Key points:

- **Tags** are the universal coupling mechanism. Tags on entities are injected into the grammar selection context. The engine never interprets tag meaning — your game defines semantics.
- **`voice_id`** optionally associates a voice with an entity. When `narrate()` is called, the first participant's voice is used by default.
- **Properties** are key-value pairs accessible via `{entity.property_key}` in grammar templates.

```rust
// Entity with custom properties
Entity {
    id: EntityId(1),
    name: "Margaret".to_string(),
    tags: ["host", "anxious", "wealthy"].into_iter().map(String::from).collect(),
    relationships: Vec::new(),
    voice_id: Some(VoiceId(100)),
    properties: HashMap::from([
        ("title".to_string(), Value::String("Lady".to_string())),
        ("age".to_string(), Value::Int(45)),
    ]),
}
```

**Relationships** connect entities:

```rust
pub struct Relationship {
    pub source: EntityId,
    pub target: EntityId,
    pub rel_type: String,     // "spouse", "rival", "ally", etc.
    pub intensity: f32,        // 0.0..=1.0
    pub tags: FxHashSet<String>,
}
```

### Voices

A voice is a data bundle (not code) that shapes how generated text sounds. Voices don't generate text — they parameterize the generation process.

A voice controls:

- **Grammar weights**: Multipliers on specific rule alternatives (e.g., a "gossip" voice might weight `social_observation` rules 2.5x higher).
- **Vocabulary pool**: Preferred words to favor and avoided words to replace via synonym rotation.
- **Markov bindings**: Which trained corpora the voice draws from and with what blend weights.
- **Structure preferences**: Target sentence length, clause complexity, question frequency.
- **Quirks**: Verbal tics or phrases (e.g., "if you know what I mean") injected at a configurable frequency.

Voices support **inheritance** — a child voice inherits from a parent and overrides specific settings.

### Grammars

Grammars are the heart of text generation. A `GrammarSet` is a collection of named `GrammarRule`s, each with:

- **`requires`**: Tags that must ALL be present in the current context (AND logic).
- **`excludes`**: Tags that must NONE be present.
- **`alternatives`**: Weighted text templates. One is chosen stochastically.

Grammar rules are matched against the current context (mood, stakes, narrative function, entity tags) and expanded recursively. Rules reference other rules, creating a tree of expansions.

### Markov Models

The Markov layer adds texture on top of grammar-generated text. It is optional — the grammar engine alone produces usable output.

Markov models are trained offline from plain text corpora using the `corpus_trainer` tool. At runtime, `{markov:corpus_id:tag}` template segments delegate to the trained model, which generates a phrase by walking n-gram probability chains.

Models support **tagged regions** — different sections of the corpus can be tagged (e.g., `[tense]`, `[warm]`) so generation can be filtered by mood or context.

Multiple models can be **blended** at runtime, mixing distributions from different corpora.

### Context and Variety

The engine maintains a sliding window of recently generated passages (default: 10) and uses it to:

1. **Detect repetition**: Repeated sentence openings, overused significant words, structural monotony (uniform sentence lengths).
2. **Remediate issues**: Swap opening words, replace overused words with synonyms, vary sentence length by splitting or combining clauses.
3. **Inject variety**: Rotate avoided vocabulary to synonyms, inject voice quirks at natural insertion points.

If the engine detects repetition issues in a generated passage, it automatically retries (up to 3 times) with a different seed offset.

---

## API Reference

### NarrativeEngine Builder

The engine is constructed with a fluent builder pattern:

```rust
let mut engine = NarrativeEngine::builder()
    // Load genre templates by name (from genre_data/ directory)
    .genre_templates(&["social_drama", "survival_thriller"])

    // Load game-specific grammars (override genre templates)
    .grammars_dir("my_game/grammars")

    // Load game-specific voices
    .voices_dir("my_game/voices")

    // Load pre-trained Markov models
    .markov_models_dir("my_game/trained_models")

    // Load event-type-to-narrative-function mappings
    .mappings("my_game/event_mappings.ron")

    // Set the deterministic seed
    .seed(12345)

    // Build the engine
    .build()?;
```

For testing or programmatic setup, you can provide components directly instead of loading from files:

```rust
let mut engine = NarrativeEngine::builder()
    .seed(42)
    .with_grammars(my_grammar_set)
    .with_voices(my_voice_registry)
    .with_markov_models(my_models)
    .with_mappings(my_mappings)
    .build()?;
```

**Builder methods:**

| Method | Description |
|---|---|
| `.genre_templates(&[&str])` | Load shipped genre template data by name |
| `.grammars_dir(path)` | Load grammar RON files from a directory |
| `.voices_dir(path)` | Load voice RON files from a directory |
| `.markov_models_dir(path)` | Load pre-trained Markov model files from a directory |
| `.mappings(path)` | Load event-to-narrative-function mapping file |
| `.seed(u64)` | Set the deterministic RNG seed |
| `.with_grammars(GrammarSet)` | Provide a pre-built GrammarSet directly |
| `.with_voices(VoiceRegistry)` | Provide a pre-built VoiceRegistry directly |
| `.with_markov_models(HashMap)` | Provide Markov models directly |
| `.with_mappings(HashMap)` | Provide event mappings directly |
| `.build()` | Construct the engine, returns `Result<NarrativeEngine, PipelineError>` |

### Narration Methods

```rust
// Generate text using the first participant's voice
let text = engine.narrate(&event, &world)?;

// Generate text using a specific voice
let text = engine.narrate_as(&event, VoiceId(101), &world)?;

// Generate multiple distinct variants for the same event
let variants = engine.narrate_variants(&event, 5, &world)?;
```

**`narrate(&event, &world)`**: The primary method. Uses the first participant's `voice_id` (or a default narrator voice if none is set). Returns a single generated passage.

**`narrate_as(&event, voice_id, &world)`**: Same as `narrate`, but forces a specific voice regardless of participant voice bindings.

**`narrate_variants(&event, count, &world)`**: Generates `count` distinct passages for the same event. Each variant uses a different seed offset, producing different text. Useful for giving the player choices or for A/B testing content.

### WorldState

The `WorldState` struct provides the engine access to your game's entity data:

```rust
pub struct WorldState<'a> {
    pub entities: &'a HashMap<EntityId, Entity>,
}
```

The engine borrows entity data for the duration of a `narrate` call. It looks up participants and locations by their `EntityId` to resolve template interpolations (`{entity.name}`, `{subject}`, etc.).

### Error Handling

All public methods return `Result<T, PipelineError>`. The error type covers all failure modes:

```rust
pub enum PipelineError {
    Grammar(GrammarError),     // Rule not found, max depth exceeded, etc.
    Voice(VoiceError),         // Voice not found in registry
    Markov(MarkovError),       // Markov generation failure
    Io(std::io::Error),        // File loading errors
    Ron(ron::error::SpannedError), // RON parsing errors
    EntityNotFound(EntityId),  // Referenced entity missing from WorldState
    NoRuleForFunction(String), // No grammar rule for this narrative function
    GenerationFailed(u32),     // All retry attempts exhausted
}
```

---

## Content Authoring

### Grammar Files (RON)

Grammar files use RON (Rusty Object Notation) format. Each file defines a map of rule names to rule definitions:

```ron
{
    "rule_name": Rule(
        requires: ["tag1", "tag2"],  // ALL must match context (AND)
        excludes: ["tag3"],          // NONE may match context
        alternatives: [
            (weight: 3, text: "Most common variant with {rule_ref} expansion"),
            (weight: 2, text: "Less common variant referencing {entity.name}"),
            (weight: 1, text: "Rare variant"),
        ],
    ),
}
```

**Rule naming conventions:**

- Narrative function entry points: `{fn_name}_opening` (e.g., `confrontation_opening`)
- Body and closing rules: `{fn_name}_body`, `{fn_name}_closing`
- Supporting rules: `body_language`, `emotional_reaction`, `dialogue_tag`, etc.

The engine looks for `{fn_name}_opening` as the entry rule when generating for a narrative function. If not found, it falls back to `{fn_name}`.

**Tag conventions in `requires`/`excludes`:**

Tags injected automatically by the pipeline:
- `fn:{function_name}` — e.g., `fn:confrontation`, `fn:revelation`
- `mood:{mood}` — e.g., `mood:tense`, `mood:warm`
- `stakes:{level}` — e.g., `stakes:high`, `stakes:critical`
- `intensity:{level}` — `intensity:high` when function intensity > 0.7, `intensity:low` when < 0.3
- Entity tags from all participants and the location

**Example: Social Drama grammar rules**

```ron
{
    "confrontation_opening": Rule(
        requires: ["fn:confrontation"],
        excludes: [],
        alternatives: [
            (weight: 3, text: "{subject} turned to face {object} directly. {deliberate_action}"),
            (weight: 3, text: "The pretense of civility dissolved. {subject} stepped closer. {body_language}"),
            (weight: 2, text: "{dialogue_tag}, {subject} said, \"We need to discuss this.\" {emotional_reaction}"),
            (weight: 2, text: "It had been building all evening. {subject} could hold back no longer. {social_observation}"),
            (weight: 1, text: "{room_detail} {subject} squared {possessive} shoulders and spoke."),
        ],
    ),

    "body_language": Rule(
        requires: [],
        excludes: [],
        alternatives: [
            (weight: 3, text: "{subject}'s jaw tightened almost imperceptibly."),
            (weight: 2, text: "{subject} clasped {possessive} hands together."),
            (weight: 2, text: "A muscle twitched near {subject}'s eye."),
            (weight: 2, text: "{subject} leaned back, creating distance."),
            (weight: 1, text: "{subject}'s fingers drummed a quiet rhythm on the armrest."),
        ],
    ),
}
```

### Template Syntax

Inside the `text` field of an alternative, these expansion markers are available:

| Syntax | Expands to | Example |
|---|---|---|
| `{rule_name}` | Recursively expand another grammar rule | `{body_language}` |
| `{markov:corpus_id:tag}` | Generate phrase from Markov model | `{markov:social_drama:tense}` |
| `{entity.name}` | Subject entity's name | `Margaret` |
| `{entity.field}` | Subject entity's property value | `{entity.title}` → `Lady` |
| `{subject}` | Subject entity's name (pronoun-aware) | `Margaret` |
| `{object}` | Object entity's name | `James` |
| `{possessive}` | Subject's possessive form | `Margaret's` |
| `{{` | Literal `{` | |
| `}}` | Literal `}` | |

**Entity bindings:** The `subject` role maps to the first participant with `role: "subject"`. The `object` role maps to the first participant with `role: "object"`. Other roles can be defined but are referenced by entity lookup.

### Voice Files (RON)

Voice files define an array of voice definitions:

```ron
[
    (
        id: VoiceId(100),
        name: "host",
        parent: None,
        grammar_weights: {
            "deliberate_action": 2.0,   // 2x more likely to select this rule
            "room_detail": 1.5,
            "emotional_reaction": 0.5,  // Half as likely
        },
        vocabulary: (
            preferred: ["indeed", "naturally", "of course"],
            avoided: ["gonna", "wanna", "stuff"],
        ),
        markov_bindings: [
            (corpus_id: "social_drama", weight: 1.0, tags: ["formal"]),
        ],
        structure_prefs: (
            avg_sentence_length: (12, 22),  // Target word count range
            clause_complexity: 0.8,         // 0.0 simple, 1.0 complex
            question_frequency: 0.1,        // 10% chance of questions
        ),
        quirks: [
            (pattern: "naturally", frequency: 0.08),       // 8% chance per passage
            (pattern: "one would think", frequency: 0.05), // 5% chance per passage
        ],
    ),
]
```

**Voice fields:**

| Field | Type | Description |
|---|---|---|
| `id` | `VoiceId(u64)` | Unique identifier |
| `name` | `String` | Human-readable label |
| `parent` | `Option<VoiceId>` | Voice to inherit from |
| `grammar_weights` | `{rule_name: f32}` | Multipliers on rule alternative weights |
| `vocabulary.preferred` | `[String]` | Words favored by this voice |
| `vocabulary.avoided` | `[String]` | Words replaced by synonyms |
| `markov_bindings` | `[MarkovBinding]` | Which corpora to draw from |
| `structure_prefs` | `StructurePrefs` | Sentence length and complexity targets |
| `quirks` | `[Quirk]` | Verbal tics injected at configurable frequency |

**Grammar weights** are the primary mechanism for making voices sound different without changing the grammar itself. A `gossip` voice might set `social_observation: 2.5` to heavily favor gossip-flavored rules, while a `host` voice sets `deliberate_action: 2.0` for more controlled, purposeful descriptions.

### Markov Corpora

Markov training corpora are plain text files with optional tag annotations:

```text
[neutral]
The afternoon sun cast long shadows across the garden path.
Birds sang from the hedgerows, indifferent to the drama unfolding indoors.

[tense]
The silence was suffocating. Every heartbeat felt like an accusation.
Words hung in the air, sharp and dangerous, waiting to draw blood.

[warm]
She reached across the table, a gesture of reconciliation.
The fire crackled softly, wrapping the room in amber light.
```

Lines prefixed with `[tag]` mark all subsequent text with that tag until the next tag or end of file. The engine can then request tag-filtered generation (e.g., "generate a phrase using only `[tense]` training data").

**Tips for writing corpora:**
- Write 40-60+ sentences per genre for meaningful Markov chains.
- Use 2-4 tags per corpus that align with your `Mood` values.
- Vary sentence length and structure within each tag section.
- Public domain fiction is a reasonable starting point, but game-specific prose produces better results.

### Event Mappings

If your game's events don't directly specify a `narrative_fn`, you can define a mapping file that associates event types with narrative functions:

```ron
[
    (event_type: "accusation", narrative_fn: Confrontation),
    (event_type: "confession", narrative_fn: Revelation),
    (event_type: "power_shift", narrative_fn: StatusChange),
    (event_type: "comic_moment", narrative_fn: ComicRelief),
]
```

The engine checks the event's `narrative_fn` field first. If present, it uses that directly. If the event relies on the mapping table, the engine looks up `event_type` in the loaded mappings.

---

## Genre Templates

Genre templates are shipped data packages (grammar rules, voices, corpora) that provide sensible defaults for common narrative genres. They dramatically reduce the authoring burden — you can build on them rather than starting from scratch.

Templates live in the `genre_data/` directory and include:

- `grammar.ron` — Genre-specific grammar rules
- `voices.ron` — Genre-appropriate voice definitions
- `corpus.txt` — Training text for Markov models

### Social Drama

**Focus:** Interpersonal dynamics, social maneuvering, emotional undercurrents.

**Narrative functions covered:** Revelation, Confrontation, Betrayal, Alliance, ComicRelief.

**Shipped voices:**

| Voice | ID | Personality |
|---|---|---|
| `host` | `VoiceId(100)` | Formal, composed, controlling. Long sentences, high complexity. |
| `gossip` | `VoiceId(101)` | Informal, excitable, observant. Short sentences, lots of questions. |
| `peacemaker` | `VoiceId(102)` | Gentle, diplomatic, indirect. Medium sentences, moderate complexity. |
| `provocateur` | `VoiceId(103)` | Sharp, blunt, enjoys tension. Short punchy sentences. |

**Supporting rules:** `deliberate_action`, `emotional_reaction`, `body_language`, `dialogue_tag`, `social_observation`, `room_detail`.

**Corpus tags:** `[tense]`, `[warm]`, `[gossip]`, `[formal]`.

### Survival Thriller

**Focus:** Environment, threat, and survival under pressure.

**Narrative functions covered:** Escalation, Discovery, Loss, Foreshadowing, StatusChange.

**Shipped voices:**

| Voice | ID | Personality |
|---|---|---|
| `radio_operator` | `VoiceId(200)` | Clipped, procedural, abbreviations. Very short sentences. |
| `survivor` | `VoiceId(201)` | Fragmented, sensory, emotional. Variable length. |
| `scientist` | `VoiceId(202)` | Analytical, precise, detached. Medium-long sentences. |
| `narrator_omniscient` | `VoiceId(203)` | Atmospheric, ominous, measured. Long sentences, high complexity. |

**Supporting rules:** `environmental_detail`, `threat_proximity`, `resource_status`, `sensory_detail`, `technical_readout`, `urgency_marker`.

**Corpus tags:** `[dread]`, `[urgent]`, `[calm_before]`, `[technical]`.

### Political Intrigue

**Focus:** Power dynamics, information asymmetry, rhetoric.

Placeholder template — extend with your own rules.

### Exploration

**Focus:** Discovery, wonder, descriptive richness.

Placeholder template — extend with your own rules.

### Blending Templates

You can load multiple genre templates simultaneously. The engine merges their grammar rule sets — rules with the same name are overridden by the later template, while unique rules coexist:

```rust
let mut grammars = GrammarSet::load_from_ron(
    Path::new("genre_data/social_drama/grammar.ron")
)?;
let thriller_grammars = GrammarSet::load_from_ron(
    Path::new("genre_data/survival_thriller/grammar.ron")
)?;
grammars.merge(thriller_grammars);
```

This enables scenarios like a tense interpersonal confrontation during a survival scenario, drawing rules from both templates depending on the event's tags.

---

## Tools

### Grammar Linter

Validates grammar coverage and rule quality.

```bash
# Lint all genre templates
cargo run --bin grammar_linter -- genre_data/

# Lint with Markov model validation
cargo run --bin grammar_linter -- genre_data/ --models-dir models/
```

**Checks performed:**

| Check | Severity | Description |
|---|---|---|
| Coverage gaps | Error | NarrativeFunction x Mood x Stakes combinations with no matching rule |
| Low variety | Warning | Rules with fewer than 3 alternatives |
| Unreachable rules | Error | Rules whose `requires` tags are never produced by any combination |
| Circular references | Error | Rule reference cycles without a base case |
| Missing corpora | Warning | `{markov:corpus_id:tag}` referencing non-existent corpus IDs |
| Template parse errors | Error | Invalid syntax in rule text templates |

**Exit codes:** 0 if no errors (warnings are acceptable), 1 if any errors found.

Run the linter after every grammar change to catch coverage gaps early. A grammar that doesn't cover all function x mood x stakes combinations will produce empty output for some events at runtime.

### Corpus Trainer

Trains Markov models from plain text corpora.

```bash
cargo run --bin corpus_trainer -- \
    --input genre_data/social_drama/corpus.txt \
    --output trained/social_drama.ron \
    --ngram 3
```

**Arguments:**

| Flag | Description |
|---|---|
| `--input <file>` | Plain text corpus file (with optional `[tag]` annotations) |
| `--output <file>` | Output file for the trained model |
| `--ngram <2\|3\|4>` | N-gram depth. 2 = bigram, 3 = trigram, 4 = 4-gram |

**N-gram depth guidance:**
- **2 (bigram):** Fast, low memory, more random output. Good for short phrases.
- **3 (trigram):** Best general-purpose balance of coherence and variety. Recommended default.
- **4 (4-gram):** More coherent but more repetitive. Needs larger corpora (100+ sentences) to avoid regurgitating source text.

### Preview Shell

Interactive REPL for testing grammar expansion and Markov generation.

```bash
cargo run --bin preview -- \
    --grammars genre_data/social_drama \
    --voices genre_data/social_drama/voices.ron \
    --seed 42
```

**Commands:**

| Command | Description |
|---|---|
| `event <fn> <mood> <stakes>` | Generate from a synthetic event. E.g., `event confrontation tense high` |
| `voice <name>` | Set the active voice. E.g., `voice gossip` |
| `entity <name> <tag1,tag2>` | Define a named entity. E.g., `entity Margaret host,anxious` |
| `seed <n>` | Change the RNG seed |
| `bulk <n>` | Generate n passages and print variety statistics |
| `help` | List all commands |
| `quit` | Exit the shell |

After each generation, the preview tool prints:
- The expanded text
- An expansion trace showing which rules were selected at each step (useful for debugging grammars)

The `bulk` command is particularly useful for evaluating variety — it generates many passages and reports unique openings, word frequency distribution, and average length.

---

## End-to-End Examples

### Dinner Party (Social Drama)

The `dinner_party` example demonstrates a complete mini-narrative using the Social Drama template.

```bash
cargo run --example dinner_party
```

**Setup:** 5 entities (Margaret the host, James her secretive husband, Eleanor the sharp-tongued gossip, Robert the peacemaker, and the dining room). Each character has a distinct voice.

**Story arc across 7 scenes:**

1. **Small Talk** (Alliance, Warm, Low) — Margaret and Robert
2. **A Whispered Aside** (Alliance, Neutral, Medium) — Eleanor and Robert
3. **Tension Builds** (Confrontation, Tense, Medium) — Eleanor and Margaret
4. **The Accusation** (Confrontation, Tense, High) — Eleanor and James
5. **The Revelation** (Revelation, Somber, Critical) — James and Margaret
6. **The Aftermath** (ComicRelief, Neutral, Low) — Robert and Eleanor
7. **The Betrayal** (Betrayal, Somber, Critical) — Margaret and James

**Key pattern:** Each scene creates an `Event` with appropriate mood/stakes/narrative_fn, then calls `engine.narrate(&event, &world)`. The engine selects the right grammar rules based on tags and generates contextually appropriate text.

```rust
let event = Event {
    event_type: "accusation".to_string(),
    participants: vec![
        EntityRef { entity_id: EntityId(3), role: "subject".to_string() },
        EntityRef { entity_id: EntityId(2), role: "object".to_string() },
    ],
    location: Some(EntityRef {
        entity_id: EntityId(100),
        role: "location".to_string(),
    }),
    mood: Mood::Tense,
    stakes: Stakes::High,
    outcome: None,
    narrative_fn: NarrativeFunction::Confrontation,
    metadata: HashMap::new(),
};

let text = engine.narrate(&event, &world)?;
```

### Jurassic Park (Survival Thriller)

The `jurassic_park` example validates that the same engine with different content produces a completely different genre feel.

```bash
cargo run --example jurassic_park
```

**Setup:** 7 entities (Dr. Grant, Dr. Malcolm, Muldoon, Control Room, Rex Paddock, Raptor Pen, Security System). Uses `radio_operator` and `narrator_omniscient` voices to alternate between terse status reports and atmospheric narration.

**Story arc across 7 scenes:**

1. **Morning Status Report** (StatusChange, Neutral, Low) — radio_operator voice
2. **Power Fluctuation** (Foreshadowing, Neutral, Medium) — narrator voice
3. **Perimeter Breach** (Escalation, Dread, High) — radio_operator voice
4. **Multiple System Failures** (Escalation, Chaotic, Critical) — narrator voice
5. **Raptor Pen Discovery** (Discovery, Dread, High) — Dr. Grant's scientist voice
6. **Critical Failure** (Loss, Somber, Critical) — radio_operator voice
7. **Final Log Entry** (Loss, Dread, Critical) — narrator voice

**Key pattern:** Uses `narrate_as()` to override voices per scene, alternating between the clipped `radio_operator` and atmospheric `narrator_omniscient`:

```rust
// Terse radio report
let text = engine.narrate_as(&event, VoiceId(200), &world)?;

// Atmospheric narration
let text = engine.narrate_as(&event, VoiceId(203), &world)?;
```

---

## Determinism and Seeding

All randomness in the engine flows through a single seedable `StdRng`. This guarantees:

- **Given the same seed, world state, and event, output is identical.** This is critical for replay, testing, multiplayer sync, and debugging.
- **The engine never uses `thread_rng()` or any non-deterministic source.**
- **Each call to `narrate()` advances the RNG state deterministically**, using `seed + generation_count` to derive the per-call seed.

```rust
// These two engines will produce identical output
let mut engine_a = NarrativeEngine::builder().seed(42).with_grammars(g.clone()).build()?;
let mut engine_b = NarrativeEngine::builder().seed(42).with_grammars(g.clone()).build()?;

assert_eq!(
    engine_a.narrate(&event, &world)?,
    engine_b.narrate(&event, &world)?
);
```

**Seed selection:** Use your game's world seed, or a hash of the current game state, or any `u64`. Different seeds produce different text for the same event.

**`narrate_variants()`** generates multiple outputs by using sequential seed offsets, so each variant is deterministically different but reproducible.

---

## Performance

The engine targets **<1ms per passage** on commodity hardware. Key performance characteristics:

- **Grammar rule matching is the hot path.** Tag intersection uses `FxHashSet` (from `rustc-hash`) for fast lookups.
- **Markov models should be pre-loaded.** Use the `corpus_trainer` tool to train models offline. Models are loaded once at engine construction and reused.
- **Allocations during generation are minimized.** The grammar expansion system pre-allocates buffers where possible.
- **No runtime dependencies on game engines.** The library is pure computation — no I/O, no networking, no threads during generation.

**Scaling guidelines:**
- Grammar sets with hundreds of rules perform well.
- Keep Markov model n-gram depth at 3 unless you have a specific reason to go higher.
- The context window (default 10 passages) has negligible overhead.

---

## Extending the Engine

### Custom Narrative Functions

Use `NarrativeFunction::Custom(String)` for game-specific narrative beats:

```rust
let event = Event {
    narrative_fn: NarrativeFunction::Custom("coronation".to_string()),
    // ...
};
```

Then create grammar rules that match:

```ron
"coronation_opening": Rule(
    requires: ["fn:coronation"],
    excludes: [],
    alternatives: [
        (weight: 3, text: "The crown was placed upon {subject}'s head. {ceremony_detail}"),
        // ...
    ],
),
```

Custom functions default to neutral pacing (0.5), valence (0.0), and intensity (0.5).

### Custom Genre Templates

Create a new genre template by adding a directory under `genre_data/`:

```
genre_data/
  my_genre/
    grammar.ron    # Grammar rules for your genre
    voices.ron     # Voice definitions
    corpus.txt     # Training text for Markov models
```

Then load it:

```rust
let grammars = GrammarSet::load_from_ron(
    Path::new("genre_data/my_genre/grammar.ron")
)?;
```

**Minimum viable grammar:** Define at least `{fn_name}_opening` rules for each narrative function you plan to use, plus a handful of supporting rules. Run the grammar linter to identify coverage gaps.

### Voice Inheritance

Create specialized voices that inherit from a base:

```ron
[
    // Base narrator voice
    (
        id: VoiceId(1),
        name: "narrator",
        parent: None,
        grammar_weights: { "room_detail": 1.5 },
        vocabulary: ( preferred: ["the"], avoided: [] ),
        markov_bindings: [],
        structure_prefs: (
            avg_sentence_length: (10, 20),
            clause_complexity: 0.6,
            question_frequency: 0.05,
        ),
        quirks: [],
    ),
    // Child: dramatic narrator
    (
        id: VoiceId(2),
        name: "dramatic_narrator",
        parent: Some(VoiceId(1)),  // Inherits from narrator
        grammar_weights: {
            "emotional_reaction": 2.0,  // Override: more dramatic
        },
        // Inherits everything else from parent
        vocabulary: ( preferred: ["alas", "behold"], avoided: [] ),
        markov_bindings: [],
        structure_prefs: (
            avg_sentence_length: (14, 28),  // Override: longer sentences
            clause_complexity: 0.9,
            question_frequency: 0.05,
        ),
        quirks: [
            (pattern: "fate would have it", frequency: 0.1),
        ],
    ),
]
```

**Inheritance resolution rules:**
- `grammar_weights`: Child overrides parent on matching keys; parent keys not overridden are preserved.
- `vocabulary`: Preferred and avoided sets are unioned.
- `markov_bindings`: Concatenated (child's bindings added to parent's).
- `structure_prefs`: Child values used; falls back to parent if not specified.
- `quirks`: Concatenated.

### Grammar Override Layering

The `GrammarSet::merge()` method enables layered grammar authoring:

```rust
// 1. Load genre template (base layer)
let mut grammars = GrammarSet::load_from_ron(
    Path::new("genre_data/social_drama/grammar.ron")
)?;

// 2. Merge game-specific overrides (override layer)
let game_grammars = GrammarSet::load_from_ron(
    Path::new("my_game/grammars/custom_rules.ron")
)?;
grammars.merge(game_grammars);
// Rules in game_grammars override same-named rules from social_drama.
// New rules are added to the set.
```

This layering lets you:
- Start with a genre template for broad coverage.
- Override specific rules with game-flavored alternatives.
- Add entirely new rules for game-specific narrative functions.

Rules in the later (merged) set take priority over same-named rules in the base set.
