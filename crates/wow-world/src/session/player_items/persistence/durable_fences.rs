//! durable fences for the existing persistence owner.

use super::*;

impl WorldSession {
    pub(crate) fn begin_durable_item_loot_persistence_like_cpp(
        &self,
    ) -> DurableItemLootPersistenceGuardLikeCpp {
        self.lifecycle
            .durable_item_loot_persistence_like_cpp
            .begin_like_cpp()
    }
    pub(crate) async fn wait_for_durable_item_loot_persistence_like_cpp(&self) {
        self.lifecycle
            .durable_item_loot_persistence_like_cpp
            .wait_until_idle_like_cpp()
            .await;
    }
    pub(crate) fn take_durable_item_loot_completions_like_cpp(
        &self,
    ) -> Vec<DurableItemLootCompletionLikeCpp> {
        self.lifecycle
            .durable_item_loot_persistence_like_cpp
            .take_completions_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn set_loot_item_store_test_commit_gate_like_cpp(
        &mut self,
        gate: Arc<tokio::sync::Notify>,
    ) {
        self.loot_item_store_test_commit_gate_like_cpp = Some(gate);
    }
}
