// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Trainer handlers: CMSG_TRAINER_LIST, CMSG_TRAINER_BUY_SPELL.
//!
//! Flow for CMSG_TRAINER_LIST:
//!   1. Parse trainer GUID from packet.
//!   2. Resolve creature entry (NPC template ID) from the live map.
//!   3. Look up TrainerId in the process-wide C++-shaped trainer store.
//!   4. Read the trainer spells from that same immutable store snapshot.
//!   5. Determine usability per spell (known / available / unavailable).
//!   6. Send SMSG_TRAINER_LIST.
//!
//! Flow for CMSG_TRAINER_BUY_SPELL:
//!   1. Parse trainer GUID, trainer ID, spell ID.
//!   2. Recompute the same immutable offer from the current snapshot.
//!   3. Reject an unavailable offer or insufficient exact discounted money.
//!   4. Stop without mutation; #158/#159 own apply/persistence/publication.
//!
//! C++ refs: `WorldSession::HandleTrainerListOpcode` / `SendTrainerList`
//! (`Handlers/NPCHandler.cpp:98-132`) and `Trainer::SendSpells` /
//! `Trainer::TeachSpell` (`Entities/Creature/Trainer.cpp:41-231`).

use std::sync::Arc;

use tracing::{info, warn};

use wow_constants::ClientOpcodes;
use wow_constants::unit::NPCFlags1;
use wow_data::{
    BattlePetClassificationLikeCpp, SkillLineAbilityCoverageLikeCpp,
    SkillRaceClassInfoMatchCoverageLikeCpp, SpellAcquisitionEffectsLookupLikeCpp,
    TRAINER_SPELL_STATE_AVAILABLE_LIKE_CPP, TRAINER_SPELL_STATE_KNOWN_LIKE_CPP,
    TRAINER_SPELL_STATE_UNAVAILABLE_LIKE_CPP, TrainerLikeCpp, TrainerStoreLikeCpp,
};
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::ClientPacket;
use wow_packet::packets::trainer::{
    TrainerBuyFailed, TrainerBuySpellRequest, TrainerListPacket, TrainerListSpell,
};

use crate::conditions;
use crate::session::WorldSession;
use crate::trainer_offer::{
    TrainerAdmissionProofLikeCpp, TrainerBattlePetProofLikeCpp, TrainerOfferDecisionLikeCpp,
    TrainerOfferInputLikeCpp, TrainerProductLikeCpp, decide_trainer_offer_like_cpp,
    trainer_price_like_cpp,
};

const TRAINER_LIST_NPC_FLAGS_LIKE_CPP: u32 = NPCFlags1::TRAINER.bits();
const TRAINER_BUY_NPC_FLAGS_LIKE_CPP: u32 = NPCFlags1::TRAINER.bits()
    | NPCFlags1::TRAINER_CLASS.bits()
    | NPCFlags1::TRAINER_PROFESSION.bits();
const TRAINER_GOSSIP_NPC_FLAGS_LIKE_CPP: u32 =
    NPCFlags1::GOSSIP.bits() | TRAINER_BUY_NPC_FLAGS_LIKE_CPP;

fn resolve_creature_trainer_like_cpp<'a>(
    store: &'a TrainerStoreLikeCpp,
    entry: u32,
    gossip_option: Option<(u32, u32)>,
) -> Option<&'a TrainerLikeCpp> {
    let selected_trainer_id = gossip_option
        .map(|(menu_id, option_id)| {
            store.get_creature_trainer_for_gossip_option_like_cpp(entry, menu_id, option_id)
        })
        .unwrap_or(0);
    let trainer_id = if selected_trainer_id != 0 {
        selected_trainer_id
    } else {
        // Target-fork C++ fallback in `Player::OnGossipSelect`.
        store.get_creature_default_trainer_like_cpp(entry)
    };
    if trainer_id == 0 {
        return None;
    }
    store.get_trainer_like_cpp(trainer_id)
}

fn trainer_list_required_npc_flags_like_cpp(gossip_option: Option<(u32, u32)>) -> u32 {
    if gossip_option.is_some() {
        // C++ validates normal selections through UNIT_NPC_FLAG_GOSSIP. This
        // legacy fork also opens a generated trainer option for trainer-only
        // creatures, so retain either proven interaction route.
        TRAINER_GOSSIP_NPC_FLAGS_LIKE_CPP
    } else {
        TRAINER_LIST_NPC_FLAGS_LIKE_CPP
    }
}

fn trainer_spell_class_race_fit_like_cpp(
    session: &WorldSession,
    spell_id: u32,
) -> TrainerAdmissionProofLikeCpp {
    let Some(skills) = session.skill_store() else {
        return TrainerAdmissionProofLikeCpp::Indeterminate;
    };
    let Ok(spell_id) = i32::try_from(spell_id) else {
        return TrainerAdmissionProofLikeCpp::Indeterminate;
    };
    let rows = match skills.skill_line_ability_coverage_by_spell_like_cpp(spell_id) {
        SkillLineAbilityCoverageLikeCpp::CoveredZero => {
            return TrainerAdmissionProofLikeCpp::Proven(true);
        }
        SkillLineAbilityCoverageLikeCpp::Indeterminate(_) => {
            return TrainerAdmissionProofLikeCpp::Indeterminate;
        }
        SkillLineAbilityCoverageLikeCpp::Rows(rows) => rows,
    };
    let race = session.player_race_like_cpp();
    let class = session.player_class_like_cpp();
    let race_mask = wow_data::skill::race_mask_for_race_like_cpp(race);
    let Some(class_mask) = class
        .checked_sub(1)
        .and_then(|bit| 1_i32.checked_shl(bit.into()))
    else {
        return TrainerAdmissionProofLikeCpp::Indeterminate;
    };
    if race_mask == 0 {
        return TrainerAdmissionProofLikeCpp::Indeterminate;
    }
    let mut indeterminate = false;
    for row in rows {
        if row.race_mask != 0 && row.race_mask & race_mask == 0 {
            continue;
        }
        if row.class_mask != 0 && row.class_mask & class_mask == 0 {
            continue;
        }
        match skills.skill_race_class_info_coverage_for_player_like_cpp(row.skill_line, race, class)
        {
            SkillRaceClassInfoMatchCoverageLikeCpp::Row(_) => {
                return TrainerAdmissionProofLikeCpp::Proven(true);
            }
            SkillRaceClassInfoMatchCoverageLikeCpp::CoveredZero => {}
            SkillRaceClassInfoMatchCoverageLikeCpp::Indeterminate(_) => indeterminate = true,
        }
    }
    if indeterminate {
        TrainerAdmissionProofLikeCpp::Indeterminate
    } else {
        TrainerAdmissionProofLikeCpp::Proven(false)
    }
}

fn trainer_spell_product_like_cpp(session: &WorldSession, spell_id: u32) -> TrainerProductLikeCpp {
    const SPELL_EFFECT_LEARN_SPELL_LIKE_CPP: u32 = 36;
    let Some(catalog) = session.spell_acquisition_catalog() else {
        return TrainerProductLikeCpp::InvalidOrUnsupportedWrapper;
    };
    let effects = match catalog.acquisition_effects_like_cpp(spell_id) {
        SpellAcquisitionEffectsLookupLikeCpp::Covered(effects) => effects,
        SpellAcquisitionEffectsLookupLikeCpp::MissingCoverage
        | SpellAcquisitionEffectsLookupLikeCpp::Indeterminate(_) => {
            return TrainerProductLikeCpp::InvalidOrUnsupportedWrapper;
        }
    };
    let mut saw_learn = false;
    let mut targets = Vec::new();
    for effect in effects {
        let Ok(effect_type) = effect.effect_type_checked() else {
            return TrainerProductLikeCpp::InvalidOrUnsupportedWrapper;
        };
        if effect_type != SPELL_EFFECT_LEARN_SPELL_LIKE_CPP {
            continue;
        }
        saw_learn = true;
        // C++ casts the wrapper on the player. Pet and other explicit target
        // families are not valid player-learning evidence.
        if effect.targets_unit_pet_like_cpp() || !matches!(effect.implicit_target_raw[0], 0 | 1) {
            return TrainerProductLikeCpp::InvalidOrUnsupportedWrapper;
        }
        let Ok(target) = effect.trigger_spell_id_checked() else {
            return TrainerProductLikeCpp::InvalidOrUnsupportedWrapper;
        };
        targets.push(target);
    }
    targets.sort_unstable();
    targets.dedup();
    if saw_learn {
        TrainerProductLikeCpp::Wrapper {
            valid_learn_targets: targets,
        }
    } else {
        TrainerProductLikeCpp::Direct
    }
}

// ── Handler registrations ─────────────────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TrainerList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_trainer_list",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TrainerBuySpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_trainer_buy_spell",
    }
}

// ── Handler implementations ───────────────────────────────────────────────────

impl WorldSession {
    fn trainer_spell_condition_proof_like_cpp(
        &self,
        trainer_id: u32,
        spell_id: u32,
    ) -> TrainerAdmissionProofLikeCpp {
        let Some(store) = self.condition_store() else {
            return TrainerAdmissionProofLikeCpp::Indeterminate;
        };
        let Some(player_object) = self.build_condition_player_object_like_cpp() else {
            return TrainerAdmissionProofLikeCpp::Indeterminate;
        };
        let player_condition_store = self.player_condition_store();
        let player_condition_context = self.represented_player_condition_context_like_cpp();
        let player_unit_snapshot = self.condition_player_unit_snapshot_like_cpp();
        let player_snapshot = self.condition_player_snapshot_like_cpp();
        let mut unsupported = false;
        let meets = conditions::is_object_meeting_trainer_spell_conditions_like_cpp(
            store.as_ref(),
            trainer_id,
            spell_id,
            Some(&player_object),
            |condition, source_info| {
                source_info.set_unit_target_snapshot(0, player_unit_snapshot);
                source_info.set_player_target_snapshot(0, player_snapshot);
                if let Some(store) = player_condition_store {
                    source_info.set_player_condition_store(store.as_ref());
                    source_info
                        .set_player_condition_context(0, player_condition_context.as_context(self));
                }
                match conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |current_area, required_area| current_area == required_area,
                ) {
                    conditions::ConditionMeetResult::Evaluated(value) => value,
                    conditions::ConditionMeetResult::Unsupported => {
                        unsupported = true;
                        false
                    }
                }
            },
        );
        if unsupported {
            TrainerAdmissionProofLikeCpp::Indeterminate
        } else {
            TrainerAdmissionProofLikeCpp::Proven(meets)
        }
    }

    fn trainer_offer_decision_like_cpp(
        &self,
        trainer_id: u32,
        trainer_spell: &wow_data::TrainerSpellLikeCpp,
        faction_template_id: u32,
    ) -> TrainerOfferDecisionLikeCpp {
        if i32::try_from(trainer_spell.spell_id).is_err() {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                crate::trainer_offer::TrainerUnavailableReasonLikeCpp::InvalidEffectiveMetadata,
            );
        }
        let Some(spell_rows) = self.complete_represented_player_spell_rows_like_cpp() else {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                crate::trainer_offer::TrainerUnavailableReasonLikeCpp::InvalidEffectiveMetadata,
            );
        };
        let Some(skill_rows) = self.complete_player_skill_records_like_cpp() else {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                crate::trainer_offer::TrainerUnavailableReasonLikeCpp::InvalidEffectiveMetadata,
            );
        };
        let required_skill = if trainer_spell.req_skill_line == 0 {
            None
        } else {
            let (Ok(skill_id), Ok(rank)) = (
                u16::try_from(trainer_spell.req_skill_line),
                u16::try_from(trainer_spell.req_skill_rank),
            ) else {
                return TrainerOfferDecisionLikeCpp::Unavailable(
                    crate::trainer_offer::TrainerUnavailableReasonLikeCpp::InvalidEffectiveMetadata,
                );
            };
            Some((u32::from(skill_id), rank))
        };
        let skill_value = |skill_id: u32| {
            u16::try_from(skill_id)
                .ok()
                .and_then(|skill_id| skill_rows.get(&skill_id).map(|row| row.value))
        };
        let knows_spell = |candidate: u32| {
            i32::try_from(candidate).ok().is_some_and(|candidate| {
                spell_rows.get(&candidate).is_some_and(|row| {
                    row.state != crate::session::RepresentedPlayerSpellStateLikeCpp::Removed
                        && !row.disabled
                })
            })
        };
        let battle_pet = match self
            .spell_acquisition_catalog()
            .map(|catalog| catalog.battle_pet_classification_like_cpp(trainer_spell.spell_id))
        {
            Some(BattlePetClassificationLikeCpp::NotBattlePet) => {
                TrainerBattlePetProofLikeCpp::NotBattlePet
            }
            Some(BattlePetClassificationLikeCpp::Species(species_id)) => {
                TrainerBattlePetProofLikeCpp::Species(species_id)
            }
            Some(BattlePetClassificationLikeCpp::Indeterminate(_)) | None => {
                TrainerBattlePetProofLikeCpp::Indeterminate
            }
        };
        let effective_price = trainer_price_like_cpp(
            trainer_spell.money_cost,
            self.trainer_price_reputation_rank_like_cpp(faction_template_id),
        );
        decide_trainer_offer_like_cpp(
            TrainerOfferInputLikeCpp {
                source_spell_id: trainer_spell.spell_id,
                is_exact_member: true,
                class_race: trainer_spell_class_race_fit_like_cpp(self, trainer_spell.spell_id),
                condition: self
                    .trainer_spell_condition_proof_like_cpp(trainer_id, trainer_spell.spell_id),
                directly_known: knows_spell(trainer_spell.spell_id),
                required_skill,
                skill_value: &skill_value,
                required_abilities: trainer_spell.req_ability,
                knows_spell: &knows_spell,
                required_level: trainer_spell.req_level,
                player_level: self.player_level_like_cpp(),
                product: trainer_spell_product_like_cpp(self, trainer_spell.spell_id),
                battle_pet,
                effective_price,
            },
            |root| self.project_trainer_spell_acquisition_like_cpp(root),
            |skills| self.plan_primary_profession_capacity_like_cpp(skills.iter().copied()),
        )
    }

    /// Handle `CMSG_TRAINER_LIST` (0x34ad).
    ///
    /// Opens the trainer window: resolves the NPC, reads the loaded trainer store,
    /// and sends SMSG_TRAINER_LIST back to the client.
    pub async fn handle_trainer_list(&mut self, hello: wow_packet::packets::gossip::Hello) {
        self.send_trainer_list_like_cpp(hello, None);
    }

    pub(crate) async fn handle_trainer_list_for_gossip_option_like_cpp(
        &mut self,
        hello: wow_packet::packets::gossip::Hello,
        gossip_menu_id: u32,
        gossip_option_id: u32,
    ) {
        self.send_trainer_list_like_cpp(hello, Some((gossip_menu_id, gossip_option_id)));
    }

    fn send_trainer_list_like_cpp(
        &mut self,
        hello: wow_packet::packets::gossip::Hello,
        gossip_option: Option<(u32, u32)>,
    ) {
        let trainer_guid = hello.unit;
        info!(
            account = self.account_id,
            trainer_guid = ?trainer_guid,
            "CMSG_TRAINER_LIST"
        );

        let required_npc_flags = trainer_list_required_npc_flags_like_cpp(gossip_option);
        let access = match self.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            required_npc_flags,
            0,
        ) {
            Some(access) => access,
            None => {
                warn!(
                    account = self.account_id,
                    trainer_guid = ?trainer_guid,
                    "Trainer GUID not found or not interactable"
                );
                return;
            }
        };
        let entry = access.entry;

        let trainer_store = match self.trainer_store_like_cpp() {
            Some(store) => Arc::clone(store),
            None => return,
        };

        // C++ `LoadCreatureTrainers` has already discarded dangling mappings.
        // Keep resolution, spell reads, and locale reads on this one immutable
        // ObjectMgr-like snapshot instead of re-querying mutable world tables.
        let Some(trainer) =
            resolve_creature_trainer_like_cpp(trainer_store.as_ref(), entry, gossip_option)
        else {
            warn!(
                account = self.account_id,
                entry = entry,
                gossip_option = ?gossip_option,
                "No creature trainer in the loaded C++ trainer store"
            );
            return;
        };
        let trainer_id = trainer.id_like_cpp();

        // C++ `SendTrainerList` removes fake death after resolving a non-zero
        // creature trainer ID and before reading the trainer definition.
        self.remove_represented_feign_death_if_needed_like_cpp();

        let mut spells: Vec<TrainerListSpell> = Vec::new();

        for trainer_spell in trainer.spells_like_cpp() {
            let spell_id = trainer_spell.spell_id as i32;
            let decision = self.trainer_offer_decision_like_cpp(
                trainer_id,
                trainer_spell,
                access.faction_template_id,
            );
            let (usable, money_cost) = match decision {
                TrainerOfferDecisionLikeCpp::Hidden(_) => continue,
                TrainerOfferDecisionLikeCpp::Known(_) => (
                    TRAINER_SPELL_STATE_KNOWN_LIKE_CPP,
                    trainer_price_like_cpp(
                        trainer_spell.money_cost,
                        self.trainer_price_reputation_rank_like_cpp(access.faction_template_id),
                    ),
                ),
                TrainerOfferDecisionLikeCpp::Unavailable(_) => (
                    TRAINER_SPELL_STATE_UNAVAILABLE_LIKE_CPP,
                    trainer_price_like_cpp(
                        trainer_spell.money_cost,
                        self.trainer_price_reputation_rank_like_cpp(access.faction_template_id),
                    ),
                ),
                TrainerOfferDecisionLikeCpp::Available(offer) => (
                    TRAINER_SPELL_STATE_AVAILABLE_LIKE_CPP,
                    offer.effective_price,
                ),
            };

            spells.push(TrainerListSpell {
                spell_id,
                money_cost,
                req_skill_line: trainer_spell.req_skill_line as i32,
                req_skill_rank: trainer_spell.req_skill_rank as i32,
                req_ability: trainer_spell.req_ability.map(|ability| ability as i32),
                usable,
                req_level: trainer_spell.req_level,
            });
        }

        info!(
            account = self.account_id,
            trainer_id = trainer_id,
            spell_count = spells.len(),
            "Sending SMSG_TRAINER_LIST"
        );

        // C++ replaces the complete InteractionData immediately before
        // publishing the successfully resolved trainer list.
        self.set_player_trainer_interaction_like_cpp(trainer_guid, trainer_id);
        self.send_packet(&TrainerListPacket {
            trainer_guid,
            trainer_type: i32::from(trainer.trainer_type_like_cpp()),
            trainer_id: trainer_id as i32,
            spells,
            greeting: trainer
                .greeting_for_locale_name_like_cpp(self.session_locale_name_like_cpp())
                .to_string(),
        });
    }

    /// Handle `CMSG_TRAINER_BUY_SPELL` (0x34ae).
    ///
    /// Revalidates and prepares the same immutable offer used by the list.
    /// Teaching, charging, persistence and success publication belong to
    /// #158/#159 and are deliberately unreachable in this slice.
    pub async fn handle_trainer_buy_spell(&mut self, mut pkt: wow_packet::WorldPacket) {
        let req = match TrainerBuySpellRequest::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to parse CMSG_TRAINER_BUY_SPELL: {e}"
                );
                return;
            }
        };

        let trainer_guid = req.trainer_guid;
        let trainer_id = req.trainer_id;
        let spell_id = req.spell_id;

        info!(
            account = self.account_id,
            trainer_id = trainer_id,
            spell_id = spell_id,
            "CMSG_TRAINER_BUY_SPELL"
        );

        let Some(access) = self.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            TRAINER_BUY_NPC_FLAGS_LIKE_CPP,
            0,
        ) else {
            warn!(
                account = self.account_id,
                trainer_guid = ?trainer_guid,
                "Trainer buy rejected: trainer not interactable"
            );
            return;
        };

        // C++ removes fake death after validating the NPC and before checking
        // the active trainer-window provenance.
        self.remove_represented_feign_death_if_needed_like_cpp();

        if !self.player_trainer_interaction_matches_like_cpp(trainer_guid, trainer_id) {
            warn!(
                account = self.account_id,
                trainer_guid = ?trainer_guid,
                trainer_id = trainer_id,
                active_source = ?self.player_interaction_source_guid_like_cpp(),
                active_trainer_id = self.player_interaction_trainer_id_like_cpp(),
                "Trainer buy rejected: active trainer interaction mismatch"
            );
            return;
        }

        // C++ resolves the exact Trainer object before `Trainer::TeachSpell`
        // performs known/level/money checks. The process-wide store excludes
        // orphan trainer_spell rows, so a generic gossip binding with ID 0 or
        // any stale/nonexistent trainer ID fails silently here.
        let Some(trainer_store) = self.trainer_store_like_cpp() else {
            return;
        };
        let Some(trainer) = trainer_store.get_trainer_like_cpp(trainer_id as u32) else {
            return;
        };
        let Some(trainer_spell) = trainer.get_spell_like_cpp(spell_id as u32).cloned() else {
            warn!(
                account = self.account_id,
                trainer_id = trainer_id,
                spell_id = spell_id,
                "Spell not in trainer's loaded C++ spell set"
            );
            self.send_packet(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
            return;
        };
        let decision = self.trainer_offer_decision_like_cpp(
            trainer_id as u32,
            &trainer_spell,
            access.faction_template_id,
        );
        let TrainerOfferDecisionLikeCpp::Available(offer) = decision else {
            self.send_packet(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
            return;
        };
        if self.player_gold_like_cpp() < u64::from(offer.effective_price) {
            self.send_packet(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 1,
            });
            return;
        }

        info!(
            account = self.account_id,
            trainer_id,
            spell_id,
            effective_price = offer.effective_price,
            "Trainer offer prepared without mutation; apply belongs to #158/#159"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use super::*;
    use crate::session::{
        AuraApplication, RepresentedAuraEffectLikeCpp, RepresentedPlayerSpellLikeCpp,
        RepresentedPlayerSpellStateLikeCpp, SessionPlayerController,
    };
    use wow_constants::unit::UnitState;
    use wow_core::guid::HighGuid;
    use wow_core::{ObjectGuid, Position};
    use wow_data::{
        ConditionEntriesByTypeStore, CreatureTrainerRowLikeCpp,
        EffectiveSpellAcquisitionRowsLikeCpp, MountStore, SkillLineAbilityRecord, SkillLineStore,
        SkillRaceClassInfoRecord, SkillStore, SkillTiersStoreLikeCpp,
        SpellAcquisitionCatalogLikeCpp, SpellAcquisitionCoverageSeedLikeCpp,
        SpellAcquisitionTableHashesLikeCpp, SpellChainStoreLikeCpp,
        SpellCustomAttributeStoreLikeCpp, SpellLearnSkillStoreLikeCpp, SpellLearnSpellStoreLikeCpp,
        SpellRequiredStoreLikeCpp, TrainerLocaleRowLikeCpp, TrainerRowLikeCpp, TrainerSpellLikeCpp,
        TrainerSpellRowLikeCpp,
    };
    use wow_packet::{ServerPacket, WorldPacket};

    const CREATURE_ENTRY: u32 = 123;
    const DEFAULT_TRAINER_ID: u32 = 7;
    const KNOWN_TRAINER_SPELL: i32 = 54_321;
    const AVAILABLE_TRAINER_SPELL: i32 = 54_322;
    const UNAVAILABLE_TRAINER_SPELL: i32 = 54_323;

    fn trainer_row(id: u32, trainer_type: u8, greeting: &str) -> TrainerRowLikeCpp {
        TrainerRowLikeCpp {
            id,
            trainer_type,
            greeting: greeting.to_string(),
        }
    }

    fn trainer_spell_row(
        trainer_id: u32,
        spell_id: i32,
        money_cost: u32,
        req_level: u8,
    ) -> TrainerSpellRowLikeCpp {
        TrainerSpellRowLikeCpp {
            trainer_id,
            spell: TrainerSpellLikeCpp {
                spell_id: spell_id as u32,
                money_cost,
                req_skill_line: 0,
                req_skill_rank: 0,
                req_ability: [0; 3],
                req_level,
            },
        }
    }

    fn trainer_store_from_rows(
        trainer_rows: Vec<TrainerRowLikeCpp>,
        spell_rows: Vec<TrainerSpellRowLikeCpp>,
        locale_rows: Vec<TrainerLocaleRowLikeCpp>,
        creature_rows: Vec<CreatureTrainerRowLikeCpp>,
    ) -> Arc<TrainerStoreLikeCpp> {
        Arc::new(
            TrainerStoreLikeCpp::from_rows_like_cpp(
                trainer_rows,
                spell_rows,
                locale_rows,
                creature_rows,
                |_| true,
                |_| true,
                |_| true,
                |_, _| true,
            )
            .store,
        )
    }

    fn standard_trainer_store(trainer_id: u32) -> Arc<TrainerStoreLikeCpp> {
        trainer_store_from_rows(
            vec![trainer_row(trainer_id, 2, "Train")],
            vec![
                trainer_spell_row(trainer_id, KNOWN_TRAINER_SPELL, 10, 1),
                trainer_spell_row(trainer_id, AVAILABLE_TRAINER_SPELL, 20, 80),
                trainer_spell_row(trainer_id, UNAVAILABLE_TRAINER_SPELL, 30, 81),
            ],
            Vec::new(),
            vec![CreatureTrainerRowLikeCpp {
                creature_id: CREATURE_ENTRY,
                trainer_id,
                menu_id: 0,
                option_id: 0,
            }],
        )
    }

    fn make_session() -> (WorldSession, flume::Receiver<Vec<u8>>) {
        let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
        let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(32);
        (
            WorldSession::new(
                1,
                "TrainerTest".into(),
                0,
                2,
                9,
                54_261,
                vec![0; 40],
                "enUS".into(),
                pkt_rx,
                send_tx,
            ),
            send_rx,
        )
    }

    fn creature_guid(counter: u32) -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, counter, 1)
    }

    fn insert_canonical_creature(
        manager: &Arc<Mutex<wow_map::MapManager>>,
        guid: ObjectGuid,
        x: f32,
        npc_flags: u32,
    ) {
        let mut creature = wow_entities::Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .set_entry(CREATURE_ENTRY);
        creature.unit_mut().world_mut().set_map(0, 0).unwrap();
        creature
            .unit_mut()
            .world_mut()
            .relocate(Position::new(x, 0.0, 0.0, 0.0));
        creature.unit_mut().world_mut().set_combat_reach(1.0);
        creature.unit_mut().set_level(80);
        creature.unit_mut().set_max_health(100);
        creature.unit_mut().set_health(100);
        creature.set_ai_identity_runtime(1, 35, npc_flags, 0);
        creature.unit_mut().world_mut().object_mut().add_to_world();

        manager
            .lock()
            .unwrap()
            .find_map_mut(0, 0)
            .expect("canonical test map")
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_creature(creature).unwrap(),
            )
            .unwrap();
    }

    struct TrainerFixture {
        session: WorldSession,
        send_rx: flume::Receiver<Vec<u8>>,
        trainer: ObjectGuid,
        other_trainer: ObjectGuid,
        vendor: ObjectGuid,
    }

    fn trainer_fixture_with_store(store: Arc<TrainerStoreLikeCpp>) -> TrainerFixture {
        let (mut session, send_rx) = make_session();
        let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
        let trainer = creature_guid(100);
        let other_trainer = creature_guid(101);
        let vendor = creature_guid(102);

        session.set_canonical_map_manager(Arc::clone(&canonical));
        session.set_trainer_store_like_cpp(store);
        session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
            wow_data::MapEntry {
                id: 0,
                instance_type: wow_data::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
        ])));
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            ObjectGuid::create_player(1, 42),
            "TrainerTester".to_string(),
            Position::new(0.0, 0.0, 0.0, 0.0),
            0,
            1,
            1,
            80,
            0,
        ));
        session.set_condition_store(Arc::new(ConditionEntriesByTypeStore::default()));
        session.set_skill_store(Arc::new(
            SkillStore::from_skill_line_abilities_and_race_class_like_cpp([], []),
        ));
        session.set_skill_line_store(Arc::new(SkillLineStore::from_entries([])));
        session.set_skill_tiers_store(Arc::new(SkillTiersStoreLikeCpp::default()));
        session.set_trait_definition_store(Arc::new(
            wow_data::trait_tree::TraitDefinitionStore::from_entries([]),
        ));
        session.set_mount_store(Arc::new(MountStore::from_entries([])));
        session.set_spell_chain_store(Arc::new(SpellChainStoreLikeCpp::default()));
        session.set_spell_custom_attribute_store(Arc::new(
            SpellCustomAttributeStoreLikeCpp::default(),
        ));
        let mut learn_skills = SpellLearnSkillStoreLikeCpp::default();
        learn_skills.covered_spell_ids.extend([
            KNOWN_TRAINER_SPELL as u32,
            AVAILABLE_TRAINER_SPELL as u32,
            UNAVAILABLE_TRAINER_SPELL as u32,
        ]);
        session.set_spell_learn_skill_store(Arc::new(learn_skills));
        session.set_spell_learn_spell_store(Arc::new(SpellLearnSpellStoreLikeCpp::default()));
        session.set_spell_required_store(Arc::new(SpellRequiredStoreLikeCpp::default()));
        session.set_spell_acquisition_catalog(Arc::new(
            SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
                [
                    KNOWN_TRAINER_SPELL as u32,
                    AVAILABLE_TRAINER_SPELL as u32,
                    UNAVAILABLE_TRAINER_SPELL as u32,
                ]
                .map(|spell_id| SpellAcquisitionCoverageSeedLikeCpp::covered(spell_id, 0)),
                EffectiveSpellAcquisitionRowsLikeCpp::default(),
                SpellAcquisitionTableHashesLikeCpp::default(),
                Vec::new(),
            ),
        ));
        session.set_known_spells_like_cpp(vec![KNOWN_TRAINER_SPELL]);
        assert!(
            session.set_complete_represented_player_spell_rows_like_cpp([
                RepresentedPlayerSpellLikeCpp {
                    spell_id: KNOWN_TRAINER_SPELL,
                    active: true,
                    disabled: false,
                    dependent: false,
                    favorite: false,
                    state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
                },
            ])
        );
        assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
        assert!(session.set_complete_represented_override_spells_like_cpp([]));
        assert!(session.set_complete_player_skill_records_like_cpp(HashMap::new(), 0));
        session
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .expect("canonical player map");
        insert_canonical_creature(&canonical, trainer, 1.0, TRAINER_BUY_NPC_FLAGS_LIKE_CPP);
        insert_canonical_creature(
            &canonical,
            other_trainer,
            2.0,
            TRAINER_BUY_NPC_FLAGS_LIKE_CPP,
        );
        insert_canonical_creature(&canonical, vendor, 3.0, NPCFlags1::VENDOR.bits());

        TrainerFixture {
            session,
            send_rx,
            trainer,
            other_trainer,
            vendor,
        }
    }

    fn trainer_fixture() -> TrainerFixture {
        trainer_fixture_with_store(standard_trainer_store(DEFAULT_TRAINER_ID))
    }

    fn trainer_buy_packet(trainer_guid: ObjectGuid, trainer_id: i32, spell_id: i32) -> WorldPacket {
        let mut packet = WorldPacket::new_empty();
        packet.write_packed_guid(&trainer_guid);
        packet.write_int32(trainer_id);
        packet.write_int32(spell_id);
        packet.reset_read();
        packet
    }

    fn seed_feign_death(session: &mut WorldSession, slot: u8) {
        let player_guid = session.player_guid().expect("active player");
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit_mut().add_unit_state(UnitState::DIED.bits());
            })
            .expect("canonical player");
        session.visible_auras.insert(
            slot,
            AuraApplication {
                spell_id: 5384,
                caster_guid: player_guid,
                slot,
                duration_total: 0,
                duration_remaining: 0,
                stack_count: 1,
                aura_flags: 0,
                effect_mask: 1,
                aura_interrupt_flags: 0,
                aura_interrupt_flags2: 0,
                represented_effect: Some(RepresentedAuraEffectLikeCpp::FeignDeath),
                represented_amount: 0,
                represented_effect_amounts: Vec::new(),
                represented_misc_value: None,
                represented_multiplier: 1.0,
                applied_at: Instant::now(),
            },
        );
    }

    fn canonical_player_has_died_state(session: &mut WorldSession) -> bool {
        session
            .mutate_canonical_player_like_cpp(|player| {
                player.unit().has_unit_state(UnitState::DIED.bits())
            })
            .expect("canonical player")
    }

    #[test]
    fn trainer_store_resolves_selected_default_and_discards_dangling_mappings_like_cpp() {
        assert_eq!(
            trainer_list_required_npc_flags_like_cpp(None),
            TRAINER_LIST_NPC_FLAGS_LIKE_CPP
        );
        assert_eq!(
            trainer_list_required_npc_flags_like_cpp(Some((900, 3))),
            TRAINER_GOSSIP_NPC_FLAGS_LIKE_CPP
        );

        let store = trainer_store_from_rows(
            vec![trainer_row(7, 0, "Selected"), trainer_row(8, 2, "Default")],
            Vec::new(),
            Vec::new(),
            vec![
                CreatureTrainerRowLikeCpp {
                    creature_id: CREATURE_ENTRY,
                    trainer_id: 8,
                    menu_id: 0,
                    option_id: 0,
                },
                CreatureTrainerRowLikeCpp {
                    creature_id: CREATURE_ENTRY,
                    trainer_id: 7,
                    menu_id: 900,
                    option_id: 3,
                },
            ],
        );
        assert_eq!(
            resolve_creature_trainer_like_cpp(store.as_ref(), CREATURE_ENTRY, Some((900, 3)))
                .map(TrainerLikeCpp::id_like_cpp),
            Some(7)
        );
        assert_eq!(
            resolve_creature_trainer_like_cpp(store.as_ref(), CREATURE_ENTRY, None)
                .map(TrainerLikeCpp::id_like_cpp),
            Some(8)
        );

        let dangling = TrainerStoreLikeCpp::from_rows_like_cpp(
            [trainer_row(8, 2, "Default")],
            Vec::<TrainerSpellRowLikeCpp>::new(),
            Vec::<TrainerLocaleRowLikeCpp>::new(),
            [
                CreatureTrainerRowLikeCpp {
                    creature_id: CREATURE_ENTRY,
                    trainer_id: 77,
                    menu_id: 900,
                    option_id: 3,
                },
                CreatureTrainerRowLikeCpp {
                    creature_id: CREATURE_ENTRY,
                    trainer_id: 8,
                    menu_id: 0,
                    option_id: 0,
                },
            ],
            |_| true,
            |_| true,
            |_| true,
            |_, _| true,
        );
        assert_eq!(
            dangling.report.skipped_creature_trainers_missing_trainer,
            vec![(CREATURE_ENTRY, 77, 900, 3)]
        );
        assert_eq!(
            resolve_creature_trainer_like_cpp(&dangling.store, CREATURE_ENTRY, Some((900, 3)))
                .map(TrainerLikeCpp::id_like_cpp),
            Some(8),
            "a selected mapping discarded at load must fall through to the default"
        );
    }

    #[test]
    fn class_race_fit_uses_effective_ability_and_race_class_rows() {
        let (mut session, _) = make_session();
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            ObjectGuid::create_player(1, 42),
            "FitTester".to_string(),
            Position::new(0.0, 0.0, 0.0, 0.0),
            0,
            1,
            1,
            80,
            0,
        ));
        let ability = SkillLineAbilityRecord {
            id: 1,
            race_mask: 1,
            skill_line: 164,
            spell: 500,
            min_skill_line_rank: 0,
            class_mask: 1,
            supercedes_spell: 0,
            acquire_method: 0,
            trivial_rank_high: 0,
            trivial_rank_low: 0,
            flags: 0,
            num_skill_ups: 0,
            skillup_skill_line_id: 0,
        };
        let race_class = SkillRaceClassInfoRecord {
            id: 2,
            race_mask: 1,
            skill_id: 164,
            class_mask: 1,
            flags: 0,
            availability: 1,
            min_level: 1,
            skill_tier_id: 0,
        };
        session.set_skill_store(Arc::new(
            SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
                [ability.clone()],
                [race_class],
            ),
        ));
        assert_eq!(
            trainer_spell_class_race_fit_like_cpp(&session, 500),
            TrainerAdmissionProofLikeCpp::Proven(true)
        );

        let wrong_race = SkillLineAbilityRecord {
            race_mask: 2,
            ..ability
        };
        session.set_skill_store(Arc::new(
            SkillStore::from_skill_line_abilities_and_race_class_like_cpp([wrong_race], []),
        ));
        assert_eq!(
            trainer_spell_class_race_fit_like_cpp(&session, 500),
            TrainerAdmissionProofLikeCpp::Proven(false)
        );
    }

    #[tokio::test]
    async fn missing_trainer_mapping_preserves_binding_and_feign_like_cpp() {
        let store = trainer_store_from_rows(
            vec![trainer_row(DEFAULT_TRAINER_ID, 2, "Unused")],
            vec![trainer_spell_row(
                DEFAULT_TRAINER_ID,
                KNOWN_TRAINER_SPELL,
                10,
                1,
            )],
            Vec::new(),
            Vec::new(),
        );
        let mut fixture = trainer_fixture_with_store(store);
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.other_trainer, 17);
        seed_feign_death(&mut fixture.session, 6);

        fixture
            .session
            .handle_trainer_list(wow_packet::packets::gossip::Hello {
                unit: fixture.trainer,
            })
            .await;

        assert!(
            fixture
                .session
                .player_trainer_interaction_matches_like_cpp(fixture.other_trainer, 17),
            "a missing ObjectMgr mapping must not replace a prior published window"
        );
        assert!(fixture.session.visible_auras.contains_key(&6));
        assert!(canonical_player_has_died_state(&mut fixture.session));
        assert!(fixture.send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn valid_trainer_list_uses_store_states_and_publishes_exact_binding_like_cpp() {
        let mut fixture = trainer_fixture();
        seed_feign_death(&mut fixture.session, 6);

        fixture
            .session
            .handle_trainer_list(wow_packet::packets::gossip::Hello {
                unit: fixture.trainer,
            })
            .await;

        assert!(
            fixture.send_rx.try_recv().is_ok(),
            "C++ removes feign death before publishing the trainer list"
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            TrainerListPacket {
                trainer_guid: fixture.trainer,
                trainer_type: 2,
                trainer_id: DEFAULT_TRAINER_ID as i32,
                spells: vec![
                    TrainerListSpell {
                        spell_id: KNOWN_TRAINER_SPELL,
                        money_cost: 10,
                        req_skill_line: 0,
                        req_skill_rank: 0,
                        req_ability: [0; 3],
                        usable: TRAINER_SPELL_STATE_KNOWN_LIKE_CPP,
                        req_level: 1,
                    },
                    TrainerListSpell {
                        spell_id: AVAILABLE_TRAINER_SPELL,
                        money_cost: 20,
                        req_skill_line: 0,
                        req_skill_rank: 0,
                        req_ability: [0; 3],
                        usable: TRAINER_SPELL_STATE_AVAILABLE_LIKE_CPP,
                        req_level: 80,
                    },
                    TrainerListSpell {
                        spell_id: UNAVAILABLE_TRAINER_SPELL,
                        money_cost: 30,
                        req_skill_line: 0,
                        req_skill_rank: 0,
                        req_ability: [0; 3],
                        usable: TRAINER_SPELL_STATE_UNAVAILABLE_LIKE_CPP,
                        req_level: 81,
                    },
                ],
                greeting: "Train".to_string(),
            }
            .to_bytes()
        );
        assert_eq!(TRAINER_SPELL_STATE_KNOWN_LIKE_CPP, 0);
        assert_eq!(TRAINER_SPELL_STATE_AVAILABLE_LIKE_CPP, 1);
        assert_eq!(TRAINER_SPELL_STATE_UNAVAILABLE_LIKE_CPP, 2);
        assert!(fixture.session.player_trainer_interaction_matches_like_cpp(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32
        ));
        assert!(!fixture.session.visible_auras.contains_key(&6));
        assert!(!canonical_player_has_died_state(&mut fixture.session));
        assert!(fixture.send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn trainer_buy_requires_exact_active_provenance_before_known_spell_like_cpp() {
        for case in ["unbound", "wrong-guid", "wrong-id"] {
            let mut fixture = if case == "wrong-id" {
                let decoy_trainer_id = DEFAULT_TRAINER_ID + 1;
                trainer_fixture_with_store(trainer_store_from_rows(
                    vec![
                        trainer_row(DEFAULT_TRAINER_ID, 2, "Default"),
                        trainer_row(decoy_trainer_id, 2, "Decoy"),
                    ],
                    vec![
                        trainer_spell_row(DEFAULT_TRAINER_ID, KNOWN_TRAINER_SPELL, 10, 1),
                        trainer_spell_row(decoy_trainer_id, KNOWN_TRAINER_SPELL, 10, 1),
                    ],
                    Vec::new(),
                    vec![CreatureTrainerRowLikeCpp {
                        creature_id: CREATURE_ENTRY,
                        trainer_id: DEFAULT_TRAINER_ID,
                        menu_id: 0,
                        option_id: 0,
                    }],
                ))
            } else {
                trainer_fixture()
            };
            fixture
                .session
                .learn_known_spell_like_cpp(KNOWN_TRAINER_SPELL);
            match case {
                "unbound" => {}
                "wrong-guid" | "wrong-id" => fixture
                    .session
                    .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID),
                _ => unreachable!(),
            }
            let (request_guid, request_id) = match case {
                "unbound" => (fixture.trainer, DEFAULT_TRAINER_ID as i32),
                "wrong-guid" => (fixture.other_trainer, DEFAULT_TRAINER_ID as i32),
                "wrong-id" => (fixture.trainer, DEFAULT_TRAINER_ID as i32 + 1),
                _ => unreachable!(),
            };

            fixture
                .session
                .handle_trainer_buy_spell(trainer_buy_packet(
                    request_guid,
                    request_id,
                    KNOWN_TRAINER_SPELL,
                ))
                .await;

            assert!(
                fixture.send_rx.try_recv().is_err(),
                "{case} must return before the observable known-spell branch"
            );
            if case != "unbound" {
                assert!(fixture.session.player_trainer_interaction_matches_like_cpp(
                    fixture.trainer,
                    DEFAULT_TRAINER_ID as i32
                ));
            }
        }
    }

    #[tokio::test]
    async fn generic_source_with_zero_trainer_id_is_silent_before_known_spell_like_cpp() {
        let mut fixture = trainer_fixture();
        fixture
            .session
            .learn_known_spell_like_cpp(KNOWN_TRAINER_SPELL);
        fixture
            .session
            .set_player_interaction_source_like_cpp(fixture.trainer);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(fixture.trainer, 0, KNOWN_TRAINER_SPELL))
            .await;

        assert!(
            fixture.send_rx.try_recv().is_err(),
            "C++ GetTrainer(0) rejects a generic gossip source before TeachSpell"
        );
        assert_eq!(
            fixture.session.player_interaction_source_guid_like_cpp(),
            Some(fixture.trainer)
        );
        assert_eq!(fixture.session.player_interaction_trainer_id_like_cpp(), 0);
    }

    #[tokio::test]
    async fn trainer_buy_signed_id_bits_and_binding_survive_repeated_failures_like_cpp() {
        let store = standard_trainer_store(u32::MAX);
        let mut fixture = trainer_fixture_with_store(store);
        fixture
            .session
            .learn_known_spell_like_cpp(KNOWN_TRAINER_SPELL);
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.trainer, u32::MAX);

        let expected = TrainerBuyFailed {
            trainer_guid: fixture.trainer,
            spell_id: KNOWN_TRAINER_SPELL,
            reason: 0,
        }
        .to_bytes();
        for _ in 0..2 {
            fixture
                .session
                .handle_trainer_buy_spell(trainer_buy_packet(
                    fixture.trainer,
                    -1,
                    KNOWN_TRAINER_SPELL,
                ))
                .await;
            assert_eq!(fixture.send_rx.try_recv().unwrap(), expected);
            assert!(
                fixture
                    .session
                    .player_trainer_interaction_matches_like_cpp(fixture.trainer, -1)
            );
        }
    }

    #[tokio::test]
    async fn available_buy_is_recomputed_but_never_mutates_or_publishes_success_in_this_slice() {
        let mut fixture = trainer_fixture();
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
        fixture.session.set_player_gold_like_cpp(100);
        let spells_before = fixture.session.known_spells_like_cpp().to_vec();
        let decision_before = fixture.session.trainer_offer_decision_like_cpp(
            DEFAULT_TRAINER_ID,
            fixture
                .session
                .trainer_store_like_cpp()
                .unwrap()
                .get_trainer_like_cpp(DEFAULT_TRAINER_ID)
                .unwrap()
                .get_spell_like_cpp(AVAILABLE_TRAINER_SPELL as u32)
                .unwrap(),
            0,
        );

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                AVAILABLE_TRAINER_SPELL,
            ))
            .await;

        assert!(matches!(
            decision_before,
            TrainerOfferDecisionLikeCpp::Available(_)
        ));
        assert_eq!(fixture.session.player_gold_like_cpp(), 100);
        assert_eq!(fixture.session.known_spells_like_cpp(), spells_before);
        assert!(fixture.send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn insufficient_money_uses_prepared_effective_price_without_mutation() {
        let mut fixture = trainer_fixture();
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
        fixture.session.set_player_gold_like_cpp(19);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                AVAILABLE_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 19);
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            TrainerBuyFailed {
                trainer_guid: fixture.trainer,
                spell_id: AVAILABLE_TRAINER_SPELL,
                reason: 1,
            }
            .to_bytes()
        );
        assert!(fixture.send_rx.try_recv().is_err());
    }

    #[test]
    fn buy_registration_remains_but_dispatch_and_legacy_mutation_are_disabled() {
        let trainer = include_str!("trainer.rs");
        let session = include_str!("../session.rs");
        assert!(trainer.contains("opcode: ClientOpcodes::TrainerBuySpell"));
        assert!(!session.contains("ClientOpcodes::TrainerBuySpell =>"));

        let buy = trainer
            .split("pub async fn handle_trainer_buy_spell")
            .nth(1)
            .expect("buy handler")
            .split("\n}\n\n#[cfg(test)]")
            .next()
            .expect("buy handler body");
        for forbidden in [
            "INS_CHARACTER_SPELL",
            "UPD_CHAR_MONEY",
            "set_player_gold_like_cpp",
            "learn_known_spell_like_cpp",
            "LearnedSpells::single",
        ] {
            assert!(
                !buy.contains(forbidden),
                "#157 must not retain legacy buy mutation/publication `{forbidden}`"
            );
        }
    }

    #[tokio::test]
    async fn valid_trainer_mismatch_removes_feign_before_silent_reject_like_cpp() {
        let mut fixture = trainer_fixture();
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
        seed_feign_death(&mut fixture.session, 6);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.other_trainer,
                DEFAULT_TRAINER_ID as i32,
                KNOWN_TRAINER_SPELL,
            ))
            .await;

        assert!(
            fixture.send_rx.try_recv().is_ok(),
            "removing feign death publishes its aura update"
        );
        assert!(fixture.send_rx.try_recv().is_err());
        assert!(!fixture.session.visible_auras.contains_key(&6));
        assert!(!canonical_player_has_died_state(&mut fixture.session));
        assert!(fixture.session.player_trainer_interaction_matches_like_cpp(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32
        ));
    }

    #[tokio::test]
    async fn invalid_trainer_does_not_remove_feign_or_binding_like_cpp() {
        let mut fixture = trainer_fixture();
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
        seed_feign_death(&mut fixture.session, 6);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.vendor,
                DEFAULT_TRAINER_ID as i32,
                KNOWN_TRAINER_SPELL,
            ))
            .await;

        assert!(fixture.send_rx.try_recv().is_err());
        assert!(fixture.session.visible_auras.contains_key(&6));
        assert!(canonical_player_has_died_state(&mut fixture.session));
        assert!(fixture.session.player_trainer_interaction_matches_like_cpp(
            fixture.trainer,
            DEFAULT_TRAINER_ID as i32
        ));
    }
}
