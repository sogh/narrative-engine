# Contributing to Narrative Engine

Thanks for your interest in contributing! Here's how to get started.

## Getting Started

1. Fork the repository and clone your fork
2. Make sure you have Rust stable installed: `rustup update stable`
3. Build the project: `cargo build`
4. Run the tests: `cargo test`

## Development Workflow

1. Create a branch for your change
2. Make your changes
3. Run `cargo test` to ensure all tests pass
4. Run `cargo clippy` and fix any warnings
5. Run `cargo fmt` to format your code
6. Submit a pull request

## Code Style

- Follow the conventions in [CLAUDE.md](CLAUDE.md)
- Use `thiserror` for error types; every public function returns `Result<T, NarrativeError>`
- Prefer `&str` over `String` in function signatures where possible
- Use newtypes for IDs (`EntityId`, `VoiceId`, etc.) — no raw integers in public APIs
- No `unwrap()` or `expect()` outside of tests
- All randomness goes through the seedable `StdRng` in the pipeline context

## Architecture

Before making changes, read the [Design Document](docs/narrative_engine_design_doc.md) to understand the architecture. Key principles:

- **Data vs. code boundary** — the engine defines the *process*, games define the *content*. Don't add game-specific logic to `src/`.
- **Narrative Functions** separate what's happening narratively from how it's expressed.
- **Tags** are the universal coupling mechanism. The engine never interprets tag meaning.

## Content Contributions

Grammar rules, voice profiles, and genre template data live in `genre_data/`. Content contributions (new voices, grammar patterns, genre templates) are welcome. Use the grammar linter to validate your changes:

```bash
cargo run --bin grammar_linter -- genre_data/
```

## Reporting Issues

Open an issue on GitHub with:

- A clear description of the problem or suggestion
- Steps to reproduce (for bugs)
- Expected vs. actual behavior
- Rust version (`rustc --version`)

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE).
