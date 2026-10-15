// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the gameobject-use and player-attack bodies that
//! moved to `wow_world_entities` (#1363 slice 4a). Each method delegates to the
//! same-named session operation; the loot-queue, quest-source, duel and group
//! hooks repeat the exact split the moved body used before the move.

use std::future::Future;

use wow_constants::unit::UnitPvpFlags;
use wow_core::{ObjectGuid, Position};
use wow_entities::{GameObjectLootSource, GatheringNodeUseSource, PhaseShift};
use wow_world_core::session::{HubMut, HubRef, ItemValuationCatalogsLikeCpp};
use wow_world_entities::{WorldEntitiesHostLikeCpp, WorldEntitiesState};

use crate::session::WorldSession;

impl WorldEntitiesHostLikeCpp for WorldSession {
    fn world_entities(&self) -> &WorldEntitiesState {
        &self.world_entities
    }

    fn world_entities_mut(&mut self) -> &mut WorldEntitiesState {
        &mut self.world_entities
    }

    fn split_world_entities_ref(&self) -> (&WorldEntitiesState, HubRef<'_>) {
        crate::session::split_world_entities_ref(self)
    }

    fn split_world_entities_mut(&mut self) -> (&mut WorldEntitiesState, HubMut<'_>) {
        crate::session::split_world_entities_mut(self)
    }

    fn queue_visible_gameobject_packet_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
        packet_bytes: Vec<u8>,
    ) -> usize {
        let (s, h) = crate::session::split_loot_ref(self);
        s.queue_visible_gameobject_packet_for_same_map_like_cpp(h, gameobject_guid, packet_bytes)
    }

    fn queue_goober_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        wow_world_loot_ops::queue_goober_gameobject_state_refresh_for_same_map_like_cpp(
            self,
            gameobject_guid,
        )
    }

    fn record_represented_gameobject_template_quest_source_like_cpp(
        &mut self,
        guid: ObjectGuid,
        template: &wow_entities::GameObjectTemplateData,
    ) {
        crate::session::cx_quest_state(self)
            .record_represented_gameobject_template_quest_source_like_cpp(guid, template);
    }

    fn canonical_player_duel_in_progress_like_cpp(
        &self,
        guid: ObjectGuid,
        opponent: ObjectGuid,
    ) -> Option<bool> {
        let (s, h) = crate::session::split_social_ref(self);
        s.canonical_player_duel_in_progress_like_cpp(h, guid, opponent)
    }

    fn current_player_is_group_visible_for_owner_like_cpp(
        &self,
        current_group_guid: Option<u64>,
        owner_guid: ObjectGuid,
    ) -> bool {
        let (s, h) = crate::session::split_social_ref(self);
        s.current_player_is_group_visible_for_owner_like_cpp(h, current_group_guid, owner_guid)
    }

    fn attacker_pvp_flags_test_fallback_like_cpp(
        &self,
        player_guid: Option<ObjectGuid>,
    ) -> Option<UnitPvpFlags> {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return player_guid.and_then(|guid| self.canonical_player_pvp_flags_like_cpp(guid));
        }
        let _ = player_guid;
        None
    }

    fn represented_unit_values_update_to_update_object_like_cpp(
        &self,
        unit_guid: ObjectGuid,
        map_id: u16,
        values_update: &wow_entities::UnitValuesUpdate,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        WorldSession::represented_unit_values_update_to_update_object_like_cpp(
            self,
            unit_guid,
            map_id,
            values_update,
        )
    }

    fn represented_player_phase_shift_like_cpp(&self) -> Option<PhaseShift> {
        WorldSession::represented_player_phase_shift_like_cpp(self)
    }

    fn ensure_canonical_world_map_for_current_player_like_cpp(
        &mut self,
    ) -> Option<wow_map::CreateMapDecision> {
        wow_world_instances::ensure_canonical_world_map_for_current_player_like_cpp(self)
    }

    fn current_player_is_in_group_guid_like_cpp(
        &self,
        current_group_guid: Option<u64>,
        group_owner: ObjectGuid,
    ) -> bool {
        WorldSession::current_player_is_in_group_guid_like_cpp(
            self,
            current_group_guid,
            group_owner,
        )
    }

    fn can_see_phase_shift_like_cpp(&self, other: &PhaseShift) -> bool {
        WorldSession::can_see_phase_shift_like_cpp(self, other)
    }

    fn remove_cached_loot_for_owner_like_cpp(&mut self, owner: ObjectGuid) {
        self.loot.remove_cached_loot_for_owner_like_cpp(owner);
    }

    fn queue_session_chest_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        wow_world_loot_ops::queue_session_chest_gameobject_state_refresh_for_same_map_like_cpp(
            self,
            gameobject_guid,
        )
    }

    fn kill_credit_gameobject_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        gameobject_entry: u32,
        gameobject_guid: wow_core::ObjectGuid,
    ) -> impl Future<Output = ()> + Send {
        WorldSession::kill_credit_gameobject_with_generator_like_cpp(
            self,
            item_guid_generator,
            gameobject_entry,
            gameobject_guid,
        )
    }

    fn open_represented_fishing_hole_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        loot_id: u32,
    ) -> impl Future<Output = ()> + Send {
        wow_world_loot_ops::open_represented_fishing_hole_with_catalogs_like_cpp(
            self,
            item_valuation,
            gameobject_guid,
            gameobject_entry,
            loot_id,
        )
    }

    fn open_represented_fishing_node_loot_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        area_id: u32,
        junk: bool,
    ) -> impl Future<Output = ()> + Send {
        wow_world_loot_ops::open_represented_fishing_node_loot_with_catalogs_like_cpp(
            self,
            item_valuation,
            gameobject_guid,
            area_id,
            junk,
        )
    }

    fn open_represented_gameobject_chest_with_template_money_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
        template_money: (u32, u32),
    ) -> impl Future<Output = ()> + Send {
        wow_world_loot_ops::open_represented_gameobject_chest_with_template_money_like_cpp(
            self,
            item_guid_generator,
            item_valuation,
            gameobject_guid,
            source,
            template_money,
        )
    }

    fn open_represented_gathering_node_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        source: GatheringNodeUseSource,
    ) -> impl Future<Output = ()> + Send {
        wow_world_loot_ops::open_represented_gathering_node_with_catalogs_like_cpp(
            self,
            item_valuation,
            gameobject_guid,
            gameobject_entry,
            source,
        )
    }

    fn remove_aura(&mut self, slot: u8) -> Result<(), &'static str> {
        WorldSession::remove_aura(self, slot)
    }

    fn remove_represented_mounted_auras_by_type_like_cpp(&mut self) -> bool {
        wow_world_spell::remove_represented_mounted_auras_by_type_like_cpp(self)
    }

    fn represented_meets_player_condition_id_like_cpp(&self, player_condition_id: u32) -> bool {
        WorldSession::represented_meets_player_condition_id_like_cpp(self, player_condition_id)
    }

    fn represented_player_at_group_reward_distance_like_cpp(
        &self,
        player_guid: ObjectGuid,
        reward_map_id: u16,
        reward_position: Position,
    ) -> bool {
        WorldSession::represented_player_at_group_reward_distance_like_cpp(
            self,
            player_guid,
            reward_map_id,
            reward_position,
        )
    }

    fn represented_player_is_same_raid_with_like_cpp(
        &self,
        player_guid: ObjectGuid,
        owner_guid: ObjectGuid,
    ) -> bool {
        WorldSession::represented_player_is_same_raid_with_like_cpp(self, player_guid, owner_guid)
    }

    fn represented_player_quest_status_like_cpp(&self, quest_id: u32) -> Option<Option<u8>> {
        wow_world_quest::represented_player_quest_status_like_cpp(self, quest_id)
    }

    fn resolved_group_guid_like_cpp(&self) -> Option<u64> {
        WorldSession::resolved_group_guid_like_cpp(self)
    }

    fn send_represented_cinematic_start_like_cpp(&mut self, cinematic_id: u32) {
        WorldSession::send_represented_cinematic_start_like_cpp(self, cinematic_id)
    }

    fn use_represented_gameobject_questgiver_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        source: wow_entities::QuestgiverUseSource,
    ) -> bool {
        wow_world_quest::use_represented_gameobject_questgiver_like_cpp(
            self,
            gameobject_guid,
            player_guid,
            gameobject_entry,
            source,
        )
    }

    fn represented_player_group_reward_state_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Option<(u8, u16, Position, bool)> {
        let (s, h) = crate::session::split_social_ref(self);
        s.represented_player_group_reward_state_like_cpp(h, player_guid)
    }

    fn calculate_kill_reputation_gain_like_cpp(&self, level: u8, rep: i32, faction: u32) -> i32 {
        wow_world_social::calculate_kill_reputation_gain_like_cpp(self, level, rep, faction)
    }

    fn cached_loot_for_owner_like_cpp(&self, owner: ObjectGuid) -> Option<&wow_loot::CreatureLoot> {
        self.loot.cached_loot_for_owner_like_cpp(owner)
    }

    fn ensure_represented_creature_kill_loot_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
    ) -> impl Future<Output = ()> + Send {
        wow_world_loot_ops::ensure_represented_creature_kill_loot_like_cpp(self, creature_guid)
    }

    fn give_xp(
        &mut self,
        xp: u32,
        victim: ObjectGuid,
        rate: f32,
    ) -> impl Future<Output = ()> + Send {
        WorldSession::give_xp(self, xp, victim, rate)
    }

    fn on_creature_killed_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_entry: u32,
        creature_guid: ObjectGuid,
    ) -> impl Future<Output = ()> + Send {
        WorldSession::on_creature_killed_with_generator_like_cpp(
            self,
            item_guid_generator,
            creature_entry,
            creature_guid,
        )
    }
}
