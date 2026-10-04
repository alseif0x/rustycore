// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Buy admission before the dirty-save and exclusive-money awaits.
//! Target NPCHandler.cpp:132–202 validates NPC, removes FeignDeath, then
//! checks trainer-window provenance. Durable acquisition remains a later phase.

use super::controller::{find_trainer_buy_spell_like_cpp, TrainerBuyAdmissionLikeCpp};
use super::TRAINER_BUY_NPC_FLAGS_LIKE_CPP;
use crate::PlayerAuraApplicationCxLikeCpp;
use wow_data::TrainerStoreLikeCpp;
use wow_packet::{ClientPacket, WorldPacket};
use wow_packet::packets::trainer::{TrainerBuyFailed, TrainerBuySpellRequest};
use wow_world_core::session::{
    AuraNpcAccessBuilderLikeCpp, PacketPublicationAccessLikeCpp,
    TrainerInteractionRoleAccessLikeCpp,
};
use wow_world_interaction::InteractionState;

/// Selected, inert admission participants. No acquisition owner is retained
/// alongside mutable Aura; canonical interaction reads use the shared role.
pub struct AppTrainerBuyAdmissionCxLikeCpp<'a> {
    aura: PlayerAuraApplicationCxLikeCpp<'a>,
    npc: AuraNpcAccessBuilderLikeCpp<'a>,
    interaction: &'a InteractionState,
    role: TrainerInteractionRoleAccessLikeCpp<'a>,
    trainer_store: Option<&'a TrainerStoreLikeCpp>,
    publication: PacketPublicationAccessLikeCpp<'a>,
    account_id: u32,
}

impl<'a> AppTrainerBuyAdmissionCxLikeCpp<'a> {
    pub fn new(
        aura: PlayerAuraApplicationCxLikeCpp<'a>,
        npc: AuraNpcAccessBuilderLikeCpp<'a>,
        interaction: &'a InteractionState,
        role: TrainerInteractionRoleAccessLikeCpp<'a>,
        trainer_store: Option<&'a TrainerStoreLikeCpp>,
        publication: PacketPublicationAccessLikeCpp<'a>,
        account_id: u32,
    ) -> Self {
        Self { aura, npc, interaction, role, trainer_store, publication, account_id }
    }

    pub fn admit_packet_like_cpp(&mut self, mut pkt: WorldPacket) -> Option<TrainerBuySpellRequest> {
        let req = match TrainerBuySpellRequest::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(
                    account = self.account_id,
                    "Failed to parse CMSG_TRAINER_BUY_SPELL: {e}"
                );
                return None;
            }
        };
        let trainer_guid = req.trainer_guid;
        let trainer_id = req.trainer_id;
        let spell_id = req.spell_id;
        tracing::info!(
            account = self.account_id, trainer_id = trainer_id, spell_id = spell_id,
            "CMSG_TRAINER_BUY_SPELL"
        );

        let Some(_access) = self.aura.trainer_npc_view_like_cpp(&self.npc)
            .represented_npc_can_interact_with_like_cpp(
                trainer_guid, TRAINER_BUY_NPC_FLAGS_LIKE_CPP, 0,
            )
        else {
            tracing::warn!(
                account = self.account_id, trainer_guid = ?trainer_guid,
                "Trainer buy rejected: trainer not interactable"
            );
            return None;
        };

        self.aura.remove_represented_feign_death_if_needed_like_cpp();

        // Fresh provenance precedes the immutable store/member lookup.
        let admission = find_trainer_buy_spell_like_cpp(
            self.interaction, &self.role, self.trainer_store,
            trainer_guid, trainer_id, spell_id as u32,
        ).map(|_| ());
        match admission {
            Err(TrainerBuyAdmissionLikeCpp::InteractionMismatch) => {
                tracing::warn!(
                    account = self.account_id,
                    trainer_guid = ?trainer_guid,
                    trainer_id = trainer_id,
                    active_source = ?self.interaction.player_interaction_source_guid_with_access_like_cpp(
                        &self.aura.trainer_npc_view_like_cpp(&self.npc),
                    ),
                    active_trainer_id = ?self.interaction.resolved_player_interaction_trainer_id_with_access_like_cpp(
                        &self.aura.trainer_npc_view_like_cpp(&self.npc),
                    ),
                    "Trainer buy rejected: active trainer interaction mismatch"
                );
                None
            }
            Err(TrainerBuyAdmissionLikeCpp::TrainerStoreUnavailable
                | TrainerBuyAdmissionLikeCpp::TrainerUnavailable) => None,
            Err(TrainerBuyAdmissionLikeCpp::SpellUnavailable) => {
                tracing::warn!(
                    account = self.account_id, trainer_id = trainer_id, spell_id = spell_id,
                    "Spell not in trainer's loaded C++ spell set"
                );
                self.publication.send_packet_realm(&TrainerBuyFailed {
                    trainer_guid, spell_id, reason: 0,
                });
                None
            }
            Ok(()) => Some(req),
        }
    }
}
