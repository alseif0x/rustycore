// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell cast handlers — CMSG_CAST_SPELL, CMSG_CANCEL_CAST, CMSG_CANCEL_CHANNELLING.
//!
//! Phase 2 ("efectos mecánicos"):
//!   1. Parse CMSG_CAST_SPELL.
//!   2. Validate known spell + cooldown.
//!   3. If cast_time > 0: send SMSG_SPELL_START, store active_spell_cast, wait.
//!   4. When cast completes: execute_spell() → apply effects + cooldown.
//!   5. If instant: execute immediately.
//!
//! Future phases will add:
//!   - Movement cancellation (CMSG_MOVE_* while casting)
//!   - Channelling spells (tick-based damage)
//!   - Interrupts & silences
//!
//! Reference: C# Game/Handlers/SpellHandler.cs, Game/Spells/Spell.cs

use std::{collections::HashMap, sync::Arc};

use rand::Rng;
use tracing::{debug, info, warn};

use wow_constants::{
    BagFamilyMask, ClientOpcodes, InventoryResult, ItemFieldFlags, ItemFlags, ItemUpdateState,
    TypeId,
};
use wow_core::ObjectGuid;
use wow_data::{DISABLE_TYPE_SPELL, DisableWorldObjectRefLikeCpp};
use wow_database::{CharStatements, SqlTransaction, WorldStatements};
use wow_entities::INVENTORY_SLOT_BAG_0;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_loot::{
    LootConditionRowLikeCpp, condition_compare_values_like_cpp,
    generate_money_loot_with_rate_like_cpp, loot_condition_reference_ids_like_cpp,
    loot_condition_reference_self_references_like_cpp,
    loot_condition_row_normalize_without_external_stores_like_cpp,
    loot_conditions_allow_player_with_references_like_cpp_representable,
};
use wow_packet::ClientPacket;
use wow_packet::packets::item::{ItemExpirePurchaseRefund, ItemInstance};
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_TYPE_ITEM_LIKE_CPP, LootEntry, LootEntryFlags, LootItemData, LootResponse,
};
use wow_packet::packets::spell::{
    CastFailed, CastSpellRequest, OpenItem, SpellCastVisual, SpellStartPkt,
};

use crate::session::WorldSession;

const LOOT_MODE_DEFAULT_LIKE_CPP: u16 = 1;
const MAX_NR_LOOT_ITEMS_LIKE_CPP: usize = 18;
const MAX_LOOT_REFERENCE_FRAMES_LIKE_CPP: u32 = 64;
const ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP: u32 = 0x0002;
const ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP: u32 = 0x0004;
const CONDITION_SOURCE_TYPE_ITEM_LOOT_TEMPLATE_LIKE_CPP: i32 = 5;
const CONDITION_SOURCE_TYPE_REFERENCE_LOOT_TEMPLATE_LIKE_CPP: i32 = 10;
const CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP: i32 = 51;
const CONDITION_TYPE_MASK_LIKE_CPP: i32 = 52;
const TYPEID_PLAYER_LIKE_CPP: u32 = 6;
const PLAYER_TYPE_MASK_LIKE_CPP: u32 = 0x0001 | 0x0020 | 0x0040;
const SPELL_FAILED_SPELL_UNAVAILABLE_LIKE_CPP: i32 = 128;
const MAP_BATTLEGROUND_LIKE_CPP: i8 = 3;
const MAP_ARENA_LIKE_CPP: i8 = 4;

// ── Handler registrations ─────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CastSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_cast_spell",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelCast,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_cancel_cast",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelChannelling,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_channelling",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::OpenItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_open_item",
    }
}

// ── Handler implementations ───────────────────────────────────────

impl WorldSession {
    /// Handle `CMSG_CAST_SPELL` (0x329C).
    ///
    /// Phase 2: cast timers + cooldowns + mechanical effects.
    ///
    /// Flow:
    /// 1. Validate spell is known.
    /// 2. Validate cooldown.
    /// 3. If cast_time > 0: initiate cast (SMSG_SPELL_START), wait for tick_active_spell_cast().
    /// 4. If instant: execute immediately.
    pub async fn handle_cast_spell(&mut self, mut pkt: wow_packet::WorldPacket) {
        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => {
                warn!("handle_cast_spell: no player_guid");
                return;
            }
        };

        let req = match CastSpellRequest::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to parse CMSG_CAST_SPELL: {e}"
                );
                return;
            }
        };

        let spell_id = req.spell_id;
        let cast_id = req.cast_id;

        debug!(
            account = self.account_id,
            spell_id = spell_id,
            cast_id = ?cast_id,
            target = ?req.target.unit,
            "CMSG_CAST_SPELL"
        );

        // ── Validation: Known spell ─────────────────────────────────────
        if !self.known_spells_like_cpp().contains(&spell_id) {
            warn!(
                account = self.account_id,
                spell_id = spell_id,
                "Cast attempt for unknown spell"
            );
            self.send_packet(&CastFailed {
                cast_id,
                spell_id,
                reason: 2, // SpellCastResult::NotKnown
                fail_arg1: 0,
                fail_arg2: 0,
            });
            return;
        }

        // ── Get spell info ──────────────────────────────────────────────
        let spell_info: &wow_data::SpellInfo = match &self.spell_store {
            Some(store) => match store.get(spell_id) {
                Some(info) => info,
                None => {
                    warn!(
                        account = self.account_id,
                        spell_id = spell_id,
                        "Spell not found in store"
                    );
                    self.send_packet(&CastFailed {
                        cast_id,
                        spell_id,
                        reason: 2,
                        fail_arg1: 0,
                        fail_arg2: 0,
                    });
                    return;
                }
            },
            None => {
                warn!(account = self.account_id, "No spell store available");
                self.send_packet(&CastFailed {
                    cast_id,
                    spell_id,
                    reason: 2,
                    fail_arg1: 0,
                    fail_arg2: 0,
                });
                return;
            }
        };

        // C++ `Spell::CheckCast`: disabled spells fail with
        // `SPELL_FAILED_SPELL_UNAVAILABLE` before cooldown/cast processing.
        if self.is_spell_disabled_for_player_like_cpp(spell_id) {
            warn!(
                account = self.account_id,
                spell_id = spell_id,
                "Cast attempt for disabled spell"
            );
            self.send_packet(&CastFailed {
                cast_id,
                spell_id,
                reason: SPELL_FAILED_SPELL_UNAVAILABLE_LIKE_CPP,
                fail_arg1: 0,
                fail_arg2: 0,
            });
            return;
        }

        // ── Validation: Cooldown ────────────────────────────────────────
        // Check global cooldown (GCD)
        if let Some(last_cast) = self.last_spell_cast_time {
            let cooldown_ms = spell_info.effective_cooldown_ms();
            let elapsed_ms = last_cast.elapsed().as_millis() as u32;

            if elapsed_ms < cooldown_ms {
                debug!(
                    account = self.account_id,
                    spell_id = spell_id,
                    remaining_ms = cooldown_ms - elapsed_ms,
                    "Spell on global cooldown"
                );
                self.send_packet(&CastFailed {
                    cast_id,
                    spell_id,
                    reason: 10, // SpellCastResult::NotReady
                    fail_arg1: 0,
                    fail_arg2: 0,
                });
                return;
            }
        }

        // Check per-spell cooldown
        if spell_info.recovery_time_ms > 0 {
            if let Some(last_spell_cast) = self.last_spell_cast_time_per_spell.get(&spell_id) {
                let elapsed_ms = last_spell_cast.elapsed().as_millis() as u32;
                let cooldown_ms = spell_info.recovery_time_ms;

                if elapsed_ms < cooldown_ms {
                    debug!(
                        account = self.account_id,
                        spell_id = spell_id,
                        remaining_ms = cooldown_ms - elapsed_ms,
                        "Spell on per-spell cooldown"
                    );
                    self.send_packet(&CastFailed {
                        cast_id,
                        spell_id,
                        reason: 10, // SpellCastResult::NotReady
                        fail_arg1: 0,
                        fail_arg2: 0,
                    });
                    return;
                }
            }
        }

        let mut spell_target = req.target.clone();
        let target_guid = if !spell_target.unit.is_empty() {
            spell_target.unit
        } else {
            spell_target.flags |= 0x2; // SpellCastTargetFlags::Unit
            spell_target.unit = player_guid;
            player_guid
        };

        // ── Initiate cast or execute immediately ─────────────────────────
        if spell_info.has_cast_time() {
            // Cast with delay — send SMSG_SPELL_START and store state
            debug!(
                account = self.account_id,
                spell_id = spell_id,
                cast_time_ms = spell_info.cast_time_ms,
                "Starting cast with timer"
            );

            let start_pkt = SpellStartPkt {
                caster: player_guid,
                cast_id,
                original_cast_id: cast_id,
                spell_id,
                visual: SpellCastVisual {
                    spell_visual_id: req.visual.spell_visual_id,
                    script_visual_id: 0,
                },
                cast_flags_ex: 0,
                target: spell_target.clone(),
                cast_time_ms: spell_info.cast_time_ms,
            };
            self.send_packet(&start_pkt);

            // Store active cast state
            self.active_spell_cast = Some(crate::session::SpellCastState {
                spell_id,
                target_guid,
                target_data: spell_target.clone(),
                cast_id,
                cast_start_time: std::time::Instant::now(),
                cast_time_ms: spell_info.cast_time_ms,
                spell_visual: SpellCastVisual {
                    spell_visual_id: req.visual.spell_visual_id,
                    script_visual_id: 0,
                },
                metadata: crate::session::SpellCastMetadata::default(),
            });

            info!(
                account = self.account_id,
                spell_id = spell_id,
                "Cast initiated ({}ms cast time)",
                spell_info.cast_time_ms
            );
        } else {
            // Instant cast — execute immediately
            debug!(
                account = self.account_id,
                spell_id = spell_id,
                "Instant cast, executing immediately"
            );
            if let Err(e) = self
                .execute_spell_with_target_data(spell_id, target_guid, spell_target)
                .await
            {
                warn!(
                    account = self.account_id,
                    "Instant spell execution failed: {}", e
                );
                return;
            }

            info!(
                account = self.account_id,
                spell_id = spell_id,
                "Instant spell executed"
            );
        }
    }

    /// Handle `CMSG_OPEN_ITEM`.
    ///
    /// This ports Trinity's initial validation and fails closed until item loot
    /// storage/generation is represented in Rust.
    pub async fn handle_open_item(&mut self, mut pkt: wow_packet::WorldPacket) {
        let open = match OpenItem::read(&mut pkt) {
            Ok(open) => open,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to parse CMSG_OPEN_ITEM: {e}"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            slot = open.slot,
            pack_slot = open.pack_slot,
            "CMSG_OPEN_ITEM"
        );

        let Some(item) = self.get_inventory_item_by_pos(open.slot, open.pack_slot) else {
            self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
            return;
        };

        let Some(flags) = self.item_template_flags(item.entry_id) else {
            self.send_equip_error(InventoryResult::ItemNotFound, Some(item.guid), None, 0, 0);
            return;
        };

        let is_wrapped = self
            .inventory_item_objects_like_cpp()
            .get(&item.guid)
            .is_some_and(|runtime_item| runtime_item.is_wrapped());

        if !flags.contains(ItemFlags::HAS_LOOT) && !is_wrapped {
            self.send_equip_error(
                InventoryResult::ClientLockedOut,
                Some(item.guid),
                None,
                0,
                0,
            );
            return;
        }

        let lock_id = self.item_template_lock_id(item.entry_id).unwrap_or(0);
        if lock_id != 0 {
            if !self.lock_entry_exists_like_cpp(u32::from(lock_id)) {
                self.send_equip_error(InventoryResult::ItemLocked, Some(item.guid), None, 0, 0);
                return;
            }

            let item_is_locked = self
                .inventory_item_objects_like_cpp()
                .get(&item.guid)
                .map_or(true, |item_object| item_object.is_locked());
            if item_is_locked {
                self.send_equip_error(InventoryResult::ItemLocked, Some(item.guid), None, 0, 0);
                return;
            }
        }

        if is_wrapped {
            self.open_wrapped_gift_like_cpp(open.slot, open.pack_slot, item.guid)
                .await;
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };

        if !self.loot_table.contains_key(&item.guid) {
            let stored_money = self.load_stored_item_money_like_cpp(item.guid).await;
            let stored_items = self.load_stored_item_items_like_cpp(item.guid).await;
            let loaded_stored_loot = stored_money.is_some() || stored_items.is_some();
            let (coins, mut items) = if loaded_stored_loot {
                (stored_money.unwrap_or(0), stored_items.unwrap_or_default())
            } else {
                let coins = {
                    let (min_money, max_money) = self
                        .load_item_template_addon_money_loot_like_cpp(item.entry_id)
                        .await;
                    generate_money_loot_with_rate_like_cpp(
                        min_money,
                        max_money,
                        self.loot_drop_rates_like_cpp().money,
                        &mut rand::thread_rng(),
                    )
                };
                let items = self
                    .generate_item_loot_template_entries_like_cpp(item.entry_id)
                    .await;
                (coins, items)
            };
            for entry in &mut items {
                entry.add_allowed_looter_like_cpp(player_guid);
            }
            if !loaded_stored_loot && (coins > 0 || !items.is_empty()) {
                self.save_new_stored_item_loot_like_cpp(item.guid, coins, &items)
                    .await;
            }

            self.loot_table.insert(
                item.guid,
                CreatureLoot {
                    loot_guid: item.guid,
                    coins,
                    unlooted_count: items
                        .iter()
                        .filter(|entry| !entry.taken)
                        .count()
                        .min(u8::MAX as usize) as u8,
                    loot_type: LOOT_TYPE_ITEM_LIKE_CPP,
                    dungeon_encounter_id: 0,
                    loot_method: 0,
                    loot_master: ObjectGuid::EMPTY,
                    round_robin_player: ObjectGuid::EMPTY,
                    player_ffa_items: Vec::new(),
                    players_looting: Vec::new(),
                    allowed_looters: vec![player_guid],
                    items,
                    looted_by_player: false,
                },
            );
        }

        self.update_inventory_item_object_like_cpp(item.guid, |item_object| {
            item_object.set_loot_generated(true);
        });

        let Some(loot) = self.loot_table.get(&item.guid) else {
            self.send_equip_error(
                InventoryResult::ClientLockedOut,
                Some(item.guid),
                None,
                0,
                0,
            );
            return;
        };

        let items: Vec<LootItemData> = loot
            .items
            .iter()
            .filter(|entry| entry.visible_in_represented_free_for_all_view_like_cpp(player_guid))
            .map(|entry| LootItemData {
                item_type: 0,
                ui_type: entry.free_for_all_ui_type_like_cpp(),
                can_trade_to_tap_list: false,
                loot: ItemInstance {
                    item_id: entry.item_id as i32,
                    ..ItemInstance::default()
                },
                loot_list_id: entry.loot_list_id,
                quantity: entry.quantity,
                loot_item_type: 0,
            })
            .collect();

        let loot_guid = loot.loot_guid;
        let coins = loot.coins;
        self.set_active_loot_guid(item.guid);
        self.send_packet(&LootResponse {
            owner: item.guid,
            loot_obj: loot_guid,
            failure_reason: 0,
            acquire_reason: LOOT_TYPE_ITEM_LIKE_CPP,
            loot_method: 0,
            threshold: 2,
            coins,
            items,
            currencies: vec![],
            acquired: true,
            ae_looting: false,
        });
    }

    async fn open_wrapped_gift_like_cpp(&mut self, bag: u8, slot: u8, item_guid: ObjectGuid) {
        let gift = match self.load_wrapped_gift_row_like_cpp(item_guid).await {
            WrappedGiftLoad::Found(gift) => gift,
            WrappedGiftLoad::Missing => {
                self.destroy_stale_wrapped_gift_like_cpp(bag, slot, item_guid)
                    .await;
                return;
            }
            WrappedGiftLoad::Unavailable => return,
        };

        let Some(durability) = self.apply_wrapped_gift_row_to_runtime_item_like_cpp(
            bag, item_guid, slot, gift.entry, gift.flags,
        ) else {
            return;
        };

        self.persist_wrapped_gift_open_like_cpp(item_guid, gift.entry, gift.flags, durability)
            .await;
        self.sync_object_accessor_player();
    }

    pub(crate) fn apply_wrapped_gift_row_to_runtime_item_like_cpp(
        &mut self,
        bag: u8,
        item_guid: ObjectGuid,
        slot: u8,
        entry: u32,
        flags: u32,
    ) -> Option<u32> {
        let current_item = self.get_inventory_item_by_pos(bag, slot)?;
        if current_item.guid != item_guid {
            return None;
        }

        let max_durability = self.item_template_max_durability(entry);
        let inventory_type = self.item_template_inventory_type(entry);
        let mut durability = None;
        let updated = self.update_inventory_item_object_like_cpp(item_guid, |item_object| {
            if item_object.is_wrapped() && item_object.object().guid() == item_guid {
                durability = Some(apply_wrapped_gift_transform_like_cpp(
                    item_object,
                    entry,
                    flags,
                    max_durability,
                ));
            }
        });
        if !updated {
            return None;
        }
        let durability = durability?;

        if bag == INVENTORY_SLOT_BAG_0 {
            self.update_inventory_item_metadata_like_cpp(slot, item_guid, entry, inventory_type);
        }

        Some(durability)
    }

    async fn load_wrapped_gift_row_like_cpp(&self, item_guid: ObjectGuid) -> WrappedGiftLoad {
        let Some(char_db) = self.char_db().map(Arc::clone) else {
            return WrappedGiftLoad::Unavailable;
        };
        let mut stmt = char_db.prepare(CharStatements::SEL_CHARACTER_GIFT_BY_ITEM);
        stmt.set_u64(0, item_guid.counter() as u64);

        match char_db.query(&stmt).await {
            Ok(result) if !result.is_empty() => WrappedGiftLoad::Found(WrappedGiftRow {
                entry: result.try_read::<u32>(0).unwrap_or(0),
                flags: result.try_read::<u32>(1).unwrap_or(0),
            }),
            Ok(_) => WrappedGiftLoad::Missing,
            Err(err) => {
                warn!(item_guid = item_guid.counter(), error = %err, "failed to load wrapped gift row");
                WrappedGiftLoad::Unavailable
            }
        }
    }

    async fn destroy_stale_wrapped_gift_like_cpp(
        &mut self,
        bag: u8,
        slot: u8,
        item_guid: ObjectGuid,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(item) = self.get_inventory_item_by_pos(bag, slot) else {
            return;
        };
        if item.guid != item_guid {
            return;
        }
        let Some(char_db) = self.char_db().map(Arc::clone) else {
            return;
        };

        let runtime_item = self
            .inventory_item_objects_like_cpp()
            .get(&item_guid)
            .cloned();
        let should_expire_refund = runtime_item
            .as_ref()
            .is_some_and(|item_object| item_object.is_refundable());

        let mut tx = SqlTransaction::new();
        if should_expire_refund {
            let mut del_refund = char_db.prepare(CharStatements::DEL_ITEM_REFUND_INSTANCE);
            del_refund.set_u64(0, item.db_guid);
            tx.append(del_refund);
        }

        let mut del_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
        del_inv.set_u64(0, player_guid.counter() as u64);
        del_inv.set_u64(1, item.db_guid);
        tx.append(del_inv);

        let mut del_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
        del_item.set_u64(0, item.db_guid);
        tx.append(del_item);

        if let Err(err) = char_db.commit_transaction(tx).await {
            warn!(
                item_guid = item_guid.counter(),
                error = %err,
                "failed to destroy stale wrapped gift"
            );
            return;
        }

        self.remove_fully_looted_runtime_item(bag, slot, item.guid);

        if should_expire_refund {
            self.send_packet(&ItemExpirePurchaseRefund {
                item_guid: item.guid,
            });
        }

        if bag == INVENTORY_SLOT_BAG_0 {
            let mut visible_item_changes = Vec::new();
            let mut virtual_item_changes = Vec::new();
            if (slot as usize) < 19 {
                visible_item_changes.push((slot, 0i32, 0u16, 0u16));
            }
            if (15..=17).contains(&slot) {
                virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
            }

            self.send_player_values_update_from_entity_bridge(
                &[(slot, ObjectGuid::EMPTY)],
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );

            if slot < 19 {
                self.send_stat_update();
            }
        }
    }

    async fn persist_wrapped_gift_open_like_cpp(
        &self,
        item_guid: ObjectGuid,
        entry: u32,
        flags: u32,
        durability: u32,
    ) {
        let Some(char_db) = self.char_db().map(Arc::clone) else {
            return;
        };

        let mut tx = SqlTransaction::new();
        let mut upd_item = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_OPEN_GIFT);
        upd_item.set_u32(0, entry);
        upd_item.set_u32(1, flags);
        upd_item.set_u32(2, durability);
        upd_item.set_u64(3, item_guid.counter() as u64);
        tx.append(upd_item);

        let mut del_gift = char_db.prepare(CharStatements::DEL_GIFT);
        del_gift.set_u64(0, item_guid.counter() as u64);
        tx.append(del_gift);

        if let Err(err) = char_db.commit_transaction(tx).await {
            warn!(item_guid = item_guid.counter(), entry, error = %err, "failed to persist wrapped gift open");
        }
    }

    async fn load_item_template_addon_money_loot_like_cpp(&self, item_entry: u32) -> (u32, u32) {
        let Some(world_db) = self.world_db() else {
            return (0, 0);
        };

        let mut stmt = world_db.prepare(WorldStatements::SEL_ITEM_TEMPLATE_ADDON_MONEY_LOOT);
        stmt.set_u32(0, item_entry);

        match world_db.query(&stmt).await {
            Ok(result) if !result.is_empty() => {
                let min_money = result.try_read::<u32>(0).unwrap_or(0);
                let max_money = result.try_read::<u32>(1).unwrap_or(0);
                (min_money, max_money)
            }
            Ok(_) => (0, 0),
            Err(err) => {
                warn!(
                    item_entry,
                    error = %err,
                    "failed to load item_template_addon money loot"
                );
                (0, 0)
            }
        }
    }

    async fn load_item_template_addon_loot_metadata_like_cpp(
        &self,
        item_entry: u32,
    ) -> ItemTemplateAddonLootMetadataLikeCpp {
        let Some(world_db) = self.world_db() else {
            return ItemTemplateAddonLootMetadataLikeCpp::default();
        };

        let mut stmt = world_db.prepare(WorldStatements::SEL_ITEM_TEMPLATE_ADDON_LOOT_METADATA);
        stmt.set_u32(0, item_entry);

        match world_db.query(&stmt).await {
            Ok(result) if !result.is_empty() => ItemTemplateAddonLootMetadataLikeCpp {
                flags_cu: result.try_read::<u32>(0).unwrap_or(0),
                quest_log_item_id: result.try_read::<i32>(1).unwrap_or(0),
            },
            Ok(_) => ItemTemplateAddonLootMetadataLikeCpp::default(),
            Err(err) => {
                warn!(
                    item_entry,
                    error = %err,
                    "failed to load item_template_addon loot metadata"
                );
                ItemTemplateAddonLootMetadataLikeCpp::default()
            }
        }
    }

    async fn load_item_template_addon_loot_metadata_for_rows_like_cpp(
        &self,
        rows: &[LootTemplateRow],
    ) -> HashMap<u32, ItemTemplateAddonLootMetadataLikeCpp> {
        let mut item_ids: Vec<u32> = rows
            .iter()
            .filter(|row| row.reference == 0 && row.item_id != 0)
            .map(|row| row.item_id)
            .collect();
        item_ids.sort_unstable();
        item_ids.dedup();

        let mut metadata = HashMap::with_capacity(item_ids.len());
        for item_id in item_ids {
            metadata.insert(
                item_id,
                self.load_item_template_addon_loot_metadata_like_cpp(item_id)
                    .await,
            );
        }

        metadata
    }

    async fn generate_item_loot_template_entries_like_cpp(
        &self,
        item_entry: u32,
    ) -> Vec<LootEntry> {
        let mut loot_items = Vec::new();
        let mut frames = Vec::new();
        let rows = self
            .load_loot_template_rows_like_cpp(LootTemplateTable::Item, item_entry)
            .await;
        let condition_references = self
            .load_loot_template_condition_reference_rows_like_cpp(&rows)
            .await;
        frames.push(LootTemplateFrame {
            rows,
            condition_references,
            index: 0,
            group_id: 0,
            groups_enqueued: false,
        });

        let mut processed_frames = 0u32;
        while let Some(mut frame) = frames.pop() {
            if frame.group_id != 0 {
                let addon_metadata = self
                    .load_item_template_addon_loot_metadata_for_rows_like_cpp(&frame.rows)
                    .await;
                let mut rng = rand::thread_rng();
                if let Some(row) = roll_group_loot_row_like_cpp(
                    &frame.rows,
                    frame.group_id,
                    |item_id| self.item_storage_template(item_id).is_some(),
                    |row| {
                        let metadata = addon_metadata
                            .get(&row.item_id)
                            .copied()
                            .unwrap_or_default();
                        self.item_loot_allowed_for_player_like_cpp_representable(
                            row.item_id,
                            row.needs_quest,
                            metadata,
                            &row.conditions,
                            &frame.condition_references,
                        )
                    },
                    |item_id| self.item_drop_rate_like_cpp(item_id),
                    &mut rng,
                ) {
                    let metadata = addon_metadata
                        .get(&row.item_id)
                        .copied()
                        .unwrap_or_default();
                    let flags = self.loot_entry_flags_for_row_like_cpp(&row, metadata);
                    add_loot_template_row_item_like_cpp(&mut loot_items, &row, flags, |item_id| {
                        self.item_storage_template(item_id)
                            .map(|template| template.max_stack_size)
                            .unwrap_or(1)
                    });
                }
                continue;
            }

            if frame.index >= frame.rows.len() {
                if !frame.groups_enqueued {
                    frame.groups_enqueued = true;
                    let mut groups: Vec<u8> = frame
                        .rows
                        .iter()
                        .filter(|row| row.reference == 0 && row.group_id != 0)
                        .map(|row| row.group_id)
                        .collect();
                    groups.sort_unstable();
                    groups.dedup();

                    if !groups.is_empty() {
                        let rows = frame.rows.clone();
                        let condition_references = frame.condition_references.clone();
                        frames.push(frame);
                        for group_id in groups.into_iter().rev() {
                            frames.push(LootTemplateFrame {
                                rows: rows.clone(),
                                condition_references: condition_references.clone(),
                                index: 0,
                                group_id,
                                groups_enqueued: true,
                            });
                        }
                    }
                }
                continue;
            }
            let row = frame.rows[frame.index].clone();
            frame.index += 1;
            let condition_references = frame.condition_references.clone();
            frames.push(frame);

            if row.loot_mode & LOOT_MODE_DEFAULT_LIKE_CPP == 0 {
                continue;
            }

            if row.group_id != 0 && row.reference == 0 {
                continue;
            }

            if row.reference > 0 {
                if !loot_template_reference_row_can_roll_like_cpp(
                    row.reference,
                    row.chance,
                    row.loot_mode,
                    row.min_count,
                ) {
                    continue;
                }
                if row.chance < 100.0
                    && !roll_chance_with_rate_like_cpp(
                        row.chance,
                        self.loot_drop_rates_like_cpp().item_referenced,
                        &mut rand::thread_rng(),
                    )
                {
                    continue;
                }

                let reference_rows = self
                    .load_loot_template_rows_like_cpp(LootTemplateTable::Reference, row.reference)
                    .await;
                let reference_condition_references = self
                    .load_loot_template_condition_reference_rows_like_cpp(&reference_rows)
                    .await;
                let max_count = referenced_loot_max_count_like_cpp(
                    row.max_count,
                    self.loot_drop_rates_like_cpp().item_referenced_amount,
                );
                for _ in 0..max_count {
                    frames.push(LootTemplateFrame {
                        rows: reference_rows.clone(),
                        condition_references: reference_condition_references.clone(),
                        index: 0,
                        group_id: row.group_id,
                        groups_enqueued: false,
                    });
                }
                processed_frames = processed_frames.saturating_add(1);
                if processed_frames > MAX_LOOT_REFERENCE_FRAMES_LIKE_CPP {
                    warn!(
                        item_entry,
                        reference = row.reference,
                        "stopped item loot reference processing after safety cap"
                    );
                    break;
                }
                continue;
            }

            let addon_metadata = self
                .load_item_template_addon_loot_metadata_like_cpp(row.item_id)
                .await;
            if !loot_template_plain_row_can_roll_like_cpp(
                row.item_id,
                row.chance,
                row.needs_quest,
                row.loot_mode,
                row.min_count,
                row.max_count,
                self.item_storage_template(row.item_id).is_some(),
                self.item_loot_allowed_for_player_like_cpp_representable(
                    row.item_id,
                    row.needs_quest,
                    addon_metadata,
                    &row.conditions,
                    &condition_references,
                ),
            ) {
                continue;
            }
            if row.chance < 100.0
                && !roll_chance_with_rate_like_cpp(
                    row.chance,
                    self.item_drop_rate_like_cpp(row.item_id),
                    &mut rand::thread_rng(),
                )
            {
                continue;
            }
            let flags = self.loot_entry_flags_for_row_like_cpp(&row, addon_metadata);
            add_loot_template_row_item_like_cpp(&mut loot_items, &row, flags, |item_id| {
                self.item_storage_template(item_id)
                    .map(|template| template.max_stack_size)
                    .unwrap_or(1)
            });
        }

        loot_items
    }

    async fn load_loot_template_rows_like_cpp(
        &self,
        table: LootTemplateTable,
        entry: u32,
    ) -> Vec<LootTemplateRow> {
        let Some(world_db) = self.world_db() else {
            return Vec::new();
        };

        let statement = match table {
            LootTemplateTable::Item => WorldStatements::SEL_ITEM_LOOT_TEMPLATE_ROWS,
            LootTemplateTable::Reference => WorldStatements::SEL_REFERENCE_LOOT_TEMPLATE_ROWS,
        };
        let mut stmt = world_db.prepare(statement);
        stmt.set_u32(0, entry);

        let mut result = match world_db.query(&stmt).await {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    entry,
                    table = table.name(),
                    error = %err,
                    "failed to load loot template rows"
                );
                return Vec::new();
            }
        };

        let mut rows = Vec::new();
        if result.is_empty() {
            return rows;
        }

        loop {
            rows.push(LootTemplateRow {
                item_id: result.try_read::<u32>(0).unwrap_or(0),
                reference: result.try_read::<u32>(1).unwrap_or(0),
                chance: result.try_read::<f32>(2).unwrap_or(0.0),
                needs_quest: result.try_read::<bool>(3).unwrap_or(false),
                loot_mode: result.try_read::<u16>(4).unwrap_or(0),
                group_id: result.try_read::<u8>(5).unwrap_or(0),
                min_count: result.try_read::<u8>(6).unwrap_or(0),
                max_count: result.try_read::<u8>(7).unwrap_or(0),
                conditions: Vec::new(),
            });

            if !result.next_row() {
                break;
            }
        }

        let condition_source_type = table.condition_source_type_like_cpp();
        for row in &mut rows {
            row.conditions = self
                .load_loot_template_condition_rows_like_cpp(
                    condition_source_type,
                    entry,
                    row.item_id,
                )
                .await;
        }

        rows
    }

    async fn load_loot_template_condition_rows_like_cpp(
        &self,
        source_type: i32,
        source_group: u32,
        source_entry: u32,
    ) -> Vec<LootConditionRowLikeCpp> {
        let Some(world_db) = self.world_db() else {
            return Vec::new();
        };

        let mut stmt = world_db.prepare(WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_ROWS);
        stmt.set_i32(0, source_type);
        stmt.set_u32(1, source_group);
        stmt.set_u32(2, source_entry);

        let mut result = match world_db.query(&stmt).await {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    source_type,
                    source_group,
                    source_entry,
                    error = %err,
                    "failed to load loot template condition rows"
                );
                return Vec::new();
            }
        };

        let mut conditions = Vec::new();
        if result.is_empty() {
            return conditions;
        }

        loop {
            let condition = LootConditionRowLikeCpp {
                else_group: result.try_read::<u32>(0).unwrap_or(0),
                condition_type_or_reference: result.try_read::<i32>(1).unwrap_or(0),
                condition_target: result.try_read::<u8>(2).unwrap_or(0),
                value1: result.try_read::<u32>(3).unwrap_or(0),
                value2: result.try_read::<u32>(4).unwrap_or(0),
                value3: result.try_read::<u32>(5).unwrap_or(0),
                string_value1: result.try_read::<String>(6).unwrap_or_default(),
                negative: result.try_read::<bool>(7).unwrap_or(false),
                script_name: result.try_read::<String>(8).unwrap_or_default(),
            };
            if !loot_condition_reference_self_references_like_cpp(
                source_type,
                condition.condition_type_or_reference,
            ) {
                if let Some(condition) =
                    loot_condition_row_normalize_without_external_stores_like_cpp(condition)
                {
                    conditions.push(condition);
                }
            }

            if !result.next_row() {
                break;
            }
        }

        conditions
    }

    async fn load_loot_template_condition_reference_rows_like_cpp(
        &self,
        rows: &[LootTemplateRow],
    ) -> HashMap<u32, Vec<LootConditionRowLikeCpp>> {
        let mut references = HashMap::new();
        let mut pending = Vec::new();
        for row in rows {
            pending.extend(loot_condition_reference_ids_like_cpp(&row.conditions));
        }

        while let Some(reference_id) = pending.pop() {
            if references.contains_key(&reference_id) {
                continue;
            }

            let reference_rows = self
                .load_loot_template_condition_reference_rows_for_id_like_cpp(reference_id)
                .await;
            for nested_reference_id in loot_condition_reference_ids_like_cpp(&reference_rows) {
                if !references.contains_key(&nested_reference_id) {
                    pending.push(nested_reference_id);
                }
            }
            references.insert(reference_id, reference_rows);
        }

        references
    }

    async fn load_loot_template_condition_reference_rows_for_id_like_cpp(
        &self,
        reference_id: u32,
    ) -> Vec<LootConditionRowLikeCpp> {
        let Ok(reference_source_type) = i32::try_from(reference_id).map(|id| -id) else {
            return Vec::new();
        };

        self.load_loot_template_condition_rows_like_cpp(reference_source_type, 0, 0)
            .await
    }

    fn item_loot_allowed_for_player_like_cpp_representable(
        &self,
        item_id: u32,
        needs_quest: bool,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
        conditions: &[LootConditionRowLikeCpp],
        condition_references: &HashMap<u32, Vec<LootConditionRowLikeCpp>>,
    ) -> bool {
        if !self.loot_conditions_allow_player_with_references_like_cpp_representable(
            conditions,
            condition_references,
        ) {
            return false;
        }

        let start_quest_id = self.item_template_start_quest_id(item_id).unwrap_or(0);
        let has_non_none_start_quest_status =
            u32::try_from(start_quest_id).ok().is_some_and(|quest_id| {
                quest_id != 0
                    && (self.player_quests.contains_key(&quest_id)
                        || self.rewarded_quests.contains(&quest_id))
            });

        let has_quest_for_item = self
            .has_incomplete_quest_objective_for_item_like_cpp_representable(item_id)
            || (addon_metadata.quest_log_item_id != 0
                && self.has_incomplete_quest_objective_for_object_id_like_cpp_representable(
                    addon_metadata.quest_log_item_id,
                ))
            || self.has_incomplete_quest_item_drop_for_item_like_cpp_representable(item_id);

        item_loot_quest_status_allows_like_cpp(
            addon_metadata.ignores_quest_status(),
            needs_quest,
            has_non_none_start_quest_status,
            has_quest_for_item,
        )
    }

    fn loot_entry_flags_for_row_like_cpp(
        &self,
        row: &LootTemplateRow,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
    ) -> LootEntryFlags {
        let template = self.item_storage_template(row.item_id);
        loot_entry_flags_for_row_metadata_like_cpp(
            row.needs_quest,
            template
                .map(|template| template.flags)
                .unwrap_or(ItemFlags::empty()),
            addon_metadata,
        )
    }

    fn has_incomplete_quest_objective_for_item_like_cpp_representable(&self, item_id: u32) -> bool {
        let Ok(item_object_id) = i32::try_from(item_id) else {
            return false;
        };

        self.has_incomplete_quest_objective_for_object_id_like_cpp_representable(item_object_id)
    }

    fn has_incomplete_quest_objective_for_object_id_like_cpp_representable(
        &self,
        item_object_id: i32,
    ) -> bool {
        let Some(quest_store) = &self.quest_store else {
            return false;
        };

        self.player_quests.values().any(|status| {
            if status.status != 1 {
                return false;
            }

            let Some(quest) = quest_store.get(status.quest_id) else {
                return false;
            };

            quest
                .objectives
                .iter()
                .enumerate()
                .any(|(fallback_index, objective)| {
                    if objective.obj_type != 1 || objective.object_id != item_object_id {
                        return false;
                    }

                    let storage_index = usize::try_from(objective.storage_index)
                        .ok()
                        .unwrap_or(fallback_index);
                    let current = status
                        .objective_counts
                        .get(storage_index)
                        .copied()
                        .unwrap_or(0);
                    current < objective.amount.max(1)
                })
        })
    }

    fn has_incomplete_quest_item_drop_for_item_like_cpp_representable(&self, item_id: u32) -> bool {
        let Some(quest_store) = &self.quest_store else {
            return false;
        };

        self.player_quests.values().any(|status| {
            if status.status != 1 {
                return false;
            }

            let Some(quest) = quest_store.get(status.quest_id) else {
                return false;
            };

            quest
                .item_drop
                .iter()
                .enumerate()
                .any(|(index, drop_item_id)| {
                    if *drop_item_id != item_id {
                        return false;
                    }

                    let Some(template) = self.item_storage_template(item_id) else {
                        return false;
                    };

                    let quantity = quest.item_drop_quantity[index];
                    let mut max_allowed_count = if quantity != 0 {
                        quantity
                    } else {
                        template.max_stack_size
                    };
                    if template.max_count > 0 {
                        max_allowed_count = max_allowed_count.min(template.max_count as u32);
                    }

                    self.direct_inventory_item_count_like_cpp_representable(item_id)
                        < max_allowed_count
                })
        })
    }

    fn direct_inventory_item_count_like_cpp_representable(&self, item_id: u32) -> u32 {
        self.inventory_items_like_cpp()
            .values()
            .filter(|inventory_item| inventory_item.entry_id == item_id)
            .filter_map(|inventory_item| {
                self.inventory_item_objects_like_cpp()
                    .get(&inventory_item.guid)
            })
            .filter(|item| !item.is_in_trade())
            .fold(0_u32, |total, item| total.saturating_add(item.count()))
    }

    fn loot_conditions_allow_player_with_references_like_cpp_representable(
        &self,
        conditions: &[LootConditionRowLikeCpp],
        condition_references: &HashMap<u32, Vec<LootConditionRowLikeCpp>>,
    ) -> bool {
        loot_conditions_allow_player_with_references_like_cpp_representable(
            conditions,
            condition_references,
            |condition| self.evaluate_loot_condition_like_cpp_representable(condition),
        )
    }

    fn evaluate_loot_condition_like_cpp_representable(
        &self,
        condition: &LootConditionRowLikeCpp,
    ) -> Option<bool> {
        match condition.condition_type_or_reference {
            0 => Some(true),
            2 => {
                if condition.value3 != 0 {
                    return None;
                }
                Some(
                    self.direct_inventory_item_count_like_cpp_representable(condition.value1)
                        >= condition.value2,
                )
            }
            6 => Some(
                player_team_for_race_cpp_representable(self.player_race_like_cpp())
                    == condition.value1,
            ),
            8 => Some(self.rewarded_quests.contains(&condition.value1)),
            9 => Some(
                self.player_quests
                    .get(&condition.value1)
                    .is_some_and(|status| status.status == 1),
            ),
            14 => Some(
                !self.player_quests.contains_key(&condition.value1)
                    && !self.rewarded_quests.contains(&condition.value1),
            ),
            15 => Some(
                player_class_mask_like_cpp(self.player_class_like_cpp())
                    .is_some_and(|mask| mask & condition.value1 != 0),
            ),
            16 => Some(
                player_race_mask_like_cpp(self.player_race_like_cpp())
                    .is_some_and(|mask| mask & condition.value1 != 0),
            ),
            20 => Some(u32::from(self.player_gender_like_cpp()) == condition.value1),
            25 => i32::try_from(condition.value1)
                .ok()
                .map(|spell_id| self.known_spells_like_cpp().contains(&spell_id)),
            27 => condition_compare_values_like_cpp(
                condition.value2,
                u32::from(self.player_level_like_cpp()),
                condition.value1,
            ),
            28 => Some(
                self.player_quests
                    .get(&condition.value1)
                    .is_some_and(|status| status.status == 2)
                    && !self.rewarded_quests.contains(&condition.value1),
            ),
            47 => Some(
                player_quest_status_mask_like_cpp(
                    self.player_quests
                        .get(&condition.value1)
                        .map(|status| status.status),
                    self.rewarded_quests.contains(&condition.value1),
                ) & condition.value2
                    != 0,
            ),
            48 => Some(
                self.player_quest_objective_progress_like_cpp_representable(condition.value1)
                    == Some(condition.value3 as i32),
            ),
            CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP => {
                Some(condition.value1 == TYPEID_PLAYER_LIKE_CPP)
            }
            CONDITION_TYPE_MASK_LIKE_CPP => Some(condition.value1 & PLAYER_TYPE_MASK_LIKE_CPP != 0),
            _ => None,
        }
    }

    fn player_quest_objective_progress_like_cpp_representable(
        &self,
        objective_id: u32,
    ) -> Option<i32> {
        let quest_store = self.quest_store.as_ref()?;

        for status in self.player_quests.values() {
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            let Some((_, objective)) = quest
                .objectives
                .iter()
                .enumerate()
                .find(|(_, objective)| objective.id == objective_id)
            else {
                continue;
            };
            let objective_index = objective.storage_index.max(0) as usize;
            return Some(
                status
                    .objective_counts
                    .get(objective_index)
                    .copied()
                    .unwrap_or(0),
            );
        }

        None
    }

    async fn load_stored_item_money_like_cpp(
        &self,
        item_guid: wow_core::ObjectGuid,
    ) -> Option<u32> {
        let char_db = self.char_db().map(Arc::clone)?;

        let mut stmt = char_db.prepare(CharStatements::SEL_ITEMCONTAINER_MONEY);
        stmt.set_u64(0, item_guid.counter() as u64);

        match char_db.query(&stmt).await {
            Ok(result) if !result.is_empty() => result.try_read::<u32>(0),
            Ok(_) => None,
            Err(err) => {
                warn!(
                    item_guid = item_guid.counter(),
                    error = %err,
                    "failed to load stored item loot money"
                );
                None
            }
        }
    }

    async fn load_stored_item_items_like_cpp(
        &self,
        item_guid: wow_core::ObjectGuid,
    ) -> Option<Vec<LootEntry>> {
        let char_db = self.char_db().map(Arc::clone)?;

        let mut stmt = char_db.prepare(CharStatements::SEL_ITEMCONTAINER_ITEMS);
        stmt.set_u64(0, item_guid.counter() as u64);

        let mut result = match char_db.query(&stmt).await {
            Ok(result) if !result.is_empty() => result,
            Ok(_) => return None,
            Err(err) => {
                warn!(
                    item_guid = item_guid.counter(),
                    error = %err,
                    "failed to load stored item loot rows"
                );
                return None;
            }
        };

        let mut items = Vec::new();
        loop {
            let item_id = result.try_read::<u32>(0).unwrap_or(0);
            let count = result.try_read::<u32>(1).unwrap_or(0);
            let item_index = result.try_read::<u32>(2).unwrap_or(u32::MAX);
            let follow_loot_rules = result.try_read::<bool>(3).unwrap_or(false);
            let freeforall = result.try_read::<bool>(4).unwrap_or(false);
            let blocked = result.try_read::<bool>(5).unwrap_or(false);
            let counted = result.try_read::<bool>(6).unwrap_or(false);
            let under_threshold = result.try_read::<bool>(7).unwrap_or(false);
            let needs_quest = result.try_read::<bool>(8).unwrap_or(false);
            let random_properties_id = result.try_read::<i32>(9).unwrap_or(0);
            let random_properties_seed = result.try_read::<i32>(10).unwrap_or(0);
            let context = result.try_read::<u8>(11).unwrap_or(0);

            if stored_item_row_can_load_like_cpp_representable(
                item_id,
                count,
                item_index,
                blocked,
                needs_quest,
                random_properties_id,
                random_properties_seed,
                context,
                self.item_storage_template(item_id).is_some(),
            ) {
                items.push(LootEntry {
                    loot_list_id: item_index as u8,
                    item_id,
                    quantity: count,
                    random_properties_id,
                    random_properties_seed,
                    item_context: context,
                    flags: LootEntryFlags {
                        follow_loot_rules,
                        freeforall,
                        blocked,
                        counted,
                        under_threshold,
                        needs_quest,
                    },
                    allowed_looters: Vec::new(),
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                });
            }

            if !result.next_row() {
                break;
            }
        }

        Some(items)
    }

    async fn save_new_stored_item_loot_like_cpp(
        &self,
        item_guid: wow_core::ObjectGuid,
        money: u32,
        items: &[LootEntry],
    ) {
        let Some(char_db) = self.char_db().map(Arc::clone) else {
            return;
        };

        let mut tx = SqlTransaction::new();

        if money > 0 {
            let mut del_money = char_db.prepare(CharStatements::DEL_ITEMCONTAINER_MONEY);
            del_money.set_u64(0, item_guid.counter() as u64);
            tx.append(del_money);

            let mut ins_money = char_db.prepare(CharStatements::INS_ITEMCONTAINER_MONEY);
            ins_money.set_u64(0, item_guid.counter() as u64);
            ins_money.set_u32(1, money);
            tx.append(ins_money);
        }

        let mut del_items = char_db.prepare(CharStatements::DEL_ITEMCONTAINER_ITEMS);
        del_items.set_u64(0, item_guid.counter() as u64);
        tx.append(del_items);

        for item in items {
            let template = self.item_storage_template(item.item_id);
            if !stored_loot_item_should_persist_like_cpp(
                template.is_some(),
                template
                    .map(|t| t.bag_family)
                    .unwrap_or(BagFamilyMask::NONE),
            ) {
                continue;
            }

            let mut ins_item = char_db.prepare(CharStatements::INS_ITEMCONTAINER_ITEMS);
            ins_item.set_u64(0, item_guid.counter() as u64);
            ins_item.set_u32(1, item.item_id);
            ins_item.set_u32(2, item.quantity);
            ins_item.set_u32(3, u32::from(item.loot_list_id));
            ins_item.set_bool(4, item.flags.follow_loot_rules);
            ins_item.set_bool(5, item.flags.freeforall);
            ins_item.set_bool(6, item.flags.blocked);
            ins_item.set_bool(7, item.flags.counted);
            ins_item.set_bool(8, item.flags.under_threshold);
            ins_item.set_bool(9, item.flags.needs_quest);
            ins_item.set_i32(10, item.random_properties_id);
            ins_item.set_i32(11, item.random_properties_seed);
            ins_item.set_u8(12, item.item_context);
            tx.append(ins_item);
        }

        if let Err(err) = char_db.commit_transaction(tx).await {
            warn!(
                item_guid = item_guid.counter(),
                money,
                error = %err,
                "failed to save stored item loot rows"
            );
        }
    }

    /// Handle `CMSG_CANCEL_CAST` — player cancels an in-progress cast.
    pub async fn handle_cancel_cast(&mut self, _pkt: wow_packet::WorldPacket) {
        // TODO: Phase 3 — implement cancel cast
    }

    /// Handle `CMSG_CANCEL_CHANNELLING` — player stops a channelled spell.
    pub async fn handle_cancel_channelling(&mut self, _pkt: wow_packet::WorldPacket) {
        // TODO: Phase 3 — implement cancel channelling
    }

    fn is_spell_disabled_for_player_like_cpp(&self, spell_id: i32) -> bool {
        let Some(disable_mgr) = self.disable_mgr() else {
            return false;
        };

        let map_id = u32::from(self.player_map_id_like_cpp());
        let (_, area_id) = self.player_zone_area_like_cpp();
        let map_instance_type = self
            .map_store()
            .and_then(|store| store.get(map_id))
            .map(|entry| entry.instance_type);

        disable_mgr.is_disabled_for_like_cpp(
            DISABLE_TYPE_SPELL,
            spell_id as u32,
            Some(DisableWorldObjectRefLikeCpp {
                type_id: TypeId::Player,
                map_id,
                area_id,
                is_pet: false,
                is_battle_arena: map_instance_type == Some(MAP_ARENA_LIKE_CPP),
                is_battleground: map_instance_type == Some(MAP_BATTLEGROUND_LIKE_CPP),
                player_map_difficulty: None,
            }),
            0,
            self.map_store().map(|store| store.as_ref()),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct WrappedGiftRow {
    entry: u32,
    flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WrappedGiftLoad {
    Found(WrappedGiftRow),
    Missing,
    Unavailable,
}

fn apply_wrapped_gift_transform_like_cpp(
    item: &mut wow_entities::Item,
    entry: u32,
    flags: u32,
    max_durability: u32,
) -> u32 {
    let durability = item.data().durability;

    item.set_gift_creator(ObjectGuid::EMPTY);
    item.object_mut().set_entry(entry);
    item.replace_all_item_flags(ItemFieldFlags::from_bits_retain(flags));
    item.set_max_durability(max_durability);
    item.set_state(ItemUpdateState::Changed);
    durability
}

fn stored_loot_item_should_persist_like_cpp(
    template_exists: bool,
    bag_family: BagFamilyMask,
) -> bool {
    if !template_exists {
        return false;
    }
    !bag_family.contains(BagFamilyMask::CURRENCY_TOKENS)
}

fn roll_chance_with_rate_like_cpp<R: Rng + ?Sized>(chance: f32, rate: f32, rng: &mut R) -> bool {
    if chance >= 100.0 {
        return true;
    }
    rng.gen_range(0.0f32..100.0f32) < chance * rate
}

fn referenced_loot_max_count_like_cpp(max_count: u8, rate: f32) -> u32 {
    ((max_count as f32) * rate) as u32
}

fn loot_template_plain_row_can_roll_like_cpp(
    item_id: u32,
    chance: f32,
    needs_quest: bool,
    loot_mode: u16,
    min_count: u8,
    max_count: u8,
    item_exists: bool,
    allowed_for_player: bool,
) -> bool {
    if item_id == 0 || !item_exists || min_count == 0 || max_count < min_count {
        return false;
    }

    if needs_quest && !allowed_for_player {
        return false;
    }

    if chance == 0.0 || (chance != 0.0 && chance < 0.000001) {
        return false;
    }

    loot_mode & LOOT_MODE_DEFAULT_LIKE_CPP != 0
}

fn loot_template_reference_row_can_roll_like_cpp(
    reference: u32,
    chance: f32,
    loot_mode: u16,
    min_count: u8,
) -> bool {
    reference != 0 && min_count != 0 && chance != 0.0 && loot_mode & LOOT_MODE_DEFAULT_LIKE_CPP != 0
}

fn loot_template_group_row_can_roll_like_cpp(
    item_id: u32,
    chance: f32,
    needs_quest: bool,
    loot_mode: u16,
    min_count: u8,
    max_count: u8,
    item_exists: bool,
    allowed_for_player: bool,
) -> bool {
    if item_id == 0
        || !item_exists
        || min_count == 0
        || max_count < min_count
        || (needs_quest && !allowed_for_player)
    {
        return false;
    }

    if chance != 0.0 && chance < 0.000001 {
        return false;
    }

    loot_mode & LOOT_MODE_DEFAULT_LIKE_CPP != 0
}

fn roll_group_loot_row_like_cpp<R, F, G, H>(
    rows: &[LootTemplateRow],
    group_id: u8,
    item_exists: F,
    allowed_for_player: G,
    item_drop_rate: H,
    rng: &mut R,
) -> Option<LootTemplateRow>
where
    R: Rng + ?Sized,
    F: Fn(u32) -> bool,
    G: Fn(&LootTemplateRow) -> bool,
    H: Fn(u32) -> f32,
{
    let possible: Vec<&LootTemplateRow> = rows
        .iter()
        .filter(|row| {
            row.group_id == group_id
                && row.reference == 0
                && loot_template_group_row_can_roll_like_cpp(
                    row.item_id,
                    row.chance,
                    row.needs_quest,
                    row.loot_mode,
                    row.min_count,
                    row.max_count,
                    item_exists(row.item_id),
                    allowed_for_player(row),
                )
        })
        .collect();

    let explicitly_chanced: Vec<&LootTemplateRow> = possible
        .iter()
        .copied()
        .filter(|row| row.chance != 0.0)
        .collect();
    if !explicitly_chanced.is_empty() {
        let mut roll = rng.gen_range(0.0f32..100.0f32);
        for row in explicitly_chanced {
            if row.chance >= 100.0 {
                return Some((*row).clone());
            }
            roll -= row.chance * item_drop_rate(row.item_id);
            if roll < 0.0 {
                return Some((*row).clone());
            }
        }
    }

    let equal_chanced: Vec<&LootTemplateRow> = possible
        .iter()
        .copied()
        .filter(|row| row.chance == 0.0)
        .collect();
    if equal_chanced.is_empty() {
        None
    } else {
        Some((*equal_chanced[rng.gen_range(0..equal_chanced.len())]).clone())
    }
}

fn stored_item_row_can_load_like_cpp_representable(
    item_id: u32,
    count: u32,
    item_index: u32,
    blocked: bool,
    _needs_quest: bool,
    _random_properties_id: i32,
    _random_properties_seed: i32,
    _context: u8,
    item_exists: bool,
) -> bool {
    item_id != 0 && item_exists && count != 0 && item_index <= u32::from(u8::MAX) && !blocked
}

#[derive(Debug, Clone)]
struct LootTemplateRow {
    item_id: u32,
    reference: u32,
    chance: f32,
    needs_quest: bool,
    loot_mode: u16,
    group_id: u8,
    min_count: u8,
    max_count: u8,
    conditions: Vec<LootConditionRowLikeCpp>,
}

#[derive(Debug)]
struct LootTemplateFrame {
    rows: Vec<LootTemplateRow>,
    condition_references: HashMap<u32, Vec<LootConditionRowLikeCpp>>,
    index: usize,
    group_id: u8,
    groups_enqueued: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ItemTemplateAddonLootMetadataLikeCpp {
    flags_cu: u32,
    quest_log_item_id: i32,
}

impl ItemTemplateAddonLootMetadataLikeCpp {
    fn ignores_quest_status(self) -> bool {
        self.flags_cu & ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP != 0
    }

    fn follows_loot_rules(self) -> bool {
        self.flags_cu & ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP != 0
    }
}

fn item_loot_quest_status_allows_like_cpp(
    ignores_quest_status: bool,
    needs_quest: bool,
    has_non_none_start_quest_status: bool,
    has_quest_for_item: bool,
) -> bool {
    ignores_quest_status
        || ((!needs_quest && !has_non_none_start_quest_status) || has_quest_for_item)
}

fn loot_entry_flags_for_row_metadata_like_cpp(
    needs_quest: bool,
    item_flags: ItemFlags,
    addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
) -> LootEntryFlags {
    LootEntryFlags {
        follow_loot_rules: !needs_quest || addon_metadata.follows_loot_rules(),
        freeforall: item_flags.contains(ItemFlags::MULTI_DROP),
        blocked: false,
        counted: false,
        under_threshold: false,
        needs_quest,
    }
}

fn player_quest_status_mask_like_cpp(status: Option<u8>, rewarded: bool) -> u32 {
    if rewarded {
        return 1 << 6;
    }

    match status {
        Some(2) => 1 << 1,
        Some(1) => 1 << 3,
        Some(3) => 1 << 5,
        _ => 1 << 0,
    }
}

fn player_class_mask_like_cpp(class_id: u8) -> Option<u32> {
    if (1..=13).contains(&class_id) {
        Some(1_u32 << (class_id - 1))
    } else {
        None
    }
}

fn player_race_mask_like_cpp(race_id: u8) -> Option<u32> {
    let bit = match race_id {
        1..=11 => race_id - 1,
        22 => 21,
        24..=32 => race_id - 1,
        34 => 11,
        35 => 12,
        36 => 13,
        37 => 14,
        52 => 16,
        70 => 15,
        _ => return None,
    };
    Some(1_u32 << bit)
}

fn player_team_for_race_cpp_representable(race: u8) -> u32 {
    match race {
        2 | 5 | 6 | 8 | 9 | 10 => 67,
        _ => 469,
    }
}

#[derive(Debug, Clone, Copy)]
enum LootTemplateTable {
    Item,
    Reference,
}

impl LootTemplateTable {
    fn name(self) -> &'static str {
        match self {
            Self::Item => "item_loot_template",
            Self::Reference => "reference_loot_template",
        }
    }

    fn condition_source_type_like_cpp(self) -> i32 {
        match self {
            Self::Item => CONDITION_SOURCE_TYPE_ITEM_LOOT_TEMPLATE_LIKE_CPP,
            Self::Reference => CONDITION_SOURCE_TYPE_REFERENCE_LOOT_TEMPLATE_LIKE_CPP,
        }
    }
}

fn add_loot_item_stacks_like_cpp(
    loot_items: &mut Vec<LootEntry>,
    item_id: u32,
    mut count: u32,
    max_stack_size: u32,
    flags: LootEntryFlags,
) {
    while count > 0 && loot_items.len() < MAX_NR_LOOT_ITEMS_LIKE_CPP {
        let quantity = count.min(max_stack_size);
        loot_items.push(LootEntry {
            loot_list_id: loot_items.len() as u8,
            item_id,
            quantity,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags,
            allowed_looters: Vec::new(),
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        });
        count = count.saturating_sub(max_stack_size);
    }
}

fn add_loot_template_row_item_like_cpp<F>(
    loot_items: &mut Vec<LootEntry>,
    row: &LootTemplateRow,
    flags: LootEntryFlags,
    max_stack_size: F,
) where
    F: Fn(u32) -> u32,
{
    let mut rng = rand::thread_rng();
    let rolled_count = rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));
    add_loot_item_stacks_like_cpp(
        loot_items,
        row.item_id,
        rolled_count,
        max_stack_size(row.item_id).max(1),
        flags,
    );
}

#[cfg(test)]
mod tests {
    use rand::{SeedableRng, rngs::StdRng};

    use wow_constants::{BagFamilyMask, ItemContext, ItemFieldFlags, ItemFlags, ItemUpdateState};
    use wow_core::ObjectGuid;
    use wow_entities::{Item, ItemCreateInfo, MAX_ITEM_SPELLS};
    use wow_loot::{
        LootConditionRowLikeCpp, condition_compare_values_like_cpp,
        loot_conditions_allow_player_like_cpp_representable,
    };
    use wow_packet::packets::loot::{LootEntry, LootEntryFlags};

    use super::{
        ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP, ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP,
        ItemTemplateAddonLootMetadataLikeCpp, LootTemplateRow, add_loot_item_stacks_like_cpp,
        apply_wrapped_gift_transform_like_cpp, item_loot_quest_status_allows_like_cpp,
        loot_entry_flags_for_row_metadata_like_cpp, loot_template_group_row_can_roll_like_cpp,
        loot_template_plain_row_can_roll_like_cpp, loot_template_reference_row_can_roll_like_cpp,
        player_class_mask_like_cpp, player_quest_status_mask_like_cpp, player_race_mask_like_cpp,
        referenced_loot_max_count_like_cpp, roll_chance_with_rate_like_cpp,
        roll_group_loot_row_like_cpp, stored_item_row_can_load_like_cpp_representable,
        stored_loot_item_should_persist_like_cpp,
    };

    #[test]
    fn open_item_wrapped_without_has_loot_uses_gift_row_like_cpp() {
        let item_guid = ObjectGuid::create_item(1, 900);
        let owner_guid = ObjectGuid::create_player(1, 42);
        let gift_creator = ObjectGuid::create_player(1, 77);
        let mut item = Item::new(0);
        item.initialize_created_state(ItemCreateInfo {
            guid: item_guid,
            item_id: 100,
            context: ItemContext::None,
            owner: Some(owner_guid),
            max_durability: 30,
            expiration: 0,
            spell_charges: [0; MAX_ITEM_SPELLS],
        });
        item.force_state(ItemUpdateState::Unchanged);
        item.set_gift_creator(gift_creator);
        item.replace_all_item_flags(ItemFieldFlags::WRAPPED);
        item.set_durability(25);

        let durability = apply_wrapped_gift_transform_like_cpp(
            &mut item,
            200,
            ItemFieldFlags::SOULBOUND.bits(),
            20,
        );

        assert_eq!(durability, 25);
        assert_eq!(item.object().entry(), 200);
        assert_eq!(item.data().gift_creator, ObjectGuid::EMPTY);
        assert_eq!(item.item_flags_bits(), ItemFieldFlags::SOULBOUND.bits());
        assert_eq!(item.data().max_durability, 20);
        assert_eq!(item.data().durability, 25);
        assert_eq!(item.update_state(), ItemUpdateState::Changed);
        assert!(!item.is_wrapped());
    }

    #[test]
    fn open_item_wrapped_gift_with_zero_durability_stays_zero_like_cpp() {
        let item_guid = ObjectGuid::create_item(1, 901);
        let owner_guid = ObjectGuid::create_player(1, 42);
        let mut item = Item::new(0);
        item.initialize_created_state(ItemCreateInfo {
            guid: item_guid,
            item_id: 100,
            context: ItemContext::None,
            owner: Some(owner_guid),
            max_durability: 30,
            expiration: 0,
            spell_charges: [0; MAX_ITEM_SPELLS],
        });
        item.force_state(ItemUpdateState::Unchanged);
        item.replace_all_item_flags(ItemFieldFlags::WRAPPED);
        item.set_durability(0);

        let durability = apply_wrapped_gift_transform_like_cpp(
            &mut item,
            200,
            ItemFieldFlags::SOULBOUND.bits(),
            20,
        );

        assert_eq!(durability, 0);
        assert_eq!(item.data().max_durability, 20);
        assert_eq!(item.data().durability, 0);
        assert_eq!(item.update_state(), ItemUpdateState::Changed);
        assert!(!item.is_wrapped());
    }

    #[test]
    fn item_money_loot_generation_matches_cpp_boundary_branches() {
        let mut rng = StdRng::seed_from_u64(0xC0FFEE);

        assert_eq!(
            wow_loot::generate_money_loot_with_rate_like_cpp(0, 0, 1.0, &mut rng),
            0
        );
        assert_eq!(
            wow_loot::generate_money_loot_with_rate_like_cpp(120, 100, 1.0, &mut rng),
            100
        );
        assert_eq!(
            wow_loot::generate_money_loot_with_rate_like_cpp(100, 100, 1.0, &mut rng),
            100
        );
        assert_eq!(
            wow_loot::generate_money_loot_with_rate_like_cpp(120, 100, 2.5, &mut rng),
            250
        );

        let small_range = wow_loot::generate_money_loot_with_rate_like_cpp(100, 200, 1.0, &mut rng);
        assert!((100..=200).contains(&small_range));

        let wide_range =
            wow_loot::generate_money_loot_with_rate_like_cpp(1_000, 100_000, 1.0, &mut rng);
        assert_eq!(wide_range & 0xFF, 0);
        assert!((((1_000 >> 8) << 8)..=((100_000 >> 8) << 8)).contains(&wide_range));
    }

    #[test]
    fn loot_rate_helpers_match_cpp_roll_inputs() {
        let mut rng = StdRng::seed_from_u64(0xA11CE);
        assert!(roll_chance_with_rate_like_cpp(100.0, 0.0, &mut rng));
        assert!(roll_chance_with_rate_like_cpp(60.0, 2.0, &mut rng));
        assert_eq!(referenced_loot_max_count_like_cpp(3, 2.0), 6);
        assert_eq!(referenced_loot_max_count_like_cpp(3, 0.5), 1);
    }

    #[test]
    fn plain_item_loot_template_validation_matches_cpp_basic_guards() {
        assert!(loot_template_plain_row_can_roll_like_cpp(
            25, 100.0, false, 1, 1, 3, true, true
        ));
        assert!(!loot_template_plain_row_can_roll_like_cpp(
            0, 100.0, false, 1, 1, 3, true, true
        ));
        assert!(!loot_template_plain_row_can_roll_like_cpp(
            25, 0.0, false, 1, 1, 3, true, true
        ));
        assert!(loot_template_plain_row_can_roll_like_cpp(
            25, 100.0, true, 1, 1, 3, true, true
        ));
        assert!(!loot_template_plain_row_can_roll_like_cpp(
            25, 100.0, true, 1, 1, 3, true, false
        ));
        assert!(!loot_template_plain_row_can_roll_like_cpp(
            25, 100.0, false, 0, 1, 3, true, true
        ));
        assert!(!loot_template_plain_row_can_roll_like_cpp(
            25, 100.0, false, 1, 0, 3, true, true
        ));
        assert!(!loot_template_plain_row_can_roll_like_cpp(
            25, 100.0, false, 1, 4, 3, true, true
        ));
        assert!(!loot_template_plain_row_can_roll_like_cpp(
            25, 100.0, false, 1, 1, 3, false, true
        ));
    }

    #[test]
    fn item_loot_quest_status_custom_metadata_matches_cpp_gate() {
        assert!(item_loot_quest_status_allows_like_cpp(
            true, true, true, false
        ));
        assert!(item_loot_quest_status_allows_like_cpp(
            false, false, false, false
        ));
        assert!(!item_loot_quest_status_allows_like_cpp(
            false, true, false, false
        ));
        assert!(!item_loot_quest_status_allows_like_cpp(
            false, false, true, false
        ));
        assert!(item_loot_quest_status_allows_like_cpp(
            false, true, false, true
        ));

        let metadata = ItemTemplateAddonLootMetadataLikeCpp {
            flags_cu: ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP,
            quest_log_item_id: 25,
        };
        assert!(metadata.ignores_quest_status());
        assert_eq!(metadata.quest_log_item_id, 25);
    }

    #[test]
    fn loot_entry_follow_rules_metadata_matches_cpp_constructor() {
        let normal = loot_entry_flags_for_row_metadata_like_cpp(
            false,
            ItemFlags::empty(),
            ItemTemplateAddonLootMetadataLikeCpp::default(),
        );
        assert!(normal.follow_loot_rules);
        assert!(!normal.needs_quest);
        assert!(!normal.freeforall);

        let quest_only_without_custom = loot_entry_flags_for_row_metadata_like_cpp(
            true,
            ItemFlags::empty(),
            ItemTemplateAddonLootMetadataLikeCpp::default(),
        );
        assert!(!quest_only_without_custom.follow_loot_rules);
        assert!(quest_only_without_custom.needs_quest);

        let quest_only_following_rules = loot_entry_flags_for_row_metadata_like_cpp(
            true,
            ItemFlags::MULTI_DROP,
            ItemTemplateAddonLootMetadataLikeCpp {
                flags_cu: ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP,
                quest_log_item_id: 0,
            },
        );
        assert!(quest_only_following_rules.follow_loot_rules);
        assert!(quest_only_following_rules.freeforall);
        assert!(quest_only_following_rules.needs_quest);
    }

    #[test]
    fn loot_entry_free_for_all_view_metadata_matches_cpp() {
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 43);
        let normal = LootEntry {
            loot_list_id: 0,
            item_id: 25,
            quantity: 1,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags: LootEntryFlags::default(),
            allowed_looters: vec![player_guid],
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        };
        assert_eq!(normal.free_for_all_ui_type_like_cpp(), 4);
        assert!(normal.is_over_threshold_like_cpp());
        assert!(normal.visible_in_represented_free_for_all_view_like_cpp(player_guid));
        assert!(!normal.visible_in_represented_free_for_all_view_like_cpp(other_guid));

        let ffa = LootEntry {
            flags: LootEntryFlags {
                freeforall: true,
                needs_quest: true,
                ..Default::default()
            },
            ..normal.clone()
        };
        assert_eq!(ffa.free_for_all_ui_type_like_cpp(), 4);
        assert!(!ffa.is_over_threshold_like_cpp());
        assert!(ffa.flags.needs_quest);
        assert!(!ffa.is_looted_for_player_like_cpp(player_guid));
        let mut looted_ffa = ffa.clone();
        looted_ffa.add_allowed_looter_like_cpp(other_guid);
        looted_ffa.mark_looted_for_player_like_cpp(player_guid);
        assert!(!looted_ffa.taken);
        assert!(looted_ffa.is_looted_for_player_like_cpp(player_guid));
        assert!(!looted_ffa.is_looted_for_player_like_cpp(other_guid));
        assert!(!looted_ffa.fully_looted_like_cpp());
        looted_ffa.mark_looted_for_player_like_cpp(other_guid);
        assert!(looted_ffa.fully_looted_like_cpp());

        let under_threshold = LootEntry {
            flags: LootEntryFlags {
                under_threshold: true,
                ..Default::default()
            },
            ..normal
        };
        assert!(!under_threshold.is_over_threshold_like_cpp());

        let taken = LootEntry {
            taken: true,
            ..under_threshold
        };
        assert!(!taken.visible_in_represented_free_for_all_view_like_cpp(player_guid));
    }

    fn condition(
        else_group: u32,
        condition_type_or_reference: i32,
        value1: u32,
        negative: bool,
    ) -> LootConditionRowLikeCpp {
        LootConditionRowLikeCpp {
            else_group,
            condition_type_or_reference,
            condition_target: 0,
            value1,
            value2: 0,
            value3: 0,
            string_value1: String::new(),
            negative,
            script_name: String::new(),
        }
    }

    #[test]
    fn loot_conditions_else_group_and_negative_match_cpp() {
        let conditions = vec![
            condition(0, 25, 100, false),
            condition(0, 25, 200, false),
            condition(1, 25, 300, true),
        ];

        assert!(loot_conditions_allow_player_like_cpp_representable(
            &conditions,
            |condition| Some(condition.value1 == 100 || condition.value1 == 200),
        ));
        assert!(loot_conditions_allow_player_like_cpp_representable(
            &conditions,
            |condition| Some(condition.value1 == 100),
        ));
        assert!(!loot_conditions_allow_player_like_cpp_representable(
            &conditions,
            |condition| Some(condition.value1 == 300),
        ));

        let mut scripted = condition(0, 25, 100, false);
        scripted.script_name = "npc_custom".to_string();
        assert!(!loot_conditions_allow_player_like_cpp_representable(
            &[scripted],
            |_| Some(true),
        ));
        assert!(loot_conditions_allow_player_like_cpp_representable(
            &[condition(0, -1, 100, false)],
            |_| Some(true),
        ));
    }

    #[test]
    fn loot_condition_compare_and_quest_state_match_cpp_values() {
        assert_eq!(condition_compare_values_like_cpp(0, 10, 10), Some(true));
        assert_eq!(condition_compare_values_like_cpp(1, 11, 10), Some(true));
        assert_eq!(condition_compare_values_like_cpp(2, 9, 10), Some(true));
        assert_eq!(condition_compare_values_like_cpp(3, 10, 10), Some(true));
        assert_eq!(condition_compare_values_like_cpp(4, 10, 10), Some(true));
        assert_eq!(condition_compare_values_like_cpp(5, 10, 10), None);

        assert_eq!(player_quest_status_mask_like_cpp(None, false), 1);
        assert_eq!(player_quest_status_mask_like_cpp(Some(2), false), 2);
        assert_eq!(player_quest_status_mask_like_cpp(Some(1), false), 8);
        assert_eq!(player_quest_status_mask_like_cpp(Some(3), false), 32);
        assert_eq!(player_quest_status_mask_like_cpp(Some(1), true), 64);

        assert_eq!(player_class_mask_like_cpp(13), Some(1 << 12));
        assert_eq!(player_class_mask_like_cpp(14), None);
        assert_eq!(player_race_mask_like_cpp(34), Some(1 << 11));
        assert_eq!(player_race_mask_like_cpp(35), Some(1 << 12));
        assert_eq!(player_race_mask_like_cpp(52), Some(1 << 16));
        assert_eq!(player_race_mask_like_cpp(70), Some(1 << 15));
        assert_eq!(player_race_mask_like_cpp(33), None);
    }

    #[test]
    fn add_loot_item_stacks_caps_like_cpp_max_nr_loot_items() {
        let mut loot_items = Vec::new();
        add_loot_item_stacks_like_cpp(&mut loot_items, 25, 45, 20, Default::default());
        assert_eq!(loot_items.len(), 3);
        assert_eq!(loot_items[0].quantity, 20);
        assert_eq!(loot_items[1].quantity, 20);
        assert_eq!(loot_items[2].quantity, 5);
        assert_eq!(loot_items[2].loot_list_id, 2);
        assert_eq!(loot_items[2].random_properties_id, 0);
        assert_eq!(loot_items[2].random_properties_seed, 0);
        assert_eq!(loot_items[2].item_context, 0);

        let mut capped = Vec::new();
        add_loot_item_stacks_like_cpp(&mut capped, 25, 100, 1, Default::default());
        assert_eq!(capped.len(), 18);
        assert_eq!(capped[17].loot_list_id, 17);
    }

    #[test]
    fn reference_loot_template_validation_matches_cpp_basic_guards() {
        assert!(loot_template_reference_row_can_roll_like_cpp(
            10, 100.0, 1, 1
        ));
        assert!(loot_template_reference_row_can_roll_like_cpp(
            10, 0.0000001, 1, 1
        ));
        assert!(!loot_template_reference_row_can_roll_like_cpp(
            0, 100.0, 1, 1
        ));
        assert!(!loot_template_reference_row_can_roll_like_cpp(
            10, 0.0, 1, 1
        ));
        assert!(!loot_template_reference_row_can_roll_like_cpp(
            10, 100.0, 0, 1
        ));
        assert!(!loot_template_reference_row_can_roll_like_cpp(
            10, 100.0, 1, 0
        ));
    }

    #[test]
    fn grouped_loot_template_roll_matches_cpp_explicit_then_equal_order() {
        assert!(loot_template_group_row_can_roll_like_cpp(
            25, 0.0, false, 1, 1, 1, true, true
        ));
        assert!(loot_template_group_row_can_roll_like_cpp(
            25, 0.0, true, 1, 1, 1, true, true
        ));
        assert!(!loot_template_group_row_can_roll_like_cpp(
            25, 0.0, true, 1, 1, 1, true, false
        ));
        assert!(!loot_template_group_row_can_roll_like_cpp(
            25, 0.0000001, false, 1, 1, 1, true, true
        ));

        let rows = vec![
            LootTemplateRow {
                item_id: 25,
                reference: 0,
                chance: 100.0,
                needs_quest: false,
                loot_mode: 1,
                group_id: 1,
                min_count: 1,
                max_count: 1,
                conditions: Vec::new(),
            },
            LootTemplateRow {
                item_id: 26,
                reference: 0,
                chance: 0.0,
                needs_quest: false,
                loot_mode: 1,
                group_id: 1,
                min_count: 1,
                max_count: 1,
                conditions: Vec::new(),
            },
        ];
        let mut rng = StdRng::seed_from_u64(0xBEEF);
        let selected =
            roll_group_loot_row_like_cpp(&rows, 1, |_| true, |_| true, |_| 1.0, &mut rng).unwrap();
        assert_eq!(selected.item_id, 25);

        let equal_rows = vec![
            LootTemplateRow {
                chance: 0.0,
                item_id: 25,
                ..rows[0].clone()
            },
            LootTemplateRow {
                chance: 0.0,
                item_id: 26,
                ..rows[1].clone()
            },
        ];
        let mut rng = StdRng::seed_from_u64(0xBEEF);
        let selected =
            roll_group_loot_row_like_cpp(&equal_rows, 1, |_| true, |_| true, |_| 1.0, &mut rng)
                .unwrap();
        assert!([25, 26].contains(&selected.item_id));
    }

    #[test]
    fn open_item_stored_loot_preserves_random_properties_and_context_like_cpp() {
        assert!(stored_item_row_can_load_like_cpp_representable(
            25, 2, 7, false, false, -77, 456, 2, true
        ));
        assert!(stored_item_row_can_load_like_cpp_representable(
            25, 2, 7, false, true, -77, 456, 2, true
        ));

        let entry = LootEntry {
            loot_list_id: 7,
            item_id: 25,
            quantity: 2,
            random_properties_id: -77,
            random_properties_seed: 456,
            item_context: 2,
            flags: LootEntryFlags::default(),
            allowed_looters: Vec::new(),
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        };
        let response_item = wow_packet::packets::loot::LootItemData {
            item_type: 0,
            ui_type: entry.free_for_all_ui_type_like_cpp(),
            can_trade_to_tap_list: false,
            loot: wow_packet::packets::item::ItemInstance {
                item_id: entry.item_id as i32,
                ..wow_packet::packets::item::ItemInstance::default()
            },
            loot_list_id: entry.loot_list_id,
            quantity: entry.quantity,
            loot_item_type: 0,
        };
        assert_eq!(entry.random_properties_id, -77);
        assert_eq!(entry.random_properties_seed, 456);
        assert_eq!(entry.item_context, 2);
        assert_eq!(response_item.loot.item_id, 25);
        assert!(response_item.loot.item_bonus.is_none());

        assert!(!stored_item_row_can_load_like_cpp_representable(
            25, 2, 256, false, false, 0, 0, 0, true
        ));
        assert!(!stored_item_row_can_load_like_cpp_representable(
            25, 2, 7, true, false, 0, 0, 0, true
        ));
    }

    #[test]
    fn stored_loot_item_persistence_skips_missing_template_and_currency_tokens_like_cpp() {
        // template missing -> no persist
        assert!(!stored_loot_item_should_persist_like_cpp(
            false,
            BagFamilyMask::NONE
        ));

        // normal template -> persist
        assert!(stored_loot_item_should_persist_like_cpp(
            true,
            BagFamilyMask::NONE
        ));

        // currency token -> no persist (C++ ItemTemplate::IsCurrencyToken)
        assert!(!stored_loot_item_should_persist_like_cpp(
            true,
            BagFamilyMask::CURRENCY_TOKENS
        ));

        // currency token combined with other families still no persist
        assert!(!stored_loot_item_should_persist_like_cpp(
            true,
            BagFamilyMask::CURRENCY_TOKENS | BagFamilyMask::HERBS
        ));
    }
}
