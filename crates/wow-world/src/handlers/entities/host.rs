// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the gameobject-use and corpse bodies that moved to
//! `wow_world_entities` (#1363 slice 4a). Each method delegates to the
//! same-named session operation; the loot queue and quest-source hooks repeat
//! the exact split the moved body used before the move.

use std::future::Future;

use wow_core::{ObjectGuid, Position};
use wow_entities::{
    GameObjectLootSource, GatheringNodeUseSource, PhaseShift, PlayerResurrectionRequestLikeCpp,
};
use wow_world_core::session::{HubMut, HubRef, ItemValuationCatalogsLikeCpp};
use wow_world_entities::{GameObjectUseHostLikeCpp, WorldEntitiesState};

use crate::session::WorldSession;

impl GameObjectUseHostLikeCpp for WorldSession {
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
        crate::session::cx_loot_ref(self)
            .queue_goober_gameobject_state_refresh_for_same_map_like_cpp(gameobject_guid)
    }

    fn count_represented_repop_at_graveyard_for_test_like_cpp(&mut self) {
        #[cfg(test)]
        {
            self.fixtures.combat.represented_repop_at_graveyard_count = self
                .fixtures
                .combat
                .represented_repop_at_graveyard_count
                .saturating_add(1);
        }
    }

    fn record_represented_gameobject_template_quest_source_like_cpp(
        &mut self,
        guid: ObjectGuid,
        template: &wow_entities::GameObjectTemplateData,
    ) {
        crate::session::cx_quest_state(self)
            .record_represented_gameobject_template_quest_source_like_cpp(guid, template);
    }

    crate::session::forward_to_world_session_like_cpp! {
        fn can_see_phase_shift_like_cpp(&self, other: &PhaseShift) -> bool;
        fn clear_represented_resurrection_request_like_cpp(&mut self) -> bool;
        fn despawn_represented_linked_trap_by_guid_like_cpp(&mut self, trap_guid: ObjectGuid);
        fn graveyard_conditions_meet_like_cpp(
            &mut self, conditions_ref: &wow_data::ConditionsReference
        ) -> bool;
        fn kill_credit_gameobject_with_generator_like_cpp(
            &mut self, item_guid_generator: &wow_core::ObjectGuidGenerator, gameobject_entry: u32,
            gameobject_guid: wow_core::ObjectGuid
        ) -> impl Future<Output = ()> + Send;
        fn open_represented_fishing_hole_with_catalogs_like_cpp(
            &mut self, item_valuation: &ItemValuationCatalogsLikeCpp, gameobject_guid: ObjectGuid,
            gameobject_entry: u32, loot_id: u32
        ) -> impl Future<Output = ()> + Send;
        fn open_represented_fishing_node_loot_with_catalogs_like_cpp(
            &mut self, item_valuation: &ItemValuationCatalogsLikeCpp, gameobject_guid: ObjectGuid,
            area_id: u32, junk: bool
        ) -> impl Future<Output = ()> + Send;
        fn open_represented_gameobject_chest_with_template_money_like_cpp(
            &mut self, item_guid_generator: &wow_core::ObjectGuidGenerator,
            item_valuation: &ItemValuationCatalogsLikeCpp, gameobject_guid: ObjectGuid,
            source: GameObjectLootSource, template_money: (u32, u32)
        ) -> impl Future<Output = ()> + Send;
        fn open_represented_gathering_node_with_catalogs_like_cpp(
            &mut self, item_valuation: &ItemValuationCatalogsLikeCpp, gameobject_guid: ObjectGuid,
            gameobject_entry: u32, source: GatheringNodeUseSource
        ) -> impl Future<Output = ()> + Send;
        fn pending_teleport_like_cpp(&self) -> Option<(u32, Position)>;
        fn remove_aura(&mut self, slot: u8) -> Result<(), &'static str>;
        fn remove_represented_mounted_auras_by_type_like_cpp(&mut self) -> bool;
        fn represented_meets_player_condition_id_like_cpp(&self, player_condition_id: u32) -> bool;
        fn represented_player_at_group_reward_distance_like_cpp(
            &self, player_guid: ObjectGuid, reward_map_id: u16, reward_position: Position
        ) -> bool;
        fn represented_player_is_same_raid_with_like_cpp(
            &self, player_guid: ObjectGuid, owner_guid: ObjectGuid
        ) -> bool;
        fn represented_player_quest_status_like_cpp(&self, quest_id: u32) -> Option<Option<u8>>;
        fn resolved_group_guid_like_cpp(&self) -> Option<u64>;
        fn schedule_represented_resurrection_after_teleport_like_cpp(
            &mut self, request: PlayerResurrectionRequestLikeCpp
        ) -> bool;
        fn send_represented_cinematic_start_like_cpp(&mut self, cinematic_id: u32);
        fn set_player_alive_like_cpp(&mut self, alive: bool);
        fn set_player_ghost_flag_like_cpp(&mut self, ghost: bool);
        fn take_represented_resurrection_request_if_requested_by_like_cpp(
            &mut self, resurrecter: ObjectGuid
        ) -> Option<PlayerResurrectionRequestLikeCpp>;
        fn teleport_to(
            &mut self, new_map: u32, new_pos: wow_core::Position
        ) -> impl Future<Output = ()> + Send;
        fn use_represented_gameobject_questgiver_like_cpp(
            &mut self, gameobject_guid: ObjectGuid, player_guid: ObjectGuid, gameobject_entry: u32,
            source: wow_entities::QuestgiverUseSource
        ) -> bool;
    }
}
