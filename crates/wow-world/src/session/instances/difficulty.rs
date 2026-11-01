//! Represented difficulty selection and its published state.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Set the C++ Difficulty.db2 store used by `sDifficultyStore`.
    pub fn set_difficulty_store(&mut self, store: Arc<DifficultyStore>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.catalogs.difficulty_store = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn represented_raid_difficulty_id_like_cpp(&self) -> u32 {
        self.resolved_raid_difficulty_id_like_cpp()
            .expect("test Player raid difficulty owner must resolve")
    }
    #[cfg(test)]
    pub(crate) fn represented_legacy_raid_difficulty_id_like_cpp(&self) -> u32 {
        self.resolved_legacy_raid_difficulty_id_like_cpp()
            .expect("test Player legacy raid difficulty owner must resolve")
    }
    pub fn set_map_difficulty_store(&mut self, store: Arc<MapDifficultyStore>) {
        self.catalogs.maps.difficulty_store = Some(store);
    }
    pub fn set_map_difficulty_x_condition_store(
        &mut self,
        store: Arc<MapDifficultyXConditionStore>,
    ) {
        self.catalogs.maps.difficulty_x_condition_store = Some(store);
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/instances/difficulty/f3_shims.rs"]
mod f3_shims;
