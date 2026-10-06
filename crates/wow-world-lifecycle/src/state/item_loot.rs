use super::SessionLifecycleState;

impl SessionLifecycleState {
    pub fn begin_durable_item_loot_persistence_like_cpp(
        &self,
    ) -> crate::DurableItemLootPersistenceGuardLikeCpp {
        self.durable_item_loot_persistence_like_cpp.begin_like_cpp()
    }

    pub async fn wait_for_durable_item_loot_persistence_like_cpp(&self) {
        self.durable_item_loot_persistence_like_cpp
            .wait_until_idle_like_cpp()
            .await;
    }

    pub fn take_durable_item_loot_completions_like_cpp(
        &self,
    ) -> Vec<crate::DurableItemLootCompletionLikeCpp> {
        self.durable_item_loot_persistence_like_cpp
            .take_completions_like_cpp()
    }
}
