// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The World operations the moved gameobject-use and player-attack bodies
//! still call (#1363 slice 4a).
//!
//! C++ `GameObject::Use` (`Entities/GameObject/GameObject.cpp`) and
//! `WorldSession::HandleAttackSwingOpcode` (`Handlers/CombatHandler.cpp`) run
//! on the Player. Their Rust bodies now live in this crate; each method below
//! is the same-named `WorldSession` operation, called at the exact point the
//! body called it before the move. Loot opening, quest credit and menus, auras,
//! groups, duels, map selection and the cfg(test) fixture steps stay with the
//! host.

use std::future::Future;

use wow_constants::unit::UnitPvpFlags;
use wow_core::{ObjectGuid, Position};
use wow_entities::{GameObjectLootSource, GatheringNodeUseSource, PhaseShift};
use wow_world_core::session::{
    HubMut, HubRef, ItemValuationCatalogsLikeCpp, SessionHubHostLikeCpp,
};

use crate::WorldEntitiesState;

pub trait WorldEntitiesHostLikeCpp: SessionHubHostLikeCpp {
    fn world_entities(&self) -> &WorldEntitiesState;
    fn world_entities_mut(&mut self) -> &mut WorldEntitiesState;
    fn split_world_entities_ref(&self) -> (&WorldEntitiesState, HubRef<'_>);
    fn split_world_entities_mut(&mut self) -> (&mut WorldEntitiesState, HubMut<'_>);
    fn queue_visible_gameobject_packet_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
        packet_bytes: Vec<u8>,
    ) -> usize;
    fn queue_goober_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize;
    fn can_see_phase_shift_like_cpp(&self, other: &PhaseShift) -> bool;
    /// The loot state's `remove_cached_loot_for_owner_like_cpp`.
    fn remove_cached_loot_for_owner_like_cpp(&mut self, owner: ObjectGuid);
    fn queue_session_chest_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize;
    fn kill_credit_gameobject_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        gameobject_entry: u32,
        gameobject_guid: wow_core::ObjectGuid,
    ) -> impl Future<Output = ()> + Send;
    fn open_represented_fishing_hole_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        loot_id: u32,
    ) -> impl Future<Output = ()> + Send;
    fn open_represented_fishing_node_loot_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        area_id: u32,
        junk: bool,
    ) -> impl Future<Output = ()> + Send;
    fn open_represented_gameobject_chest_with_template_money_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
        template_money: (u32, u32),
    ) -> impl Future<Output = ()> + Send;
    fn open_represented_gathering_node_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        source: GatheringNodeUseSource,
    ) -> impl Future<Output = ()> + Send;
    fn remove_aura(&mut self, slot: u8) -> Result<(), &'static str>;
    fn remove_represented_mounted_auras_by_type_like_cpp(&mut self) -> bool;
    fn represented_meets_player_condition_id_like_cpp(&self, player_condition_id: u32) -> bool;
    fn represented_player_at_group_reward_distance_like_cpp(
        &self,
        player_guid: ObjectGuid,
        reward_map_id: u16,
        reward_position: Position,
    ) -> bool;
    fn represented_player_is_same_raid_with_like_cpp(
        &self,
        player_guid: ObjectGuid,
        owner_guid: ObjectGuid,
    ) -> bool;
    fn represented_player_quest_status_like_cpp(&self, quest_id: u32) -> Option<Option<u8>>;
    fn resolved_group_guid_like_cpp(&self) -> Option<u64>;
    fn send_represented_cinematic_start_like_cpp(&mut self, cinematic_id: u32);
    fn use_represented_gameobject_questgiver_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        source: wow_entities::QuestgiverUseSource,
    ) -> bool;
    fn record_represented_gameobject_template_quest_source_like_cpp(
        &mut self,
        guid: ObjectGuid,
        template: &wow_entities::GameObjectTemplateData,
    );
    fn represented_unit_values_update_to_update_object_like_cpp(
        &self,
        unit_guid: ObjectGuid,
        map_id: u16,
        values_update: &wow_entities::UnitValuesUpdate,
    ) -> Option<wow_packet::packets::update::UpdateObject>;
    fn represented_player_phase_shift_like_cpp(&self) -> Option<PhaseShift>;
    fn ensure_canonical_world_map_for_current_player_like_cpp(
        &mut self,
    ) -> Option<wow_map::CreateMapDecision>;
    fn current_player_is_in_group_guid_like_cpp(
        &self,
        current_group_guid: Option<u64>,
        group_owner: ObjectGuid,
    ) -> bool;
    fn canonical_player_duel_in_progress_like_cpp(
        &self,
        guid: ObjectGuid,
        opponent: ObjectGuid,
    ) -> Option<bool>;
    fn current_player_is_group_visible_for_owner_like_cpp(
        &self,
        current_group_guid: Option<u64>,
        owner_guid: ObjectGuid,
    ) -> bool;
    fn attacker_pvp_flags_test_fallback_like_cpp(
        &self,
        player_guid: Option<ObjectGuid>,
    ) -> Option<UnitPvpFlags>;
}
