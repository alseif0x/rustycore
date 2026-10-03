use std::collections::BTreeMap;

use crate::{InstanceState, RepresentedAdventureMapStartQuestLikeCpp};

impl InstanceState {
    pub fn set_fixture_dungeon_difficulty_for_test_like_cpp(&mut self, difficulty_id: u32) {
        self.instance_test_fixture_like_cpp
            .represented_dungeon_difficulty_id_like_cpp = difficulty_id;
    }

    pub fn set_fixture_raid_difficulty_for_test_like_cpp(&mut self, difficulty_id: u32) {
        self.instance_test_fixture_like_cpp
            .represented_raid_difficulty_id_like_cpp = difficulty_id;
    }

    pub fn set_fixture_legacy_raid_difficulty_for_test_like_cpp(&mut self, difficulty_id: u32) {
        self.instance_test_fixture_like_cpp
            .represented_legacy_raid_difficulty_id_like_cpp = difficulty_id;
    }

    pub fn represented_instance_reset_times_for_test_like_cpp(&self) -> &BTreeMap<u32, u64> {
        &self.represented_instance_reset_times_like_cpp
    }

    pub fn replace_represented_instance_reset_times_for_test_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = (u32, u64)>,
    ) {
        self.represented_instance_reset_times_like_cpp.clear();
        for (instance_id, release_time) in rows {
            self.represented_instance_reset_times_like_cpp
                .entry(instance_id)
                .or_insert(release_time);
        }
    }

    pub fn insert_represented_instance_reset_time_for_test_like_cpp(
        &mut self,
        instance_id: u32,
        release_time: u64,
    ) -> Option<u64> {
        self.represented_instance_reset_times_like_cpp
            .insert(instance_id, release_time)
    }

    pub fn take_nonempty_represented_instance_reset_times_for_test_like_cpp(
        &mut self,
    ) -> Option<BTreeMap<u32, u64>> {
        (!self.represented_instance_reset_times_like_cpp.is_empty())
            .then(|| std::mem::take(&mut self.represented_instance_reset_times_like_cpp))
    }

    pub fn represented_explored_zones_for_test_like_cpp(
        &self,
    ) -> &[u64; wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP] {
        &self.represented_explored_zones_like_cpp
    }

    pub fn replace_represented_explored_zones_for_test_like_cpp(
        &mut self,
        blocks: [u64; wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP],
    ) {
        self.represented_explored_zones_like_cpp = blocks;
    }

    pub fn set_represented_explored_zone_block_for_test_like_cpp(
        &mut self,
        index: usize,
        value: u64,
    ) {
        self.represented_explored_zones_like_cpp[index] = value;
    }

    pub fn represented_pending_bind_for_test_like_cpp(
        &self,
    ) -> Option<crate::RepresentedPendingBind> {
        self.pending_bind
    }

    pub fn set_represented_pending_bind_for_test_like_cpp(
        &mut self,
        pending_bind: Option<crate::RepresentedPendingBind>,
    ) {
        self.pending_bind = pending_bind;
    }

    pub fn represented_confirmed_pending_binds_for_test_like_cpp(&self) -> &[u32] {
        &self.represented_confirmed_pending_binds
    }

    pub fn record_represented_confirmed_pending_bind_for_test_like_cpp(
        &mut self,
        instance_id: u32,
    ) {
        self.represented_confirmed_pending_binds.push(instance_id);
    }

    pub fn record_represented_adventure_map_start_quest_for_test_like_cpp(
        &mut self,
        request: RepresentedAdventureMapStartQuestLikeCpp,
    ) {
        self.represented_adventure_map_start_quest_requests_like_cpp
            .push(request);
    }

    pub fn represented_adventure_map_start_quest_requests_for_test_like_cpp(
        &self,
    ) -> &[RepresentedAdventureMapStartQuestLikeCpp] {
        &self.represented_adventure_map_start_quest_requests_like_cpp
    }

    pub fn record_represented_area_zone_criteria_for_test_like_cpp(
        &mut self,
        event: crate::RepresentedAreaZoneCriteriaLikeCpp,
    ) {
        self.represented_area_zone_criteria_like_cpp.push(event);
    }

    pub fn record_represented_reveal_world_map_overlay_criteria_for_test_like_cpp(
        &mut self,
        area_id: u32,
    ) {
        self.represented_reveal_world_map_overlay_criteria_like_cpp
            .push(area_id);
    }
}
