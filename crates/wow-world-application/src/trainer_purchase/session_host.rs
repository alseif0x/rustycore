// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The World session as the host of the trainer contexts (#1363 slice 12a),
//! and the `CMSG_TRAINER_LIST` entry points and offer decision moved with them
//! from the World `handlers/trainer.rs`. C++ refs:
//! `WorldSession::HandleTrainerListOpcode` / `SendTrainerList`
//! (`Handlers/NPCHandler.cpp:98-132`).

use wow_world_core::session::{HubMut, SessionHubHostLikeCpp};
use wow_world_instances::InstanceState;
use wow_world_interaction::InteractionState;
use wow_world_inventory::InventoryState;
use wow_world_lifecycle::SessionLifecycleState;
use wow_world_loot::LootState;
use wow_world_quest::SessionQuestState;
use wow_world_social::SessionSocialLimits;
use wow_world_spell::SessionSpellState;

use super::TrainerOfferDecisionLikeCpp;
use super::session_context::trainer_list_publication_context_like_cpp;

/// The World session fields the trainer contexts borrow, split once.
pub struct TrainerSessionFieldsLikeCpp<'a> {
    pub hub: HubMut<'a>,
    pub lifecycle: &'a mut SessionLifecycleState,
    pub inventory: &'a mut InventoryState,
    pub spell_state: &'a mut SessionSpellState,
    pub quest_state: &'a mut SessionQuestState,
    pub loot: &'a LootState,
    pub interaction: &'a mut InteractionState,
    pub social: &'a SessionSocialLimits,
    pub instances: &'a InstanceState,
}

pub trait TrainerSessionHostLikeCpp: SessionHubHostLikeCpp {
    /// The disjoint field borrows of the World session.
    fn split_trainer_mut(&mut self) -> TrainerSessionFieldsLikeCpp<'_>;
    /// The World crate's `cfg!(test)`: only the World test build's session
    /// returns its fixtures from `test_fixtures_ref_like_cpp`.
    fn world_test_consumer_like_cpp(&self) -> bool {
        #[cfg(any(test, feature = "test-fixtures"))]
        let world_test = self.test_fixtures_ref_like_cpp().is_some();
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let world_test = false;
        world_test
    }
}

pub fn trainer_offer_decision_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
    trainer_id: u32,
    trainer_spell: &wow_data::TrainerSpellLikeCpp,
    faction_template_id: u32,
) -> TrainerOfferDecisionLikeCpp {
    trainer_list_publication_context_like_cpp(host).trainer_offer_decision_like_cpp(
        trainer_id,
        trainer_spell,
        faction_template_id,
    )
}

/// Handle `CMSG_TRAINER_LIST` (0x34ad).
///
/// Opens the trainer window: resolves the NPC, reads the loaded trainer store,
/// and sends SMSG_TRAINER_LIST back to the client.
pub async fn handle_trainer_list<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
    hello: wow_packet::packets::gossip::Hello,
) {
    send_trainer_list_like_cpp(host, hello, None);
}

pub async fn handle_trainer_list_for_gossip_option_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
    hello: wow_packet::packets::gossip::Hello,
    gossip_menu_id: u32,
    gossip_option_id: u32,
) {
    send_trainer_list_like_cpp(host, hello, Some((gossip_menu_id, gossip_option_id)));
}

fn send_trainer_list_like_cpp<H: TrainerSessionHostLikeCpp>(
    host: &mut H,
    hello: wow_packet::packets::gossip::Hello,
    gossip_option: Option<(u32, u32)>,
) {
    trainer_list_publication_context_like_cpp(host)
        .send_trainer_list_like_cpp(hello, gossip_option);
}
