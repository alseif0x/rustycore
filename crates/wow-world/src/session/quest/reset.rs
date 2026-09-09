//! Seasonal, daily and weekly represented quest resets.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Set the represented QuestPoolMgr active snapshot shared reference.
    pub fn set_quest_pool_store(&mut self, store: Arc<wow_data::quest::QuestPoolStoreLikeCpp>) {
        self.quests.pool_store = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn seasonal_quest_bucket_like_cpp(
        &self,
        event_id: u16,
    ) -> Option<BTreeMap<u32, u64>> {
        self.player_quest_gameplay_snapshot_like_cpp()?
            .seasonal_quests
            .get(&event_id)
            .cloned()
    }
    #[cfg(test)]
    pub(crate) fn set_seasonal_quest_changed_like_cpp_for_test(&mut self, changed: bool) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.seasonal_quest_changed = changed;
        });
    }
    #[cfg(test)]
    pub(crate) fn seasonal_quest_changed_like_cpp(&self) -> bool {
        self.player_quest_gameplay_snapshot_like_cpp()
            .expect("test Player quest owner resolves")
            .seasonal_quest_changed
    }
}
