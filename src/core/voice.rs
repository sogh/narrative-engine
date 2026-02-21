/// Voice system — persona/tone bundles that shape generated text.

use serde::{Deserialize, Serialize};

use crate::schema::entity::VoiceId;

/// A voice definition that shapes how text sounds.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Voice {
    pub id: VoiceId,
    pub name: String,
    pub parent: Option<VoiceId>,
}

/// Registry of all loaded voices.
#[derive(Debug, Clone, Default)]
pub struct VoiceRegistry {
    // Will be populated in Prompt 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_registry_default() {
        let registry = VoiceRegistry::default();
        assert!(format!("{:?}", registry).contains("VoiceRegistry"));
    }
}
