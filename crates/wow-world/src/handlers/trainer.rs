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
use wow_packet::packets::spell::PlaySpellVisualKit;
use wow_packet::packets::trainer::{
    TrainerBuyFailed, TrainerBuySpellRequest, TrainerListPacket, TrainerListSpell,
};
use wow_packet::{ClientPacket, ServerPacket};

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

enum PreparedTrainerAcquisitionLikeCpp {
    Durable(crate::spell_acquisition::PreparedPlayerSpellAcquisitionLikeCpp),
    ActionsOnly(crate::spell_acquisition::PreparedPlayerSpellAcquisitionActionsLikeCpp),
    NoChange,
}

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
        handler: |session, mut pkt| {
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
        handler: |session, pkt| {
            Box::pin(async move { session.handle_trainer_buy_spell(pkt).await })
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
            self.save_current_player_to_db_like_cpp().await;
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
                self.execute_battle_pet_trainer_purchase_like_cpp(
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
        let prepared = match crate::spell_acquisition::prepare_player_spell_acquisition_like_cpp(
            &offer.acquisition_plan,
            &offer.profession_plan,
            &current_snapshot,
        ) {
            Ok(crate::spell_acquisition::PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(
                prepared,
            )) => PreparedTrainerAcquisitionLikeCpp::Durable(prepared),
            Ok(crate::spell_acquisition::PreparedPlayerSpellAcquisitionOutcomeLikeCpp::ActionsOnly(
                prepared,
            )) => PreparedTrainerAcquisitionLikeCpp::ActionsOnly(prepared),
            Ok(crate::spell_acquisition::PreparedPlayerSpellAcquisitionOutcomeLikeCpp::NoChange) => {
                PreparedTrainerAcquisitionLikeCpp::NoChange
            }
            Ok(crate::spell_acquisition::PreparedPlayerSpellAcquisitionOutcomeLikeCpp::AlreadyApplied)
            | Err(_) => {
                self.send_packet_realm(&TrainerBuyFailed {
                    trainer_guid,
                    spell_id,
                    reason: 0,
                });
                return;
            }
        };
        let runtime_valid = match &prepared {
            PreparedTrainerAcquisitionLikeCpp::Durable(prepared) => {
                crate::spell_acquisition::validate_prepared_player_spell_acquisition_runtime_like_cpp(
                    self, prepared,
                )
            }
            PreparedTrainerAcquisitionLikeCpp::ActionsOnly(prepared) => {
                crate::spell_acquisition::validate_prepared_player_spell_acquisition_actions_runtime_like_cpp(
                    self, prepared,
                )
            }
            PreparedTrainerAcquisitionLikeCpp::NoChange => Ok(()),
        };
        if runtime_valid.is_err() {
            self.send_packet_realm(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
            return;
        }

        let committed_money_persistence = match &prepared {
            PreparedTrainerAcquisitionLikeCpp::Durable(prepared) => {
                self.commit_exclusive_player_money_and_spell_acquisition_like_cpp(
                    money_persistence,
                    prepared,
                    old_money,
                    new_money,
                )
                .await
            }
            PreparedTrainerAcquisitionLikeCpp::ActionsOnly(_)
            | PreparedTrainerAcquisitionLikeCpp::NoChange => {
                self.commit_exclusive_trainer_money_only_like_cpp(
                    money_persistence,
                    old_money,
                    new_money,
                )
                .await
            }
        };
        let Some(money_persistence) = committed_money_persistence else {
            return;
        };

        // C++ successful observable order: ModifyMoney, trainer visual,
        // player visual, then the direct LearnSpell/triggered wrapper actions.
        // Every durable mutation already committed; keep the exclusion until
        // all covered runtime owners and packets have consumed that result.
        let installed = match prepared {
            PreparedTrainerAcquisitionLikeCpp::Durable(prepared) => {
                let publish_skill_fields =
                    prepared.source_snapshot.skills != prepared.runtime_snapshot.skills;
                crate::spell_acquisition::install_prepared_player_spell_acquisition_runtime_like_cpp(
                    self, &prepared,
                )
                .map(|actions| (publish_skill_fields, Some(actions)))
            }
            PreparedTrainerAcquisitionLikeCpp::ActionsOnly(prepared) => {
                crate::spell_acquisition::install_prepared_player_spell_acquisition_actions_runtime_like_cpp(
                    self, prepared,
                )
                .map(|actions| (false, Some(actions)))
            }
            PreparedTrainerAcquisitionLikeCpp::NoChange => Ok((false, None)),
        };
        let (publish_skill_fields, actions) = match installed {
            Ok(installed) => installed,
            Err(_) => {
                self.kick(
                    "committed trainer acquisition could not install runtime state; relog required",
                );
                return;
            }
        };
        if !self.stage_player_money_change_like_cpp(old_money, new_money) {
            self.kick("canonical Player money owner became unavailable after trainer COMMIT");
            return;
        }
        if old_money != new_money {
            self.send_player_values_update_from_entity_bridge(&[], &[], &[], &[], Some(new_money));
        }
        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            self.kick("trainer socket ordering fence failed after durable acquisition");
            return;
        }
        let trainer_visual = PlaySpellVisualKit {
            unit: trainer_guid,
            kit_record_id: 179,
            kit_type: 0,
            duration: 0,
            mounted_visual: false,
        };
        let player_visual = PlaySpellVisualKit {
            unit: player_guid,
            kit_record_id: 362,
            kit_type: 1,
            duration: 0,
            mounted_visual: false,
        };
        // C++ `Trainer.cpp:108,121-125`: a spell with a confirmed battle-pet
        // species never emits the trainer/player visual kits, even when the
        // wrapper cast itself proceeds. The socket-ordering fences stay so
        // the money update and the acquisition stream keep their order.
        if offer.battle_pet_species_id.is_none() {
            self.send_packet_realm(&trainer_visual);
            self.broadcast_creature_packet_from_position_to_visible_set_realm_like_cpp(
                trainer_guid,
                fresh_access.position,
                trainer_visual.to_bytes(),
            );
            self.send_packet_realm(&player_visual);
            self.broadcast_to_movement_set_realm_like_cpp(player_visual.to_bytes(), true);
        }
        if !self
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await
        {
            self.kick("trainer socket ordering fence failed after durable acquisition");
            return;
        }
        if publish_skill_fields {
            self.send_complete_player_skill_values_update_like_cpp();
        }
        if let Some(actions) = actions
            && crate::spell_acquisition::apply_prepared_player_spell_acquisition_actions_like_cpp(
                self, &actions,
            )
            .is_err()
        {
            self.kick(
                "committed trainer acquisition could not publish runtime state; relog required",
            );
            return;
        }
        drop(money_persistence);
        self.drain_represented_quest_objective_progress_like_cpp()
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
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use super::*;
    use crate::session::{
        AuraApplication, RepresentedAuraEffectLikeCpp, RepresentedPlayerSpellLikeCpp,
        RepresentedPlayerSpellStateLikeCpp, SessionPlayerController, SessionState,
    };
    use wow_constants::unit::UnitState;
    use wow_core::guid::HighGuid;
    use wow_core::{ObjectGuid, Position};
    use wow_data::{
        ConditionEntriesByTypeStore, CreatureTrainerRowLikeCpp,
        EffectiveSpellAcquisitionRowsLikeCpp, MountStore, SkillLineAbilityRecord, SkillLineEntry,
        SkillLineStore, SkillRaceClassInfoRecord, SkillStore, SkillTiersRowLikeCpp,
        SkillTiersStoreLikeCpp, SpellAcquisitionCatalogLikeCpp,
        SpellAcquisitionCoverageSeedLikeCpp, SpellAcquisitionEffectLikeCpp,
        SpellAcquisitionMiscLikeCpp, SpellAcquisitionTableHashesLikeCpp, SpellChainStoreLikeCpp,
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
    const WRAPPER_TRAINER_SPELL: i32 = 54_324;
    const WRAPPER_LEARNED_SPELL: i32 = 54_325;

    fn spell_target_restriction_row(
        id: u32,
        spell_id: u32,
        difficulty_id: u8,
        target_creature_type: i16,
    ) -> wow_data::SpellTargetRestrictionsEntry {
        wow_data::SpellTargetRestrictionsEntry {
            id,
            difficulty_id,
            cone_degrees: 0.0,
            max_targets: 0,
            max_target_level: 0,
            target_creature_type,
            targets: 0,
            width: 0.0,
            spell_id,
        }
    }

    fn player_learn_effect(
        record_id: u32,
        wrapper_spell_id: u32,
        learned_spell_id: u32,
    ) -> SpellAcquisitionEffectLikeCpp {
        SpellAcquisitionEffectLikeCpp {
            record_id,
            spell_id_raw: i64::from(wrapper_spell_id),
            difficulty_id_raw: 0,
            effect_index_raw: 0,
            effect_type_raw: 36,
            effect_aura_raw: 0,
            effect_mechanic_raw: 0,
            effect_attributes_raw: 0,
            effect_base_points_raw: 0,
            effect_die_sides_raw: 0,
            effect_chain_targets_raw: 0,
            effect_points_per_resource_bits: 0.0_f32.to_bits(),
            effect_real_points_per_level_bits: 0.0_f32.to_bits(),
            effect_coefficient_bits: 0.0_f32.to_bits(),
            effect_variance_bits: 0.0_f32.to_bits(),
            effect_trigger_spell_raw: i64::from(learned_spell_id),
            effect_item_type_raw: 0,
            effect_misc_value_raw: [0, 0],
            implicit_target_raw: [1, 0],
        }
    }

    fn player_aura_effect(
        record_id: u32,
        spell_id: u32,
        aura_type: i64,
        aura_misc_value: i64,
    ) -> SpellAcquisitionEffectLikeCpp {
        SpellAcquisitionEffectLikeCpp {
            record_id,
            spell_id_raw: i64::from(spell_id),
            difficulty_id_raw: 0,
            effect_index_raw: 0,
            effect_type_raw: 6,
            effect_aura_raw: aura_type,
            effect_mechanic_raw: 0,
            effect_attributes_raw: 0,
            effect_base_points_raw: 0,
            effect_die_sides_raw: 0,
            effect_chain_targets_raw: 0,
            effect_points_per_resource_bits: 0.0_f32.to_bits(),
            effect_real_points_per_level_bits: 0.0_f32.to_bits(),
            effect_coefficient_bits: 0.0_f32.to_bits(),
            effect_variance_bits: 0.0_f32.to_bits(),
            effect_trigger_spell_raw: 0,
            effect_item_type_raw: 0,
            effect_misc_value_raw: [aura_misc_value, 0],
            implicit_target_raw: [1, 0],
        }
    }

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

    fn assert_trainer_charge_and_visuals_like_cpp(fixture: &mut TrainerFixture) {
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            wow_packet::packets::update::UpdateObject::player_money_update(
                fixture.session.player_guid().unwrap(),
                fixture.session.player_map_id_like_cpp(),
                75,
                None,
            )
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            PlaySpellVisualKit {
                unit: fixture.trainer,
                kit_record_id: 179,
                kit_type: 0,
                duration: 0,
                mounted_visual: false,
            }
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            PlaySpellVisualKit {
                unit: fixture.session.player_guid().unwrap(),
                kit_record_id: 362,
                kit_type: 1,
                duration: 0,
                mounted_visual: false,
            }
            .to_bytes()
        );
        assert!(fixture.send_rx.try_recv().is_err());
    }

    fn trainer_fixture_with_store(store: Arc<TrainerStoreLikeCpp>) -> TrainerFixture {
        trainer_fixture_with_store_and_map_difficulty(store, 0)
    }

    fn trainer_fixture_with_store_and_map_difficulty(
        store: Arc<TrainerStoreLikeCpp>,
        map_difficulty: u8,
    ) -> TrainerFixture {
        let (mut session, send_rx) = make_session();
        let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
        if map_difficulty != 0 {
            canonical.lock().unwrap().create_map_entry(
                0,
                0,
                map_difficulty,
                wow_map::ManagedMapKind::World,
            );
            session.set_difficulty_store(Arc::new(wow_data::DifficultyStore::from_entries([
                wow_data::DifficultyEntry {
                    id: u32::from(map_difficulty),
                    instance_type: 0,
                    flags: 0,
                    fallback_difficulty_id: 0,
                    toggle_difficulty_id: 0,
                },
            ])));
        }
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
        session.set_disable_mgr(Arc::new(wow_data::DisableMgrLikeCpp::default()));
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
        session.set_player_aura_authority_complete_like_cpp(true);
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
        session.set_spell_linked_store(Arc::new(wow_data::SpellLinkedStoreLikeCpp::default()));
        session.set_spell_pet_aura_store(Arc::new(wow_data::SpellPetAuraStoreLikeCpp::default()));
        session.set_spell_target_restrictions_store(Arc::new(
            wow_data::SpellTargetRestrictionsStore::from_entries([]),
        ));
        session.set_spell_aura_restrictions_store(Arc::new(
            wow_data::SpellAuraRestrictionsStore::from_entries([]),
        ));
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
        session
            .ensure_canonical_world_map_for_current_player_like_cpp()
            .expect("canonical player map");
        // The production login path hydrates `Player::SetFactionForRace`
        // before publishing the Player. This synthetic fixture has no
        // ChrRaces store, so install the exact canonical prerequisite here.
        session.set_player_faction_template_like_cpp(1);
        assert!(session.set_complete_player_skill_records_like_cpp(HashMap::new(), 0));
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

    fn trainer_wrapper_fixture() -> TrainerFixture {
        trainer_wrapper_fixture_with_map_difficulty(0)
    }

    fn trainer_wrapper_fixture_with_map_difficulty(map_difficulty: u8) -> TrainerFixture {
        let store = trainer_store_from_rows(
            vec![trainer_row(DEFAULT_TRAINER_ID, 2, "Train")],
            vec![trainer_spell_row(
                DEFAULT_TRAINER_ID,
                WRAPPER_TRAINER_SPELL,
                25,
                1,
            )],
            Vec::new(),
            vec![CreatureTrainerRowLikeCpp {
                creature_id: CREATURE_ENTRY,
                trainer_id: DEFAULT_TRAINER_ID,
                menu_id: 0,
                option_id: 0,
            }],
        );
        let mut fixture = trainer_fixture_with_store_and_map_difficulty(store, map_difficulty);
        let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
        let learned_id = WRAPPER_LEARNED_SPELL as u32;
        fixture.session.set_spell_acquisition_catalog(Arc::new(
            SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
                [wrapper_id, learned_id]
                    .map(|spell_id| SpellAcquisitionCoverageSeedLikeCpp::covered(spell_id, 0)),
                EffectiveSpellAcquisitionRowsLikeCpp {
                    spell_effects: vec![player_learn_effect(1, wrapper_id, learned_id)],
                    ..Default::default()
                },
                SpellAcquisitionTableHashesLikeCpp::default(),
                Vec::new(),
            ),
        ));
        let mut learn_skills = SpellLearnSkillStoreLikeCpp::default();
        learn_skills
            .covered_spell_ids
            .extend([wrapper_id, learned_id]);
        fixture
            .session
            .set_spell_learn_skill_store(Arc::new(learn_skills));
        fixture
            .session
            .set_spell_acquisition_static_authority_like_cpp([wrapper_id], []);
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
        fixture.session.set_player_gold_like_cpp(100);
        fixture
            .session
            .set_loot_money_persistence_test_result_like_cpp(true);
        fixture
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
                difficulty_id: 0,
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

    fn seed_unclassified_active_aura(session: &mut WorldSession, slot: u8) {
        let player_guid = session.player_guid().expect("active player");
        session.visible_auras.insert(
            slot,
            AuraApplication {
                spell_id: 999,
                difficulty_id: 0,
                caster_guid: player_guid,
                slot,
                duration_total: 0,
                duration_remaining: 0,
                stack_count: 1,
                aura_flags: 0,
                effect_mask: 1,
                aura_interrupt_flags: 0,
                aura_interrupt_flags2: 0,
                represented_effect: None,
                represented_amount: 0,
                represented_effect_amounts: Vec::new(),
                represented_misc_value: None,
                represented_multiplier: 1.0,
                applied_at: Instant::now(),
            },
        );
    }

    fn install_wrapper_and_aura_catalog(
        session: &mut WorldSession,
        aura_spell_id: u32,
        aura_type: i64,
        aura_misc_value: i64,
        wrapper_effect_attributes: i64,
        wrapper_spell_attributes: i64,
    ) {
        let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
        let learned_id = WRAPPER_LEARNED_SPELL as u32;
        let mut learn_effect = player_learn_effect(1, wrapper_id, learned_id);
        learn_effect.effect_attributes_raw = wrapper_effect_attributes;
        session.set_spell_acquisition_catalog(Arc::new(
            SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
                [wrapper_id, learned_id, aura_spell_id]
                    .map(|spell_id| SpellAcquisitionCoverageSeedLikeCpp::covered(spell_id, 0)),
                EffectiveSpellAcquisitionRowsLikeCpp {
                    spell_effects: vec![
                        learn_effect,
                        player_aura_effect(2, aura_spell_id, aura_type, aura_misc_value),
                    ],
                    spell_misc: vec![SpellAcquisitionMiscLikeCpp {
                        record_id: 3,
                        spell_id_raw: i64::from(wrapper_id),
                        difficulty_id_raw: 0,
                        attributes_raw: [wrapper_spell_attributes, 0],
                        show_future_spell_player_condition_id_raw: 0,
                    }],
                    ..Default::default()
                },
                SpellAcquisitionTableHashesLikeCpp::default(),
                Vec::new(),
            ),
        ));
    }

    fn install_aura_link(session: &mut WorldSession, aura_spell_id: u32, effect: i32) {
        let mut linked = wow_data::SpellLinkedStoreLikeCpp::default();
        linked.effects_by_type_and_trigger.insert(
            (wow_data::SpellLinkedTypeLikeCpp::Aura, aura_spell_id),
            vec![effect],
        );
        session.set_spell_linked_store(Arc::new(linked));
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
    fn trainer_condition_supported_or_success_outweighs_irrelevant_uncertainty_like_cpp() {
        assert_eq!(
            trainer_condition_admission_proof_like_cpp(true, true),
            TrainerAdmissionProofLikeCpp::Proven(true)
        );
        assert_eq!(
            trainer_condition_admission_proof_like_cpp(false, true),
            TrainerAdmissionProofLikeCpp::Indeterminate
        );
        assert_eq!(
            trainer_condition_admission_proof_like_cpp(false, false),
            TrainerAdmissionProofLikeCpp::Proven(false)
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
                "wrong-guid" | "wrong-id" => {
                    fixture.session.set_player_trainer_interaction_like_cpp(
                        fixture.trainer,
                        DEFAULT_TRAINER_ID,
                    );
                }
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
    async fn available_buy_commits_once_then_publishes_cpp_visual_and_learning_order() {
        let mut fixture = trainer_fixture();
        let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(8);
        let instance_fence = wow_network::SocketWriteFenceLikeCpp::default();
        let realm_fence = wow_network::SocketWriteFenceLikeCpp::default();
        fixture
            .session
            .install_realm_send_channel_for_test(realm_tx);
        fixture
            .session
            .set_send_write_fence_like_cpp(instance_fence.clone());
        fixture
            .session
            .install_realm_send_write_fence_for_test(realm_fence.clone());

        let (observed_tx, observed_rx) = flume::unbounded::<(&'static str, Vec<u8>)>();
        let instance_rx = fixture.send_rx.clone();
        let instance_observed_tx = observed_tx.clone();
        let instance_writer = tokio::spawn(async move {
            while let Ok(packet) = instance_rx.recv_async().await {
                if instance_fence.acknowledge_marker_like_cpp(&packet) {
                    continue;
                }
                if instance_observed_tx.send(("instance", packet)).is_err() {
                    break;
                }
            }
        });
        let realm_writer = tokio::spawn(async move {
            while let Ok(packet) = realm_rx.recv_async().await {
                if realm_fence.acknowledge_marker_like_cpp(&packet) {
                    continue;
                }
                if observed_tx.send(("realm", packet)).is_err() {
                    break;
                }
            }
        });
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
        fixture.session.set_player_gold_like_cpp(100);
        fixture
            .session
            .set_loot_money_persistence_test_result_like_cpp(true);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                AVAILABLE_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 80);
        assert!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&AVAILABLE_TRAINER_SPELL)
        );
        let player_guid = fixture.session.player_guid().unwrap();
        assert_eq!(
            observed_rx.recv_async().await.unwrap(),
            (
                "instance",
                wow_packet::packets::update::UpdateObject::player_money_update(
                    player_guid,
                    fixture.session.player_map_id_like_cpp(),
                    80,
                    None,
                )
                .to_bytes()
            )
        );
        assert_eq!(
            observed_rx.recv_async().await.unwrap(),
            (
                "realm",
                PlaySpellVisualKit {
                    unit: fixture.trainer,
                    kit_record_id: 179,
                    kit_type: 0,
                    duration: 0,
                    mounted_visual: false,
                }
                .to_bytes()
            )
        );
        assert_eq!(
            observed_rx.recv_async().await.unwrap(),
            (
                "realm",
                PlaySpellVisualKit {
                    unit: player_guid,
                    kit_record_id: 362,
                    kit_type: 1,
                    duration: 0,
                    mounted_visual: false,
                }
                .to_bytes()
            )
        );
        assert_eq!(
            observed_rx.recv_async().await.unwrap(),
            (
                "instance",
                wow_packet::packets::trainer::LearnedSpells::single(AVAILABLE_TRAINER_SPELL)
                    .to_bytes()
            )
        );
        assert!(observed_rx.try_recv().is_err());

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                AVAILABLE_TRAINER_SPELL,
            ))
            .await;
        assert_eq!(fixture.session.player_gold_like_cpp(), 80);
        assert_eq!(
            observed_rx.recv_async().await.unwrap(),
            (
                "realm",
                TrainerBuyFailed {
                    trainer_guid: fixture.trainer,
                    spell_id: AVAILABLE_TRAINER_SPELL,
                    reason: 0,
                }
                .to_bytes()
            )
        );
        assert!(observed_rx.try_recv().is_err());
        instance_writer.abort();
        realm_writer.abort();
    }

    #[tokio::test]
    async fn stalled_instance_writer_commits_but_never_publishes_realm_visuals_like_cpp() {
        let mut fixture = trainer_fixture();
        let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(8);
        let instance_fence = wow_network::SocketWriteFenceLikeCpp::default();
        fixture
            .session
            .install_realm_send_channel_for_test(realm_tx);
        fixture
            .session
            .set_send_write_fence_like_cpp(instance_fence.clone());
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
        fixture.session.set_player_gold_like_cpp(100);
        fixture
            .session
            .set_loot_money_persistence_test_result_like_cpp(true);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                AVAILABLE_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 80);
        assert!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&AVAILABLE_TRAINER_SPELL)
        );
        assert_eq!(
            fixture.session.state(),
            crate::session::SessionState::Disconnecting
        );
        assert!(realm_rx.try_recv().is_err());

        let player_guid = fixture.session.player_guid().unwrap();
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            wow_packet::packets::update::UpdateObject::player_money_update(
                player_guid,
                fixture.session.player_map_id_like_cpp(),
                80,
                None,
            )
            .to_bytes()
        );
        let marker = fixture.send_rx.try_recv().unwrap();
        assert!(instance_fence.acknowledge_marker_like_cpp(&marker));
        assert!(fixture.send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn stalled_instance_writer_keeps_committed_dual_wield_runtime_effect() {
        let mut fixture = trainer_wrapper_fixture();
        let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
        let learned_id = WRAPPER_LEARNED_SPELL as u32;
        let mut dual_wield_effect = player_learn_effect(2, wrapper_id, learned_id);
        dual_wield_effect.effect_index_raw = 1;
        dual_wield_effect.effect_type_raw = 40; // SPELL_EFFECT_DUAL_WIELD
        dual_wield_effect.effect_trigger_spell_raw = 0;
        fixture.session.set_spell_acquisition_catalog(Arc::new(
            SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
                [wrapper_id, learned_id]
                    .map(|spell_id| SpellAcquisitionCoverageSeedLikeCpp::covered(spell_id, 0)),
                EffectiveSpellAcquisitionRowsLikeCpp {
                    spell_effects: vec![
                        player_learn_effect(1, wrapper_id, learned_id),
                        dual_wield_effect,
                    ],
                    ..Default::default()
                },
                SpellAcquisitionTableHashesLikeCpp::default(),
                Vec::new(),
            ),
        ));
        let (realm_tx, realm_rx) = flume::bounded::<Vec<u8>>(8);
        let instance_fence = wow_network::SocketWriteFenceLikeCpp::default();
        fixture
            .session
            .install_realm_send_channel_for_test(realm_tx);
        fixture
            .session
            .set_send_write_fence_like_cpp(instance_fence);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
        assert!(
            fixture
                .session
                .mutate_canonical_player_like_cpp(|player| {
                    player.unit().can_dual_wield_like_cpp()
                })
                .expect("canonical player"),
            "non-packet runtime actions must install immediately after commit"
        );
        assert!(fixture
            .session
            .represented_spell_acquisition_post_commit_actions_like_cpp()
            .iter()
            .any(|action| matches!(
                action,
                crate::spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield { .. }
            )));
        assert_eq!(
            fixture.session.state(),
            crate::session::SessionState::Disconnecting
        );
        assert!(realm_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn audited_castable_wrapper_commits_its_projected_target_once() {
        let mut fixture = trainer_wrapper_fixture();

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_TRAINER_SPELL),
            "C++ casts a trainer wrapper; it does not learn the wrapper row"
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            wow_packet::packets::update::UpdateObject::player_money_update(
                fixture.session.player_guid().unwrap(),
                fixture.session.player_map_id_like_cpp(),
                75,
                None,
            )
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            PlaySpellVisualKit {
                unit: fixture.trainer,
                kit_record_id: 179,
                kit_type: 0,
                duration: 0,
                mounted_visual: false,
            }
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            PlaySpellVisualKit {
                unit: fixture.session.player_guid().unwrap(),
                kit_record_id: 362,
                kit_type: 1,
                duration: 0,
                mounted_visual: false,
            }
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            wow_packet::packets::trainer::LearnedSpells::single(WRAPPER_LEARNED_SPELL).to_bytes()
        );
        assert!(fixture.send_rx.try_recv().is_err());

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;
        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            TrainerBuyFailed {
                trainer_guid: fixture.trainer,
                spell_id: WRAPPER_TRAINER_SPELL,
                reason: 0,
            }
            .to_bytes()
        );
        assert!(fixture.send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn active_difficulty_channeled_wrapper_fails_closed_before_charge() {
        let mut fixture = trainer_wrapper_fixture();
        let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
        let learned_id = WRAPPER_LEARNED_SPELL as u32;
        fixture.session.set_spell_acquisition_catalog(Arc::new(
            SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
                [wrapper_id, learned_id]
                    .map(|spell_id| SpellAcquisitionCoverageSeedLikeCpp::covered(spell_id, 0)),
                EffectiveSpellAcquisitionRowsLikeCpp {
                    spell_effects: vec![player_learn_effect(1, wrapper_id, learned_id)],
                    spell_misc: vec![SpellAcquisitionMiscLikeCpp {
                        record_id: 2,
                        spell_id_raw: i64::from(wrapper_id),
                        difficulty_id_raw: 0,
                        // SPELL_ATTR1_IS_CHANNELLED
                        attributes_raw: [0, 0x0000_0004],
                        show_future_spell_player_condition_id_raw: 0,
                    }],
                    ..Default::default()
                },
                SpellAcquisitionTableHashesLikeCpp::default(),
                Vec::new(),
            ),
        ));

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 100);
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            TrainerBuyFailed {
                trainer_guid: fixture.trainer,
                spell_id: WRAPPER_TRAINER_SPELL,
                reason: 0,
            }
            .to_bytes()
        );
        assert!(fixture.send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn trainer_wrapper_consumes_active_difficulty_effect_rows_like_cpp() {
        const HEROIC_LEARNED_SPELL: u32 = 54_326;
        let store = trainer_store_from_rows(
            vec![trainer_row(DEFAULT_TRAINER_ID, 2, "Train")],
            vec![trainer_spell_row(
                DEFAULT_TRAINER_ID,
                WRAPPER_TRAINER_SPELL,
                25,
                1,
            )],
            Vec::new(),
            vec![CreatureTrainerRowLikeCpp {
                creature_id: CREATURE_ENTRY,
                trainer_id: DEFAULT_TRAINER_ID,
                menu_id: 0,
                option_id: 0,
            }],
        );
        let mut fixture = trainer_fixture_with_store_and_map_difficulty(store, 2);
        let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
        let normal_learned_id = WRAPPER_LEARNED_SPELL as u32;
        let mut heroic_effect = player_learn_effect(2, wrapper_id, HEROIC_LEARNED_SPELL);
        heroic_effect.difficulty_id_raw = 2;
        fixture.session.set_spell_acquisition_catalog(Arc::new(
            SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
                [
                    SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 0),
                    SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 2),
                    SpellAcquisitionCoverageSeedLikeCpp::covered(normal_learned_id, 0),
                    SpellAcquisitionCoverageSeedLikeCpp::covered(HEROIC_LEARNED_SPELL, 0),
                ],
                EffectiveSpellAcquisitionRowsLikeCpp {
                    spell_effects: vec![
                        player_learn_effect(1, wrapper_id, normal_learned_id),
                        heroic_effect,
                    ],
                    ..Default::default()
                },
                SpellAcquisitionTableHashesLikeCpp::default(),
                Vec::new(),
            ),
        ));
        let mut learn_skills = SpellLearnSkillStoreLikeCpp::default();
        learn_skills.covered_spell_ids.extend([
            wrapper_id,
            normal_learned_id,
            HEROIC_LEARNED_SPELL,
        ]);
        fixture
            .session
            .set_spell_learn_skill_store(Arc::new(learn_skills));
        fixture
            .session
            .set_spell_acquisition_static_authority_like_cpp([wrapper_id], []);
        fixture
            .session
            .set_spell_target_restrictions_store(Arc::new(
                wow_data::SpellTargetRestrictionsStore::from_entries([
                    spell_target_restriction_row(1, wrapper_id, 0, 1 << (3 - 1)),
                    spell_target_restriction_row(2, wrapper_id, 2, 1 << (7 - 1)),
                ]),
            ));
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
        fixture.session.set_player_gold_like_cpp(100);
        fixture
            .session
            .set_loot_money_persistence_test_result_like_cpp(true);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&(HEROIC_LEARNED_SPELL as i32))
        );
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
    }

    #[tokio::test]
    async fn active_difficulty_pet_aura_hook_fails_closed_before_charge() {
        const HEROIC_LEARNED_SPELL: u32 = 54_327;
        let mut fixture = trainer_wrapper_fixture_with_map_difficulty(2);
        let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
        let learned_id = WRAPPER_LEARNED_SPELL as u32;
        let mut heroic_effect = player_learn_effect(2, wrapper_id, HEROIC_LEARNED_SPELL);
        heroic_effect.difficulty_id_raw = 2;
        heroic_effect.effect_index_raw = 1;
        fixture.session.set_spell_acquisition_catalog(Arc::new(
            SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
                [
                    SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 0),
                    SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 2),
                    SpellAcquisitionCoverageSeedLikeCpp::covered(learned_id, 0),
                    SpellAcquisitionCoverageSeedLikeCpp::covered(HEROIC_LEARNED_SPELL, 0),
                ],
                EffectiveSpellAcquisitionRowsLikeCpp {
                    spell_effects: vec![
                        player_learn_effect(1, wrapper_id, learned_id),
                        heroic_effect,
                    ],
                    ..Default::default()
                },
                SpellAcquisitionTableHashesLikeCpp::default(),
                Vec::new(),
            ),
        ));
        let mut learn_skills = SpellLearnSkillStoreLikeCpp::default();
        learn_skills
            .covered_spell_ids
            .extend([wrapper_id, learned_id, HEROIC_LEARNED_SPELL]);
        fixture
            .session
            .set_spell_learn_skill_store(Arc::new(learn_skills));
        let pet_auras = wow_data::SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
            [wow_data::SpellPetAuraRowLikeCpp {
                spell_id: wrapper_id,
                effect_index: 1,
                pet_entry: 0,
                aura_id: 90_001,
            }],
            |_, _| {
                wow_data::SpellPetAuraSourceLookupLikeCpp::Found(
                    wow_data::SpellPetAuraSourceEffectLikeCpp {
                        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUMMY,
                        apply_aura_name: 0,
                        target_a: wow_data::TARGET_UNIT_PET_LIKE_CPP,
                        calc_value: 0,
                    },
                )
            },
            |_| true,
        );
        assert_eq!(pet_auras.loaded_row_count, 1);
        fixture
            .session
            .set_spell_pet_aura_store(Arc::new(pet_auras.store));

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 100);
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&(HEROIC_LEARNED_SPELL as i32))
        );
        while fixture.send_rx.try_recv().is_ok() {}

        let (disable_mgr, report) = wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
            [wow_data::DisableDbRowLikeCpp {
                source_type: wow_data::DISABLE_TYPE_SPELL,
                entry: wrapper_id,
                flags: wow_data::disable_mgr::SPELL_DISABLE_PLAYER,
                params_0: String::new(),
                params_1: String::new(),
            }],
            wow_data::DisableMgrRefsLikeCpp {
                spell_store: fixture.session.spell_store().map(AsRef::as_ref),
                ..Default::default()
            },
        );
        assert_eq!(report.loaded_count, 1);
        fixture.session.set_disable_mgr(Arc::new(disable_mgr));
        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&(HEROIC_LEARNED_SPELL as i32))
        );
        assert_trainer_charge_and_visuals_like_cpp(&mut fixture);
    }

    #[tokio::test]
    async fn player_disable_stops_cast_effects_after_charge_and_visuals_like_cpp() {
        let mut fixture = trainer_wrapper_fixture();
        let (disable_mgr, report) = wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
            [wow_data::DisableDbRowLikeCpp {
                source_type: wow_data::DISABLE_TYPE_SPELL,
                entry: WRAPPER_TRAINER_SPELL as u32,
                flags: wow_data::disable_mgr::SPELL_DISABLE_PLAYER,
                params_0: String::new(),
                params_1: String::new(),
            }],
            wow_data::DisableMgrRefsLikeCpp {
                spell_store: fixture.session.spell_store().map(AsRef::as_ref),
                ..Default::default()
            },
        );
        assert_eq!(report.loaded_count, 1);
        fixture.session.set_disable_mgr(Arc::new(disable_mgr));

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            wow_packet::packets::update::UpdateObject::player_money_update(
                fixture.session.player_guid().unwrap(),
                fixture.session.player_map_id_like_cpp(),
                75,
                None,
            )
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            PlaySpellVisualKit {
                unit: fixture.trainer,
                kit_record_id: 179,
                kit_type: 0,
                duration: 0,
                mounted_visual: false,
            }
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            PlaySpellVisualKit {
                unit: fixture.session.player_guid().unwrap(),
                kit_record_id: 362,
                kit_type: 1,
                duration: 0,
                mounted_visual: false,
            }
            .to_bytes()
        );
        assert!(fixture.send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn active_difficulty_aura_restriction_fails_after_charge_like_cpp() {
        let mut fixture = trainer_wrapper_fixture_with_map_difficulty(2);
        let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
        let learned_id = WRAPPER_LEARNED_SPELL as u32;
        let mut active_effect = player_learn_effect(2, wrapper_id, learned_id);
        active_effect.difficulty_id_raw = 2;
        fixture.session.set_spell_acquisition_catalog(Arc::new(
            SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
                [
                    SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 0),
                    SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 2),
                    SpellAcquisitionCoverageSeedLikeCpp::covered(learned_id, 0),
                ],
                EffectiveSpellAcquisitionRowsLikeCpp {
                    spell_effects: vec![
                        player_learn_effect(1, wrapper_id, learned_id),
                        active_effect,
                    ],
                    ..Default::default()
                },
                SpellAcquisitionTableHashesLikeCpp::default(),
                Vec::new(),
            ),
        ));
        fixture
            .session
            .set_spell_acquisition_static_authority_like_cpp([wrapper_id], []);
        fixture.session.set_spell_aura_restrictions_store(Arc::new(
            wow_data::SpellAuraRestrictionsStore::from_entries([
                wow_data::SpellAuraRestrictionsEntry {
                    id: 1,
                    difficulty_id: 2,
                    caster_aura_state: 0,
                    target_aura_state: 0,
                    exclude_caster_aura_state: 0,
                    exclude_target_aura_state: 0,
                    caster_aura_spell: 999,
                    target_aura_spell: 0,
                    exclude_caster_aura_spell: 0,
                    exclude_target_aura_spell: 0,
                    spell_id: wrapper_id,
                },
            ]),
        ));
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
        fixture.session.set_player_gold_like_cpp(100);
        fixture
            .session
            .set_loot_money_persistence_test_result_like_cpp(true);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
        assert_trainer_charge_and_visuals_like_cpp(&mut fixture);
    }

    #[tokio::test]
    async fn incompatible_wrapper_self_target_fails_after_charge_like_cpp() {
        let mut fixture = trainer_wrapper_fixture();
        fixture
            .session
            .set_spell_target_restrictions_store(Arc::new(
                wow_data::SpellTargetRestrictionsStore::from_entries([
                    spell_target_restriction_row(
                        1,
                        WRAPPER_TRAINER_SPELL as u32,
                        0,
                        1 << (3 - 1), // CREATURE_TYPEMASK_BEAST, not player/humanoid
                    ),
                ]),
            ));

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
        assert_trainer_charge_and_visuals_like_cpp(&mut fixture);
    }

    #[tokio::test]
    async fn definite_target_failure_precedes_unsupported_pet_aura_hook_like_cpp() {
        let mut fixture = trainer_wrapper_fixture();
        let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
        let pet_auras = wow_data::SpellPetAuraStoreLikeCpp::load_spell_pet_auras_like_cpp(
            [wow_data::SpellPetAuraRowLikeCpp {
                spell_id: wrapper_id,
                effect_index: 0,
                pet_entry: 0,
                aura_id: 90_002,
            }],
            |_, _| {
                wow_data::SpellPetAuraSourceLookupLikeCpp::Found(
                    wow_data::SpellPetAuraSourceEffectLikeCpp {
                        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_DUMMY,
                        apply_aura_name: 0,
                        target_a: wow_data::TARGET_UNIT_PET_LIKE_CPP,
                        calc_value: 0,
                    },
                )
            },
            |_| true,
        );
        assert_eq!(pet_auras.loaded_row_count, 1);
        fixture
            .session
            .set_spell_pet_aura_store(Arc::new(pet_auras.store));
        fixture
            .session
            .set_spell_target_restrictions_store(Arc::new(
                wow_data::SpellTargetRestrictionsStore::from_entries([
                    spell_target_restriction_row(
                        1,
                        wrapper_id,
                        0,
                        1 << (3 - 1), // CREATURE_TYPEMASK_BEAST, not player/humanoid
                    ),
                ]),
            ));

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
        assert_trainer_charge_and_visuals_like_cpp(&mut fixture);
    }

    #[tokio::test]
    async fn incomplete_persisted_aura_authority_stops_before_charge_like_cpp() {
        let mut fixture = trainer_wrapper_fixture();
        fixture
            .session
            .set_player_aura_authority_complete_like_cpp(false);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 100);
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
    }

    #[tokio::test]
    async fn creature_disable_does_not_block_the_player_cast_like_cpp() {
        let mut fixture = trainer_wrapper_fixture();
        let (disable_mgr, report) = wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
            [wow_data::DisableDbRowLikeCpp {
                source_type: wow_data::DISABLE_TYPE_SPELL,
                entry: WRAPPER_TRAINER_SPELL as u32,
                flags: wow_data::disable_mgr::SPELL_DISABLE_CREATURE,
                params_0: String::new(),
                params_1: String::new(),
            }],
            wow_data::DisableMgrRefsLikeCpp {
                spell_store: fixture.session.spell_store().map(AsRef::as_ref),
                ..Default::default()
            },
        );
        assert_eq!(report.loaded_count, 1);
        fixture.session.set_disable_mgr(Arc::new(disable_mgr));

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
    }

    #[tokio::test]
    async fn map_scoped_player_disable_only_blocks_matching_trainer_map_like_cpp() {
        for (disabled_map, expected_known) in [(0_u32, false), (1_u32, true)] {
            let mut fixture = trainer_wrapper_fixture();
            let (disable_mgr, report) = wow_data::DisableMgrLikeCpp::from_rows_like_cpp(
                [wow_data::DisableDbRowLikeCpp {
                    source_type: wow_data::DISABLE_TYPE_SPELL,
                    entry: WRAPPER_TRAINER_SPELL as u32,
                    flags: wow_data::disable_mgr::SPELL_DISABLE_PLAYER
                        | wow_data::disable_mgr::SPELL_DISABLE_MAP,
                    params_0: disabled_map.to_string(),
                    params_1: String::new(),
                }],
                wow_data::DisableMgrRefsLikeCpp {
                    spell_store: fixture.session.spell_store().map(AsRef::as_ref),
                    ..Default::default()
                },
            );
            assert_eq!(report.loaded_count, 1);
            fixture.session.set_disable_mgr(Arc::new(disable_mgr));

            fixture
                .session
                .handle_trainer_buy_spell(trainer_buy_packet(
                    fixture.trainer,
                    DEFAULT_TRAINER_ID as i32,
                    WRAPPER_TRAINER_SPELL,
                ))
                .await;

            assert_eq!(fixture.session.player_gold_like_cpp(), 75);
            assert_eq!(
                fixture
                    .session
                    .known_spells_like_cpp()
                    .contains(&WRAPPER_LEARNED_SPELL),
                expected_known
            );
        }
    }

    #[tokio::test]
    async fn trainer_player_cast_applies_caster_owned_skill_like_cpp() {
        let mut fixture = trainer_wrapper_fixture();
        let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
        let mut skill_effect = player_learn_effect(1, wrapper_id, 0);
        skill_effect.effect_type_raw =
            i64::from(wow_data::spell::spell_effect_types::SPELL_EFFECT_SKILL);
        skill_effect.effect_index_raw = 1;
        skill_effect.effect_trigger_spell_raw = 0;
        skill_effect.effect_misc_value_raw[0] = 164;
        skill_effect.effect_base_points_raw = 1;
        skill_effect.implicit_target_raw = [0, 0];
        fixture.session.set_spell_acquisition_catalog(Arc::new(
            SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
                [
                    SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 0),
                    SpellAcquisitionCoverageSeedLikeCpp::covered(WRAPPER_LEARNED_SPELL as u32, 0),
                ],
                EffectiveSpellAcquisitionRowsLikeCpp {
                    spell_effects: vec![
                        player_learn_effect(2, wrapper_id, WRAPPER_LEARNED_SPELL as u32),
                        skill_effect,
                    ],
                    ..Default::default()
                },
                SpellAcquisitionTableHashesLikeCpp::default(),
                Vec::new(),
            ),
        ));
        fixture.session.set_skill_store(Arc::new(
            SkillStore::from_skill_line_abilities_and_race_class_like_cpp(
                [],
                [SkillRaceClassInfoRecord {
                    id: 1,
                    race_mask: 0,
                    skill_id: 164,
                    class_mask: 0,
                    flags: 0,
                    availability: 1,
                    min_level: 1,
                    skill_tier_id: 1,
                }],
            ),
        ));
        fixture
            .session
            .set_skill_line_store(Arc::new(SkillLineStore::from_entries([SkillLineEntry {
                id: 164,
                display_name: "Blacksmithing".to_string(),
                alternate_verb: String::new(),
                description: String::new(),
                horde_display_name: String::new(),
                override_source_info_display_name: String::new(),
                category_id: wow_data::skill::SKILL_CATEGORY_SECONDARY_LIKE_CPP,
                spell_icon_file_id: 0,
                can_link: 0,
                parent_skill_line_id: 0,
                parent_tier_index: 0,
                flags: 0,
                spell_book_spell_id: 0,
            }])));
        let mut tier_values = [0; wow_data::MAX_SKILL_STEP_LIKE_CPP];
        tier_values[0] = 75;
        fixture.session.set_skill_tiers_store(Arc::new(
            SkillTiersStoreLikeCpp::from_rows_like_cpp([SkillTiersRowLikeCpp {
                id: 1,
                value: tier_values,
            }]),
        ));

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        let skill = &fixture.session.player_skill_records_like_cpp()[&164];
        assert_eq!((skill.step, skill.value, skill.max), (1, 1, 75));
        assert!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
    }

    #[tokio::test]
    async fn wrapper_with_missing_live_aura_metadata_fails_before_charge_or_publication() {
        let mut fixture = trainer_wrapper_fixture();
        seed_unclassified_active_aura(&mut fixture.session, 2);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 100);
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            TrainerBuyFailed {
                trainer_guid: fixture.trainer,
                spell_id: WRAPPER_TRAINER_SPELL,
                reason: 0,
            }
            .to_bytes()
        );
        assert!(fixture.send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn wrapper_ignores_covered_non_immunity_aura_like_cpp() {
        let mut fixture = trainer_wrapper_fixture();
        install_wrapper_and_aura_catalog(&mut fixture.session, 999, 79, 0, 0, 0);
        seed_unclassified_active_aura(&mut fixture.session, 2);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
    }

    #[tokio::test]
    async fn wrapper_ignores_effect_immunity_for_an_unrelated_effect_like_cpp() {
        let mut fixture = trainer_wrapper_fixture();
        install_wrapper_and_aura_catalog(
            &mut fixture.session,
            999,
            37, // SPELL_AURA_EFFECT_IMMUNITY
            10, // SPELL_EFFECT_HEAL, not the wrapper's learn effect
            0,
            0,
        );
        seed_unclassified_active_aura(&mut fixture.session, 2);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
    }

    #[tokio::test]
    async fn wrapper_effect_immunity_removes_only_its_matching_effect_like_cpp() {
        let mut fixture = trainer_wrapper_fixture();
        let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
        let learned_id = WRAPPER_LEARNED_SPELL as u32;
        let aura_spell_id = 999;
        let learn_effect = player_learn_effect(1, wrapper_id, learned_id);
        let mut dual_wield_effect = player_learn_effect(2, wrapper_id, learned_id);
        dual_wield_effect.effect_index_raw = 1;
        dual_wield_effect.effect_type_raw = 40; // SPELL_EFFECT_DUAL_WIELD
        dual_wield_effect.effect_trigger_spell_raw = 0;
        fixture.session.set_spell_acquisition_catalog(Arc::new(
            SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
                [wrapper_id, learned_id, aura_spell_id]
                    .map(|spell_id| SpellAcquisitionCoverageSeedLikeCpp::covered(spell_id, 0)),
                EffectiveSpellAcquisitionRowsLikeCpp {
                    spell_effects: vec![
                        learn_effect,
                        dual_wield_effect,
                        player_aura_effect(
                            3,
                            aura_spell_id,
                            37, // SPELL_AURA_EFFECT_IMMUNITY
                            40, // SPELL_EFFECT_DUAL_WIELD
                        ),
                    ],
                    ..Default::default()
                },
                SpellAcquisitionTableHashesLikeCpp::default(),
                Vec::new(),
            ),
        ));
        seed_unclassified_active_aura(&mut fixture.session, 2);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL),
            "C++ preserves the non-immunized learn effect bit"
        );
        assert!(
            !fixture
                .session
                .mutate_canonical_player_like_cpp(|player| {
                    player.unit().can_dual_wield_like_cpp()
                })
                .expect("canonical player"),
            "the immunized dual-wield effect bit must not execute"
        );
    }

    #[tokio::test]
    async fn wrapper_matches_negative_aura_link_to_the_cast_spell_like_cpp() {
        let mut unrelated = trainer_wrapper_fixture();
        install_wrapper_and_aura_catalog(&mut unrelated.session, 999, 79, 0, 0, 0);
        install_aura_link(&mut unrelated.session, 999, -12_345);
        seed_unclassified_active_aura(&mut unrelated.session, 2);
        unrelated
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                unrelated.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;
        assert_eq!(unrelated.session.player_gold_like_cpp(), 75);

        let mut matching = trainer_wrapper_fixture();
        install_wrapper_and_aura_catalog(&mut matching.session, 999, 79, 0, 0, 0);
        install_aura_link(&mut matching.session, 999, -WRAPPER_TRAINER_SPELL);
        seed_unclassified_active_aura(&mut matching.session, 2);
        matching
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                matching.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;
        assert_eq!(matching.session.player_gold_like_cpp(), 75);
        assert!(
            !matching
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
    }

    #[tokio::test]
    async fn wrapper_with_full_immunity_still_charges_and_publishes_visuals_like_cpp() {
        let mut fixture = trainer_wrapper_fixture();
        install_wrapper_and_aura_catalog(
            &mut fixture.session,
            999,
            37, // SPELL_AURA_EFFECT_IMMUNITY
            36, // SPELL_EFFECT_LEARN_SPELL
            0,
            0,
        );
        seed_unclassified_active_aura(&mut fixture.session, 2);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            wow_packet::packets::update::UpdateObject::player_money_update(
                fixture.session.player_guid().unwrap(),
                fixture.session.player_map_id_like_cpp(),
                75,
                None,
            )
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            PlaySpellVisualKit {
                unit: fixture.trainer,
                kit_record_id: 179,
                kit_type: 0,
                duration: 0,
                mounted_visual: false,
            }
            .to_bytes()
        );
        assert_eq!(
            fixture.send_rx.try_recv().unwrap(),
            PlaySpellVisualKit {
                unit: fixture.session.player_guid().unwrap(),
                kit_record_id: 362,
                kit_type: 1,
                duration: 0,
                mounted_visual: false,
            }
            .to_bytes()
        );
        assert!(fixture.send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn wrapper_immunity_aura_preserves_its_creation_difficulty_like_cpp() {
        let store = trainer_store_from_rows(
            vec![trainer_row(DEFAULT_TRAINER_ID, 2, "Train")],
            vec![trainer_spell_row(
                DEFAULT_TRAINER_ID,
                WRAPPER_TRAINER_SPELL,
                25,
                1,
            )],
            Vec::new(),
            vec![CreatureTrainerRowLikeCpp {
                creature_id: CREATURE_ENTRY,
                trainer_id: DEFAULT_TRAINER_ID,
                menu_id: 0,
                option_id: 0,
            }],
        );
        let mut fixture = trainer_fixture_with_store_and_map_difficulty(store, 2);
        let wrapper_id = WRAPPER_TRAINER_SPELL as u32;
        let learned_id = WRAPPER_LEARNED_SPELL as u32;
        let aura_spell_id = 999;
        let mut difficulty_immunity = player_aura_effect(
            3,
            aura_spell_id,
            37, // SPELL_AURA_EFFECT_IMMUNITY
            36, // SPELL_EFFECT_LEARN_SPELL
        );
        difficulty_immunity.difficulty_id_raw = 2;
        fixture.session.set_spell_acquisition_catalog(Arc::new(
            SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
                [
                    SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 0),
                    SpellAcquisitionCoverageSeedLikeCpp::covered(wrapper_id, 2),
                    SpellAcquisitionCoverageSeedLikeCpp::covered(learned_id, 0),
                    SpellAcquisitionCoverageSeedLikeCpp::covered(aura_spell_id, 0),
                    SpellAcquisitionCoverageSeedLikeCpp::covered(aura_spell_id, 2),
                ],
                EffectiveSpellAcquisitionRowsLikeCpp {
                    spell_effects: vec![
                        player_learn_effect(1, wrapper_id, learned_id),
                        player_aura_effect(2, aura_spell_id, 79, 0),
                        difficulty_immunity,
                    ],
                    ..Default::default()
                },
                SpellAcquisitionTableHashesLikeCpp::default(),
                Vec::new(),
            ),
        ));
        let mut learn_skills = SpellLearnSkillStoreLikeCpp::default();
        learn_skills
            .covered_spell_ids
            .extend([wrapper_id, learned_id]);
        fixture
            .session
            .set_spell_learn_skill_store(Arc::new(learn_skills));
        fixture
            .session
            .set_spell_acquisition_static_authority_like_cpp([wrapper_id], []);
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
        fixture.session.set_player_gold_like_cpp(100);
        fixture
            .session
            .set_loot_money_persistence_test_result_like_cpp(true);
        seed_unclassified_active_aura(&mut fixture.session, 2);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.current_map_difficulty_id_like_cpp(), 2);
        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL),
            "the retained difficulty-0 aura must not acquire its map-difficulty-2 immunity effect"
        );
    }

    #[tokio::test]
    async fn wrapper_effect_no_immunity_attribute_bypasses_effect_immunity_like_cpp() {
        let mut fixture = trainer_wrapper_fixture();
        install_wrapper_and_aura_catalog(
            &mut fixture.session,
            999,
            37, // SPELL_AURA_EFFECT_IMMUNITY
            36, // SPELL_EFFECT_LEARN_SPELL
            1,  // SpellEffectAttributes::NoImmunity
            0,
        );
        seed_unclassified_active_aura(&mut fixture.session, 2);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
    }

    #[tokio::test]
    async fn wrapper_spell_no_immunities_attribute_bypasses_effect_immunity_like_cpp() {
        let mut fixture = trainer_wrapper_fixture();
        install_wrapper_and_aura_catalog(
            &mut fixture.session,
            999,
            37, // SPELL_AURA_EFFECT_IMMUNITY
            36, // SPELL_EFFECT_LEARN_SPELL
            0,
            0x2000_0000, // SPELL_ATTR0_NO_IMMUNITIES
        );
        seed_unclassified_active_aura(&mut fixture.session, 2);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 75);
        assert!(
            fixture
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );

        let mut linked_id_immunity = trainer_wrapper_fixture();
        install_wrapper_and_aura_catalog(
            &mut linked_id_immunity.session,
            999,
            37,
            36,
            0,
            0x2000_0000,
        );
        install_aura_link(&mut linked_id_immunity.session, 999, -WRAPPER_TRAINER_SPELL);
        seed_unclassified_active_aura(&mut linked_id_immunity.session, 2);
        linked_id_immunity
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                linked_id_immunity.trainer,
                DEFAULT_TRAINER_ID as i32,
                WRAPPER_TRAINER_SPELL,
            ))
            .await;
        assert_eq!(linked_id_immunity.session.player_gold_like_cpp(), 75);
        assert!(
            !linked_id_immunity
                .session
                .known_spells_like_cpp()
                .contains(&WRAPPER_LEARNED_SPELL)
        );
    }

    #[tokio::test]
    async fn definite_trainer_commit_failure_never_charges_grants_or_publishes() {
        let mut fixture = trainer_fixture();
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
        fixture.session.set_player_gold_like_cpp(100);
        fixture
            .session
            .set_loot_money_persistence_test_result_like_cpp(false);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                AVAILABLE_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 100);
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&AVAILABLE_TRAINER_SPELL)
        );
        assert!(fixture.send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn missing_character_database_never_charges_grants_or_publishes() {
        let mut fixture = trainer_fixture();
        fixture
            .session
            .set_player_trainer_interaction_like_cpp(fixture.trainer, DEFAULT_TRAINER_ID);
        fixture.session.set_player_gold_like_cpp(100);

        fixture
            .session
            .handle_trainer_buy_spell(trainer_buy_packet(
                fixture.trainer,
                DEFAULT_TRAINER_ID as i32,
                AVAILABLE_TRAINER_SPELL,
            ))
            .await;

        assert_eq!(fixture.session.player_gold_like_cpp(), 100);
        assert!(
            !fixture
                .session
                .known_spells_like_cpp()
                .contains(&AVAILABLE_TRAINER_SPELL)
        );
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
    fn buy_registration_carries_the_call_while_legacy_shortcuts_stay_disabled() {
        let trainer = include_str!("trainer.rs");
        assert!(trainer.contains("opcode: ClientOpcodes::TrainerBuySpell"));
        // #359: the registration is the only declaration; it carries the call.
        assert!(trainer.contains("session.handle_trainer_buy_spell(pkt).await"));

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
                "#159 must use the prepared atomic boundary, not legacy shortcut `{forbidden}`"
            );
        }
    }

    #[tokio::test]
    async fn trainer_buy_wire_dispatch_reaches_handler_only_when_logged_in_like_cpp() {
        const MISSING_SPELL: i32 = 99_001;

        fn trainer_buy_wire_packet(
            trainer_guid: ObjectGuid,
            trainer_id: i32,
            spell_id: i32,
        ) -> WorldPacket {
            let mut packet = WorldPacket::new_empty();
            packet.write_uint16(ClientOpcodes::TrainerBuySpell as u16);
            packet.write_packed_guid(&trainer_guid);
            packet.write_int32(trainer_id);
            packet.write_int32(spell_id);
            packet.reset_read();
            packet
        }

        let mut logged_in = trainer_fixture();
        logged_in.session.set_state(SessionState::LoggedIn);
        logged_in
            .session
            .set_player_trainer_interaction_like_cpp(logged_in.trainer, DEFAULT_TRAINER_ID);
        logged_in
            .session
            .dispatch_packet(trainer_buy_wire_packet(
                logged_in.trainer,
                DEFAULT_TRAINER_ID as i32,
                MISSING_SPELL,
            ))
            .await;
        assert_eq!(
            logged_in.send_rx.try_recv().expect("TrainerBuyFailed"),
            TrainerBuyFailed {
                trainer_guid: logged_in.trainer,
                spell_id: MISSING_SPELL,
                reason: 0,
            }
            .to_bytes()
        );
        assert!(logged_in.send_rx.try_recv().is_err());

        let mut authed = trainer_fixture();
        authed
            .session
            .set_player_trainer_interaction_like_cpp(authed.trainer, DEFAULT_TRAINER_ID);
        authed
            .session
            .dispatch_packet(trainer_buy_wire_packet(
                authed.trainer,
                DEFAULT_TRAINER_ID as i32,
                MISSING_SPELL,
            ))
            .await;
        assert!(
            authed.send_rx.try_recv().is_err(),
            "LoggedIn metadata must reject TrainerBuySpell while the session is Authed"
        );
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
