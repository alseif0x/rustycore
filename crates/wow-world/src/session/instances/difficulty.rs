//! Represented difficulty selection and its published state.
//!
//! Moved out of the Session root under #613. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn create_map_difficulty_context_like_cpp(
        &self,
        map_id: u32,
        difficulty_id: wow_map::Difficulty,
    ) -> Option<wow_map::CreateMapDifficultyContext> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.create_map_difficulty_context_like_cpp(hub, map_id, difficulty_id)
    }
    pub(in crate::session) fn represented_player_difficulty_id_for_map_entry_like_cpp(
        &self,
        map_id: u32,
        map_entry: wow_data::map::MapEntry,
    ) -> Option<wow_map::Difficulty> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.represented_player_difficulty_id_for_map_entry_like_cpp(hub, map_id, map_entry)
    }
    pub(in crate::session) fn represented_group_difficulty_id_for_map_entry_like_cpp(
        &self,
        map_id: u32,
        map_entry: wow_data::map::MapEntry,
        group: &GroupInfo,
    ) -> wow_map::Difficulty {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.represented_group_difficulty_id_for_map_entry_like_cpp(hub, map_id, map_entry, group)
    }
    /// Set the C++ Difficulty.db2 store used by `sDifficultyStore`.
    pub fn set_difficulty_store(&mut self, store: Arc<DifficultyStore>) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.catalogs.difficulty_store = Some(store);
    }
    pub(crate) fn player_difficulty_preferences_snapshot_like_cpp(
        &self,
    ) -> Option<(u32, u32, u32)> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.player_difficulty_preferences_snapshot_like_cpp(hub)
    }
    pub(in crate::session) fn replace_player_difficulty_preferences_like_cpp(
        &mut self,
        dungeon: u32,
        raid: u32,
        legacy_raid: u32,
    ) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.replace_player_difficulty_preferences_like_cpp(&mut hub, dungeon, raid, legacy_raid)
    }
    pub(crate) fn resolved_dungeon_difficulty_id_like_cpp(&self) -> Option<u32> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.resolved_dungeon_difficulty_id_like_cpp(hub)
    }
    pub(crate) fn resolved_raid_difficulty_id_like_cpp(&self) -> Option<u32> {
        self.player_difficulty_preferences_snapshot_like_cpp()
            .map(|preferences| preferences.1)
    }
    pub(crate) fn resolved_legacy_raid_difficulty_id_like_cpp(&self) -> Option<u32> {
        self.player_difficulty_preferences_snapshot_like_cpp()
            .map(|preferences| preferences.2)
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
    pub(crate) fn represented_dungeon_difficulty_packet_like_cpp(
        &self,
    ) -> Option<DungeonDifficultySet> {
        let (state, hub) = crate::session::split_instances_ref(self);
        state.represented_dungeon_difficulty_packet_like_cpp(hub)
    }
    pub(crate) fn apply_group_difficulty_like_cpp(
        &mut self,
        group_guid: u64,
        difficulty_id: u32,
        kind: wow_social::group::GroupDifficultyKindLikeCpp,
    ) {
        let mut cx = self.build_instance_difficulty_handler_cx_like_cpp();
        cx.apply_group_difficulty_like_cpp(group_guid, difficulty_id, kind);
    }
    pub(in crate::session) fn reconcile_group_difficulty_like_cpp(
        &mut self,
        dungeon: u32,
        raid: u32,
        legacy_raid: u32,
    ) -> bool {
        let (state, mut hub) = crate::session::split_instances_mut(self);
        state.reconcile_group_difficulty_like_cpp(&mut hub, dungeon, raid, legacy_raid)
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
    #[allow(dead_code)]
    pub(crate) fn represented_failed_map_difficulty_x_condition_like_cpp(
        &self,
        map_difficulty_id: u32,
    ) -> Option<u32> {
        let store = self.catalogs.maps.difficulty_x_condition_store.as_ref()?;
        let player_conditions = self.catalogs.player_condition_store.as_ref()?;
        let context = self.represented_player_condition_context_like_cpp()?;
        store.failed_condition_like_cpp(map_difficulty_id, player_conditions, |condition| {
            context
                .as_context(self)
                .is_some_and(|context| is_player_meeting_condition_like_cpp(condition, &context))
        })
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/instances/difficulty/f3_shims.rs"]
mod f3_shims;
