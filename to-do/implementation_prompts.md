# Narrative Engine — Implementation Prompts

A sequential series of prompts to build the narrative engine from the ground up. Each prompt corresponds roughly to a milestone from the design doc. Complete them in order — later prompts depend on earlier ones.

Read `CLAUDE.md` and `docs/design.md` before starting. Refer back to them throughout.

---

## Prompt 0: Project Scaffolding

> Set up the Rust project structure for the `narrative-engine` crate. Create `Cargo.toml` with dependencies: `serde`, `ron`, `rand`, `rustc-hash`, `thiserror`. Create the module hierarchy from CLAUDE.md with empty `mod.rs` files and placeholder structs. Add binary targets in `Cargo.toml` for `corpus_trainer`, `grammar_linter`, and `preview` under `tools/`. Create `genre_data/` directories for the four genre templates. Create `tests/fixtures/` with an empty test grammar and a small test corpus (5-10 sentences of generic prose). Everything should compile with `cargo build` and `cargo test` should pass (even if tests are trivial). Copy `docs/design.md` into the project.

---

## Prompt 1: Schema Layer

> Implement the schema types in `src/schema/`. These are the foundational data structures everything else builds on.
>
> **entity.rs:** `EntityId` newtype over `u64`. `Entity` struct with `id`, `name: String`, `tags: FxHashSet<String>`, `relationships: Vec<Relationship>`, `voice_id: Option<VoiceId>`, `properties: HashMap<String, Value>`. `Value` should be an enum covering `String`, `Float(f64)`, `Int(i64)`, `Bool`. Implement `Entity::has_tag(&self, tag: &str) -> bool` and `Entity::has_all_tags(&self, tags: &[&str]) -> bool`.
>
> **relationship.rs:** `Relationship` struct with `source: EntityId`, `target: EntityId`, `rel_type: String`, `intensity: f32` (clamped 0.0..=1.0), `tags: FxHashSet<String>`.
>
> **narrative_fn.rs:** `NarrativeFunction` enum with variants: `Revelation`, `Escalation`, `Confrontation`, `Betrayal`, `Alliance`, `Discovery`, `Loss`, `ComicRelief`, `Foreshadowing`, `StatusChange`, `Custom(String)`. Each variant carries its properties as described in the design doc's narrative function taxonomy table. Implement `NarrativeFunction::pacing(&self) -> f32`, `NarrativeFunction::valence(&self) -> f32`, and `NarrativeFunction::intensity(&self) -> f32` that return normalized values derived from the variant's properties.
>
> **event.rs:** `Mood` enum: `Neutral`, `Tense`, `Warm`, `Dread`, `Euphoric`, `Somber`, `Chaotic`, `Intimate`. `Stakes` enum: `Trivial`, `Low`, `Medium`, `High`, `Critical`. `Outcome` enum: `Success`, `Failure`, `Partial`, `Ambiguous`. `Event` struct as described in the design doc. `EntityRef` should be a lightweight struct holding `EntityId` plus a `role: String` (e.g., "subject", "target", "witness").
>
> All types: derive `Debug`, `Clone`, `Serialize`, `Deserialize`. Write unit tests for tag matching, narrative function property calculation, and entity construction. All tests pass with `cargo test`.

---

## Prompt 2: Grammar Data Model

> Implement the grammar data model in `src/core/grammar.rs`. This prompt is just the types and loading — not the expansion runtime yet.
>
> **Types:** `GrammarRule` has a `name: String`, `requires: Vec<String>` (tag preconditions), `excludes: Vec<String>`, and `alternatives: Vec<Alternative>`. `Alternative` has `weight: u32` and `template: Template`. `Template` is a parsed representation of a text pattern — a `Vec<TemplateSegment>` where `TemplateSegment` is an enum: `Literal(String)`, `RuleRef(String)`, `MarkovRef { corpus: String, tag: String }`, `EntityField { field: String }`, `PronounRef { role: String }`. `GrammarSet` is the top-level container: a `HashMap<String, GrammarRule>`.
>
> **Parsing:** Implement `Template::parse(input: &str) -> Result<Template>` that parses template strings. `{rule_name}` becomes `RuleRef`, `{markov:corpus:tag}` becomes `MarkovRef`, `{entity.field}` becomes `EntityField`, `{subject}` / `{object}` / `{possessive}` become `PronounRef`. Everything else is `Literal`. Handle edge cases: escaped braces `{{`, empty braces, nested braces (error).
>
> **Loading:** Implement `GrammarSet::load_from_ron(path: &Path) -> Result<GrammarSet>` and `GrammarSet::merge(&mut self, other: GrammarSet)` where `other`'s rules override `self`'s rules with the same name. This is how genre templates get overridden by game-specific grammars.
>
> Write tests: template parsing for all segment types, RON round-trip (serialize then deserialize a GrammarSet), merge precedence. Create a small test grammar RON file in `tests/fixtures/test_grammar.ron` with 3-4 rules covering different segment types.

---

## Prompt 3: Grammar Expansion Runtime

> Implement the grammar expansion engine — the core runtime that takes a `GrammarRule` and produces text.
>
> **SelectionContext:** A struct that accumulates state during expansion. Holds: active tags (`FxHashSet<String>`), active mood, active stakes, entity bindings (`HashMap<String, &Entity>`), and a `depth: u32` counter (to prevent infinite recursion, max depth 20).
>
> **Rule matching:** `GrammarSet::find_matching_rules(&self, ctx: &SelectionContext) -> Vec<&GrammarRule>` returns all rules whose `requires` tags are a subset of the context's active tags and whose `excludes` tags have no intersection with the context's active tags.
>
> **Weighted selection:** Given a list of `Alternative`s, select one using the seeded RNG with weights. Implement as `select_alternative(alts: &[Alternative], rng: &mut StdRng) -> &Alternative`.
>
> **Expansion:** `GrammarSet::expand(&self, rule_name: &str, ctx: &mut SelectionContext, rng: &mut StdRng) -> Result<String>` is the main entry point. It finds the named rule, selects an alternative, then walks the template segments: `Literal` → emit directly, `RuleRef` → recursively call `expand`, `MarkovRef` → for now, emit a placeholder like `[markov:corpus:tag]`, `EntityField` → look up from ctx bindings, `PronounRef` → look up from ctx bindings (just use the entity name for now; pronoun resolution comes later).
>
> **Tag propagation:** When a rule is selected, its `requires` tags are added to the context for child expansions. This is how earlier choices constrain later ones.
>
> Write integration tests: expand a simple rule tree 3 levels deep, verify deterministic output with same seed, verify different output with different seed, verify max depth error, verify tag propagation affects child rule selection. Use the test grammar from Prompt 2.

---

## Prompt 4: Voice System

> Implement the voice system in `src/core/voice.rs`.
>
> **VoiceId** newtype over `u64`. **Voice** struct with: `id: VoiceId`, `name: String`, `parent: Option<VoiceId>` (for inheritance), `grammar_weights: HashMap<String, f32>` (rule name → weight multiplier, default 1.0), `vocabulary: VocabularyPool`, `markov_bindings: Vec<MarkovBinding>`, `structure_prefs: StructurePrefs`, `quirks: Vec<Quirk>`.
>
> **VocabularyPool:** `preferred: FxHashSet<String>`, `avoided: FxHashSet<String>`. These are used by the variety pass later, not during grammar expansion.
>
> **MarkovBinding:** `corpus_id: String`, `weight: f32`, `tags: Vec<String>`.
>
> **StructurePrefs:** `avg_sentence_length: (u32, u32)` (min, max word count range), `clause_complexity: f32` (0.0 = simple, 1.0 = complex), `question_frequency: f32` (0.0..1.0).
>
> **Quirk:** `pattern: String`, `frequency: f32` (probability of injecting per passage). Quirks are phrases or verbal tics like "you see" or "if you will" that get occasionally inserted.
>
> **Voice resolution:** `VoiceRegistry` holds all loaded voices. `VoiceRegistry::resolve(&self, id: VoiceId) -> ResolvedVoice` walks the inheritance chain and merges: child grammar_weights override parent, vocabulary pools union, markov_bindings concatenate, structure_prefs take child values (falling back to parent), quirks concatenate.
>
> **Integration with grammar:** Modify `SelectionContext` to accept an optional `ResolvedVoice`. When selecting alternatives, multiply each alternative's weight by the voice's `grammar_weights` entry for that rule (default 1.0 if not specified). This is how voices shift grammar probabilities without changing the grammar itself.
>
> Write tests: voice inheritance resolution, grammar weight modification affects selection distribution (run 1000 expansions and verify statistical shift), RON serialization round-trip. Create a test voice RON file in `tests/fixtures/`.

---

## Prompt 5: Markov Chain Generator

> Implement the Markov chain system in `src/core/markov.rs`.
>
> **Training (offline):** `MarkovModel` stores n-gram probability tables. `MarkovTrainer::train(text: &str, n: usize) -> MarkovModel` processes raw text into a model. Support n-gram depths 2-4. Tokenize on whitespace with punctuation as separate tokens. Track sentence boundaries (`.`, `!`, `?`) as special tokens so generation doesn't cross sentence boundaries.
>
> Support tagged regions in source corpora: lines prefixed with `[tag]` apply that tag to subsequent text until the next tag or end of file. Store tag associations in the model so the runtime can request tag-filtered generation.
>
> **Serialization:** `MarkovModel` implements `Serialize`/`Deserialize`. The trained model is saved as a binary RON or bincode file by the `corpus_trainer` tool and loaded at runtime.
>
> **Generation:** `MarkovModel::generate(&self, rng: &mut StdRng, tag: Option<&str>, min_words: usize, max_words: usize) -> Result<String>`. Start from a sentence-start state, walk the chain selecting next tokens by weighted probability, stop at a sentence boundary within the word count range. If no valid completion is found within max_words, truncate at the last complete sentence.
>
> **Blending:** `MarkovBlender::generate(models: &[(& MarkovModel, f32)], rng: &mut StdRng, tag: Option<&str>, min_words: usize, max_words: usize) -> Result<String>`. At each step, sample from all models weighted by their blend factor, then select the next token from the combined distribution.
>
> **Integration:** Update grammar expansion so `MarkovRef` segments call into the Markov system instead of emitting placeholders. The pipeline passes loaded `MarkovModel`s through the context.
>
> **corpus_trainer tool:** Implement the binary in `tools/corpus_trainer.rs`. CLI: `corpus_trainer --input <file.txt> --output <model.bin> --ngram <2|3|4>`.
>
> Write tests: train on a small fixed corpus, verify deterministic generation with seed, verify tag filtering produces different output than unfiltered, verify sentence boundaries are respected, verify blending changes distribution. Add a small test corpus to `tests/fixtures/test_corpus.txt` (20-30 sentences across 2 tags).

---

## Prompt 6: Context and Variety System

> Implement the anti-repetition and text quality systems in `src/core/context.rs` and `src/core/variety.rs`.
>
> **context.rs — NarrativeContext:** Maintains a sliding window of the last N generated passages (configurable, default 10). Tracks: recent sentence openings (first 3 words), recently used "significant" words (nouns, adjectives — approximated by word length > 4 and not in a stopword list), entity mention counts (for pronoun decisions).
>
> `NarrativeContext::check_repetition(&self, candidate: &str) -> Vec<RepetitionIssue>` scans a candidate string and returns issues: `RepeatedOpening` (same first 3 words as a recent passage), `OverusedWord { word, count }`, `StructuralMonotony` (sentence length stddev below threshold across recent + candidate).
>
> `NarrativeContext::record(&mut self, text: &str)` adds a passage to the sliding window.
>
> **variety.rs — VarietyPass:** Post-processing transforms applied to generated text before final output.
>
> `VarietyPass::apply(text: &str, voice: &ResolvedVoice, ctx: &NarrativeContext, rng: &mut StdRng) -> String` runs these transforms in order:
> 1. **Synonym rotation:** For words in the voice's `avoided` set that appear in the text, attempt replacement from a small built-in synonym table. Don't do anything fancy here — just a hardcoded map of ~50 common overused words and 3-4 alternatives each.
> 2. **Quirk injection:** Roll against each of the voice's quirks' frequency. If triggered, insert the quirk phrase at a natural insertion point (after a comma, before a period).
> 3. **Repetition remediation:** If `NarrativeContext::check_repetition` returns issues, apply minimal fixes: swap opening words, replace overused words with synonyms, vary sentence length by splitting or combining clauses (simple heuristic: split at "and"/"but" conjunctions, or combine short adjacent sentences with a comma-and).
>
> Write tests: repetition detection with known repeated inputs, synonym rotation, quirk injection frequency (statistical test over many runs), structural monotony detection.

---

## Prompt 7: Pipeline Integration

> Wire everything together in `src/core/pipeline.rs` and create the public API in `src/lib.rs`.
>
> **NarrativeEngine:** The top-level struct. Built via `NarrativeEngine::builder()` with methods: `.genre_templates(&[&str])`, `.grammars_dir(path)`, `.voices_dir(path)`, `.markov_models_dir(path)`, `.mappings(path)`, `.seed(u64)`, `.build() -> Result<NarrativeEngine>`. The builder loads all grammars (genre templates first, then game-specific which override), all voices into a `VoiceRegistry`, all Markov models, and the event-to-narrative-function mapping table.
>
> **Mappings file:** Define the RON format for event-type-to-narrative-function mappings. A mapping entry associates an `event_type` string with a `NarrativeFunction` variant and default properties. The game can also set `narrative_fn` directly on the `Event` to bypass the mapping table.
>
> **WorldState:** A lightweight struct the game passes to `narrate()` containing: `entities: &HashMap<EntityId, Entity>`, plus any other state the pipeline needs to resolve entity references.
>
> **Pipeline execution (`narrate`):**
> 1. Resolve the event's `narrative_fn` (from event directly, or via mapping table).
> 2. Build `SelectionContext` from: event mood/stakes as tags, narrative function properties as tags, participant entity tags, location entity tags.
> 3. Select voice: use the first participant's `voice_id`, or a default narrator voice.
> 4. Resolve voice through `VoiceRegistry`.
> 5. Determine entry grammar rule name from narrative function (convention: `"{fn_name}_opening"` e.g., `"confrontation_opening"`).
> 6. Expand grammar with context, voice, and Markov models.
> 7. Run variety pass.
> 8. Check against narrative context for repetition; if issues found and retries < 3, re-expand with a different seed offset.
> 9. Record in narrative context and return.
>
> **Public API methods:**
> - `engine.narrate(&event, &world_state) -> Result<String>`
> - `engine.narrate_as(&event, voice_id, &world_state) -> Result<String>`
> - `engine.narrate_variants(&event, count, &world_state) -> Result<Vec<String>>`
>
> Write an end-to-end integration test: create a minimal grammar set (3-4 rules), a voice, a small trained Markov model, define an event, and verify `narrate()` produces non-empty text that changes with different seeds. Put test data in `tests/fixtures/`.

---

## Prompt 8: Social Drama Genre Template

> Create the first genre template: Social Drama. This validates that the content authoring experience works and that the engine produces quality output for a real genre.
>
> **Grammar rules** in `genre_data/social_drama/grammar.ron`: Write rules for at least these narrative functions: `Revelation`, `Confrontation`, `Betrayal`, `Alliance`, `ComicRelief`. Each narrative function needs an `_opening`, `_body`, and `_closing` rule. Each rule should have at least 4-5 weighted alternatives. Use a mix of `Literal`, `RuleRef`, `EntityField`, `PronounRef`, and `MarkovRef` segments. Include supporting rules for: `deliberate_action`, `emotional_reaction`, `body_language`, `dialogue_tag`, `social_observation`, `room_detail`.
>
> **Voices** in `genre_data/social_drama/voices.ron`: Define at least 4 voices: `host` (formal, composed, controlling), `gossip` (informal, excitable, observant), `peacemaker` (gentle, diplomatic, indirect), `provocateur` (sharp, blunt, enjoys tension). Each voice should have distinct grammar weight overrides and structure preferences.
>
> **Test corpus** in `genre_data/social_drama/corpus.txt`: Write 40-60 sentences of social drama prose across tags `[tense]`, `[warm]`, `[gossip]`, `[formal]`. These will be used to train the Markov model for this genre.
>
> **Validation:** Run the grammar linter to verify full coverage. Run the preview tool to generate 20+ passages and manually review for quality, variety, and genre-appropriateness. Fix any rules that produce awkward output.

---

## Prompt 9: Survival Thriller Genre Template

> Create the second genre template: Survival Thriller. This validates that the engine is genuinely reusable across genres and that genre templates can coexist.
>
> **Grammar rules** in `genre_data/survival_thriller/grammar.ron`: Rules for: `Escalation`, `Discovery`, `Loss`, `Foreshadowing`, `StatusChange`. Include supporting rules for: `environmental_detail`, `threat_proximity`, `resource_status`, `sensory_detail`, `technical_readout`, `urgency_marker`. Survival thriller should produce noticeably different sentence structures: shorter, more fragmented at high tension; more atmospheric at low tension. Use mood tags to differentiate.
>
> **Voices** in `genre_data/survival_thriller/voices.ron`: Define: `radio_operator` (clipped, procedural, abbreviations), `survivor` (fragmented, sensory, emotional), `scientist` (analytical, precise, detached), `narrator_omniscient` (atmospheric, ominous, measured).
>
> **Test corpus** in `genre_data/survival_thriller/corpus.txt`: 40-60 sentences across `[dread]`, `[urgent]`, `[calm_before]`, `[technical]`.
>
> **Blending test:** Write a test that loads BOTH Social Drama and Survival Thriller templates, creates events spanning both genres (e.g., a tense interpersonal confrontation during a survival scenario), and verifies the merged grammar set produces coherent output drawing from both rule sets.

---

## Prompt 10: Grammar Linter Tool

> Implement the grammar linter as a proper CLI tool in `tools/grammar_linter.rs`.
>
> **Coverage analysis:** For each `NarrativeFunction` variant × each `Mood` variant × each `Stakes` level, check that at least one grammar rule matches. Report uncovered combinations as errors.
>
> **Rule quality checks:**
> - Rules with fewer than 3 alternatives: warning (low variety).
> - Rules that are never reachable (requires tags that no NarrativeFunction + Mood + Stakes combination produces): error.
> - Recursive rule references that form a cycle without a base case: error.
> - `MarkovRef` segments referencing corpus IDs that don't exist in the loaded models: warning.
> - Template parse errors in any rule: error.
>
> **Output:** Print a structured report. Exit code 0 if no errors (warnings ok), 1 if errors.
>
> **Integration with cargo test:** Add a test in `tests/` that runs the linter against all shipped genre templates and asserts exit code 0.

---

## Prompt 11: Preview Tool

> Implement the interactive preview shell in `tools/preview.rs`.
>
> **Features:**
> - Load grammars, voices, and Markov models from specified directories.
> - REPL loop where you can type commands:
>   - `event <type> <mood> <stakes>` — generate from a synthetic event with those parameters.
>   - `voice <name>` — set active voice.
>   - `entity <name> <tag1,tag2,...>` — define a named entity for use in generation.
>   - `seed <n>` — set RNG seed.
>   - `bulk <n>` — generate n passages and print variety statistics (unique openings, word frequency distribution, average length).
>   - `help` — list commands.
>   - `quit` — exit.
> - After each generation, print the expanded text and a trace showing which rules were selected at each expansion step (useful for debugging grammars).

---

## Prompt 12: Dinner Party Example

> Create a complete working example in `examples/dinner_party.rs` that demonstrates the engine end-to-end.
>
> Define 4-5 entities (Margaret, James, Eleanor, Robert, the dining room) with relationships and tags. Define a sequence of 6-8 events that tell a mini story: small talk → tension builds → accusation → revelation → aftermath. Use the Social Drama genre template.
>
> For each event, call `engine.narrate()` and print the output with a header showing the event type and participants. Use a fixed seed so the output is reproducible and can serve as documentation.
>
> The example should compile and run with `cargo run --example dinner_party` and produce a readable multi-paragraph narrative.

---

## Prompt 13: Jurassic Park Example

> Create `examples/jurassic_park.rs` using the Survival Thriller genre template.
>
> Define entities: Rex Paddock, Raptor Pen, Control Room, Security System, Dr. Grant, Dr. Malcolm, Muldoon. Define a sequence of events: routine status → power warning → perimeter breach → escalation → discovery of damage → critical failure. Mix narrative functions: StatusChange, Foreshadowing, Escalation, Discovery, Loss.
>
> Use the `radio_operator` and `narrator_omniscient` voices to alternate between terse status reports and atmospheric narration. Fixed seed, reproducible output.
>
> This example validates that the same engine with different content produces a completely different genre feel.

---

## Notes on Prompt Usage

**Iterate within each prompt.** These prompts are starting points. After each implementation, review the output, run tests, and refine. It's expected that grammar rules especially will need multiple rounds of tuning.

**Don't skip the linter.** Running the grammar linter after every content change catches coverage gaps early. A grammar that doesn't cover all function×mood×stakes combinations will produce empty output for some events at runtime.

**Corpus quality matters enormously.** The Markov layer is only as good as its training text. Invest time in writing or curating corpora that genuinely sound like the target genre. Public domain fiction is a reasonable starting point for prototyping but game-specific prose will always produce better results.

**The genre templates are living documents.** Expect to revise them continuously as you test with real game events. The first version of any grammar will be too thin — add alternatives aggressively when you notice repetition in generated output.
