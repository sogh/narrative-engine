# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/), and this project adheres to [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-02-23

### Added

- Core pipeline: Event → Narrative Function → Voice → Grammar → Markov → Variety → Context → Output
- Schema layer: entities, events, narrative functions, relationships
- Stochastic grammar engine with tag-based rule matching and weighted alternatives
- Markov chain phrase generator (bigram/trigram) with corpus training
- Voice system with inheritance and grammar weight overrides
- Context tracker for recency-aware variety
- Genre templates: social drama, survival thriller, political intrigue, exploration
- RON-based data format for grammars, voices, and corpora configs
- Tools: grammar linter, corpus trainer, interactive preview shell
- Examples: dinner party (social drama), dino park (survival thriller)
- Comprehensive user guide and design documentation
