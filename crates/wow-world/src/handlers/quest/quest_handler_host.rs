// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the quest packet entry points that moved to
//! `wow_world_quest` (#1363 slice 6c). Each session method forwards to the
//! same-named session field, split or operation; the registrar's catalog view
//! lends the three fields its legacy closures destructured.

use wow_packet::packets::gossip::GossipMessage;
use wow_world_core::session::HubMut;
use wow_world_entities::WorldEntitiesState;
use wow_world_quest::{QuestHandlerHostLikeCpp, QuestHandlerSessionHostLikeCpp, SessionQuestState};

use super::*;
use crate::session::SessionHandlerCatalogsLikeCpp;

impl QuestHandlerSessionHostLikeCpp for WorldSession {
    fn quest_state_mut(&mut self) -> &mut SessionQuestState {
        &mut self.quest_state
    }
    fn world_entities(&self) -> &WorldEntitiesState {
        &self.world_entities
    }
    fn split_world_entities_mut(&mut self) -> (&mut WorldEntitiesState, HubMut<'_>) {
        crate::session::split_world_entities_mut(self)
    }
    fn resolved_group_guid_like_cpp(&self) -> Option<u64> {
        wow_world_instances::group_membership::resolved_group_guid_like_cpp(self)
    }
    fn build_gossip_menu(
        &mut self,
        entry: u32,
        npc_flags: u32,
        npc_guid: ObjectGuid,
    ) -> impl Future<Output = Option<GossipMessage>> + Send {
        wow_world_character::build_gossip_menu(self, entry, npc_flags, npc_guid)
    }
    fn send_represented_creature_trainer_gossip_menu_like_cpp(
        &mut self,
        npc_guid: ObjectGuid,
        entry: u32,
        npc_flags: u32,
    ) -> bool {
        wow_world_character::send_represented_creature_trainer_gossip_menu_like_cpp(
            self, npc_guid, entry, npc_flags,
        )
    }
    fn record_represented_adventure_map_start_quest_like_cpp(
        &mut self,
        request: RepresentedAdventureMapStartQuestLikeCpp,
    ) {
        WorldSession::record_represented_adventure_map_start_quest_like_cpp(self, request)
    }
    fn reward_represented_quest_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
        choice: QuestChoiceItemLikeCpp,
    ) -> impl Future<Output = bool> + Send {
        WorldSession::reward_represented_quest_with_generator_like_cpp(
            self,
            item_guid_generator,
            quest,
            quest_giver_guid,
            choice,
        )
    }
    fn drain_represented_quest_objective_progress_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send {
        WorldSession::drain_represented_quest_objective_progress_with_generator_like_cpp(
            self,
            item_guid_generator,
        )
    }
}

impl QuestHandlerHostLikeCpp<SessionHandlerCatalogsLikeCpp> for WorldSession {
    fn adventure_map_pois_like_cpp(
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) -> &wow_data::AdventureMapPoiStore {
        catalogs.adventure_map_pois.as_ref()
    }
    fn quest_info_like_cpp(
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) -> &wow_data::progression_rewards::QuestInfoStore {
        catalogs.quest_info.as_ref()
    }
    fn item_guid_generator_like_cpp(
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) -> &wow_core::ObjectGuidGenerator {
        catalogs.id_generators.item.as_ref()
    }
}
