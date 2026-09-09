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
//!   4. Under the exclusive money boundary, atomically persist the fee and the
//!      exact prepared spell/skill result.
//!   5. Install committed runtime state, then publish money, visual kits and
//!      acquisition actions in C++ success order.
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
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
#[cfg(test)]
use wow_packet::ServerPacket;
#[cfg(test)]
use wow_packet::packets::spell::PlaySpellVisualKit;
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
    let Some(catalog) = session.spell_catalogs.spell_acquisition_catalog() else {
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
        if effect.targets_unit_pet_like_cpp() || !effect.targets_player_like_cpp() {
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

fn trainer_condition_admission_proof_like_cpp(
    meets: bool,
    saw_unsupported: bool,
) -> TrainerAdmissionProofLikeCpp {
    if meets {
        // C++ ElseGroups are ORed. A supported passing group proves the
        // result even when another, irrelevant group is not representable.
        TrainerAdmissionProofLikeCpp::Proven(true)
    } else if saw_unsupported {
        TrainerAdmissionProofLikeCpp::Indeterminate
    } else {
        TrainerAdmissionProofLikeCpp::Proven(false)
    }
}

// ── Handler registrations ─────────────────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TrainerList,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_trainer_list",
        handler: |session, _catalogs, mut pkt| {
            Box::pin(async move {
                match wow_packet::packets::gossip::Hello::read(&mut pkt) {
                    Ok(hello) => session.handle_trainer_list(hello).await,
                    Err(e) => tracing::warn!("Failed to read TrainerList: {e}"),
                }
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TrainerBuySpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_trainer_buy_spell",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_trainer_buy_spell_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        catalogs.battle_pet_trainer_selection.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
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
        let Some(player_condition_context) = self.represented_player_condition_context_like_cpp()
        else {
            return TrainerAdmissionProofLikeCpp::Indeterminate;
        };
        let Some(player_unit_snapshot) = self.condition_player_unit_snapshot_like_cpp() else {
            return TrainerAdmissionProofLikeCpp::Indeterminate;
        };
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
                    if let Some(context) = player_condition_context.as_context(self) {
                        source_info.set_player_condition_context(0, context);
                    }
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
        trainer_condition_admission_proof_like_cpp(meets, unsupported)
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
        let acquisition = self.spell_catalogs.spell_acquisition_catalog();
        let battle_pet = match acquisition
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
                // C++ `Trainer::GetSpellState` has no battle-pet cap gate, so
                // a confirmed purchasable species renders available; the
                // silent cap only applies inside `Trainer::TeachSpell`.
                TrainerOfferDecisionLikeCpp::AvailableBattlePet(offer) => (
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
    /// Revalidates the immutable offer under the exclusive character-money
    /// boundary, commits its fee and prepared acquisition once, then publishes
    /// the C++ money/visual/learning order from the committed result.
    pub async fn handle_trainer_buy_spell_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        battle_pet_selection_store: &wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp,
        mut pkt: wow_packet::WorldPacket,
    ) {
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

        let Some(_access) = self.represented_npc_can_interact_with_like_cpp(
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
                active_trainer_id = ?self.resolved_player_interaction_trainer_id_like_cpp(),
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
        let Some(_trainer_spell) = trainer.get_spell_like_cpp(spell_id as u32).cloned() else {
            warn!(
                account = self.account_id,
                trainer_id = trainer_id,
                spell_id = spell_id,
                "Spell not in trainer's loaded C++ spell set"
            );
            self.send_packet_realm(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
            return;
        };

        // Ordinary in-world LearnSpell/skill mutations remain dirty until
        // Player::SaveToDB. Persist that current authority before preparing
        // the trainer's absolute replacement; rejecting normal dirty state
        // would make trainers unusable between autosaves. The duplicate-login
        // claim keeps this save and the following purchase under the same sole
        // live Player authority, while the purchase revalidates everything
        // after both awaits.
        if let Ok(snapshot) = self.spell_acquisition_snapshot_like_cpp(
            crate::spell_acquisition::PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            std::collections::BTreeMap::new(),
        ) && crate::spell_acquisition::snapshot_has_pending_durable_save_like_cpp(&snapshot)
        {
            self.save_current_player_to_db_with_generator_like_cpp(item_guid_generator)
                .await;
        }
        // Close detached money admission and reconcile every previously
        // admitted payout before deriving the price, balance or acquisition
        // snapshot that will be persisted.
        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };

        // The await above is an intentional race boundary. Re-resolve every
        // mutable/current authority rather than trusting the preliminary
        // membership proof retained only to match the early C++ failure path.
        let Some(fresh_access) = self.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            TRAINER_BUY_NPC_FLAGS_LIKE_CPP,
            0,
        ) else {
            return;
        };
        if !self.player_trainer_interaction_matches_like_cpp(trainer_guid, trainer_id) {
            return;
        }
        let Some(fresh_trainer_spell) = self
            .trainer_store_like_cpp()
            .and_then(|store| store.get_trainer_like_cpp(trainer_id as u32))
            .and_then(|trainer| trainer.get_spell_like_cpp(spell_id as u32))
            .cloned()
        else {
            self.send_packet_realm(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
            return;
        };
        let decision = self.trainer_offer_decision_like_cpp(
            trainer_id as u32,
            &fresh_trainer_spell,
            fresh_access.faction_template_id,
        );
        let offer = match decision {
            TrainerOfferDecisionLikeCpp::Available(offer) => offer,
            TrainerOfferDecisionLikeCpp::AvailableBattlePet(offer) => {
                // Issue #161: the recoverable saga owns the battle-pet
                // branch end to end (admission, charge, durable command,
                // one pet, completion, compensation and publication).
                self.execute_battle_pet_trainer_purchase_with_generator_like_cpp(
                    item_guid_generator,
                    battle_pet_selection_store,
                    money_persistence,
                    trainer_guid,
                    trainer_id as u32,
                    offer,
                )
                .await;
                return;
            }
            _ => {
                self.send_packet_realm(&TrainerBuyFailed {
                    trainer_guid,
                    spell_id,
                    reason: 0,
                });
                return;
            }
        };
        // C++ `Trainer.cpp:99-109`: every spell with a confirmed battle-pet
        // species — castable or not — applies the silent per-species
        // capacity gate (no packet, no charge) before the money check.
        if let Some(species_id) = offer.battle_pet_species_id {
            let capped = self
                .battle_pet_account_owner_lease_like_cpp()
                .map(|(owner, _)| owner.has_max_pet_count_like_cpp(species_id, self.player_guid()))
                .unwrap_or(true);
            if capped {
                return;
            }
        }
        let Some(old_money) = self.resolved_player_money_like_cpp() else {
            return;
        };
        let price = u64::from(offer.effective_price);
        if old_money < price {
            self.send_packet_realm(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 1,
            });
            return;
        }
        let new_money = old_money - price;

        let Ok(current_snapshot) = self.spell_acquisition_snapshot_like_cpp(
            crate::spell_acquisition::PlayerAcquisitionLifecycleLikeCpp::InWorld,
            offer
                .acquisition_plan
                .source_snapshot
                .future_player_condition_resolutions
                .clone(),
            offer
                .acquisition_plan
                .source_snapshot
                .cast_resolutions
                .clone(),
        ) else {
            self.send_packet_realm(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
            return;
        };
        let Some(player_guid) = current_snapshot.character_guid else {
            self.send_packet_realm(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
            return;
        };
        let completion = crate::spell_acquisition::execute_trainer_acquisition_like_cpp(
            self,
            money_persistence,
            &offer,
            &current_snapshot,
            old_money,
            new_money,
            &crate::spell_acquisition::TrainerAcquisitionPublicationLikeCpp {
                trainer_guid,
                player_guid,
                trainer_position: fresh_access.position,
                suppress_visuals: offer.battle_pet_species_id.is_some(),
            },
        )
        .await;
        use crate::spell_acquisition::TrainerAcquisitionResultLikeCpp as Result;
        match completion.result {
            Result::Applied => {}
            Result::InvalidPreparation => {
                self.send_packet_realm(&TrainerBuyFailed {
                    trainer_guid,
                    spell_id,
                    reason: 0,
                });
                return;
            }
            Result::PersistenceUnavailable => return,
            Result::RuntimeInstallationFailed => {
                self.kick(
                    "committed trainer acquisition could not install runtime state; relog required",
                );
                return;
            }
            Result::MoneyOwnerUnavailable => {
                self.kick("canonical Player money owner became unavailable after trainer COMMIT");
                return;
            }
            Result::WriterFenceFailed => {
                self.kick("trainer socket ordering fence failed after durable acquisition");
                return;
            }
            Result::PublicationFailed => {
                self.kick(
                    "committed trainer acquisition could not publish runtime state; relog required",
                );
                return;
            }
        }
        // Preserve the exclusion until failure handling above has completed.
        drop(completion);
        self.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;

        info!(
            account = self.account_id,
            trainer_id,
            spell_id,
            effective_price = offer.effective_price,
            remaining_money = new_money,
            "Trainer purchase committed and published"
        );
    }

    #[cfg(test)]
    pub async fn handle_trainer_buy_spell(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        let selection = self
            .battle_pet_selection_store_like_cpp()
            .cloned()
            .unwrap_or_else(|| {
                Arc::new(wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp::default())
            });
        self.handle_trainer_buy_spell_with_generator_like_cpp(
            generators.item.as_ref(),
            selection.as_ref(),
            pkt,
        )
        .await;
    }
}

#[cfg(test)]
#[path = "trainer/tests/mod.rs"]
mod tests;
