# Narrative Engine

**A Reusable Library for Procedural Text Generation in Games**

Version 0.1 — February 2026 | Status: Draft

---

## 1. Overview

### 1.1 Problem Statement

Games with procedurally generated narratives face a fundamental tension: LLMs produce fresh, contextually responsive text but are too large, too slow, and too expensive to embed in a shipped game. Template systems are fast and controllable but feel repetitive after extended play. There is no widely available middleware that occupies the space between these extremes.

### 1.2 Vision

Narrative Engine is a Rust library (crate) that generates thematically appropriate, contextually aware text at runtime without neural network inference. It produces text that feels written rather than assembled, using a pipeline of simulation-driven events, stochastic grammars, and Markov-trained phrase generation. The library is designed to be reusable across radically different game genres.

### 1.3 Design Principles

- **Separation of engine from content:** the runtime is genre-agnostic; all thematic flavor lives in data files.
- **Narrative function over narrative content:** the engine understands abstract concepts like "revelation" and "escalation," not specific plot points.
- **Deterministic reproducibility:** all generation is seeded, enabling replay, testing, and debugging.
- **Zero runtime dependencies** beyond the Rust standard library and the game's own entity data.
- **Offline tooling** for corpus training and grammar validation ships alongside the runtime.

### 1.4 Target Applications

The library is designed to serve games across a spectrum of genres. The following examples illustrate the range:

| Application | Genre Blend | Primary Text Output |
|---|---|---|
| Dinner Party Sim | Social Drama | Dialogue, internal monologue, gossip |
| Promenade | Exploration + Social | Environmental narration, NPC commentary |
| Game of Thrones Sim | Political Intrigue + Social Drama | Letters, scheming dialogue, proclamations |
| Dino Park Sim | Survival Thriller | Status reports, radio chatter, warning signs |
| Battlestar Galactica Sim | Survival Thriller + Political Intrigue | Military comms, political speeches, logs |

---

## 2. Architecture

### 2.1 Pipeline Overview

Text generation follows a linear pipeline with feedback loops for quality control:

> **Generation Pipeline**
>
> Game Simulation → Structured Event → Narrative Function Mapping → Voice Selection → Grammar Expansion → Markov Phrase Fill → Variety Pass → Context Check → Output Text

Each stage is independently testable and replaceable. A game can bypass stages it doesn't need — a game with hand-authored events can skip the simulation layer entirely and feed structured events directly into the pipeline.

### 2.2 Module Map

```
narrative-engine/
├── core/
│   ├── pipeline.rs        # Event → Text orchestration
│   ├── grammar.rs         # Stochastic grammar runtime
│   ├── markov.rs          # Trainable phrase generator
│   ├── voice.rs           # Persona/tone bundles
│   ├── context.rs         # Anti-repetition, pronoun tracking
│   └── variety.rs         # Clause reordering, synonym rotation
├── schema/
│   ├── event.rs           # Universal event structure
│   ├── entity.rs          # Universal entity structure
│   ├── narrative_fn.rs    # Narrative function taxonomy
│   └── relationship.rs    # Entity-to-entity connections
├── genre_templates/
│   ├── social_drama/
│   ├── survival_thriller/
│   ├── political_intrigue/
│   └── exploration/
└── tools/
    ├── corpus_trainer.rs  # Offline Markov training
    ├── grammar_linter.rs  # Validate rule coverage
    └── preview.rs         # Interactive test shell
```

---

## 3. Core Systems

### 3.1 Schema Layer

#### 3.1.1 Entity

An Entity is anything that can participate in a narrative event: a person, creature, place, object, or abstract concept. The engine does not interpret tag semantics — it uses tags solely for grammar rule matching.

```rust
Entity {
    id: EntityId,
    name: String,
    tags: HashSet<String>,
    relationships: Vec<Relationship>,
    voice_id: Option<VoiceId>,
    properties: HashMap<String, Value>,
}
```

Tags are free-form strings defined by the game. Example tags for a dinner party character: `["host", "anxious", "hiding-secret", "wealthy"]`. Example tags for a Dino Park location: `["location", "paddock", "compromised", "high-danger"]`.

#### 3.1.2 Event

An Event is a structured record of something that happened in the game simulation. Events are the sole input to the narrative pipeline.

```rust
Event {
    event_type: String,
    participants: Vec<EntityRef>,
    location: Option<EntityRef>,
    mood: Mood,
    stakes: Stakes,
    outcome: Option<Outcome>,
    narrative_fn: NarrativeFunction,
    metadata: HashMap<String, Value>,
}
```

The `event_type` is game-defined (e.g., "accusation", "breach", "discovery"). The `narrative_fn` is engine-defined and maps the game event to an abstract narrative function (see Section 3.2).

#### 3.1.3 Relationship

Relationships are typed, directional edges between entities with a numerical intensity value. The engine uses these to select appropriate language (formal vs. intimate, hostile vs. friendly) without understanding the game's specific relationship semantics.

```rust
Relationship {
    source: EntityId,
    target: EntityId,
    rel_type: String,       // game-defined: "rival", "lover", "predator"
    intensity: f32,          // 0.0..1.0
    tags: HashSet<String>,   // "secret", "one-sided", "deteriorating"
}
```

### 3.2 Narrative Function Taxonomy

> **Key Insight**
>
> Narrative function is the most important abstraction in the engine. It separates WHAT narratively is happening (a secret being revealed, tension escalating) from HOW it's expressed in a specific genre. This is the layer that makes the engine reusable across games.

Every game event maps to one or more narrative functions. The engine ships with a core taxonomy that covers the most common narrative beats:

| Function | Description | Properties |
|---|---|---|
| Revelation | Hidden information becomes known | pacing, audience (who learns), weight |
| Escalation | Stakes or tension increase | from_level, to_level, trigger |
| Confrontation | Two entities in direct opposition | power_balance, witnesses, subtext |
| Betrayal | Trust is violated | depth, premeditation, visibility |
| Alliance | Entities align interests | sincerity, conditionality, public/private |
| Discovery | Something new is found or understood | significance, danger, wonder |
| Loss | Something valued is taken or destroyed | permanence, agency, grief_level |
| Comic Relief | Tension is broken with humor | darkness, timing, source |
| Foreshadowing | Future events are hinted at | subtlety, dread, misdirection |
| Status Change | An entity's position shifts | direction (up/down), domain, permanence |

Games can extend this taxonomy with custom narrative functions. The engine requires only that each function declare its rendering properties (pacing, valence, intensity) so the grammar system knows how to shape the output.

### 3.3 Stochastic Grammar Engine

#### 3.3.1 Grammar Rule Format

Grammars are defined in data files (RON, JSON, or a custom DSL) as sets of named rules. Each rule has weighted alternatives that can reference other rules recursively. Rules can carry tag-based preconditions that restrict when they fire.

```ron
// Example grammar rule (RON format)
"accusation_opening": Rule(
    requires: ["mood:tense"],
    alternatives: [
        (weight: 3, text: "{subject} set down {possessive} {held_item}"),
        (weight: 2, text: "{subject} turned slowly"),
        (weight: 1, text: "The room fell silent as {subject} stood"),
    ]
)
```

#### 3.3.2 Context-Aware Selection

The grammar engine maintains a selection context that accumulates as rules expand. Earlier expansions constrain later ones through tag propagation. If a character is tagged "formal," sentence structures that use contractions are suppressed. If the mood is "urgent," long subordinate clauses are penalized. This produces text where tone is consistent within a passage without requiring the game author to write separate grammars for every combination.

#### 3.3.3 Genre Template Inheritance

Genre templates provide a base set of grammar rules for common narrative functions. A game's grammar can extend, override, or disable rules from one or more genre templates. The inheritance model is simple: game-specific rules take precedence; genre rules provide fallbacks. A game can blend templates — loading both "social_drama" and "political_intrigue" creates a rule set that covers both interpersonal scenes and power-play scenes.

### 3.4 Markov Phrase Generator

#### 3.4.1 Corpus Training

Corpora are plain text files organized by theme, voice, or genre. The offline training tool processes these into serialized n-gram probability tables. Multiple corpora can be trained independently, and the runtime can blend outputs from several trained models with configurable weights.

The trainer supports configurable n-gram depth (recommended: 2–3 for phrase fragments, 3–4 for longer passages), boundary markers to prevent generation from crossing sentence or paragraph boundaries, and tag annotations in the source corpus that allow the runtime to request phrases matching specific qualities (e.g., `[ominous]`, `[whimsical]`).

#### 3.4.2 Runtime Integration

The grammar engine delegates to the Markov generator for specific expansion slots. A grammar rule might produce the skeleton `"She looked at him and said {markov:dialogue:accusatory}"` where the Markov generator fills in a phrase trained on accusatory dialogue. The grammar provides structure; Markov provides texture.

### 3.5 Voice System

A Voice is a data bundle that shapes how text sounds for a specific speaker, narrator, or document type. Voices are composed of:

- **Grammar weight overrides:** shift probabilities toward certain sentence structures.
- **Vocabulary pools:** preferred and avoided words for this voice.
- **Markov corpus bindings:** which trained corpora this voice draws from.
- **Structural preferences:** average sentence length, clause complexity, question frequency.
- **Quirks:** recurring phrases, verbal tics, or speech patterns injected at low frequency.

Voices can inherit from other voices. A "Ship Captain" voice might inherit from a base "Military" voice and override formality settings and add nautical vocabulary.

### 3.6 Context and Variety System

#### 3.6.1 Anti-Repetition

The context system maintains a sliding window of recently generated text. Before finalizing output, the variety pass checks for repeated sentence openings, overused words, and structural monotony (e.g., three consecutive sentences of similar length). Violations trigger re-generation of the offending segment with increased randomness.

#### 3.6.2 Pronoun Tracking

The context system tracks which entities have been mentioned and whether they're the current subject. It enables pronoun substitution ("Margaret" becomes "she" on subsequent mentions within a passage) and prevents ambiguous references when multiple entities of the same apparent gender are in scope.

#### 3.6.3 Clause Reordering

For sentences with multiple independent clauses, the variety system can permute clause order to break structural patterns. "She crossed the room and picked up the glass" can become "Picking up the glass, she crossed the room" with no change in semantic content.

---

## 4. Data Flow

### 4.1 End-to-End Example: Dinner Party

This walkthrough shows how a single event flows through the complete pipeline.

**Step 1: Simulation Emits Event**

```rust
Event {
    event_type: "accusation",
    participants: [margaret, james],
    location: dining_room,
    mood: Mood::Tense,
    stakes: Stakes::High,
    narrative_fn: NarrativeFunction::Confrontation {
        power_balance: 0.6,  // margaret has advantage
        witnesses: [eleanor, robert],
        subtext: "affair"
    },
    metadata: { "held_item": "wine glass" }
}
```

**Step 2: Voice Selection**

The pipeline selects Margaret's voice (defined by the game) and applies it as the active voice for this generation. Margaret's voice favors measured, precise sentence structures with formal vocabulary.

**Step 3: Grammar Expansion**

The grammar engine matches rules tagged with `[mood:tense, fn:confrontation, voice:formal]`. A selected rule might produce the skeleton:

```
"{subject} {deliberate_action}. {markov:dialogue:accusatory}"
```

Which expands through further rule resolution to:

```
"Margaret set down her wine glass with deliberate care. {markov:dialogue:accusatory}"
```

**Step 4: Markov Fill**

The Markov generator, drawing from a corpus trained on tense social confrontation dialogue, produces a phrase fragment. The generator is constrained to produce a complete sentence of 8–18 words.

```
"Margaret set down her wine glass with deliberate care.
 I think we should discuss what you were doing in Kensington last Thursday."
```

**Step 5: Variety and Context Check**

The context system verifies this doesn't repeat recent patterns, applies pronoun resolution if needed, and clears the output for delivery.

### 4.2 End-to-End Example: Dino Park

The same pipeline, different content:

**Step 1: Event**

```rust
Event {
    event_type: "perimeter_breach",
    participants: [rex_paddock, security_system],
    mood: Mood::Dread,
    stakes: Stakes::Critical,
    narrative_fn: NarrativeFunction::Escalation {
        from_level: 0.3, to_level: 0.8, trigger: "power_failure"
    },
}
```

**Steps 2–5:**

Voice: "security_terminal" (terse, technical, impersonal). Grammar matches `[fn:escalation, voice:technical, mood:dread]`. Markov fills from a corpus of technical thriller prose. Output:

```
"PADDOCK 9 — PERIMETER STATUS: COMPROMISED. Fence voltage
 dropped below containment threshold at 02:47. No response
 from automated recovery. Manual override required."
```

---

## 5. Game Integration

### 5.1 What the Game Provides

A game integrating the Narrative Engine supplies the following data artifacts:

- **Entity definitions** with tags and relationships (runtime data from the game simulation).
- **Event-to-NarrativeFunction mappings:** a table or function that maps game-specific event types to the engine's narrative function taxonomy.
- **Voice definitions** for each speaking entity or narrator style.
- **Grammar extensions** that build on genre templates with game-specific rules.
- **Trained Markov corpora:** raw text files processed by the offline trainer into binary models shipped with the game.

### 5.2 File Structure Convention

```
my_game/
├── narrative/
│   ├── corpora/           # Raw .txt files for Markov training
│   │   ├── social_tension.txt
│   │   ├── gothic_atmosphere.txt
│   │   └── military_comms.txt
│   ├── grammars/          # .ron or .json grammar rules
│   │   ├── base.ron           # extends genre template
│   │   └── characters/
│   ├── voices/            # .ron voice definitions
│   ├── trained/           # Binary Markov models (generated offline)
│   └── mappings.ron       # event_type → NarrativeFunction table
```

### 5.3 API Surface

The public API is intentionally small:

```rust
// Initialize with game content
let engine = NarrativeEngine::builder()
    .genre_templates(&["social_drama", "political_intrigue"])
    .grammars_dir("narrative/grammars")
    .voices_dir("narrative/voices")
    .markov_models_dir("narrative/trained")
    .mappings("narrative/mappings.ron")
    .seed(world_seed)
    .build()?;

// Generate text from an event
let text = engine.narrate(&event, &world_state)?;

// Generate with specific voice override
let text = engine.narrate_as(&event, &voice_id, &world_state)?;

// Generate multiple variants for player choice
let options = engine.narrate_variants(&event, 3, &world_state)?;
```

---

## 6. Genre Templates

Genre templates are shipped starter grammars and voice presets that provide a foundation for specific genres. They are not required — a game can build grammars entirely from scratch — but they dramatically reduce the authoring burden.

### 6.1 Social Drama

Oriented around interpersonal dynamics. Sentence structures emphasize body language, subtext, dialogue beats, and emotional undercurrents. Vocabulary pools include social/emotional language. Default voices: host, guest, gossip, peacemaker, provocateur.

### 6.2 Survival Thriller

Oriented around environment and threat. Sentence structures emphasize sensory detail, spatial awareness, urgency, and resource status. Short, punchy sentences dominate at high tension; longer atmospheric sentences at low tension. Default voices: radio_operator, survivor, scientist, narrator_omniscient.

### 6.3 Political Intrigue

Oriented around power dynamics and information asymmetry. Sentence structures emphasize formality gradients, public vs. private speech, coded language, and rhetorical devices. Default voices: ruler, advisor, spy, populist, chronicler.

### 6.4 Exploration

Oriented around discovery and wonder. Sentence structures emphasize descriptive richness, comparison, scale, and novelty. Default voices: explorer, field_journal, local_guide, narrator_reflective.

### 6.5 Template Blending

When a game loads multiple genre templates, the engine merges their rule sets. Conflicts are resolved by specificity: a rule with more precondition tags wins over a more general rule. If two rules are equally specific, the game's own rules take priority, then the first-loaded genre template. This allows natural blending: a Game of Thrones sim loading both Social Drama and Political Intrigue gets rules for both feast scenes and throne room scenes without conflict.

---

## 7. Quality and Testing

### 7.1 Grammar Linter

The offline grammar linter verifies rule coverage: for each narrative function, mood, and stakes combination, does at least one grammar rule match? Unreachable rules (preconditions that can never be satisfied) are flagged. Rules with fewer than three alternatives are warned as low-variety.

### 7.2 Bulk Generation Testing

The preview tool generates thousands of outputs for a given event configuration and produces statistics: vocabulary diversity, sentence length distribution, repetition frequency, and structural variety. This enables data-driven tuning of grammar weights and Markov parameters before shipping.

### 7.3 Deterministic Seeding

All random selection is driven by a seedable PRNG. Given the same seed, world state, and event, the engine produces identical output. This enables regression testing, bug reproduction, and "favorite moment" replay in games that want that feature.

---

## 8. Scope and Limitations

### 8.1 What This Engine Is Not

- **Not an LLM.** It does not understand meaning, cannot reason about plot, and cannot answer questions about its own output.
- **Not a dialogue tree system.** It generates text for display, not interactive conversation branches.
- **Not a story planner.** It narrates events; it does not decide what should happen next. Story planning belongs in the game's simulation layer.

### 8.2 Known Limitations

- Long-range coherence across multiple generated passages requires the game to manage context and feed it back through the pipeline. The engine has no built-in memory beyond its sliding context window.
- Markov generation quality is bounded by corpus quality and size. Small or homogeneous corpora produce recognizably repetitive phrases after extended play.
- Grammar authoring has a learning curve. The system is powerful but requires investment in rule writing and testing. The genre templates mitigate this for common cases.

### 8.3 Future Considerations

- Optional integration with a small local language model (via llama.cpp bindings) as an alternative phrase generation backend for games that can tolerate the size and latency cost.
- A visual grammar authoring tool for non-programmer narrative designers.
- Localization support: grammar rules and corpora per language, with the engine handling structural differences between languages.
- A Wave Function Collapse text mode: defining text fragments as tiles with adjacency constraints for a different approach to generation that may complement or replace the Markov layer in some use cases.

---

## 9. Development Milestones

| Milestone | Deliverable | Validates |
|---|---|---|
| M0: Schema | Entity, Event, NarrativeFunction types compile | Data model is expressive enough |
| M1: Grammar Runtime | Stochastic grammar engine with tag-based rule matching | Rules expand correctly with context propagation |
| M2: Markov Layer | Corpus trainer + runtime phrase generator | Generated phrases are coherent at n-gram level |
| M3: Pipeline Integration | Event → Text pipeline with voice selection | End-to-end generation works for a single game |
| M4: Genre Templates | At least 2 genre templates with full rule sets | Templates are reusable across different game types |
| M5: Quality Tools | Grammar linter + bulk generation tester | Content quality is measurable and improvable |
| M6: Second Game | Port to a second, different-genre game | Engine is genuinely reusable, not overfitted |

> **Starting Point**
>
> M0 and M1 are the critical path. Get the schema right and the grammar engine working, then validate with hand-authored grammars for a single game before investing in the Markov layer. The grammar engine alone, with sufficient rule depth, can produce surprisingly good text.
