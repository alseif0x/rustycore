// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell cast handlers — CMSG_CAST_SPELL, CMSG_CANCEL_CAST, CMSG_CANCEL_CHANNELLING.
//!
//! Normal requests decode/adapt into `player_cast`; canonical state and
//! publication adapters live under `session/player_cast`. Immediate and queued
//! requests share preparation, while the existing driver consumes timed casts.
//! Other represented spell/item handlers retain their explicit operation paths.
//! Reference: Classic Game/Handlers/SpellHandler.cpp, Player.cpp and Spell.cpp.

use std::collections::HashMap;

use rand::Rng;
use tracing::{debug, warn};

use wow_constants::{
    BagFamilyMask, ClientOpcodes, InventoryResult, ItemFieldFlags, ItemFlags, ItemUpdateState,
    TypeId,
};
use wow_core::ObjectGuid;
use wow_data::{DISABLE_TYPE_SPELL, DisableWorldObjectRefLikeCpp};
use wow_entities::INVENTORY_SLOT_BAG_0;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
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
    CancelModSpeedNoControlAuras, CancelMountAura, CancelQueuedSpell, CastSpellRequest, OpenItem,
    SelfRes, SpellClick,
};
use wow_packet::packets::totem::TotemDestroyed;

use crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP;
use crate::session::{
    AreaTriggerCatalogsLikeCpp, RepresentedPendingSpellCastRequestLikeCpp, WorldSession,
};

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
const MAP_BATTLEGROUND_LIKE_CPP: i8 = 3;
const MAP_ARENA_LIKE_CPP: i8 = 4;
fn normalize_item_money_loot_bounds_like_cpp(min_money: u32, max_money: u32) -> (u32, u32) {
    if min_money > max_money {
        (max_money, min_money)
    } else {
        (min_money, max_money)
    }
}

// ── Handler registrations ─────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CastSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_cast_spell",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_cast_spell_with_catalogs_like_cpp(
                        catalogs.area_triggers.as_ref(),
                        catalogs.creature_spawns.as_ref(),
                        catalogs.progression.as_ref(),
                        &catalogs.player_grid_loader,
                        catalogs.id_generators.item.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelCast,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_cancel_cast",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_cancel_cast(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_aura",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_cancel_aura(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelAutoRepeatSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_auto_repeat_spell",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_cancel_auto_repeat_spell(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelChannelling,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_channelling",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_cancel_channelling(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelGrowthAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_growth_aura",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_cancel_growth_aura(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelMountAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_mount_aura",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_cancel_mount_aura(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::CancelQueuedSpell,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_cancel_queued_spell",
        handler: |session, _catalogs, pkt| {
            Box::pin(async move { session.handle_cancel_queued_spell(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::OpenItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_open_item",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_open_item(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SelfRes,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_self_res",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_self_res_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        catalogs.creature_spawns.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::PetCancelAura,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_pet_cancel_aura",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_pet_cancel_aura(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::TotemDestroyed,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_totem_destroyed",
        handler: |session, _catalogs, pkt| Box::pin(async move { session.handle_totem_destroyed(pkt).await }),
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SpellClick,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_spell_click",
        handler: |session, catalogs, pkt| {
            Box::pin(async move {
                session
                    .handle_spell_click_with_generator_like_cpp(
                        catalogs.id_generators.item.as_ref(),
                        catalogs.creature_spawns.as_ref(),
                        pkt,
                    )
                    .await
            })
        },
    }
}

// ── Handler implementations ───────────────────────────────────────

impl WorldSession {
    /// Handle `CMSG_CAST_SPELL` (0x329C).
    ///
    /// Decode movement and the original client request, then enter the shared
    /// application admission/preparation path for both instant and timed casts.
    pub async fn handle_cast_spell_with_catalogs_like_cpp(
        &mut self,
        area_trigger_catalogs: &AreaTriggerCatalogsLikeCpp,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        progression: &crate::session::ProgressionCatalogsLikeCpp,
        player_grid_loader: &crate::session::PlayerGridLoadResolverLikeCpp,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        mut pkt: wow_packet::WorldPacket,
    ) {
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

        let original_spell_id = req.spell_id;
        let cast_id = req.cast_id;

        debug!(
            account = self.account_id,
            spell_id = original_spell_id,
            cast_id = ?cast_id,
            target = ?req.target.unit,
            "CMSG_CAST_SPELL"
        );

        // C++ ignores a nonexistent spell before applying embedded movement.
        if self
            .spell_store()
            .and_then(|store| store.get(original_spell_id))
            .is_none()
        {
            warn!(
                account = self.account_id,
                spell_id = original_spell_id,
                "Ignoring cast request without an effective spell"
            );
            return;
        }

        // C++ `WorldSession::HandleCastSpellOpcode` applies an embedded
        // `MoveUpdate` through `HandleMovementOpcode(CMSG_MOVE_STOP, ...)`
        // after validating the `SpellInfo` and before the spell cast request
        // continues.
        if let Some(move_update) = req.move_update.clone() {
            self.handle_movement_info_with_catalogs_like_cpp(
                area_trigger_catalogs,
                creature_spawn_catalogs,
                progression,
                player_grid_loader,
                Some(ClientOpcodes::MoveStop),
                move_update,
            )
            .await;
        }

        let target_guid = if req.target.unit.is_empty() {
            player_guid
        } else {
            req.target.unit
        };
        let request = RepresentedPendingSpellCastRequestLikeCpp {
            cast_id,
            spell_id: original_spell_id,
            casting_unit_guid: player_guid,
            target_guid,
            target_data: crate::spell_cast_adapter::retain_targets(req.target),
            spell_visual: wow_entities::SpellCastVisualLikeCpp {
                spell_visual_id: req.visual.spell_visual_id,
                script_visual_id: 0,
            },
            metadata: crate::session::SpellCastMetadata {
                from_client: true,
                misc: req.misc,
                // C++ `HandleCastSpellOpcode` copies the request trajectory
                // into `m_targets`; `SpellCastTargets::HasTraj()` then gates
                // `CAST_FLAG_ADJUST_MISSILE` in `Spell::SendSpellGo`.
                request_has_trajectory_like_cpp: req.has_trajectory_like_cpp,
                request_trajectory_pitch_like_cpp: req.trajectory_pitch_like_cpp,
                ..Default::default()
            },
        };
        if crate::player_cast::request(self, request) {
            self.tick_pending_spell_cast_request_with_generator_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
            )
            .await;
        }
    }

    #[cfg(test)]
    pub async fn handle_cast_spell(&mut self, pkt: wow_packet::WorldPacket) {
        let area_trigger_catalogs = self.area_trigger_catalogs_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let progression = self.progression_catalogs_for_test_like_cpp();
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_cast_spell_with_catalogs_like_cpp(
            &area_trigger_catalogs,
            &creature_spawn_catalogs,
            &progression,
            &crate::session::SessionHandlerCatalogsLikeCpp::default().player_grid_loader,
            generators.item.as_ref(),
            pkt,
        )
        .await;
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
            .resolved_inventory_item_object_like_cpp(item.guid)
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
                .resolved_inventory_item_object_like_cpp(item.guid)
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
        self.open_active_item_loot_view_like_cpp(player_guid, item.guid)
            .await;
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

    pub(crate) async fn open_active_item_loot_view_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
    ) {
        if self.has_active_non_item_loot_views_like_cpp() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.add_active_loot_view_owner_like_cpp(item_guid);
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
        let Some(port) = self.stored_item_persistence_port_like_cpp() else {
            return WrappedGiftLoad::Unavailable;
        };
        match port
            .load_wrapped_gift_like_cpp(item_guid.counter() as u64)
            .await
        {
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Loaded(row) => {
                WrappedGiftLoad::Found(WrappedGiftRow {
                    entry: row.entry,
                    flags: row.flags,
                })
            }
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Missing => WrappedGiftLoad::Missing,
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(item_guid = item_guid.counter(), error = %reason, "failed to load wrapped gift row");
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
        let Some(port) = self.stored_item_persistence_port_like_cpp() else {
            return;
        };

        let runtime_item = self.resolved_inventory_item_object_like_cpp(item_guid);
        let should_expire_refund = runtime_item
            .as_ref()
            .is_some_and(|item_object| item_object.is_refundable());

        match port
            .destroy_inventory_item_like_cpp(
                wow_persistence::InventoryItemDestroyPersistenceRequestLikeCpp {
                    owner_guid: player_guid.counter() as u64,
                    item_guid: item.db_guid,
                    expire_refund: should_expire_refund,
                },
            )
            .await
        {
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                warn!(item_guid = item_guid.counter(), error = %reason, "failed to destroy stale wrapped gift");
                return;
            }
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
        let Some(port) = self.stored_item_persistence_port_like_cpp() else {
            return;
        };
        let outcome = port
            .open_wrapped_gift_like_cpp(wow_persistence::WrappedGiftOpenPersistenceRequestLikeCpp {
                item_guid: item_guid.counter() as u64,
                entry,
                flags,
                durability,
            })
            .await;
        if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
        | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
        {
            warn!(item_guid = item_guid.counter(), entry, error = %reason, "failed to persist wrapped gift open");
        }
    }

    async fn load_item_template_addon_money_loot_like_cpp(&self, item_entry: u32) -> (u32, u32) {
        let Some(port) = self.item_template_addon_catalog_persistence_port_like_cpp() else {
            return (0, 0);
        };

        match port
            .load_item_template_addon_money_like_cpp(
                wow_persistence::ItemTemplateAddonCatalogRequestLikeCpp { item_entry },
            )
            .await
        {
            wow_persistence::ItemTemplateAddonMoneyOutcomeLikeCpp::Found(row) => {
                match (row.min_money, row.max_money) {
                    (Some(min_money), Some(max_money)) => {
                        if min_money > max_money {
                            // ObjectMgr::LoadItemTemplateAddon swaps invalid item
                            // bounds before storing the template. GameObject addon
                            // money deliberately does not share this normalization.
                            warn!(
                                item_entry,
                                min_money,
                                max_money,
                                "minimum item money loot exceeded maximum; swapping like C++"
                            );
                        }
                        normalize_item_money_loot_bounds_like_cpp(min_money, max_money)
                    }
                    _ => {
                        warn!(
                            item_entry,
                            "failed to decode item_template_addon money loot as C++ uint32 columns"
                        );
                        (0, 0)
                    }
                }
            }
            wow_persistence::ItemTemplateAddonMoneyOutcomeLikeCpp::Missing => (0, 0),
            wow_persistence::ItemTemplateAddonMoneyOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    item_entry,
                    error = %reason,
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
        let Some(port) = self.item_template_addon_catalog_persistence_port_like_cpp() else {
            return ItemTemplateAddonLootMetadataLikeCpp::default();
        };

        match port
            .load_item_template_addon_loot_metadata_like_cpp(
                wow_persistence::ItemTemplateAddonCatalogRequestLikeCpp { item_entry },
            )
            .await
        {
            wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Found(row) => {
                ItemTemplateAddonLootMetadataLikeCpp {
                    flags_cu: row.flags_cu,
                    quest_log_item_id: row.quest_log_item_id,
                }
            }
            wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Missing => {
                ItemTemplateAddonLootMetadataLikeCpp::default()
            }
            wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    item_entry,
                    error = %reason,
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
        let Some(port) = self.loot_template_catalog_persistence_port_like_cpp() else {
            return Vec::new();
        };

        let persistence_table = match table {
            LootTemplateTable::Item => wow_persistence::LootTemplateTablePersistenceLikeCpp::Item,
            LootTemplateTable::Reference => {
                wow_persistence::LootTemplateTablePersistenceLikeCpp::Reference
            }
        };
        let persistence_rows = match port
            .load_loot_template_rows_like_cpp(persistence_table, entry)
            .await
        {
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    entry,
                    table = table.name(),
                    error = %reason,
                    "failed to load loot template rows"
                );
                return Vec::new();
            }
        };

        let mut rows = persistence_rows
            .into_iter()
            .map(|row| LootTemplateRow {
                item_id: row.item_id,
                reference: row.reference,
                chance: row.chance,
                needs_quest: row.needs_quest,
                loot_mode: row.loot_mode,
                group_id: row.group_id,
                min_count: row.min_count,
                max_count: row.max_count,
                conditions: Vec::new(),
            })
            .collect::<Vec<_>>();

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
        let Some(port) = self.loot_template_catalog_persistence_port_like_cpp() else {
            return Vec::new();
        };

        let rows = match port
            .load_loot_condition_rows_like_cpp(source_type, source_group, source_entry)
            .await
        {
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::LootTemplateCatalogOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    source_type,
                    source_group,
                    source_entry,
                    error = %reason,
                    "failed to load loot template condition rows"
                );
                return Vec::new();
            }
        };

        let mut conditions = Vec::new();
        for row in rows {
            let condition = LootConditionRowLikeCpp {
                else_group: row.else_group,
                condition_type_or_reference: row.condition_type_or_reference,
                condition_target: row.condition_target,
                value1: row.value1,
                value2: row.value2,
                value3: row.value3,
                string_value1: row.string_value1,
                negative: row.negative,
                script_name: row.script_name,
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

        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let start_quest_id = self.item_template_start_quest_id(item_id).unwrap_or(0);
        let has_non_none_start_quest_status =
            u32::try_from(start_quest_id).ok().is_some_and(|quest_id| {
                quest_id != 0
                    && (quests.statuses.contains_key(&quest_id)
                        || quests.rewarded_quest_ids.contains(&quest_id))
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
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };

        quests.statuses.values().any(|status| {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
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
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };

        quests.statuses.values().any(|status| {
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
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
                        .is_some_and(|count| count < max_allowed_count)
                })
        })
    }

    fn direct_inventory_item_count_like_cpp_representable(&self, item_id: u32) -> Option<u32> {
        Some(
            self.resolved_inventory_items_like_cpp()?
                .values()
                .filter(|inventory_item| inventory_item.entry_id == item_id)
                .filter_map(|inventory_item| {
                    self.resolved_inventory_item_object_like_cpp(inventory_item.guid)
                })
                .filter(|item| !item.is_in_trade())
                .fold(0_u32, |total, item| total.saturating_add(item.count())),
        )
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
        let quests = self.player_quest_gameplay_snapshot_like_cpp()?;
        match condition.condition_type_or_reference {
            0 => Some(true),
            2 => {
                if condition.value3 != 0 {
                    return None;
                }
                Some(
                    self.direct_inventory_item_count_like_cpp_representable(condition.value1)?
                        >= condition.value2,
                )
            }
            6 => Some(
                player_team_for_race_cpp_representable(self.player_race_like_cpp())
                    == condition.value1,
            ),
            8 => Some(quests.rewarded_quest_ids.contains(&condition.value1)),
            9 => Some(
                quests
                    .statuses
                    .get(&condition.value1)
                    .is_some_and(|status| status.status == QUEST_STATUS_INCOMPLETE_LIKE_CPP),
            ),
            14 => Some(
                !quests.statuses.contains_key(&condition.value1)
                    && !quests.rewarded_quest_ids.contains(&condition.value1),
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
                quests
                    .statuses
                    .get(&condition.value1)
                    .is_some_and(|status| status.status == 2)
                    && !quests.rewarded_quest_ids.contains(&condition.value1),
            ),
            47 => Some(
                player_quest_status_mask_like_cpp(
                    quests
                        .statuses
                        .get(&condition.value1)
                        .map(|status| status.status),
                    quests.rewarded_quest_ids.contains(&condition.value1),
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
        let quests = self.player_quest_gameplay_snapshot_like_cpp()?;

        for status in quests.statuses.values() {
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
        let port = self.stored_item_persistence_port_like_cpp()?;
        match port
            .load_stored_item_money_like_cpp(item_guid.counter() as u64)
            .await
        {
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Loaded(money) => Some(money),
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Missing => None,
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    item_guid = item_guid.counter(),
                    error = %reason,
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
        let port = self.stored_item_persistence_port_like_cpp()?;
        let rows = match port
            .load_stored_item_loot_like_cpp(item_guid.counter() as u64)
            .await
        {
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Loaded(rows) => rows,
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Missing => return None,
            wow_persistence::StoredItemLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    item_guid = item_guid.counter(),
                    error = %reason,
                    "failed to load stored item loot rows"
                );
                return None;
            }
        };

        let mut items = Vec::new();
        for row in rows {
            if stored_item_row_can_load_like_cpp_representable(
                row.item_id,
                row.count,
                row.item_index,
                row.blocked,
                row.needs_quest,
                row.random_properties_id,
                row.random_properties_seed,
                row.context,
                self.item_storage_template(row.item_id).is_some(),
            ) {
                items.push(LootEntry {
                    loot_list_id: row.item_index as u8,
                    item_id: row.item_id,
                    quantity: row.count,
                    random_properties_id: row.random_properties_id,
                    random_properties_seed: row.random_properties_seed,
                    item_context: row.context,
                    flags: LootEntryFlags {
                        follow_loot_rules: row.follow_loot_rules,
                        freeforall: row.free_for_all,
                        blocked: row.blocked,
                        counted: row.counted,
                        under_threshold: row.under_threshold,
                        needs_quest: row.needs_quest,
                    },
                    allowed_looters: Vec::new(),
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                });
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
        let Some(port) = self.stored_item_persistence_port_like_cpp() else {
            return;
        };
        let mut rows = Vec::new();
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

            rows.push(wow_persistence::StoredItemLootPersistenceRowLikeCpp {
                item_id: item.item_id,
                count: item.quantity,
                item_index: u32::from(item.loot_list_id),
                follow_loot_rules: item.flags.follow_loot_rules,
                free_for_all: item.flags.freeforall,
                blocked: item.flags.blocked,
                counted: item.flags.counted,
                under_threshold: item.flags.under_threshold,
                needs_quest: item.flags.needs_quest,
                random_properties_id: item.random_properties_id,
                random_properties_seed: item.random_properties_seed,
                context: item.item_context,
            });
        }
        let outcome = port
            .save_stored_item_loot_like_cpp(wow_persistence::StoredItemLootSaveRequestLikeCpp {
                item_guid: item_guid.counter() as u64,
                money,
                items: rows,
            })
            .await;
        if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
        | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
        {
            warn!(
                item_guid = item_guid.counter(),
                money,
                error = %reason,
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
    pub async fn handle_spell_click_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        mut pkt: wow_packet::WorldPacket,
    ) {
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
            .execute_represented_spell_click_plan_with_generator_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
                spell_click.unit_guid,
                &plan,
            )
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

    #[cfg(test)]
    pub async fn handle_spell_click(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_spell_click_with_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            pkt,
        )
        .await;
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

        self.cancel_client_cast_request_like_cpp(
            (request.spell_id != 0).then_some(request.spell_id as i32),
        );
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
        if self.player_moved_unit_guid_like_cpp() != Some(request.target_guid) {
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
    pub async fn handle_self_res_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &crate::session::CreatureSpawnCatalogsLikeCpp,
        mut pkt: wow_packet::WorldPacket,
    ) {
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
        // C++ `HandleSelfResOpcode` uses
        // `CastSpell(_player, SpellID, GetMap()->GetDifficultyID())`, whose
        // trigger flags are TRIGGERED_NONE: not a triggered cast, and the
        // global cooldown applies.
        if self
            .execute_server_triggered_spell_like_cpp(
                item_guid_generator,
                creature_spawn_catalogs,
                request.spell_id,
                player_guid,
                crate::session::SpellCastMetadata::default(),
            )
            .await
            .is_ok()
        {
            self.remove_represented_self_res_spell_like_cpp(request.spell_id);
        }
    }

    #[cfg(test)]
    pub async fn handle_self_res(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.handle_self_res_with_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            pkt,
        )
        .await;
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

    pub(crate) fn is_spell_disabled_for_player_like_cpp(&self, spell_id: i32) -> bool {
        let Some(disable_mgr) = self.disable_mgr() else {
            return false;
        };

        let map_id = u32::from(self.player_map_id_like_cpp());
        let Some((_, area_id)) = self.player_zone_area_like_cpp() else {
            return true;
        };
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
#[path = "spell/tests/mod.rs"]
mod tests;
