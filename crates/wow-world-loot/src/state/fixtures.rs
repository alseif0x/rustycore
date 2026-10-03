use super::LootState;
use crate::{RepresentedLootRollCriteriaEvent, RepresentedLootRollState};
use wow_core::ObjectGuid;
use wow_loot::OwnedLootAuthority;

impl LootState {
    pub fn pass_on_group_loot_for_test_like_cpp(&self) -> bool {
        self.pass_on_group_loot
    }

    pub fn set_pass_on_group_loot_for_test_like_cpp(&mut self, value: bool) {
        self.pass_on_group_loot = value;
    }

    pub fn loot_specialization_id_for_test_like_cpp(&self) -> u32 {
        self.loot_specialization_id
    }

    pub fn active_loot_view_owner_count_for_test_like_cpp(&self) -> usize {
        self.active_loot_view_owners.len()
    }

    pub fn represented_loot_cache_generation_for_test_like_cpp(
        &self,
        owner: ObjectGuid,
    ) -> Option<&u64> {
        self.represented_loot_cache_generations_like_cpp.get(&owner)
    }

    pub fn insert_active_loot_view_generation_for_test_like_cpp(
        &mut self,
        owner: ObjectGuid,
        generation: u64,
    ) -> Option<u64> {
        self.active_loot_view_generations_like_cpp
            .insert(owner, generation)
    }

    pub fn insert_active_loot_view_authority_for_test_like_cpp(
        &mut self,
        owner: ObjectGuid,
        authority: OwnedLootAuthority,
    ) -> Option<OwnedLootAuthority> {
        self.active_loot_view_authorities_like_cpp
            .insert(owner, authority)
    }

    pub fn insert_represented_loot_roll_for_test_like_cpp(
        &mut self,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        roll: RepresentedLootRollState,
    ) -> Option<RepresentedLootRollState> {
        self.represented_loot_rolls
            .insert((loot_obj, loot_list_id), roll)
    }

    pub fn represented_loot_roll_criteria_events_for_test_like_cpp(
        &self,
    ) -> &[RepresentedLootRollCriteriaEvent] {
        &self.represented_loot_roll_criteria_events
    }

    pub fn insert_represented_gameobject_tap_list_for_test_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        tappers: Vec<ObjectGuid>,
    ) -> Option<Vec<ObjectGuid>> {
        self.represented_gameobject_tap_lists
            .insert(gameobject_guid, tappers)
    }

    pub fn insert_locked_dungeon_encounter_for_test_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> bool {
        self.represented_locked_dungeon_encounters
            .insert((player_guid, dungeon_encounter_id))
    }

}
