//! Session-owned read callbacks and supervised commit handles.
//!
//! Workers never retain Session, Player or transport. Read completion is not
//! commit admission: only the Session callback pass may consume a prepared read.

use super::super::WorldSession;

impl WorldSession {
    /// The production driver invokes this after packet dispatch. Full World/Map
    /// coordination remains separate; this method never waits for a DB worker.
    pub fn process_ready_character_rename_callbacks_like_cpp(&mut self) {
        self.lifecycle.character_rename_process_ready_like_cpp();
        if self.lifecycle.character_rename_has_worker_failure_like_cpp() {
            // Join failure is not an ordinary DB rejection or proven rollback.
            // Retire this Session and let composition drain/classify remaining work.
            self.kick("Character rename worker failed; completion unproven");
            return;
        }
        let pending_delivery_count = self
            .lifecycle
            .character_rename_pending_delivery_count_like_cpp();
        let pending_result_count = self
            .lifecycle
            .character_rename_pending_result_count_like_cpp();
        for index in pending_delivery_count..pending_result_count {
            let (guid, outcome) = self
                .lifecycle
                .character_rename_pending_result_at_like_cpp(index);
            let delivery = self.enqueue_character_rename_like_cpp(guid, outcome);
            self.lifecycle
                .character_rename_enqueue_pending_delivery_like_cpp(delivery);
        }
        if self.lifecycle.character_rename_poll_pending_deliveries_like_cpp() {
            self.kick("Character rename response channel closed");
            return;
        }
    }

    /// Composition calls this before disconnect save and Session retirement.
    /// Pending reads cannot admit new commits; submitted writes are joined.
    /// Cancelling this await retains remaining handles for a repeated drain.
    pub async fn finish_character_rename_callbacks_like_cpp(&mut self) -> bool {
        self.lifecycle.character_rename_finish_like_cpp().await
    }
}
