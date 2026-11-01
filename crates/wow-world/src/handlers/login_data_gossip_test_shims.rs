// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the login data and gossip bodies moved to
//! `wow_world_character` (#1363 slice 8e). Each forwards to the moved body with
//! this session as its host, so the scenarios keep their method syntax and the
//! character hotspot's test lines do not grow. `handle_binder_activate` came
//! verbatim from the `cfg(test)` method of `handlers/character/gossip.rs` (it
//! now names the moved free function).
//!
//! The one-statement wrappers at the end came verbatim from
//! `session/{spell_state/cooldown,spell_state/spell,npc_interaction}.rs`: their
//! only production callers were the moved bodies, which now make the same split
//! and state call through the host.

#![cfg(test)]

use wow_core::ObjectGuid;
use wow_packet::packets::gossip::{GossipSelectOption, Hello};
use wow_packet::packets::misc::{SpellChargeEntry, SpellHistoryEntry};
use wow_packet::packets::update::TraitConfigCreateData;

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) async fn handle_binder_activate(&mut self, hello: Hello) {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_character::handle_binder_activate_with_generator_like_cpp(
            self,
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            hello,
        )
        .await;
    }

    pub(crate) async fn handle_gossip_hello(&mut self, hello: Hello) {
        wow_world_character::handle_gossip_hello(self, hello).await
    }

    pub(crate) async fn handle_gossip_select_option(&mut self, select: GossipSelectOption) {
        wow_world_character::handle_gossip_select_option(self, select).await
    }

    pub(crate) fn skill_rewarded_spell_changes_for_login_like_cpp(
        &self,
        skill_id: u16,
        skill_value: u16,
        race: u8,
        class: u8,
        level: u8,
    ) -> wow_data::SkillRewardedSpellChangesLikeCpp {
        wow_world_character::skill_rewarded_spell_changes_for_login_like_cpp(
            self,
            skill_id,
            skill_value,
            race,
            class,
            level,
        )
    }

    pub(crate) async fn load_character_spell_history_packets_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> (Vec<SpellHistoryEntry>, Vec<SpellChargeEntry>) {
        wow_world_character::load_character_spell_history_packets_like_cpp(self, guid).await
    }

    pub(crate) async fn load_active_player_trait_configs_like_cpp(
        &mut self,
        node_entries: &wow_data::trait_tree::TraitNodeEntryStore,
        guid: ObjectGuid,
    ) -> Vec<TraitConfigCreateData> {
        wow_world_character::load_active_player_trait_configs_like_cpp(self, node_entries, guid)
            .await
    }

    pub(crate) fn mark_represented_character_spell_cooldowns_loaded_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.mark_represented_character_spell_cooldowns_loaded_like_cpp(&mut hub)
    }
    pub(crate) fn record_loaded_character_spell_cooldown_like_cpp(
        &mut self,
        spell_id: u32,
        item_id: u32,
        cooldown_end_unix_secs: i64,
        category_id: u32,
        category_end_unix_secs: i64,
    ) {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.record_loaded_character_spell_cooldown_like_cpp(
            &mut hub,
            spell_id,
            item_id,
            cooldown_end_unix_secs,
            category_id,
            category_end_unix_secs,
        )
    }
    pub(crate) fn mark_represented_character_spell_charges_loaded_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.mark_represented_character_spell_charges_loaded_like_cpp(&mut hub)
    }
    pub(crate) fn record_loaded_character_spell_charge_like_cpp(
        &mut self,
        category_id: u32,
        recharge_start_unix_secs: i64,
        recharge_end_unix_secs: i64,
    ) {
        let (state, mut hub) = crate::session::split_spell_state_mut(self);
        state.record_loaded_character_spell_charge_like_cpp(
            &mut hub,
            category_id,
            recharge_start_unix_secs,
            recharge_end_unix_secs,
        )
    }
    pub(crate) fn set_player_interaction_source_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
    ) -> bool {
        let (state, hub) = crate::session::split_interaction(self);
        state.set_player_interaction_source_like_cpp(hub, source_guid)
    }
}
