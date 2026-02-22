//! Narrative Engine — procedural text generation for games.
//!
//! Generates thematically appropriate, contextually aware text at runtime
//! without neural network inference, using a pipeline of simulation-driven
//! events, stochastic grammars, and Markov-trained phrase generation.

pub mod core;
pub mod genre_templates;
pub mod schema;

// Public API re-exports
pub use core::pipeline::{NarrativeEngine, NarrativeEngineBuilder, PipelineError, WorldState};
pub use schema::entity::{Entity, EntityId, Value, VoiceId};
pub use schema::event::{EntityRef, Event, Mood, Outcome, Stakes};
pub use schema::narrative_fn::NarrativeFunction;
