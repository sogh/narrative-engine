//! Narrative Engine — procedural text generation for games.
//!
//! Generates thematically appropriate, contextually aware text at runtime
//! without neural network inference, using a pipeline of simulation-driven
//! events, stochastic grammars, and Markov-trained phrase generation.

pub mod core;
pub mod genre_templates;
pub mod schema;
