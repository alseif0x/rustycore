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
//!   2. Validate: spell not already known, level sufficient, enough gold.
//!   3. Deduct gold, persist to DB, insert character_spell, update known_spells.
//!   4. Send SMSG_LEARNED_SPELLS (success) or SMSG_TRAINER_BUY_FAILED (error).
//!
//! C++ refs: `WorldSession::HandleTrainerListOpcode` / `SendTrainerList`
//! (`Handlers/NPCHandler.cpp:98-132`) and `Trainer::SendSpells` /
//! `Trainer::TeachSpell` (`Entities/Creature/Trainer.cpp:41-231`).

use std::sync::Arc;

use tracing::{info, warn};

use wow_constants::ClientOpcodes;
use wow_constants::unit::NPCFlags1;
use wow_data::{
    TRAINER_SPELL_STATE_AVAILABLE_LIKE_CPP, TRAINER_SPELL_STATE_KNOWN_LIKE_CPP,
    TRAINER_SPELL_STATE_UNAVAILABLE_LIKE_CPP, TrainerLikeCpp, TrainerStoreLikeCpp,
};
use wow_database::SqlTransaction;
use wow_database::statements::character::CharStatements;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::ClientPacket;
use wow_packet::packets::trainer::{
    LearnedSpells, TrainerBuyFailed, TrainerBuySpellRequest, TrainerListPacket, TrainerListSpell,
};

use crate::conditions;
use crate::session::WorldSession;

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
        let entry = match self.represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            required_npc_flags,
            0,
        ) {
            Some(access) => access.entry,
            None => {
                warn!(
                    account = self.account_id,
                    trainer_guid = ?trainer_guid,
                    "Trainer GUID not found or not interactable"
                );
                return;
            }
        };

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

        let player_level = self.player_level_like_cpp();
        let condition_store = self.condition_store().cloned();
        let player_condition_store = self.player_condition_store().cloned();
        let player_condition_context = self.represented_player_condition_context_like_cpp();
        let player_condition_object = self.build_condition_player_object_like_cpp();
        let player_unit_snapshot = self.condition_player_unit_snapshot_like_cpp();
        let player_snapshot = self.condition_player_snapshot_like_cpp();
        let mut spells: Vec<TrainerListSpell> = Vec::new();

        for trainer_spell in trainer.spells_like_cpp() {
            let spell_id = trainer_spell.spell_id as i32;
            if let Some(store) = condition_store.as_ref() {
                let Some(player_object) = player_condition_object.as_ref() else {
                    warn!(
                        account = self.account_id,
                        trainer_id = trainer_id,
                        spell_id = spell_id,
                        "Trainer spell condition check failed closed: missing player object"
                    );
                    continue;
                };
                let meets = conditions::is_object_meeting_trainer_spell_conditions_like_cpp(
                    store.as_ref(),
                    trainer_id,
                    trainer_spell.spell_id,
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
                    continue;
                }
            }

            let usable = if self.known_spells_like_cpp().contains(&spell_id) {
                TRAINER_SPELL_STATE_KNOWN_LIKE_CPP
            } else if player_level >= trainer_spell.req_level {
                TRAINER_SPELL_STATE_AVAILABLE_LIKE_CPP
            } else {
                TRAINER_SPELL_STATE_UNAVAILABLE_LIKE_CPP
            };

            spells.push(TrainerListSpell {
                spell_id,
                money_cost: trainer_spell.money_cost,
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

        if self
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
                "Trainer buy rejected: trainer not interactable"
            );
            return;
        }

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
        let Some(trainer_spell) = trainer.get_spell_like_cpp(spell_id as u32) else {
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
        let money_cost = trainer_spell.money_cost;
        let req_level = trainer_spell.req_level;

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

        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };

        // ── Deduct gold ────────────────────────────────────────────────────
        let old_money = self.player_gold_like_cpp();
        let new_money = old_money.saturating_sub(money_cost as u64);
        let mut tx = SqlTransaction::new();
        let mut upd_money = char_db.prepare(CharStatements::UPD_CHAR_MONEY);
        upd_money.set_u64(0, new_money);
        upd_money.set_u64(1, player_guid.counter() as u64);
        tx.append(upd_money);

        // ── Persist spell to character_spell ───────────────────────────────
        let mut ins_spell = char_db.prepare(CharStatements::INS_CHARACTER_SPELL);
        ins_spell.set_u64(0, player_guid.counter() as u64);
        ins_spell.set_i32(1, spell_id);
        tx.append(ins_spell);
        let Some(money_persistence) = self
            .commit_exclusive_player_money_transaction_like_cpp(
                money_persistence,
                char_db.as_ref(),
                tx,
                old_money,
                new_money,
                "trainer spell purchase",
            )
            .await
        else {
            warn!(
                account = self.account_id,
                spell_id = spell_id,
                "TrainerBuySpell: atomic money/spell transaction did not commit"
            );
            self.send_packet(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
            return;
        };
        // Publish every runtime field represented by the committed money/spell
        // transaction before reopening money-payout admission. There must be
        // no cancellation point between COMMIT and this publication.
        self.stage_player_money_change_like_cpp(old_money, new_money);
        self.learn_known_spell_like_cpp(spell_id);
        self.sync_object_accessor_player();
        self.sync_player_registry_state_like_cpp();
        drop(money_persistence);

        // ── Update in-memory state ─────────────────────────────────────────
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

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use super::*;
    use crate::session::{AuraApplication, RepresentedAuraEffectLikeCpp, SessionPlayerController};
    use wow_constants::unit::UnitState;
    use wow_core::guid::HighGuid;
    use wow_core::{ObjectGuid, Position};
    use wow_data::{
        CreatureTrainerRowLikeCpp, TrainerLocaleRowLikeCpp, TrainerRowLikeCpp, TrainerSpellLikeCpp,
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
        fixture
            .session
            .learn_known_spell_like_cpp(KNOWN_TRAINER_SPELL);
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
