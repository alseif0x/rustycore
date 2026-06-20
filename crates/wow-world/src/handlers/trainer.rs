// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Trainer handlers: CMSG_TRAINER_LIST, CMSG_TRAINER_BUY_SPELL.
//!
//! Flow for CMSG_TRAINER_LIST:
//!   1. Parse trainer GUID from packet.
//!   2. Resolve creature entry (NPC template ID) from in-memory tracker or DB.
//!   3. Look up TrainerId in creature_trainer by creature entry.
//!   4. Load spells from trainer_spell for that TrainerId.
//!   5. Determine usability per spell (known / available / unavailable).
//!   6. Send SMSG_TRAINER_LIST.
//!
//! Flow for CMSG_TRAINER_BUY_SPELL:
//!   1. Parse trainer GUID, trainer ID, spell ID.
//!   2. Validate: spell not already known, level sufficient, enough gold.
//!   3. Deduct gold, persist to DB, insert character_spell, update known_spells.
//!   4. Send SMSG_LEARNED_SPELLS (success) or SMSG_TRAINER_BUY_FAILED (error).
//!
//! C# reference: Game/Handlers/NPCHandler.cs, Game/Entities/Creature/Trainer.cs

use std::sync::Arc;

use tracing::{info, warn};

use wow_constants::ClientOpcodes;
use wow_constants::unit::NPCFlags1;
use wow_database::statements::character::CharStatements;
use wow_database::statements::world::WorldStatements;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::ClientPacket;
use wow_packet::packets::trainer::{
    LearnedSpells, TrainerBuyFailed, TrainerBuySpellRequest, TrainerListPacket, TrainerListSpell,
};

use crate::conditions;
use crate::session::{RepresentedQuestObjectiveProgressEventLikeCpp, WorldSession};

const TRAINER_LIST_NPC_FLAGS_LIKE_CPP: u32 = NPCFlags1::TRAINER.bits();
const TRAINER_BUY_NPC_FLAGS_LIKE_CPP: u32 = NPCFlags1::TRAINER.bits()
    | NPCFlags1::TRAINER_CLASS.bits()
    | NPCFlags1::TRAINER_PROFESSION.bits();

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
    /// Handle `CMSG_TRAINER_LIST` (0x34ad).
    ///
    /// Opens the trainer window: resolves the NPC, loads spells from DB,
    /// and sends SMSG_TRAINER_LIST back to the client.
    pub async fn handle_trainer_list(&mut self, hello: wow_packet::packets::gossip::Hello) {
        let trainer_guid = hello.unit;
        info!(
            account = self.account_id,
            trainer_guid = ?trainer_guid,
            "CMSG_TRAINER_LIST"
        );

        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        // ── Resolve creature entry ─────────────────────────────────────────
        let entry = if self.has_canonical_map_manager_like_cpp() {
            match self.represented_npc_can_interact_with_like_cpp(
                trainer_guid,
                TRAINER_LIST_NPC_FLAGS_LIKE_CPP,
                0,
            ) {
                Some(access) => access.entry,
                None => {
                    warn!(
                        account = self.account_id,
                        trainer_guid = ?trainer_guid,
                        "Trainer GUID not found or not interactable in canonical map"
                    );
                    return;
                }
            }
        } else {
            match self.mutate_world_creature(trainer_guid, |creature| creature.entry()) {
                Some(entry) => entry,
                None => {
                    // Fallback: query DB by spawn GUID
                    let mut stmt = world_db.prepare(WorldStatements::SEL_CREATURE_ENTRY_BY_GUID);
                    stmt.set_u64(0, trainer_guid.low_value() as u64);
                    let fallback = match tokio::time::timeout(
                        std::time::Duration::from_secs(2),
                        world_db.query(&stmt),
                    )
                    .await
                    {
                        Ok(Ok(r)) if !r.is_empty() => r.try_read::<u32>(0),
                        _ => None,
                    };
                    match fallback {
                        Some(e) => {
                            info!(
                                account = self.account_id,
                                "Trainer entry {} resolved from DB (GUID not in tracker)", e
                            );
                            e
                        }
                        None => {
                            warn!(
                                account = self.account_id,
                                trainer_guid = ?trainer_guid,
                                "Trainer GUID not found in tracker or DB"
                            );
                            return;
                        }
                    }
                }
            }
        };

        // ── Look up TrainerId ──────────────────────────────────────────────
        let mut ct_stmt = world_db.prepare(WorldStatements::SEL_TRAINER_BY_CREATURE);
        ct_stmt.set_u32(0, entry);
        let trainer_id =
            match tokio::time::timeout(std::time::Duration::from_secs(2), world_db.query(&ct_stmt))
                .await
            {
                Ok(Ok(r)) if !r.is_empty() => match r.try_read::<u32>(0) {
                    Some(id) => id,
                    None => {
                        warn!(
                            account = self.account_id,
                            entry = entry,
                            "creature_trainer row has no TrainerId"
                        );
                        return;
                    }
                },
                _ => {
                    warn!(
                        account = self.account_id,
                        entry = entry,
                        "No creature_trainer row for entry"
                    );
                    return;
                }
            };

        // ── Load trainer info (type + greeting) ────────────────────────────
        let (trainer_type, greeting) = {
            let mut ti_stmt = world_db.prepare(WorldStatements::SEL_TRAINER_INFO);
            ti_stmt.set_u32(0, trainer_id);
            match tokio::time::timeout(std::time::Duration::from_secs(2), world_db.query(&ti_stmt))
                .await
            {
                Ok(Ok(r)) if !r.is_empty() => {
                    let t = r.try_read::<i32>(1).unwrap_or(0);
                    let g = r.try_read::<String>(2).unwrap_or_default();
                    (t, g)
                }
                _ => (0i32, String::new()),
            }
        };

        // ── Load trainer spells ────────────────────────────────────────────
        let mut ts_stmt = world_db.prepare(WorldStatements::SEL_TRAINER_SPELLS);
        ts_stmt.set_u32(0, trainer_id);
        let mut ts_result =
            match tokio::time::timeout(std::time::Duration::from_secs(5), world_db.query(&ts_stmt))
                .await
            {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    warn!(
                        account = self.account_id,
                        trainer_id = trainer_id,
                        "trainer_spell query failed: {e}"
                    );
                    return;
                }
                Err(_) => {
                    warn!(
                        account = self.account_id,
                        trainer_id = trainer_id,
                        "trainer_spell query timed out"
                    );
                    return;
                }
            };

        let player_level = self.player_level_like_cpp();
        let condition_store = self.condition_store().cloned();
        let player_condition_store = self.player_condition_store().cloned();
        let player_condition_context = self.represented_player_condition_context_like_cpp();
        let player_condition_object = self.build_condition_player_object_like_cpp();
        let player_unit_snapshot = self.condition_player_unit_snapshot_like_cpp();
        let player_snapshot = self.condition_player_snapshot_like_cpp();
        let mut spells: Vec<TrainerListSpell> = Vec::new();

        if !ts_result.is_empty() {
            loop {
                let spell_id: i32 = ts_result.try_read(0).unwrap_or(0);
                let money_cost: u32 = ts_result.try_read::<u32>(1).unwrap_or(0);
                let req_skill_line: i32 = ts_result.try_read(2).unwrap_or(0);
                let req_skill_rank: i32 = ts_result.try_read(3).unwrap_or(0);
                let req_ability1: i32 = ts_result.try_read(4).unwrap_or(0);
                let req_ability2: i32 = ts_result.try_read(5).unwrap_or(0);
                let req_ability3: i32 = ts_result.try_read(6).unwrap_or(0);
                let req_level: u8 = ts_result.try_read::<u8>(7).unwrap_or(0);

                if let Some(store) = condition_store.as_ref() {
                    let Some(player_object) = player_condition_object.as_ref() else {
                        warn!(
                            account = self.account_id,
                            trainer_id = trainer_id,
                            spell_id = spell_id,
                            "Trainer spell condition check failed closed: missing player object"
                        );
                        if !ts_result.next_row() {
                            break;
                        }
                        continue;
                    };
                    let Some(spell_id_u32) = u32::try_from(spell_id).ok() else {
                        if !ts_result.next_row() {
                            break;
                        }
                        continue;
                    };
                    let meets = conditions::is_object_meeting_trainer_spell_conditions_like_cpp(
                        store.as_ref(),
                        trainer_id,
                        spell_id_u32,
                        Some(player_object),
                        |condition, source_info| {
                            source_info.set_unit_target_snapshot(0, player_unit_snapshot);
                            source_info.set_player_target_snapshot(0, player_snapshot);
                            if let Some(store) = player_condition_store.as_ref() {
                                source_info.set_player_condition_store(store.as_ref());
                                source_info.set_player_condition_context(
                                    0,
                                    player_condition_context.as_context(self),
                                );
                            }
                            match conditions::condition_meets_basic_like_cpp(
                                condition,
                                source_info,
                                |current_area, required_area| current_area == required_area,
                            ) {
                                conditions::ConditionMeetResult::Evaluated(value) => value,
                                conditions::ConditionMeetResult::Unsupported => {
                                    warn!(
                                        account = self.account_id,
                                        trainer_id = trainer_id,
                                        spell_id = spell_id,
                                        condition_type = ?condition.condition_type,
                                        "Trainer spell condition check failed closed: unsupported condition type"
                                    );
                                    false
                                }
                            }
                        },
                    );
                    if !meets {
                        if !ts_result.next_row() {
                            break;
                        }
                        continue;
                    }
                }

                // Determine usability:
                // 2 = already known, 1 = available (level ok), 0 = unavailable (too low level)
                let usable: u8 = if self.known_spells_like_cpp().contains(&spell_id) {
                    2
                } else if player_level >= req_level {
                    1
                } else {
                    0
                };

                spells.push(TrainerListSpell {
                    spell_id,
                    money_cost,
                    req_skill_line,
                    req_skill_rank,
                    req_ability: [req_ability1, req_ability2, req_ability3],
                    usable,
                    req_level,
                });

                if !ts_result.next_row() {
                    break;
                }
            }
        }

        info!(
            account = self.account_id,
            trainer_id = trainer_id,
            spell_count = spells.len(),
            "Sending SMSG_TRAINER_LIST"
        );

        self.send_packet(&TrainerListPacket {
            trainer_guid,
            trainer_type,
            trainer_id: trainer_id as i32,
            spells,
            greeting,
        });
    }

    /// Handle `CMSG_TRAINER_BUY_SPELL` (0x34ae).
    ///
    /// Validates the purchase (level, gold), deducts cost, inserts character_spell,
    /// updates in-memory state, and sends SMSG_LEARNED_SPELLS on success.
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

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => {
                warn!(
                    account = self.account_id,
                    "handle_trainer_buy_spell: no player_guid"
                );
                return;
            }
        };

        if self.has_canonical_map_manager_like_cpp()
            && self
                .represented_npc_can_interact_with_like_cpp(
                    trainer_guid,
                    TRAINER_BUY_NPC_FLAGS_LIKE_CPP,
                    0,
                )
                .is_none()
        {
            warn!(
                account = self.account_id,
                trainer_guid = ?trainer_guid,
                "Trainer buy rejected: trainer not interactable in canonical map"
            );
            return;
        }

        // ── Already known? ─────────────────────────────────────────────────
        if self.known_spells_like_cpp().contains(&spell_id) {
            warn!(
                account = self.account_id,
                spell_id = spell_id,
                "Player already knows spell"
            );
            self.send_packet(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0, // service unavailable (already known)
            });
            return;
        }

        // ── Load spell requirements from DB ────────────────────────────────
        let world_db = match self.world_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut ts_stmt = world_db.prepare(WorldStatements::SEL_TRAINER_SPELLS);
        ts_stmt.set_u32(0, trainer_id as u32);

        let ts_result =
            match tokio::time::timeout(std::time::Duration::from_secs(5), world_db.query(&ts_stmt))
                .await
            {
                Ok(Ok(r)) => r,
                Ok(Err(e)) => {
                    warn!(account = self.account_id, "trainer_spell query failed: {e}");
                    return;
                }
                Err(_) => {
                    warn!(account = self.account_id, "trainer_spell query timed out");
                    return;
                }
            };

        // Find the matching spell in results
        let mut money_cost: u32 = 0;
        let mut req_level: u8 = 0;
        let mut found = false;

        if !ts_result.is_empty() {
            let mut result = ts_result;
            loop {
                let row_spell_id: i32 = result.try_read(0).unwrap_or(0);
                if row_spell_id == spell_id {
                    money_cost = result.try_read::<u32>(1).unwrap_or(0);
                    req_level = result.try_read::<u8>(7).unwrap_or(0);
                    found = true;
                    break;
                }
                if !result.next_row() {
                    break;
                }
            }
        }

        if !found {
            warn!(
                account = self.account_id,
                trainer_id = trainer_id,
                spell_id = spell_id,
                "Spell not in trainer's list"
            );
            self.send_packet(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
            return;
        }

        // ── Validate level ─────────────────────────────────────────────────
        if self.player_level_like_cpp() < req_level {
            warn!(
                account = self.account_id,
                spell_id = spell_id,
                player_level = self.player_level_like_cpp(),
                req_level = req_level,
                "Player level too low for spell"
            );
            self.send_packet(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
            return;
        }

        // ── Validate gold ──────────────────────────────────────────────────
        if self.player_gold_like_cpp() < money_cost as u64 {
            warn!(
                account = self.account_id,
                spell_id = spell_id,
                player_gold = self.player_gold_like_cpp(),
                money_cost = money_cost,
                "Player doesn't have enough gold for spell"
            );
            self.send_packet(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 1, // not enough money
            });
            return;
        }

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        // ── Deduct gold ────────────────────────────────────────────────────
        let old_money = self.player_gold_like_cpp();
        let new_money = old_money.saturating_sub(money_cost as u64);
        self.enqueue_represented_quest_objective_progress_like_cpp(
            RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                old_money,
                new_money,
            },
        );
        self.set_player_gold_like_cpp(new_money);
        let mut upd_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        upd_money.set_u64(0, self.player_gold_like_cpp());
        upd_money.set_u64(1, player_guid.counter() as u64);
        if let Err(e) = char_db.execute(&upd_money).await {
            warn!(
                account = self.account_id,
                "TrainerBuySpell: update money failed: {e}"
            );
        }

        // ── Persist spell to character_spell ───────────────────────────────
        let mut ins_spell = char_db.prepare(CharStatements::INS_CHARACTER_SPELL);
        ins_spell.set_u64(0, player_guid.counter() as u64);
        ins_spell.set_i32(1, spell_id);
        if let Err(e) = char_db.execute(&ins_spell).await {
            warn!(
                account = self.account_id,
                spell_id = spell_id,
                "TrainerBuySpell: insert character_spell failed: {e}"
            );
        }

        // ── Update in-memory state ─────────────────────────────────────────
        self.learn_known_spell_like_cpp(spell_id);
        self.sync_player_registry_state_like_cpp();
        self.drain_represented_quest_objective_progress_like_cpp()
            .await;

        info!(
            account = self.account_id,
            player_guid = ?player_guid,
            spell_id = spell_id,
            money_cost = money_cost,
            remaining_gold = self.player_gold_like_cpp(),
            "Player learned spell from trainer"
        );

        // ── Send gold update to client ─────────────────────────────────────
        self.send_player_values_update_from_entity_bridge(
            &[],
            &[],
            &[],
            &[],
            Some(self.player_gold_like_cpp()),
        );

        // ── Send SMSG_LEARNED_SPELLS ───────────────────────────────────────
        self.send_packet(&LearnedSpells::single(spell_id));
    }
}
