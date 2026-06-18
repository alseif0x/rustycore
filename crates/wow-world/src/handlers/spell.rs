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
    SpellCastResult, TypeId,
};
use wow_core::ObjectGuid;
use wow_data::{DISABLE_TYPE_SPELL, DisableWorldObjectRefLikeCpp};
use wow_database::{CharStatements, SqlTransaction, WorldStatements};
use wow_entities::INVENTORY_SLOT_BAG_0;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_loot::{
    LootConditionRowLikeCpp, condition_compare_values_like_cpp,
    loot_condition_reference_ids_like_cpp, loot_condition_reference_self_references_like_cpp,
    loot_condition_row_normalize_without_external_stores_like_cpp,
    loot_conditions_allow_player_with_references_like_cpp_representable,
};
use wow_packet::ClientPacket;
use wow_packet::packets::item::{ItemExpirePurchaseRefund, ItemInstance};
use wow_packet::packets::loot::{
    CreatureLoot, LOOT_TYPE_ITEM_LIKE_CPP, LootEntry, LootEntryFlags, LootItemData, LootResponse,
};
use wow_packet::packets::pet::PetCancelAura;
use wow_packet::packets::spell::{
    CancelAura, CancelAutoRepeatSpell, CancelCast, CancelChannelling, CancelGrowthAura,
    CancelModSpeedNoControlAuras, CancelMountAura, CancelQueuedSpell, CastFailed, CastSpellRequest,
    OpenItem, SelfRes, SpellCastVisual, SpellClick, SpellStartPkt,
};
use wow_packet::packets::totem::TotemDestroyed;

use crate::session::{RepresentedPendingSpellCastRequestLikeCpp, WorldSession};

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
        opcode: ClientOpcodes::CancelAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_aura",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelAutoRepeatSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_auto_repeat_spell",
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
        opcode: ClientOpcodes::CancelGrowthAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_growth_aura",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelMountAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_mount_aura",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelQueuedSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_queued_spell",
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

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SelfRes,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_self_res",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PetCancelAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_pet_cancel_aura",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TotemDestroyed,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_totem_destroyed",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SpellClick,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_spell_click",
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

        // ── Get spell info ──────────────────────────────────────────────
        let spell_info: wow_data::SpellInfo = match &self.spell_store {
            Some(store) => match store.get(spell_id) {
                Some(info) => info.clone(),
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

        // C++ `WorldSession::HandleCastSpellOpcode` applies an embedded
        // `MoveUpdate` through `HandleMovementOpcode(CMSG_MOVE_STOP, ...)`
        // after validating the `SpellInfo` and before the spell cast request
        // continues.
        if let Some(move_update) = req.move_update.clone() {
            self.handle_movement_info_like_cpp(Some(ClientOpcodes::MoveStop), move_update)
                .await;
        }

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

        let mut spell_target = req.target.clone();
        let target_guid = if !spell_target.unit.is_empty() {
            spell_target.unit
        } else {
            spell_target.flags |= 0x2; // SpellCastTargetFlags::Unit
            spell_target.unit = player_guid;
            player_guid
        };

        // C++ `Player::CanRequestSpellCast` allows client spell queueing only
        // inside the final 400 ms of global cooldown/cast completion. Outside
        // that window, `HandleCastSpellOpcode` sends SPELL_FAILED_SPELL_IN_PROGRESS.
        let remaining_gcd_ms = self.remaining_global_cooldown_ms_like_cpp(&spell_info);
        let remaining_active_cast_ms = self.remaining_active_spell_cast_ms_like_cpp();
        if remaining_gcd_ms > 0 || remaining_active_cast_ms > 0 {
            if !self.can_request_represented_spell_cast_like_cpp(&spell_info) {
                debug!(
                    account = self.account_id,
                    spell_id = spell_id,
                    remaining_gcd_ms = remaining_gcd_ms,
                    remaining_active_cast_ms = remaining_active_cast_ms,
                    "Spell request rejected outside C++ spell queue window"
                );
                self.send_packet(&CastFailed {
                    cast_id,
                    spell_id,
                    reason: SpellCastResult::SpellInProgress as i32,
                    fail_arg1: 0,
                    fail_arg2: 0,
                });
                return;
            }

            self.request_represented_spell_cast_like_cpp(
                RepresentedPendingSpellCastRequestLikeCpp {
                    cast_id,
                    spell_id,
                    casting_unit_guid: player_guid,
                    target_guid,
                    target_data: spell_target,
                    spell_visual: SpellCastVisual {
                        spell_visual_id: req.visual.spell_visual_id,
                        script_visual_id: 0,
                    },
                    metadata: crate::session::SpellCastMetadata {
                        from_client: true,
                        misc: req.misc,
                        original_cast_id: cast_id,
                        ..crate::session::SpellCastMetadata::default()
                    },
                },
            );
            return;
        }

        // Check per-spell cooldown. C++ spell queueing is driven by global
        // cooldown/current cast; represented per-spell cooldowns still fail
        // closed until full SpellHistory parity is ported.
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
                        reason: SpellCastResult::NotReady as i32,
                        fail_arg1: 0,
                        fail_arg2: 0,
                    });
                    return;
                }
            }
        }

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
                    self.represented_money_loot_with_rate_like_cpp(
                        min_money,
                        max_money,
                        self.loot_drop_rates_like_cpp().money,
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
        &mut self,
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

        let mut rng = self.represented_runtime_subrng_like_cpp();
        let mut processed_frames = 0u32;
        while let Some(mut frame) = frames.pop() {
            if frame.group_id != 0 {
                let addon_metadata = self
                    .load_item_template_addon_loot_metadata_for_rows_like_cpp(&frame.rows)
                    .await;
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
                    add_loot_template_row_item_like_cpp(
                        &mut loot_items,
                        &row,
                        flags,
                        |item_id| {
                            self.item_storage_template(item_id)
                                .map(|template| template.max_stack_size)
                                .unwrap_or(1)
                        },
                        &mut rng,
                    );
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
                        &mut rng,
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
                    &mut rng,
                )
            {
                continue;
            }
            let flags = self.loot_entry_flags_for_row_like_cpp(&row, addon_metadata);
            add_loot_template_row_item_like_cpp(
                &mut loot_items,
                &row,
                flags,
                |item_id| {
                    self.item_storage_template(item_id)
                        .map(|template| template.max_stack_size)
                        .unwrap_or(1)
                },
                &mut rng,
            );
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

    /// Handle `CMSG_SPELL_CLICK`.
    ///
    /// C++ `WorldSession::HandleSpellClick` resolves an in-world creature, pet,
    /// or vehicle and then delegates to `Unit::HandleSpellClick`. Rust already
    /// represents the packet shape plus spellclick stores/conditions/visibility;
    /// executing spellclick casts, vehicle seat handling, and AI callbacks stays
    /// in the next bounded runtime slice.
    pub async fn handle_spell_click(&mut self, mut pkt: wow_packet::WorldPacket) {
        let spell_click = match SpellClick::read(&mut pkt) {
            Ok(spell_click) => spell_click,
            Err(e) => {
                warn!(
                    account = self.account_id,
                    "Failed to parse CMSG_SPELL_CLICK: {e}"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            target = ?spell_click.unit_guid,
            try_auto_dismount = spell_click.try_auto_dismount,
            "CMSG_SPELL_CLICK"
        );

        let plan = self.represented_handle_spell_click_plan_like_cpp(spell_click.unit_guid);
        debug!(
            account = self.account_id,
            target = ?spell_click.unit_guid,
            casts = plan.casts.len(),
            exact_context_unrepresented = plan.exact_context_unrepresented,
            ai_on_spell_click_unrepresented = plan.ai_on_spell_click_unrepresented,
            "CMSG_SPELL_CLICK represented execution plan"
        );
        let outcome = self
            .execute_represented_spell_click_plan_like_cpp(spell_click.unit_guid, &plan)
            .await;
        debug!(
            account = self.account_id,
            target = ?spell_click.unit_guid,
            planned_casts = outcome.planned_casts,
            executed_casts = outcome.executed_casts,
            skipped_unrepresented_caster = outcome.skipped_unrepresented_caster,
            skipped_unrepresented_target = outcome.skipped_unrepresented_target,
            skipped_unrepresented_original_caster = outcome.skipped_unrepresented_original_caster,
            failed_casts = outcome.failed_casts,
            "CMSG_SPELL_CLICK represented execution outcome"
        );
    }

    /// Handle `CMSG_CANCEL_CAST` — player cancels an in-progress cast.
    pub async fn handle_cancel_cast(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match CancelCast::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CancelCast parse failed: {error}"
                );
                return;
            }
        };

        let Some(active_cast) = self.active_spell_cast.as_ref() else {
            return;
        };

        if request.spell_id != 0 && active_cast.spell_id != request.spell_id as i32 {
            return;
        }

        self.active_spell_cast = None;
    }

    /// Handle `CMSG_CANCEL_AURA` — player requests removing a cancelable owned aura.
    pub async fn handle_cancel_aura(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match CancelAura::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CancelAura parse failed: {error}"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            spell_id = request.spell_id,
            caster_guid = ?request.caster_guid,
            "CMSG_CANCEL_AURA parsed"
        );
        let Some(spell_store) = self.spell_store() else {
            return;
        };
        if spell_store.get(request.spell_id).is_none()
            || spell_store.has_attribute0_like_cpp(
                request.spell_id,
                wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL,
            )
        {
            return;
        }
        if spell_store.is_channeled_like_cpp(request.spell_id) {
            self.interrupt_current_channeled_spell_like_cpp(request.spell_id);
            return;
        }
        if spell_store.is_passive_like_cpp(request.spell_id) {
            return;
        }
        self.remove_represented_cancelable_owned_aura_like_cpp(
            request.spell_id,
            request.caster_guid,
        );
    }

    /// Handle `CMSG_CANCEL_AUTO_REPEAT_SPELL`.
    pub async fn handle_cancel_auto_repeat_spell(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = CancelAutoRepeatSpell::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "CancelAutoRepeatSpell parse failed: {error}"
            );
        }
        // C++ interrupts CURRENT_AUTOREPEAT_SPELL. Rust does not yet represent
        // a separate auto-repeat current-spell slot, so this remains silent.
    }

    /// Handle `CMSG_CANCEL_CHANNELLING` — player stops a channelled spell.
    pub async fn handle_cancel_channelling(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match CancelChannelling::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CancelChannelling parse failed: {error}"
                );
                return;
            }
        };

        let Some(spell_store) = self.spell_store() else {
            return;
        };

        if spell_store.get(request.channel_spell).is_none()
            || spell_store.has_attribute0_like_cpp(
                request.channel_spell,
                wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL,
            )
        {
            return;
        }

        debug!(
            account = self.account_id,
            channel_spell = request.channel_spell,
            reason = request.reason,
            "CMSG_CANCEL_CHANNELLING parsed"
        );
        self.interrupt_current_channeled_spell_like_cpp(request.channel_spell);
    }

    /// Handle `CMSG_CANCEL_GROWTH_AURA`.
    pub async fn handle_cancel_growth_aura(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = CancelGrowthAura::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "CancelGrowthAura parse failed: {error}"
            );
        }
        self.remove_represented_growth_auras_cancelable_like_cpp();
    }

    /// Handle the represented `CMSG_CANCEL_MOD_SPEED_NO_CONTROL_AURAS`.
    ///
    /// The inspected opcode table assigns this packet to the shared unresolved
    /// `0xBADD` value, so `WorldSession` probes this handler from that branch
    /// and falls through to other 0xBADD packet shapes when the target does not
    /// match C++ `Player::GetUnitBeingMoved()`.
    pub async fn try_handle_cancel_mod_speed_no_control_auras_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) -> bool {
        let request = match CancelModSpeedNoControlAuras::read(&mut pkt) {
            Ok(request) if pkt.is_empty() => request,
            _ => return false,
        };
        if self.player_moved_unit_guid_like_cpp() != request.target_guid {
            return false;
        }

        self.remove_represented_mod_speed_no_control_auras_cancelable_like_cpp();
        true
    }

    /// Handle `CMSG_CANCEL_MOUNT_AURA`.
    pub async fn handle_cancel_mount_aura(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = CancelMountAura::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "CancelMountAura parse failed: {error}"
            );
        }
        self.remove_represented_mount_auras_cancelable_like_cpp();
    }

    /// Handle `CMSG_CANCEL_QUEUED_SPELL`.
    pub async fn handle_cancel_queued_spell(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = CancelQueuedSpell::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "CancelQueuedSpell parse failed: {error}"
            );
            return;
        }
        // C++ cancels `Player::CancelPendingCastRequest`, not the current
        // non-melee spell. The represented queue is separate from
        // `active_spell_cast`, so this keeps casts already in progress alive.
        self.cancel_pending_spell_cast_request_like_cpp();
    }

    /// Handle `CMSG_SELF_RES`.
    pub async fn handle_self_res(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match SelfRes::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(account = self.account_id, "SelfRes parse failed: {error}");
                return;
            }
        };

        debug!(
            account = self.account_id,
            spell_id = request.spell_id,
            "CMSG_SELF_RES parsed"
        );
        if !self.has_represented_self_res_spell_like_cpp(request.spell_id) {
            return;
        }
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if self
            .execute_spell(request.spell_id, player_guid)
            .await
            .is_ok()
        {
            self.remove_represented_self_res_spell_like_cpp(request.spell_id);
        }
    }

    /// Handle `CMSG_PET_CANCEL_AURA`.
    pub async fn handle_pet_cancel_aura(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match PetCancelAura::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "PetCancelAura parse failed: {error}"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            pet_guid = ?request.pet_guid,
            spell_id = request.spell_id,
            "CMSG_PET_CANCEL_AURA parsed"
        );
        self.cancel_represented_pet_aura_like_cpp(request.pet_guid, request.spell_id);
    }

    /// Handle `CMSG_TOTEM_DESTROYED`.
    pub async fn handle_totem_destroyed(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match TotemDestroyed::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "TotemDestroyed parse failed: {error}"
                );
                return;
            }
        };

        debug!(
            account = self.account_id,
            slot = request.slot,
            totem_guid = ?request.totem_guid,
            "CMSG_TOTEM_DESTROYED parsed"
        );
        self.destroy_represented_totem_like_cpp(request.slot, request.totem_guid);
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
    rng: &mut impl Rng,
) where
    F: Fn(u32) -> u32,
{
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
    use std::sync::{Arc, Mutex};
    use std::time::Instant;

    use rand::{Rng, SeedableRng, rngs::StdRng};

    use wow_constants::{
        BagFamilyMask, DeathState, ItemContext, ItemFieldFlags, ItemFlags, ItemUpdateState,
        ServerOpcodes, SpellCastResult,
    };
    use wow_core::{ObjectGuid, Position, guid::HighGuid};
    use wow_entities::{
        AppliedAuraRef, Creature, Item, ItemCreateInfo, MAX_ITEM_SPELLS, Pet, PetType, Player,
        UNIT_MASK_TOTEM,
    };
    use wow_loot::{
        LootConditionRowLikeCpp, condition_compare_values_like_cpp,
        loot_conditions_allow_player_like_cpp_representable,
    };
    use wow_packet::WorldPacket;
    use wow_packet::packets::loot::{LootEntry, LootEntryFlags};
    use wow_packet::packets::movement::MovementInfo;
    use wow_packet::packets::spell::{SpellCastVisual, SpellTargetData};

    use super::{
        ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP, ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP,
        ItemTemplateAddonLootMetadataLikeCpp, LOOT_MODE_DEFAULT_LIKE_CPP, LootTemplateRow,
        add_loot_item_stacks_like_cpp, add_loot_template_row_item_like_cpp,
        apply_wrapped_gift_transform_like_cpp, item_loot_quest_status_allows_like_cpp,
        loot_entry_flags_for_row_metadata_like_cpp, loot_template_group_row_can_roll_like_cpp,
        loot_template_plain_row_can_roll_like_cpp, loot_template_reference_row_can_roll_like_cpp,
        player_class_mask_like_cpp, player_quest_status_mask_like_cpp, player_race_mask_like_cpp,
        referenced_loot_max_count_like_cpp, roll_chance_with_rate_like_cpp,
        roll_group_loot_row_like_cpp, stored_item_row_can_load_like_cpp_representable,
        stored_loot_item_should_persist_like_cpp,
    };
    use crate::session::{
        AuraApplication, RepresentedAuraEffectLikeCpp, RepresentedPendingSpellCastRequestLikeCpp,
        SessionPlayerController, SharedCanonicalMapManager, SpellCastMetadata, SpellCastState,
    };

    fn make_session() -> (crate::session::WorldSession, flume::Receiver<Vec<u8>>) {
        let (_pkt_tx, pkt_rx) = flume::bounded(100);
        let (send_tx, send_rx) = flume::bounded(100);

        (
            crate::session::WorldSession::new(
                1,
                "TestAccount".into(),
                0,
                2,
                9,
                54261,
                vec![0u8; 40],
                "esES".into(),
                pkt_rx,
                send_tx,
            ),
            send_rx,
        )
    }

    fn shared_canonical_map_manager() -> SharedCanonicalMapManager {
        Arc::new(Mutex::new(wow_map::MapManager::default()))
    }

    fn add_canonical_test_player_on_map(
        canonical: &SharedCanonicalMapManager,
        guid: ObjectGuid,
        position: Position,
        map_id: u32,
        instance_id: u32,
    ) {
        let mut player = Player::new(Some(1), false);
        player.unit_mut().world_mut().object_mut().create(guid);
        player.unit_mut().world_mut().set_name("SpellHandlerPlayer");
        player
            .unit_mut()
            .world_mut()
            .set_map(map_id, instance_id)
            .unwrap();
        player.unit_mut().world_mut().relocate(position);
        player.unit_mut().world_mut().object_mut().add_to_world();

        canonical
            .lock()
            .unwrap()
            .create_world_map(map_id, instance_id)
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
            .unwrap();
    }

    fn install_canonical_player(
        session: &mut crate::session::WorldSession,
        canonical: &SharedCanonicalMapManager,
        player_guid: ObjectGuid,
    ) {
        let position = Position::new(10.0, 20.0, 30.0, 0.0);
        session.set_canonical_map_manager(Arc::clone(canonical));
        session.attach_player_controller_like_cpp(SessionPlayerController::new(
            player_guid,
            "SpellHandlerPlayer".to_string(),
            position,
            571,
            1,
            1,
            80,
            0,
        ));
        add_canonical_test_player_on_map(canonical, player_guid, position, 571, 0);
    }

    fn add_canonical_test_pet_on_map(
        canonical: &SharedCanonicalMapManager,
        owner_guid: ObjectGuid,
        pet_guid: ObjectGuid,
        spell_id: u32,
        alive: bool,
    ) {
        let mut pet = Pet::new(owner_guid, PetType::Hunter);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(pet_guid);
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .set_map(571, 0)
            .unwrap();
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .relocate(Position::new(10.5, 20.5, 30.0, 0.0));
        pet.creature_mut()
            .unit_mut()
            .world_mut()
            .object_mut()
            .add_to_world();
        pet.creature_mut().unit_mut().set_max_health(100);
        pet.creature_mut()
            .unit_mut()
            .set_health(if alive { 100 } else { 0 });
        if !alive {
            pet.creature_mut()
                .unit_mut()
                .set_death_state(DeathState::Dead);
        }
        let aura = AppliedAuraRef::new(spell_id, ObjectGuid::EMPTY, 0, 0x1);
        pet.creature_mut()
            .unit_mut()
            .subsystems_mut()
            .auras
            .add_applied(aura);

        canonical
            .lock()
            .unwrap()
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .insert_map_object_record(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
            .unwrap();
    }

    fn add_canonical_test_creature_on_map(
        canonical: &SharedCanonicalMapManager,
        guid: ObjectGuid,
        position: Position,
        map_id: u32,
        instance_id: u32,
        is_totem: bool,
    ) {
        let mut creature = Creature::new(false);
        creature.unit_mut().world_mut().object_mut().create(guid);
        creature.unit_mut().world_mut().object_mut().set_entry(777);
        creature
            .unit_mut()
            .world_mut()
            .set_map(map_id, instance_id)
            .unwrap();
        creature.unit_mut().world_mut().relocate(position);
        creature.unit_mut().world_mut().object_mut().add_to_world();
        if is_totem {
            creature.add_unit_type_mask_like_cpp(UNIT_MASK_TOTEM);
        }

        canonical
            .lock()
            .unwrap()
            .find_map_mut(map_id, instance_id)
            .unwrap()
            .map_mut()
            .insert_map_object_record(
                wow_entities::MapObjectRecord::new_creature(creature).unwrap(),
            )
            .unwrap();
    }

    fn set_canonical_player_pet_guid(
        canonical: &SharedCanonicalMapManager,
        player_guid: ObjectGuid,
        pet_guid: ObjectGuid,
    ) {
        canonical
            .lock()
            .unwrap()
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .set_pet_guid(pet_guid);
    }

    fn set_canonical_player_charmed_guid(
        canonical: &SharedCanonicalMapManager,
        player_guid: ObjectGuid,
        charmed_guid: ObjectGuid,
    ) {
        canonical
            .lock()
            .unwrap()
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .charmed_guid = Some(charmed_guid);
    }

    fn canonical_pet_has_applied_aura(
        canonical: &SharedCanonicalMapManager,
        pet_guid: ObjectGuid,
        aura: AppliedAuraRef,
    ) -> bool {
        canonical
            .lock()
            .unwrap()
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_pet(pet_guid)
            .unwrap()
            .creature()
            .unit()
            .subsystems()
            .auras
            .has_applied(aura)
    }

    fn canonical_creature_has_applied_aura(
        canonical: &SharedCanonicalMapManager,
        creature_guid: ObjectGuid,
        aura: AppliedAuraRef,
    ) -> bool {
        canonical
            .lock()
            .unwrap()
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_creature(creature_guid)
            .unwrap()
            .unit()
            .subsystems()
            .auras
            .has_applied(aura)
    }

    fn set_canonical_player_summon_slot(
        canonical: &SharedCanonicalMapManager,
        player_guid: ObjectGuid,
        slot: usize,
        guid: ObjectGuid,
    ) {
        canonical
            .lock()
            .unwrap()
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .control
            .set_summon_slot(slot, guid);
    }

    fn canonical_player_summon_slot(
        canonical: &SharedCanonicalMapManager,
        player_guid: ObjectGuid,
        slot: usize,
    ) -> ObjectGuid {
        canonical
            .lock()
            .unwrap()
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .unwrap()
            .unit()
            .subsystems()
            .control
            .summon_slots[slot]
    }

    fn canonical_creature_exists(canonical: &SharedCanonicalMapManager, guid: ObjectGuid) -> bool {
        canonical
            .lock()
            .unwrap()
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_creature(guid)
            .is_some()
    }

    fn install_active_spell_cast(
        session: &mut crate::session::WorldSession,
        spell_id: i32,
        cast_id: ObjectGuid,
    ) {
        let player_guid = ObjectGuid::create_player(1, 42);
        session.active_spell_cast = Some(SpellCastState {
            spell_id,
            target_guid: player_guid,
            target_data: SpellTargetData {
                flags: 0x2, // SpellCastTargetFlags::Unit
                unit: player_guid,
                ..Default::default()
            },
            cast_id,
            cast_start_time: std::time::Instant::now(),
            cast_time_ms: 30_000,
            spell_visual: super::SpellCastVisual {
                spell_visual_id: 1,
                script_visual_id: 0,
            },
            metadata: SpellCastMetadata::default(),
        });
    }

    fn install_pending_spell_cast_request(
        session: &mut crate::session::WorldSession,
        spell_id: i32,
        cast_id: ObjectGuid,
    ) {
        session.represented_pending_spell_cast_request_like_cpp =
            Some(RepresentedPendingSpellCastRequestLikeCpp {
                cast_id,
                spell_id,
                casting_unit_guid: ObjectGuid::create_player(1, 42),
                target_guid: ObjectGuid::create_player(1, 42),
                target_data: SpellTargetData {
                    flags: 0x2,
                    unit: ObjectGuid::create_player(1, 42),
                    ..SpellTargetData::default()
                },
                spell_visual: SpellCastVisual {
                    spell_visual_id: 0,
                    script_visual_id: 0,
                },
                metadata: SpellCastMetadata::default(),
            });
    }

    fn install_canonical_channeled_spell(
        session: &mut crate::session::WorldSession,
        player_guid: ObjectGuid,
        spell_id: u32,
    ) -> wow_entities::CurrentSpellRef {
        let spell = wow_entities::CurrentSpellRef::new(spell_id, Some(player_guid), None)
            .with_state(wow_constants::SpellState::Delayed);
        session.mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .set_current_cast_spell(wow_entities::CurrentSpellSlot::Channeled, spell);
        });
        spell
    }

    fn canonical_channeled_spell_id(session: &mut crate::session::WorldSession) -> Option<u32> {
        session
            .mutate_canonical_player_like_cpp(|player| {
                player
                    .unit()
                    .current_spell(wow_entities::CurrentSpellSlot::Channeled)
                    .map(|spell| spell.spell_id)
            })
            .flatten()
    }

    fn cancel_cast_packet(cast_id: ObjectGuid, spell_id: u32) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&cast_id);
        pkt.write_uint32(spell_id);
        pkt.reset_read();
        pkt
    }

    fn cancel_channelling_packet(channel_spell: i32, reason: i32) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(channel_spell);
        pkt.write_int32(reason);
        pkt.reset_read();
        pkt
    }

    fn cast_spell_packet(spell_id: i32, caster_guid: ObjectGuid) -> WorldPacket {
        cast_spell_packet_with_move_update(spell_id, caster_guid, None)
    }

    fn cast_spell_packet_with_move_update(
        spell_id: i32,
        caster_guid: ObjectGuid,
        move_update: Option<MovementInfo>,
    ) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&caster_guid);
        pkt.write_int32(0);
        pkt.write_int32(0);
        pkt.write_int32(spell_id);
        SpellCastVisual {
            spell_visual_id: 0,
            script_visual_id: 0,
        }
        .write(&mut pkt);
        pkt.write_float(0.0);
        pkt.write_float(0.0);
        pkt.write_packed_guid(&ObjectGuid::EMPTY);
        pkt.write_uint32(0);
        pkt.write_uint32(0);
        pkt.write_uint32(0);
        pkt.write_bits(0, 5);
        pkt.write_bit(move_update.is_some());
        pkt.write_bits(0, 2);
        pkt.write_bit(false);
        pkt.flush_bits();
        SpellTargetData {
            flags: 0x2,
            unit: caster_guid,
            ..SpellTargetData::default()
        }
        .write(&mut pkt);
        if let Some(move_update) = move_update {
            move_update.write(&mut pkt);
        }
        pkt.reset_read();
        pkt
    }

    fn basic_spell_store(spell_ids: impl IntoIterator<Item = i32>) -> Arc<wow_data::SpellStore> {
        let mut spell_store = wow_data::SpellStore::new();
        for spell_id in spell_ids {
            spell_store.insert(
                spell_id,
                wow_data::SpellInfo {
                    spell_id,
                    cast_time_ms: 0,
                    cooldown_ms: 0,
                    recovery_time_ms: 0,
                    effect_type: 0,
                    effect_base_points: 0,
                    effect_bonus_coefficient: 0.0,
                    aura_type: None,
                    display_flags: 0,
                    requires_spell_focus: 0,
                    effects: Vec::new(),
                },
            );
        }
        Arc::new(spell_store)
    }

    fn spell_store_with_global_cooldown(
        spell_id: i32,
        cooldown_ms: u32,
    ) -> Arc<wow_data::SpellStore> {
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                effects: Vec::new(),
            },
        );
        Arc::new(spell_store)
    }

    fn mounted_spell_store(spell_id: i32, creature_entry: i32) -> Arc<wow_data::SpellStore> {
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 77,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOUNTED),
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
                    effect_base_points: 77,
                    effect_misc_value_1: creature_entry,
                    ..Default::default()
                }],
            },
        );
        Arc::new(spell_store)
    }

    fn mounted_flying_spell_store(spell_id: i32, creature_entry: i32) -> Arc<wow_data::SpellStore> {
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 77,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOUNTED),
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![
                    wow_data::SpellEffectInfo {
                        effect_index: 0,
                        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
                        effect_base_points: 77,
                        effect_misc_value_1: creature_entry,
                        ..Default::default()
                    },
                    wow_data::SpellEffectInfo {
                        effect_index: 1,
                        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED,
                        effect_base_points: 280,
                        ..Default::default()
                    },
                ],
            },
        );
        Arc::new(spell_store)
    }

    fn shapeshift_spell_store(spell_id: i32, form_id: i32) -> Arc<wow_data::SpellStore> {
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT),
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT,
                    effect_misc_value_1: form_id,
                    ..Default::default()
                }],
            },
        );
        Arc::new(spell_store)
    }

    fn mounted_spell_store_with_active_shapeshift_aura(
        mount_spell_id: i32,
        shapeshift_spell_id: i32,
        form_id: i32,
    ) -> Arc<wow_data::SpellStore> {
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(
            mount_spell_id,
            wow_data::SpellInfo {
                spell_id: mount_spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 77,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOUNTED),
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
                    effect_base_points: 77,
                    ..Default::default()
                }],
            },
        );
        spell_store.insert(
            shapeshift_spell_id,
            wow_data::SpellInfo {
                spell_id: shapeshift_spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT),
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT,
                    effect_misc_value_1: form_id,
                    ..Default::default()
                }],
            },
        );
        Arc::new(spell_store)
    }

    fn mounted_spell_store_with_transform_spell(
        mount_spell_id: i32,
        transform_spell_id: i32,
        allow_while_mounted: bool,
    ) -> Arc<wow_data::SpellStore> {
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(
            mount_spell_id,
            wow_data::SpellInfo {
                spell_id: mount_spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 77,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOUNTED),
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
                    effect_base_points: 77,
                    ..Default::default()
                }],
            },
        );
        spell_store.insert(
            transform_spell_id,
            wow_data::SpellInfo {
                spell_id: transform_spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_TRANSFORM),
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
                    ..Default::default()
                }],
            },
        );
        if allow_while_mounted {
            let mut attributes = [0; 15];
            attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_ALLOW_WHILE_MOUNTED;
            spell_store.insert_spell_misc_attributes_like_cpp(transform_spell_id, attributes);
        }
        Arc::new(spell_store)
    }

    fn active_shapeshift_aura_for_test(spell_id: i32, caster_guid: ObjectGuid) -> AuraApplication {
        AuraApplication {
            spell_id,
            caster_guid,
            slot: 0,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 0x0000_0001,
            effect_mask: 1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        }
    }

    fn set_canonical_player_display_for_test(
        canonical: &SharedCanonicalMapManager,
        player_guid: ObjectGuid,
        display_id: u32,
        set_native: bool,
    ) {
        let mut manager = canonical.lock().unwrap();
        let player = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_player_mut(player_guid)
            .unwrap();
        player.unit_mut().set_display_id(display_id, set_native);
    }

    fn creature_display_info_extra_for_test(
        id: u32,
        display_race_id: i8,
    ) -> wow_data::CreatureDisplayInfoExtraEntry {
        wow_data::CreatureDisplayInfoExtraEntry {
            id,
            display_race_id,
            display_sex_id: 0,
            display_class_id: 0,
            skin_id: 0,
            face_id: 0,
            hair_style_id: 0,
            hair_color_id: 0,
            facial_hair_id: 0,
            flags: 0,
            bake_material_resources_id: 0,
            hd_bake_material_resources_id: 0,
            custom_display_option: [0; 3],
        }
    }

    fn chr_races_entry_for_test(
        id: u32,
        flags: i32,
    ) -> wow_data::character_progression::ChrRacesEntry {
        wow_data::character_progression::ChrRacesEntry {
            id,
            client_prefix: String::new(),
            client_file_string: String::new(),
            name: String::new(),
            flags,
            male_display_id: 0,
            female_display_id: 0,
            high_res_male_display_id: 0,
            high_res_female_display_id: 0,
            res_sickness_spell_id: 0,
            splash_sound_id: 0,
            create_screen_file_data_id: 0,
            select_screen_file_data_id: 0,
            low_res_screen_file_data_id: 0,
            altered_form_start_visual_kit_id: [0; 3],
            altered_form_finish_visual_kit_id: [0; 3],
            heritage_armor_achievement_id: 0,
            starting_level: 1,
            ui_display_order: 0,
            playable_race_bit: 0,
            female_skeleton_file_data_id: 0,
            male_skeleton_file_data_id: 0,
            helmet_anim_scaling_race_id: 0,
            transmogrify_disabled_slot_mask: 0,
            faction_id: 0,
            cinematic_sequence_id: 0,
            base_language: 0,
            creature_type: 0,
            alliance: 0,
            race_related: 0,
            unaltered_visual_race_id: 0,
            default_class_id: 0,
            neutral_race_id: 0,
        }
    }

    fn set_transformed_display_mount_check_stores_for_test(
        session: &mut crate::session::WorldSession,
        transformed_display_id: u32,
        model_flags: u32,
        race_flags: i32,
    ) {
        let display_extra_id = 91;
        let model_id = 92;
        let race_id = 7;
        session.set_creature_display_info_store(Arc::new(
            wow_data::CreatureDisplayInfoStore::from_entries([
                wow_data::CreatureDisplayInfoEntry {
                    id: transformed_display_id,
                    model_id,
                    extended_display_info_id: display_extra_id,
                    creature_model_scale: 1.0,
                },
            ]),
        ));
        session.set_creature_display_info_extra_store(Arc::new(
            wow_data::CreatureDisplayInfoExtraStore::from_entries([
                creature_display_info_extra_for_test(display_extra_id as u32, race_id),
            ]),
        ));
        session.set_creature_model_data_store(Arc::new(
            wow_data::CreatureModelDataStore::from_entries([wow_data::CreatureModelDataEntry {
                id: u32::from(model_id),
                flags: model_flags,
                collision_height: 2.0,
                model_scale: 1.0,
                mount_height: 0.0,
            }]),
        ));
        session.set_chr_races_store(Arc::new(
            wow_data::character_progression::ChrRacesStore::from_entries([
                chr_races_entry_for_test(u32::from(race_id as u8), race_flags),
            ]),
        ));
    }

    fn mounted_spell_store_with_no_aura_cancel(
        spell_id: i32,
        creature_entry: i32,
    ) -> Arc<wow_data::SpellStore> {
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 77,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOUNTED),
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
                    effect_base_points: 77,
                    effect_misc_value_1: creature_entry,
                    ..Default::default()
                }],
            },
        );
        let mut attributes = [0; 15];
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL;
        spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
        Arc::new(spell_store)
    }

    fn channeled_spell_store(spell_id: i32) -> Arc<wow_data::SpellStore> {
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                effects: Vec::new(),
            },
        );
        let mut attributes = [0; 15];
        attributes[1] = wow_data::spell::attributes::SPELL_ATTR1_IS_CHANNELLED;
        spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
        Arc::new(spell_store)
    }

    fn channeled_spell_store_with_no_aura_cancel(spell_id: i32) -> Arc<wow_data::SpellStore> {
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                effects: Vec::new(),
            },
        );
        let mut attributes = [0; 15];
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL;
        attributes[1] = wow_data::spell::attributes::SPELL_ATTR1_IS_CHANNELLED;
        spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
        Arc::new(spell_store)
    }

    fn mod_scale_spell_store(spell_id: i32, no_aura_cancel: bool) -> Arc<wow_data::SpellStore> {
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 50,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_SCALE),
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_SCALE,
                    effect_base_points: 50,
                    ..Default::default()
                }],
            },
        );
        if no_aura_cancel {
            let mut attributes = [0; 15];
            attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL;
            spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
        }
        Arc::new(spell_store)
    }

    fn mod_speed_no_control_spell_store(
        spell_id: i32,
        no_aura_cancel: bool,
    ) -> Arc<wow_data::SpellStore> {
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_base_points: 50,
                effect_bonus_coefficient: 0.0,
                aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_NO_CONTROL),
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                    effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_NO_CONTROL,
                    effect_base_points: 50,
                    ..Default::default()
                }],
            },
        );
        if no_aura_cancel {
            let mut attributes = [0; 15];
            attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_NO_AURA_CANCEL;
            spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
        }
        Arc::new(spell_store)
    }

    fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
        let mut opcodes = Vec::new();
        while let Ok(bytes) = send_rx.try_recv() {
            if let Some(opcode) = WorldPacket::from_bytes(&bytes).server_opcode() {
                opcodes.push(opcode);
            }
        }
        opcodes
    }

    fn drain_server_packet_bytes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<Vec<u8>> {
        let mut packets = Vec::new();
        while let Ok(bytes) = send_rx.try_recv() {
            packets.push(bytes);
        }
        packets
    }

    fn cast_failed_reason_like_cpp(bytes: &[u8]) -> i32 {
        let mut packet = WorldPacket::from_bytes(bytes);
        assert_eq!(
            packet.server_opcode(),
            Some(ServerOpcodes::CastFailed),
            "expected CastFailed packet"
        );
        let _ = packet.read_uint16().expect("opcode");
        let _ = packet.read_packed_guid().expect("cast id");
        let _ = packet.read_int32().expect("spell id");
        packet.read_int32().expect("reason")
    }

    fn cast_failed_fields_like_cpp(bytes: &[u8]) -> (ObjectGuid, i32, i32) {
        let mut packet = WorldPacket::from_bytes(bytes);
        assert_eq!(
            packet.server_opcode(),
            Some(ServerOpcodes::CastFailed),
            "expected CastFailed packet"
        );
        let _ = packet.read_uint16().expect("opcode");
        let cast_id = packet.read_packed_guid().expect("cast id");
        let spell_id = packet.read_int32().expect("spell id");
        let reason = packet.read_int32().expect("reason");
        (cast_id, spell_id, reason)
    }

    fn mount_result_like_cpp(bytes: &[u8]) -> i32 {
        let mut packet = WorldPacket::from_bytes(bytes);
        assert_eq!(
            packet.server_opcode(),
            Some(ServerOpcodes::MountResult),
            "expected MountResult packet"
        );
        let _ = packet.read_uint16().expect("opcode");
        packet.read_int32().expect("result")
    }

    fn cancel_aura_packet(spell_id: i32, caster_guid: ObjectGuid) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(spell_id);
        pkt.write_packed_guid(&caster_guid);
        pkt.reset_read();
        pkt
    }

    fn cancel_mod_speed_no_control_packet(target_guid: ObjectGuid) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&target_guid);
        pkt.reset_read();
        pkt
    }

    fn int32_spell_packet(spell_id: i32) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_int32(spell_id);
        pkt.reset_read();
        pkt
    }

    fn self_res_spell_store(spell_id: i32) -> Arc<wow_data::SpellStore> {
        let mut spell_store = wow_data::SpellStore::new();
        spell_store.insert(
            spell_id,
            wow_data::SpellInfo {
                spell_id,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![wow_data::SpellEffectInfo {
                    effect_index: 0,
                    effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT,
                    effect_base_points: -35,
                    effect_misc_value_1: 77,
                    ..Default::default()
                }],
            },
        );
        Arc::new(spell_store)
    }

    fn pet_cancel_aura_packet(pet_guid: ObjectGuid, spell_id: u32) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&pet_guid);
        pkt.write_uint32(spell_id);
        pkt.reset_read();
        pkt
    }

    fn totem_destroyed_packet(slot: u8, totem_guid: ObjectGuid) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint8(slot);
        pkt.write_packed_guid(&totem_guid);
        pkt.reset_read();
        pkt
    }

    #[tokio::test]
    async fn cancel_cast_clears_matching_active_cast_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 7);
        install_active_spell_cast(&mut session, 12_345, cast_id);

        session
            .handle_cancel_cast(cancel_cast_packet(cast_id, 12_345))
            .await;

        assert!(session.active_spell_cast.is_none());
    }

    #[tokio::test]
    async fn cancel_cast_mismatched_spell_preserves_active_cast_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 7);
        install_active_spell_cast(&mut session, 12_345, cast_id);

        session
            .handle_cancel_cast(cancel_cast_packet(cast_id, 67_890))
            .await;

        assert_eq!(
            session
                .active_spell_cast
                .as_ref()
                .map(|active_cast| active_cast.spell_id),
            Some(12_345)
        );
    }

    #[tokio::test]
    async fn cancel_channelling_interrupts_matching_player_channel_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 8);
        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_spell_store(basic_spell_store([12_345]));
        install_active_spell_cast(&mut session, 12_345, cast_id);
        install_canonical_channeled_spell(&mut session, player_guid, 12_345);

        session
            .handle_cancel_channelling(cancel_channelling_packet(12_345, 40))
            .await;

        assert_eq!(canonical_channeled_spell_id(&mut session), None);
        assert!(session.active_spell_cast.is_none());
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn cancel_channelling_mismatched_spell_preserves_channel_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 9);
        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_spell_store(basic_spell_store([67_890]));
        install_active_spell_cast(&mut session, 12_345, cast_id);
        let spell = install_canonical_channeled_spell(&mut session, player_guid, 12_345);

        session
            .handle_cancel_channelling(cancel_channelling_packet(67_890, 40))
            .await;

        assert_eq!(
            canonical_channeled_spell_id(&mut session),
            Some(spell.spell_id)
        );
        assert_eq!(
            session
                .active_spell_cast
                .as_ref()
                .map(|active_cast| active_cast.spell_id),
            Some(12_345)
        );
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn cancel_channelling_no_aura_cancel_spell_preserves_channel_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 12);
        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_spell_store(mounted_spell_store_with_no_aura_cancel(12_345, 0));
        install_active_spell_cast(&mut session, 12_345, cast_id);
        let spell = install_canonical_channeled_spell(&mut session, player_guid, 12_345);

        session
            .handle_cancel_channelling(cancel_channelling_packet(12_345, 40))
            .await;

        assert_eq!(
            canonical_channeled_spell_id(&mut session),
            Some(spell.spell_id)
        );
        assert_eq!(
            session
                .active_spell_cast
                .as_ref()
                .map(|active_cast| active_cast.spell_id),
            Some(12_345)
        );
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn cancel_channelling_zero_spell_preserves_channel_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 10);
        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_spell_store(basic_spell_store([12_345]));
        install_active_spell_cast(&mut session, 12_345, cast_id);
        let spell = install_canonical_channeled_spell(&mut session, player_guid, 12_345);

        session
            .handle_cancel_channelling(cancel_channelling_packet(0, 40))
            .await;

        assert_eq!(
            canonical_channeled_spell_id(&mut session),
            Some(spell.spell_id)
        );
        assert_eq!(
            session
                .active_spell_cast
                .as_ref()
                .map(|active_cast| active_cast.spell_id),
            Some(12_345)
        );
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn cancel_channelling_missing_spellinfo_preserves_channel_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 11);
        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_spell_store(basic_spell_store([]));
        install_active_spell_cast(&mut session, 12_345, cast_id);
        let spell = install_canonical_channeled_spell(&mut session, player_guid, 12_345);

        session
            .handle_cancel_channelling(cancel_channelling_packet(12_345, 40))
            .await;

        assert_eq!(
            canonical_channeled_spell_id(&mut session),
            Some(spell.spell_id)
        );
        assert_eq!(
            session
                .active_spell_cast
                .as_ref()
                .map(|active_cast| active_cast.spell_id),
            Some(12_345)
        );
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn cancel_queued_spell_clears_only_pending_request_like_cpp() {
        let (mut session, send_rx) = make_session();
        let active_cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 7);
        let pending_cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 8);
        install_active_spell_cast(&mut session, 12_345, active_cast_id);
        install_pending_spell_cast_request(&mut session, 67_890, pending_cast_id);

        session
            .handle_cancel_queued_spell(WorldPacket::new_empty())
            .await;

        assert_eq!(
            session
                .active_spell_cast
                .as_ref()
                .map(|active_cast| active_cast.spell_id),
            Some(12_345)
        );
        assert!(
            session
                .represented_pending_spell_cast_request_like_cpp
                .is_none()
        );

        let packets = drain_server_packet_bytes(&send_rx);
        assert_eq!(packets.len(), 1);
        assert_eq!(
            cast_failed_fields_like_cpp(&packets[0]),
            (pending_cast_id, 67_890, 32),
            "C++ Player::CancelPendingCastRequest sends SPELL_FAILED_DONT_REPORT for the queued request"
        );
    }

    #[tokio::test]
    async fn cancel_queued_spell_without_pending_request_is_silent_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_cancel_queued_spell(WorldPacket::new_empty())
            .await;

        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn cancel_aura_without_matching_represented_aura_stays_silent_like_cpp() {
        let (mut session, send_rx) = make_session();
        let caster_guid = ObjectGuid::create_player(1, 42);

        session
            .handle_cancel_aura(cancel_aura_packet(12_345, caster_guid))
            .await;

        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn cancel_aura_channeled_spell_interrupts_matching_channel_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 13);
        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_spell_store(channeled_spell_store(12_345));
        install_active_spell_cast(&mut session, 12_345, cast_id);
        install_canonical_channeled_spell(&mut session, player_guid, 12_345);

        session
            .handle_cancel_aura(cancel_aura_packet(12_345, ObjectGuid::EMPTY))
            .await;

        assert_eq!(canonical_channeled_spell_id(&mut session), None);
        assert!(session.active_spell_cast.is_none());
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn cancel_aura_channeled_mismatched_current_spell_preserves_channel_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 14);
        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_spell_store(channeled_spell_store(67_890));
        install_active_spell_cast(&mut session, 12_345, cast_id);
        let spell = install_canonical_channeled_spell(&mut session, player_guid, 12_345);

        session
            .handle_cancel_aura(cancel_aura_packet(67_890, ObjectGuid::EMPTY))
            .await;

        assert_eq!(
            canonical_channeled_spell_id(&mut session),
            Some(spell.spell_id)
        );
        assert_eq!(
            session
                .active_spell_cast
                .as_ref()
                .map(|active_cast| active_cast.spell_id),
            Some(12_345)
        );
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn cancel_aura_channeled_no_aura_cancel_preserves_channel_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 15);
        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_spell_store(channeled_spell_store_with_no_aura_cancel(12_345));
        install_active_spell_cast(&mut session, 12_345, cast_id);
        let spell = install_canonical_channeled_spell(&mut session, player_guid, 12_345);

        session
            .handle_cancel_aura(cancel_aura_packet(12_345, ObjectGuid::EMPTY))
            .await;

        assert_eq!(
            canonical_channeled_spell_id(&mut session),
            Some(spell.spell_id)
        );
        assert_eq!(
            session
                .active_spell_cast
                .as_ref()
                .map(|active_cast| active_cast.spell_id),
            Some(12_345)
        );
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn cancel_aura_removes_matching_represented_mount_aura_like_cpp() {
        let (mut session, send_rx) = make_session();
        let caster_guid = ObjectGuid::create_player(1, 42);
        let effect = wow_data::SpellEffectInfo {
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
            effect_base_points: 77,
            effect_misc_value_1: 0,
            ..Default::default()
        };

        session
            .apply_represented_mounted_aura_for_test_like_cpp(12_345, caster_guid, &effect)
            .unwrap();
        session.set_spell_store(mounted_spell_store(12_345, 0));
        assert!(session.player_mounted_like_cpp());

        session
            .handle_cancel_aura(cancel_aura_packet(12_345, caster_guid))
            .await;

        assert!(!session.player_mounted_like_cpp());
        assert!(!send_rx.is_empty());
    }

    #[tokio::test]
    async fn cancel_aura_no_aura_cancel_spell_preserves_represented_mount_like_cpp() {
        let (mut session, send_rx) = make_session();
        let caster_guid = ObjectGuid::create_player(1, 42);
        let effect = wow_data::SpellEffectInfo {
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
            effect_base_points: 77,
            effect_misc_value_1: 0,
            ..Default::default()
        };

        session
            .apply_represented_mounted_aura_for_test_like_cpp(12_345, caster_guid, &effect)
            .unwrap();
        session.set_spell_store(mounted_spell_store_with_no_aura_cancel(12_345, 0));
        let _ = drain_server_opcodes(&send_rx);

        session
            .handle_cancel_aura(cancel_aura_packet(12_345, caster_guid))
            .await;

        assert!(session.player_mounted_like_cpp());
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn cancel_aura_preserves_represented_mount_from_other_caster_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let caster_guid = ObjectGuid::create_player(1, 42);
        let other_caster_guid = ObjectGuid::create_player(1, 43);
        let effect = wow_data::SpellEffectInfo {
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
            effect_base_points: 77,
            effect_misc_value_1: 0,
            ..Default::default()
        };

        session
            .apply_represented_mounted_aura_for_test_like_cpp(12_345, caster_guid, &effect)
            .unwrap();
        session.set_spell_store(mounted_spell_store(12_345, 0));

        session
            .handle_cancel_aura(cancel_aura_packet(12_345, other_caster_guid))
            .await;

        assert!(session.player_mounted_like_cpp());
    }

    #[tokio::test]
    async fn cancel_aura_empty_caster_matches_represented_mount_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let caster_guid = ObjectGuid::create_player(1, 42);
        let effect = wow_data::SpellEffectInfo {
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
            effect_base_points: 77,
            effect_misc_value_1: 0,
            ..Default::default()
        };

        session
            .apply_represented_mounted_aura_for_test_like_cpp(12_345, caster_guid, &effect)
            .unwrap();
        session.set_spell_store(mounted_spell_store(12_345, 0));

        session
            .handle_cancel_aura(cancel_aura_packet(12_345, ObjectGuid::EMPTY))
            .await;

        assert!(!session.player_mounted_like_cpp());
    }

    #[tokio::test]
    async fn cancel_empty_spell_handlers_stay_silent_without_runtime_slots_like_cpp() {
        let (mut session, send_rx) = make_session();

        session
            .handle_cancel_auto_repeat_spell(WorldPacket::new_empty())
            .await;
        session
            .handle_cancel_growth_aura(WorldPacket::new_empty())
            .await;
        session
            .handle_cancel_mount_aura(WorldPacket::new_empty())
            .await;

        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn cancel_growth_aura_removes_represented_mod_scale_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(player_guid));
        session.set_spell_store(mod_scale_spell_store(12_345, false));

        session
            .execute_spell(12_345, player_guid)
            .await
            .expect("represented mod-scale aura should apply");
        let _ = drain_server_opcodes(&send_rx);
        assert!(session.visible_auras.values().any(|aura| {
            aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModScale)
        }));

        session
            .handle_cancel_growth_aura(WorldPacket::new_empty())
            .await;

        assert!(!session.visible_auras.values().any(|aura| {
            aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModScale)
        }));
    }

    #[tokio::test]
    async fn cancel_growth_aura_no_aura_cancel_preserves_mod_scale_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(player_guid));
        session.set_spell_store(mod_scale_spell_store(12_345, true));

        session
            .execute_spell(12_345, player_guid)
            .await
            .expect("represented no-aura-cancel mod-scale aura should apply");
        let _ = drain_server_opcodes(&send_rx);

        session
            .handle_cancel_growth_aura(WorldPacket::new_empty())
            .await;

        assert!(session.visible_auras.values().any(|aura| {
            aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModScale)
        }));
    }

    #[tokio::test]
    async fn cancel_mod_speed_no_control_removes_matching_mover_aura_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(player_guid));
        session.set_spell_store(mod_speed_no_control_spell_store(12_345, false));

        session
            .execute_spell(12_345, player_guid)
            .await
            .expect("represented mod-speed-no-control aura should apply");
        let _ = drain_server_opcodes(&send_rx);
        assert!(session.visible_auras.values().any(|aura| {
            aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModSpeedNoControl)
        }));

        assert!(
            session
                .try_handle_cancel_mod_speed_no_control_auras_like_cpp(
                    cancel_mod_speed_no_control_packet(player_guid),
                )
                .await
        );

        assert!(!session.visible_auras.values().any(|aura| {
            aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModSpeedNoControl)
        }));
    }

    #[tokio::test]
    async fn cancel_mod_speed_no_control_ignores_non_mover_guid_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 43);
        session.set_player_guid(Some(player_guid));
        session.set_spell_store(mod_speed_no_control_spell_store(12_345, false));

        session
            .execute_spell(12_345, player_guid)
            .await
            .expect("represented mod-speed-no-control aura should apply");
        let _ = drain_server_opcodes(&send_rx);

        assert!(
            !session
                .try_handle_cancel_mod_speed_no_control_auras_like_cpp(
                    cancel_mod_speed_no_control_packet(other_guid),
                )
                .await
        );

        assert!(session.visible_auras.values().any(|aura| {
            aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModSpeedNoControl)
        }));
    }

    #[tokio::test]
    async fn cancel_mod_speed_no_control_no_aura_cancel_preserves_aura_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        session.set_player_guid(Some(player_guid));
        session.set_spell_store(mod_speed_no_control_spell_store(12_345, true));

        session
            .execute_spell(12_345, player_guid)
            .await
            .expect("represented no-aura-cancel mod-speed-no-control aura should apply");
        let _ = drain_server_opcodes(&send_rx);

        assert!(
            session
                .try_handle_cancel_mod_speed_no_control_auras_like_cpp(
                    cancel_mod_speed_no_control_packet(player_guid),
                )
                .await
        );

        assert!(session.visible_auras.values().any(|aura| {
            aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::ModSpeedNoControl)
        }));
    }

    #[tokio::test]
    async fn cancel_mount_aura_no_aura_cancel_spell_preserves_mount_like_cpp() {
        let (mut session, send_rx) = make_session();
        let caster_guid = ObjectGuid::create_player(1, 42);
        let effect = wow_data::SpellEffectInfo {
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
            effect_base_points: 77,
            effect_misc_value_1: 0,
            ..Default::default()
        };

        session
            .apply_represented_mounted_aura_for_test_like_cpp(12_345, caster_guid, &effect)
            .unwrap();
        session.set_spell_store(mounted_spell_store_with_no_aura_cancel(12_345, 0));
        let _ = drain_server_opcodes(&send_rx);

        session
            .handle_cancel_mount_aura(WorldPacket::new_empty())
            .await;

        assert!(session.player_mounted_like_cpp());
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn self_res_unlisted_spell_stays_silent_like_cpp() {
        let (mut session, send_rx) = make_session();
        let spell_id = 20_000;

        session.set_player_guid(Some(ObjectGuid::create_player(1, 20_000)));
        session.set_spell_store(self_res_spell_store(spell_id));
        session.set_player_health_like_cpp(0, 100);

        session.handle_self_res(int32_spell_packet(spell_id)).await;

        assert!(!session.player_is_alive_like_cpp());
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn self_res_listed_spell_casts_and_removes_self_res_spell_like_cpp() {
        let (mut session, send_rx) = make_session();
        let spell_id = 20_001;

        session.set_player_guid(Some(ObjectGuid::create_player(1, 20_001)));
        session.set_spell_store(self_res_spell_store(spell_id));
        session.set_player_health_like_cpp(0, 100);
        session.add_represented_self_res_spell_like_cpp(spell_id);

        session.handle_self_res(int32_spell_packet(spell_id)).await;

        assert!(session.player_is_alive_like_cpp());
        assert_eq!(session.player_health_like_cpp(), 35);
        assert!(!session.has_represented_self_res_spell_like_cpp(spell_id));
        assert!(!send_rx.is_empty());
    }

    #[tokio::test]
    async fn self_res_missing_spell_info_keeps_self_res_spell_like_cpp() {
        let (mut session, send_rx) = make_session();
        let spell_id = 20_002;

        session.set_player_guid(Some(ObjectGuid::create_player(1, 20_002)));
        session.set_player_health_like_cpp(0, 100);
        session.add_represented_self_res_spell_like_cpp(spell_id);

        session.handle_self_res(int32_spell_packet(spell_id)).await;

        assert!(!session.player_is_alive_like_cpp());
        assert!(session.has_represented_self_res_spell_like_cpp(spell_id));
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn pet_cancel_aura_removes_owned_pet_aura_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 777, 42);
        let spell_id = 12_345;
        let aura = AppliedAuraRef::new(spell_id, ObjectGuid::EMPTY, 0, 0x1);
        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_spell_store(basic_spell_store([spell_id as i32]));
        set_canonical_player_pet_guid(&canonical, player_guid, pet_guid);
        add_canonical_test_pet_on_map(&canonical, player_guid, pet_guid, spell_id, true);

        session
            .handle_pet_cancel_aura(pet_cancel_aura_packet(pet_guid, spell_id))
            .await;

        assert!(!canonical_pet_has_applied_aura(&canonical, pet_guid, aura));
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn cast_spell_applies_embedded_move_update_like_cpp() {
        let (mut session, _send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let spell_id = 13_337;
        let moved_position = Position::new(33.0, 44.0, 55.0, 1.25);
        let move_update = MovementInfo {
            guid: player_guid,
            time: 12_345,
            position: moved_position,
            ..MovementInfo::default()
        };

        session.set_player_guid(Some(player_guid));
        session.set_player_map_position_like_cpp(571, Position::new(10.0, 20.0, 30.0, 0.0));
        session.set_known_spells_like_cpp(vec![spell_id]);
        session.set_spell_store(basic_spell_store([spell_id]));

        session
            .handle_cast_spell(cast_spell_packet_with_move_update(
                spell_id,
                player_guid,
                Some(move_update),
            ))
            .await;

        assert_eq!(session.player_position_like_cpp(), Some(moved_position));
    }

    #[tokio::test]
    async fn cast_spell_rejects_gcd_outside_spell_queue_window_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let spell_id = 13_338;
        session.set_player_guid(Some(player_guid));
        session.set_known_spells_like_cpp(vec![spell_id]);
        session.set_spell_store(spell_store_with_global_cooldown(spell_id, 1_500));
        session.last_spell_cast_time = Some(std::time::Instant::now());

        session
            .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
            .await;

        assert!(
            session
                .represented_pending_spell_cast_request_like_cpp
                .is_none()
        );
        let packets = drain_server_packet_bytes(&send_rx);
        assert_eq!(packets.len(), 1);
        assert_eq!(
            cast_failed_reason_like_cpp(&packets[0]),
            SpellCastResult::SpellInProgress as i32,
            "C++ HandleCastSpellOpcode sends SPELL_FAILED_SPELL_IN_PROGRESS when CanRequestSpellCast rejects the request"
        );
    }

    #[tokio::test]
    async fn cast_spell_queues_within_spell_queue_window_and_executes_after_gcd_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let spell_id = 13_339;
        session.set_player_guid(Some(player_guid));
        session.set_known_spells_like_cpp(vec![spell_id]);
        session.set_spell_store(spell_store_with_global_cooldown(spell_id, 1_500));
        session.last_spell_cast_time =
            Some(std::time::Instant::now() - std::time::Duration::from_millis(1_200));

        session
            .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
            .await;

        assert!(
            session
                .represented_pending_spell_cast_request_like_cpp
                .as_ref()
                .is_some_and(|pending| pending.spell_id == spell_id)
        );
        assert!(
            send_rx.is_empty(),
            "C++ RequestSpellCast only queues while GCD is still active"
        );

        session.last_spell_cast_time =
            Some(std::time::Instant::now() - std::time::Duration::from_millis(1_500));
        session.tick_pending_spell_cast_request_like_cpp().await;

        assert!(
            session
                .represented_pending_spell_cast_request_like_cpp
                .is_none()
        );
        let opcodes = drain_server_opcodes(&send_rx);
        assert!(opcodes.contains(&ServerOpcodes::SpellGo));
        assert!(opcodes.contains(&ServerOpcodes::CooldownEvent));
    }

    #[tokio::test]
    async fn cast_spell_rejects_active_cast_outside_spell_queue_window_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let active_cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 20);
        let queued_spell_id = 13_340;
        session.set_player_guid(Some(player_guid));
        session.set_known_spells_like_cpp(vec![12_345, queued_spell_id]);
        session.set_spell_store(basic_spell_store([12_345, queued_spell_id]));
        install_active_spell_cast(&mut session, 12_345, active_cast_id);

        session
            .handle_cast_spell(cast_spell_packet(queued_spell_id, player_guid))
            .await;

        assert!(
            session
                .represented_pending_spell_cast_request_like_cpp
                .is_none()
        );
        let packets = drain_server_packet_bytes(&send_rx);
        assert_eq!(packets.len(), 1);
        assert_eq!(
            cast_failed_reason_like_cpp(&packets[0]),
            SpellCastResult::SpellInProgress as i32
        );
    }

    #[tokio::test]
    async fn cast_spell_queues_near_active_cast_finish_and_executes_after_cast_like_cpp() {
        let (mut session, send_rx) = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let active_cast_id = ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 21);
        let queued_spell_id = 13_341;
        session.set_player_guid(Some(player_guid));
        session.set_known_spells_like_cpp(vec![12_345, queued_spell_id]);
        session.set_spell_store(basic_spell_store([12_345, queued_spell_id]));
        install_active_spell_cast(&mut session, 12_345, active_cast_id);
        if let Some(active) = session.active_spell_cast.as_mut() {
            active.cast_start_time =
                std::time::Instant::now() - std::time::Duration::from_millis(29_700);
        }

        session
            .handle_cast_spell(cast_spell_packet(queued_spell_id, player_guid))
            .await;

        assert!(
            session
                .represented_pending_spell_cast_request_like_cpp
                .as_ref()
                .is_some_and(|pending| pending.spell_id == queued_spell_id)
        );
        assert!(send_rx.is_empty());

        if let Some(active) = session.active_spell_cast.as_mut() {
            active.cast_start_time =
                std::time::Instant::now() - std::time::Duration::from_millis(30_000);
        }
        session.tick_active_spell_cast().await;
        session.tick_pending_spell_cast_request_like_cpp().await;

        assert!(
            session
                .represented_pending_spell_cast_request_like_cpp
                .is_none()
        );
        let opcodes = drain_server_opcodes(&send_rx);
        assert!(opcodes.contains(&ServerOpcodes::SpellGo));
        assert!(opcodes.contains(&ServerOpcodes::CooldownEvent));
    }

    #[tokio::test]
    async fn cast_known_account_mount_spell_applies_mounted_aura_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let spell_id = 12345;

        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_known_spells_like_cpp(vec![spell_id]);
        session.set_spell_store(mounted_spell_store(spell_id, 0));
        session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
            wow_data::MountEntry {
                id: 7,
                mount_type_id: 0,
                flags: 0,
                source_type_enum: 0,
                source_spell_id: spell_id,
                player_condition_id: 0,
                mount_fly_ride_height: 0.0,
                ui_model_scene_id: 0,
            },
        ])));
        session.set_mount_x_display_store(Arc::new(wow_data::MountXDisplayStore::from_entries([
            wow_data::MountXDisplayEntry {
                id: 1,
                creature_display_info_id: 4321,
                player_condition_id: 0,
                mount_id: 7,
            },
        ])));

        session
            .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
            .await;

        assert!(session.player_mounted_like_cpp());
        let opcodes = drain_server_opcodes(&send_rx);
        assert!(opcodes.contains(&ServerOpcodes::SpellGo));
        assert!(opcodes.contains(&ServerOpcodes::AuraUpdate));
        assert!(opcodes.contains(&ServerOpcodes::UpdateObject));

        let manager = canonical.lock().unwrap();
        let player = manager
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .unwrap();
        assert_eq!(player.unit().data().mount_display_id, 4321);
    }

    #[tokio::test]
    async fn cast_mount_spell_fails_not_here_without_mount_capability_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 43);
        let spell_id = 12_346;

        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_known_spells_like_cpp(vec![spell_id]);
        session.set_spell_store(mounted_spell_store(spell_id, 1234));
        session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
            wow_data::MountEntry {
                id: 8,
                mount_type_id: 7,
                flags: 0,
                source_type_enum: 0,
                source_spell_id: spell_id,
                player_condition_id: 0,
                mount_fly_ride_height: 0.0,
                ui_model_scene_id: 0,
            },
        ])));

        session
            .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
            .await;

        assert!(!session.player_mounted_like_cpp());
        let packets = drain_server_packet_bytes(&send_rx);
        assert_eq!(packets.len(), 1);
        assert_eq!(
            cast_failed_reason_like_cpp(&packets[0]),
            SpellCastResult::NotHere as i32
        );
    }

    #[tokio::test]
    async fn cast_flying_mount_spell_fails_only_abovewater_in_water_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 44);
        let spell_id = 12_347;

        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_known_spells_like_cpp(vec![spell_id]);
        session.set_player_liquid_status_like_cpp(crate::session::LIQUID_MAP_IN_WATER_LIKE_CPP);
        session.set_spell_store(mounted_flying_spell_store(spell_id, 1234));
        session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
            wow_data::MountEntry {
                id: 9,
                mount_type_id: 0,
                flags: 0,
                source_type_enum: 0,
                source_spell_id: spell_id,
                player_condition_id: 0,
                mount_fly_ride_height: 0.0,
                ui_model_scene_id: 0,
            },
        ])));

        session
            .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
            .await;

        assert!(!session.player_mounted_like_cpp());
        let packets = drain_server_packet_bytes(&send_rx);
        assert_eq!(packets.len(), 1);
        assert_eq!(
            cast_failed_reason_like_cpp(&packets[0]),
            SpellCastResult::OnlyAbovewater as i32
        );
    }

    #[tokio::test]
    async fn cast_shapeshift_mount_form_fails_not_here_without_capability_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 45);
        let spell_id = 12_348;
        let form_id = 55;

        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_known_spells_like_cpp(vec![spell_id]);
        session.set_spell_store(shapeshift_spell_store(spell_id, form_id));
        session.set_spell_shapeshift_form_store(Arc::new(
            wow_data::SpellShapeshiftFormStore::from_entries([
                wow_data::SpellShapeshiftFormEntry {
                    id: form_id as u32,
                    name: "Mounted Form".to_string(),
                    creature_type: 0,
                    flags: 0,
                    attack_icon_file_id: 0,
                    bonus_action_bar: 0,
                    combat_round_time: 0,
                    damage_variance: 0.0,
                    mount_type_id: 7,
                    creature_display_id: [0; 4],
                    preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
                },
            ]),
        ));

        session
            .handle_cast_spell(cast_spell_packet(spell_id, player_guid))
            .await;

        let packets = drain_server_packet_bytes(&send_rx);
        assert_eq!(packets.len(), 1);
        assert_eq!(
            cast_failed_reason_like_cpp(&packets[0]),
            SpellCastResult::NotHere as i32
        );
    }

    #[tokio::test]
    async fn cast_mount_spell_in_disallowed_shapeshift_form_sends_mount_result_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 46);
        let mount_spell_id = 12_349;
        let shapeshift_spell_id = 22_349;
        let form_id = 56;

        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_known_spells_like_cpp(vec![mount_spell_id]);
        session.set_spell_store(mounted_spell_store_with_active_shapeshift_aura(
            mount_spell_id,
            shapeshift_spell_id,
            form_id,
        ));
        session.set_spell_shapeshift_form_store(Arc::new(
            wow_data::SpellShapeshiftFormStore::from_entries([
                wow_data::SpellShapeshiftFormEntry {
                    id: form_id as u32,
                    name: "Non Stance Form".to_string(),
                    creature_type: 0,
                    flags: 0,
                    attack_icon_file_id: 0,
                    bonus_action_bar: 0,
                    combat_round_time: 0,
                    damage_variance: 0.0,
                    mount_type_id: 0,
                    creature_display_id: [0; 4],
                    preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
                },
            ]),
        ));
        session.visible_auras.insert(
            0,
            active_shapeshift_aura_for_test(shapeshift_spell_id, player_guid),
        );

        session
            .handle_cast_spell(cast_spell_packet(mount_spell_id, player_guid))
            .await;

        assert!(!session.player_mounted_like_cpp());
        let packets = drain_server_packet_bytes(&send_rx);
        assert_eq!(packets.len(), 1);
        assert_eq!(mount_result_like_cpp(&packets[0]), 8);
    }

    #[tokio::test]
    async fn cast_mount_spell_in_stance_shapeshift_form_is_allowed_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 47);
        let mount_spell_id = 12_350;
        let shapeshift_spell_id = 22_350;
        let form_id = 57;

        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_known_spells_like_cpp(vec![mount_spell_id]);
        session.set_spell_store(mounted_spell_store_with_active_shapeshift_aura(
            mount_spell_id,
            shapeshift_spell_id,
            form_id,
        ));
        session.set_spell_shapeshift_form_store(Arc::new(
            wow_data::SpellShapeshiftFormStore::from_entries([
                wow_data::SpellShapeshiftFormEntry {
                    id: form_id as u32,
                    name: "Stance Form".to_string(),
                    creature_type: 0,
                    flags: 0x0000_0001,
                    attack_icon_file_id: 0,
                    bonus_action_bar: 0,
                    combat_round_time: 0,
                    damage_variance: 0.0,
                    mount_type_id: 0,
                    creature_display_id: [0; 4],
                    preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
                },
            ]),
        ));
        session.visible_auras.insert(
            0,
            active_shapeshift_aura_for_test(shapeshift_spell_id, player_guid),
        );

        session
            .handle_cast_spell(cast_spell_packet(mount_spell_id, player_guid))
            .await;

        assert!(session.player_mounted_like_cpp());
        let opcodes = drain_server_opcodes(&send_rx);
        assert!(!opcodes.contains(&ServerOpcodes::MountResult));
        assert!(!opcodes.contains(&ServerOpcodes::CastFailed));
        assert!(opcodes.contains(&ServerOpcodes::SpellGo));
    }

    #[tokio::test]
    async fn cast_mount_spell_in_disallowed_transformed_display_sends_mount_result_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 48);
        let mount_spell_id = 12_351;
        let transformed_display_id = 88_001;

        install_canonical_player(&mut session, &canonical, player_guid);
        set_canonical_player_display_for_test(
            &canonical,
            player_guid,
            transformed_display_id,
            false,
        );
        session.set_known_spells_like_cpp(vec![mount_spell_id]);
        session.set_spell_store(mounted_spell_store(mount_spell_id, 0));
        set_transformed_display_mount_check_stores_for_test(
            &mut session,
            transformed_display_id,
            0,
            0,
        );

        session
            .handle_cast_spell(cast_spell_packet(mount_spell_id, player_guid))
            .await;

        assert!(!session.player_mounted_like_cpp());
        let packets = drain_server_packet_bytes(&send_rx);
        assert_eq!(packets.len(), 1);
        assert_eq!(mount_result_like_cpp(&packets[0]), 8);
    }

    #[tokio::test]
    async fn cast_mount_spell_with_mountable_transform_spell_is_allowed_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 50);
        let mount_spell_id = 12_353;
        let transform_spell_id = 22_353;
        let transformed_display_id = 88_003;

        install_canonical_player(&mut session, &canonical, player_guid);
        set_canonical_player_display_for_test(
            &canonical,
            player_guid,
            transformed_display_id,
            false,
        );
        session.set_known_spells_like_cpp(vec![mount_spell_id]);
        session.set_spell_store(mounted_spell_store_with_transform_spell(
            mount_spell_id,
            transform_spell_id,
            true,
        ));
        set_transformed_display_mount_check_stores_for_test(
            &mut session,
            transformed_display_id,
            0,
            0,
        );
        session.visible_auras.insert(
            0,
            active_shapeshift_aura_for_test(transform_spell_id, player_guid),
        );

        session
            .handle_cast_spell(cast_spell_packet(mount_spell_id, player_guid))
            .await;

        assert!(session.player_mounted_like_cpp());
        let opcodes = drain_server_opcodes(&send_rx);
        assert!(!opcodes.contains(&ServerOpcodes::MountResult));
        assert!(!opcodes.contains(&ServerOpcodes::CastFailed));
        assert!(opcodes.contains(&ServerOpcodes::SpellGo));
    }

    #[tokio::test]
    async fn cast_mount_spell_in_mountable_transformed_model_is_allowed_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 49);
        let mount_spell_id = 12_352;
        let transformed_display_id = 88_002;

        install_canonical_player(&mut session, &canonical, player_guid);
        set_canonical_player_display_for_test(
            &canonical,
            player_guid,
            transformed_display_id,
            false,
        );
        session.set_known_spells_like_cpp(vec![mount_spell_id]);
        session.set_spell_store(mounted_spell_store(mount_spell_id, 0));
        set_transformed_display_mount_check_stores_for_test(
            &mut session,
            transformed_display_id,
            0x0000_0080,
            0,
        );

        session
            .handle_cast_spell(cast_spell_packet(mount_spell_id, player_guid))
            .await;

        assert!(session.player_mounted_like_cpp());
        let opcodes = drain_server_opcodes(&send_rx);
        assert!(!opcodes.contains(&ServerOpcodes::MountResult));
        assert!(!opcodes.contains(&ServerOpcodes::CastFailed));
        assert!(opcodes.contains(&ServerOpcodes::SpellGo));
    }

    #[tokio::test]
    async fn pet_cancel_aura_missing_spellinfo_preserves_pet_aura_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 777, 43);
        let spell_id = 12_346;
        let aura = AppliedAuraRef::new(spell_id, ObjectGuid::EMPTY, 0, 0x1);
        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_spell_store(basic_spell_store([]));
        set_canonical_player_pet_guid(&canonical, player_guid, pet_guid);
        add_canonical_test_pet_on_map(&canonical, player_guid, pet_guid, spell_id, true);

        session
            .handle_pet_cancel_aura(pet_cancel_aura_packet(pet_guid, spell_id))
            .await;

        assert!(canonical_pet_has_applied_aura(&canonical, pet_guid, aura));
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn pet_cancel_aura_non_owned_pet_preserves_aura_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_player_guid = ObjectGuid::create_player(1, 43);
        let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 777, 44);
        let spell_id = 12_347;
        let aura = AppliedAuraRef::new(spell_id, ObjectGuid::EMPTY, 0, 0x1);
        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_spell_store(basic_spell_store([spell_id as i32]));
        add_canonical_test_pet_on_map(&canonical, other_player_guid, pet_guid, spell_id, true);

        session
            .handle_pet_cancel_aura(pet_cancel_aura_packet(pet_guid, spell_id))
            .await;

        assert!(canonical_pet_has_applied_aura(&canonical, pet_guid, aura));
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn pet_cancel_aura_dead_pet_sends_feedback_and_preserves_aura_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 777, 45);
        let spell_id = 12_348;
        let aura = AppliedAuraRef::new(spell_id, ObjectGuid::EMPTY, 0, 0x1);
        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_spell_store(basic_spell_store([spell_id as i32]));
        set_canonical_player_pet_guid(&canonical, player_guid, pet_guid);
        add_canonical_test_pet_on_map(&canonical, player_guid, pet_guid, spell_id, false);

        session
            .handle_pet_cancel_aura(pet_cancel_aura_packet(pet_guid, spell_id))
            .await;

        assert!(canonical_pet_has_applied_aura(&canonical, pet_guid, aura));
        let bytes = send_rx.try_recv().expect("pet action feedback");
        assert_eq!(
            u16::from_le_bytes([bytes[0], bytes[1]]),
            ServerOpcodes::PetActionFeedback as u16
        );
        assert_eq!(&bytes[2..6], &0i32.to_le_bytes());
        assert_eq!(
            bytes[6],
            wow_packet::packets::pet::PET_ACTION_FEEDBACK_DEAD_LIKE_CPP
        );
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn pet_cancel_aura_removes_charmed_creature_aura_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let player_guid = ObjectGuid::create_player(1, 42);
        let creature_guid =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 46);
        let spell_id = 12_349;
        let aura = AppliedAuraRef::new(spell_id, ObjectGuid::EMPTY, 0, 0x1);
        install_canonical_player(&mut session, &canonical, player_guid);
        session.set_spell_store(basic_spell_store([spell_id as i32]));
        set_canonical_player_charmed_guid(&canonical, player_guid, creature_guid);
        add_canonical_test_creature_on_map(
            &canonical,
            creature_guid,
            Position::new(11.0, 21.0, 30.0, 0.0),
            571,
            0,
            false,
        );
        {
            let mut guard = canonical.lock().unwrap();
            let creature = guard
                .find_map_mut(571, 0)
                .unwrap()
                .map_mut()
                .get_typed_creature_mut(creature_guid)
                .unwrap();
            creature.unit_mut().set_max_health(100);
            creature.unit_mut().set_health(100);
            creature.unit_mut().subsystems_mut().auras.add_applied(aura);
        }

        session
            .handle_pet_cancel_aura(pet_cancel_aura_packet(creature_guid, spell_id))
            .await;

        assert!(!canonical_creature_has_applied_aura(
            &canonical,
            creature_guid,
            aura
        ));
        assert!(send_rx.is_empty());
    }

    fn install_canonical_totem_for_session(
        session: &mut crate::session::WorldSession,
        canonical: &SharedCanonicalMapManager,
        slot: usize,
        is_totem: bool,
    ) -> (ObjectGuid, ObjectGuid) {
        let player_guid = ObjectGuid::create_player(1, 42);
        let totem_guid =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, slot as i64);
        install_canonical_player(session, canonical, player_guid);
        set_canonical_player_summon_slot(canonical, player_guid, slot, totem_guid);
        add_canonical_test_creature_on_map(
            canonical,
            totem_guid,
            Position::new(10.5, 20.5, 30.0, 0.0),
            571,
            0,
            is_totem,
        );
        (player_guid, totem_guid)
    }

    #[tokio::test]
    async fn totem_destroyed_despawns_matching_totem_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let slot = wow_entities::UNIT_SUMMON_SLOT_TOTEM + 2;
        let (player_guid, totem_guid) =
            install_canonical_totem_for_session(&mut session, &canonical, slot, true);

        session
            .handle_totem_destroyed(totem_destroyed_packet(2, totem_guid))
            .await;

        assert!(!canonical_creature_exists(&canonical, totem_guid));
        assert_eq!(
            canonical_player_summon_slot(&canonical, player_guid, slot),
            ObjectGuid::EMPTY
        );
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn totem_destroyed_empty_guid_matches_slot_totem_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let slot = wow_entities::UNIT_SUMMON_SLOT_TOTEM + 1;
        let (player_guid, totem_guid) =
            install_canonical_totem_for_session(&mut session, &canonical, slot, true);

        session
            .handle_totem_destroyed(totem_destroyed_packet(1, ObjectGuid::EMPTY))
            .await;

        assert!(!canonical_creature_exists(&canonical, totem_guid));
        assert_eq!(
            canonical_player_summon_slot(&canonical, player_guid, slot),
            ObjectGuid::EMPTY
        );
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn totem_destroyed_mismatched_guid_preserves_totem_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let slot = wow_entities::UNIT_SUMMON_SLOT_TOTEM;
        let (player_guid, totem_guid) =
            install_canonical_totem_for_session(&mut session, &canonical, slot, true);
        let other_guid =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 901);

        session
            .handle_totem_destroyed(totem_destroyed_packet(0, other_guid))
            .await;

        assert!(canonical_creature_exists(&canonical, totem_guid));
        assert_eq!(
            canonical_player_summon_slot(&canonical, player_guid, slot),
            totem_guid
        );
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn totem_destroyed_remote_control_preserves_totem_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let slot = wow_entities::UNIT_SUMMON_SLOT_TOTEM;
        let (player_guid, totem_guid) =
            install_canonical_totem_for_session(&mut session, &canonical, slot, true);
        let controlled_guid =
            ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 902);
        session.set_player_moved_unit_guid_like_cpp(controlled_guid);

        session
            .handle_totem_destroyed(totem_destroyed_packet(0, totem_guid))
            .await;

        assert!(canonical_creature_exists(&canonical, totem_guid));
        assert_eq!(
            canonical_player_summon_slot(&canonical, player_guid, slot),
            totem_guid
        );
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn totem_destroyed_out_of_range_slot_preserves_totem_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let slot = wow_entities::UNIT_SUMMON_SLOT_TOTEM;
        let (player_guid, totem_guid) =
            install_canonical_totem_for_session(&mut session, &canonical, slot, true);

        session
            .handle_totem_destroyed(totem_destroyed_packet(4, totem_guid))
            .await;

        assert!(canonical_creature_exists(&canonical, totem_guid));
        assert_eq!(
            canonical_player_summon_slot(&canonical, player_guid, slot),
            totem_guid
        );
        assert!(send_rx.is_empty());
    }

    #[tokio::test]
    async fn totem_destroyed_non_totem_creature_preserves_slot_like_cpp() {
        let (mut session, send_rx) = make_session();
        let canonical = shared_canonical_map_manager();
        let slot = wow_entities::UNIT_SUMMON_SLOT_TOTEM;
        let (player_guid, totem_guid) =
            install_canonical_totem_for_session(&mut session, &canonical, slot, false);

        session
            .handle_totem_destroyed(totem_destroyed_packet(0, totem_guid))
            .await;

        assert!(canonical_creature_exists(&canonical, totem_guid));
        assert_eq!(
            canonical_player_summon_slot(&canonical, player_guid, slot),
            totem_guid
        );
        assert!(send_rx.is_empty());
    }

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
    fn add_loot_template_row_item_uses_caller_rng_like_cpp_urand_count() {
        let row = LootTemplateRow {
            item_id: 25,
            reference: 0,
            chance: 100.0,
            needs_quest: false,
            loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
            group_id: 0,
            min_count: 2,
            max_count: 7,
            conditions: Vec::new(),
        };
        let mut expected_rng = StdRng::seed_from_u64(0x5151);
        let expected_count = expected_rng.gen_range(2..=7);

        let mut rng = StdRng::seed_from_u64(0x5151);
        let mut loot_items = Vec::new();
        add_loot_template_row_item_like_cpp(
            &mut loot_items,
            &row,
            Default::default(),
            |_| 20,
            &mut rng,
        );

        assert_eq!(loot_items.len(), 1);
        assert_eq!(loot_items[0].quantity, expected_count);
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
