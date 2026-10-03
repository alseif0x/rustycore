//! Session-owned read callbacks and supervised commit handles.
//!
//! Workers never retain Session, Player or transport. Read completion is not
//! commit admission: only the Session callback pass may consume a prepared read.

use super::super::WorldSession;

impl WorldSession {
    /// The production driver invokes this after packet dispatch. Full World/Map
    /// coordination remains separate; this method never waits for a DB worker.
    pub fn process_ready_character_rename_callbacks_like_cpp(&mut self) {
        self.lifecycle
            .character_rename_callbacks
            .process_ready();
        if self.lifecycle.character_rename_callbacks.has_worker_failure() {
            // Join failure is not an ordinary DB rejection or proven rollback.
            // Retire this Session and let composition drain/classify remaining work.
            self.kick("Character rename worker failed; completion unproven");
            return;
        }
        let pending_delivery_count = self
            .lifecycle
            .character_rename_callbacks
            .pending_delivery_count();
        let pending_result_count = self
            .lifecycle
            .character_rename_callbacks
            .pending_result_count();
        for index in pending_delivery_count..pending_result_count {
            let (guid, outcome) = self
                .lifecycle
                .character_rename_callbacks
                .pending_result_at(index);
            let delivery = self.enqueue_character_rename_like_cpp(guid, outcome);
            self.lifecycle
                .character_rename_callbacks
                .enqueue_pending_delivery(delivery);
        }
        if self
            .lifecycle
            .character_rename_callbacks
            .poll_pending_deliveries()
        {
            self.kick("Character rename response channel closed");
            return;
        }
    }

    /// Composition calls this before disconnect save and Session retirement.
    /// Pending reads cannot admit new commits; submitted writes are joined.
    /// Cancelling this await retains remaining handles for a repeated drain.
    pub async fn finish_character_rename_callbacks_like_cpp(&mut self) -> bool {
        self.lifecycle.character_rename_callbacks.finish().await
    }
}

impl crate::session::state::SessionLifecycleState {
    pub(crate) fn submit_character_rename_like_cpp(
        &mut self,
        port: std::sync::Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>,
        guid: wow_core::ObjectGuid,
        name: String,
    ) -> bool {
        self.character_rename_callbacks.submit(port, guid, name)
    }
}
