//! Ready-only character-list continuations; no autonomous simulation clock.

use super::super::WorldSession;

impl WorldSession {
    pub(crate) fn submit_character_rename_like_cpp(
        &mut self,
        port: std::sync::Arc<dyn wow_persistence::CharacterAdministrationPersistencePortLikeCpp>,
        guid: wow_core::ObjectGuid,
        name: String,
    ) -> bool {
        self.character_rename_callbacks.submit(port, guid, name)
    }

    /// The production driver invokes this after packet dispatch. Full World/Map
    /// coordination remains separate; this method never waits for a DB worker.
    pub fn process_ready_character_rename_callbacks_like_cpp(&mut self) {
        for (guid, outcome) in self.character_rename_callbacks.process_ready() {
            self.complete_character_rename_like_cpp(guid, outcome);
        }
    }

    /// Composition calls this before disconnect save and Session retirement.
    /// Pending reads cannot admit new commits; submitted writes are joined.
    /// Cancelling this await retains remaining handles for a repeated drain.
    pub async fn finish_character_rename_callbacks_like_cpp(&mut self) -> bool {
        self.character_rename_callbacks.finish().await
    }
}
