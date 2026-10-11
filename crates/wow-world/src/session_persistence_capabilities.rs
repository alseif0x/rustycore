//! Typed persistence capabilities installed by the application composition root.

use std::sync::Arc;

use crate::session::{SessionPersistencePortsLikeCpp, WorldSession};

impl WorldSession {
    /// Install the complete production persistence graph atomically.
    pub fn set_required_persistence_capabilities_like_cpp(
        &mut self,
        capabilities: SessionPersistencePortsLikeCpp,
    ) {
        self.lifecycle
            .set_required_persistence_capabilities_like_cpp(capabilities);
    }

    pub fn set_character_administration_persistence_port_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>,
    ) {
        self.lifecycle
            .set_character_administration_persistence_port_like_cpp(port)
    }
}

#[cfg(test)]
#[path = "../unit_tests/session_persistence_capabilities/test_only.rs"]
mod test_only;
