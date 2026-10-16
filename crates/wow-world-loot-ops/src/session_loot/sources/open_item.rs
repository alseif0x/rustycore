// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item containers: `CMSG_OPEN_ITEM`, the wrapped gift and the item loot
//! template generation (C++ `WorldSession::HandleOpenItemOpcode`,
//! `SpellHandler.cpp:66`; #1363 slice 4c, moved from the World
//! `handlers/spell/ops_1.rs`).

use super::*;

use wow_entities::INVENTORY_SLOT_BAG_0;
use wow_packet::ClientPacket;
use wow_packet::packets::item::{ItemExpirePurchaseRefund, ItemInstance};
use wow_packet::packets::loot::{LOOT_TYPE_ITEM_LIKE_CPP, LootEntry, LootItemData, LootResponse};
use wow_packet::packets::spell::OpenItem;
use wow_world_lifecycle::loot_template_rules::{
    ItemTemplateAddonLootMetadataLikeCpp, LOOT_MODE_DEFAULT_LIKE_CPP, LootTemplateFrame,
    MAX_LOOT_REFERENCE_FRAMES_LIKE_CPP, add_loot_template_row_item_like_cpp,
    loot_template_plain_row_can_roll_like_cpp, loot_template_reference_row_can_roll_like_cpp,
    normalize_item_money_loot_bounds_like_cpp, referenced_loot_max_count_like_cpp,
    roll_chance_with_rate_like_cpp, roll_group_loot_row_like_cpp,
};
use wow_world_lifecycle::{LootTemplateRow, LootTemplateTable, WrappedGiftLoad};

/// The World operations the moved item-container bodies call beyond
/// [`LootSessionHostLikeCpp`] (#1363 slice 4c): each is the same-named
/// `WorldSession` operation, called at the point the body called it before the
/// move.
pub trait OpenItemHostLikeCpp: LootSessionHostLikeCpp {
    fn get_inventory_item_by_pos(&self, bag: u8, slot: u8) -> Option<InventoryItem>;
    fn apply_wrapped_gift_row_to_runtime_item_like_cpp(
        &mut self,
        bag: u8,
        item_guid: ObjectGuid,
        slot: u8,
        entry: u32,
        flags: u32,
    ) -> Option<u32>;
    /// `crate::session::split_inventory_mut` (#1363 slice 12c).
    fn split_inventory_mut(
        &mut self,
    ) -> (
        &mut wow_world_inventory::InventoryState,
        wow_world_core::session::HubMut<'_>,
    );
    fn send_stat_update(&mut self) -> bool;
}

/// Handle `CMSG_OPEN_ITEM`.
///
/// This ports Trinity's initial validation and fails closed until item loot
/// storage/generation is represented in Rust.
pub async fn handle_open_item<H: OpenItemHostLikeCpp>(
    host: &mut H,
    mut pkt: wow_packet::WorldPacket,
) {
    let open = match OpenItem::read(&mut pkt) {
        Ok(open) => open,
        Err(e) => {
            warn!(
                account = host.core().account_id,
                "Failed to parse CMSG_OPEN_ITEM: {e}"
            );
            return;
        }
    };

    debug!(
        account = host.core().account_id,
        slot = open.slot,
        pack_slot = open.pack_slot,
        "CMSG_OPEN_ITEM"
    );

    let Some(item) = host.get_inventory_item_by_pos(open.slot, open.pack_slot) else {
        host.core()
            .send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
        return;
    };

    let Some(flags) = host.hub_ref().catalogs.item_template_flags(item.entry_id) else {
        host.core()
            .send_equip_error(InventoryResult::ItemNotFound, Some(item.guid), None, 0, 0);
        return;
    };

    let is_wrapped = host
        .resolved_inventory_item_object_like_cpp(item.guid)
        .is_some_and(|runtime_item| runtime_item.is_wrapped());

    if !flags.contains(ItemFlags::HAS_LOOT) && !is_wrapped {
        host.core().send_equip_error(
            InventoryResult::ClientLockedOut,
            Some(item.guid),
            None,
            0,
            0,
        );
        return;
    }

    let lock_id = host
        .hub_ref()
        .catalogs
        .item_template_lock_id(item.entry_id)
        .unwrap_or(0);
    if lock_id != 0 {
        if !host
            .hub_ref()
            .catalogs
            .lock_entry_exists_like_cpp(u32::from(lock_id))
        {
            host.core()
                .send_equip_error(InventoryResult::ItemLocked, Some(item.guid), None, 0, 0);
            return;
        }

        let item_is_locked = host
            .resolved_inventory_item_object_like_cpp(item.guid)
            .map_or(true, |item_object| item_object.is_locked());
        if item_is_locked {
            host.core()
                .send_equip_error(InventoryResult::ItemLocked, Some(item.guid), None, 0, 0);
            return;
        }
    }

    if is_wrapped {
        open_wrapped_gift_like_cpp(host, open.slot, open.pack_slot, item.guid).await;
        return;
    }

    let Some(player_guid) = host.player_guid() else {
        return;
    };

    if !host.loot().cached_loot_contains_owner_like_cpp(item.guid) {
        let stored_money = host
            .lifecycle()
            .load_stored_item_money_like_cpp(item.guid)
            .await;
        let stored_items = {
            let (s, h) = (host.lifecycle(), host.hub_ref());
            s.load_stored_item_items_like_cpp(h, item.guid).await
        };
        let loaded_stored_loot = stored_money.is_some() || stored_items.is_some();
        let (coins, mut items) = if loaded_stored_loot {
            (stored_money.unwrap_or(0), stored_items.unwrap_or_default())
        } else {
            let coins = {
                let (min_money, max_money) =
                    load_item_template_addon_money_loot_like_cpp(host, item.entry_id).await;
                host.represented_money_loot_with_rate_like_cpp(
                    min_money,
                    max_money,
                    host.loot_drop_rates_like_cpp().money,
                )
            };
            let items = generate_item_loot_template_entries_like_cpp(host, item.entry_id).await;
            (coins, items)
        };
        for entry in &mut items {
            entry.add_allowed_looter_like_cpp(player_guid);
        }
        if !loaded_stored_loot && (coins > 0 || !items.is_empty()) {
            save_new_stored_item_loot_like_cpp(host, item.guid, coins, &items).await;
        }

        host.loot_mut().insert_cached_loot_for_owner_like_cpp(
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

    let _ = host.apply_inventory_item_object_updates_like_cpp(
        item.guid,
        &[wow_entities::ItemObjectUpdateLikeCpp::SetLootGenerated(
            true,
        )],
    );

    let Some(loot) = host.loot().cached_loot_for_owner_like_cpp(item.guid) else {
        host.core().send_equip_error(
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
    open_active_item_loot_view_like_cpp(host, player_guid, item.guid).await;
    host.send_packet(&LootResponse {
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

pub async fn open_active_item_loot_view_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    player_guid: ObjectGuid,
    item_guid: ObjectGuid,
) {
    if host.has_active_non_item_loot_views_like_cpp() {
        host.do_loot_release_all_like_cpp(player_guid).await;
    }
    host.loot_mut()
        .add_active_loot_view_owner_like_cpp(item_guid);
}

pub(crate) async fn open_wrapped_gift_like_cpp<H: OpenItemHostLikeCpp>(
    host: &mut H,
    bag: u8,
    slot: u8,
    item_guid: ObjectGuid,
) {
    let gift = match host
        .lifecycle()
        .load_wrapped_gift_row_like_cpp(item_guid)
        .await
    {
        WrappedGiftLoad::Found(gift) => gift,
        WrappedGiftLoad::Missing => {
            destroy_stale_wrapped_gift_like_cpp(host, bag, slot, item_guid).await;
            return;
        }
        WrappedGiftLoad::Unavailable => return,
    };

    let Some(durability) = host.apply_wrapped_gift_row_to_runtime_item_like_cpp(
        bag, item_guid, slot, gift.entry, gift.flags,
    ) else {
        return;
    };

    host.lifecycle()
        .persist_wrapped_gift_open_like_cpp(item_guid, gift.entry, gift.flags, durability)
        .await;
}

pub(crate) async fn destroy_stale_wrapped_gift_like_cpp<H: OpenItemHostLikeCpp>(
    host: &mut H,
    bag: u8,
    slot: u8,
    item_guid: ObjectGuid,
) {
    let Some(player_guid) = host.player_guid() else {
        return;
    };
    let Some(item) = host.get_inventory_item_by_pos(bag, slot) else {
        return;
    };
    if item.guid != item_guid {
        return;
    }
    let Some(port) = host.lifecycle().stored_item_persistence_port_like_cpp() else {
        return;
    };

    let runtime_item = host.resolved_inventory_item_object_like_cpp(item_guid);
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

    remove_fully_looted_runtime_item(host, bag, slot, item.guid);

    if should_expire_refund {
        host.send_packet(&ItemExpirePurchaseRefund {
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

        host.send_player_values_update_from_entity_bridge(
            &[(slot, ObjectGuid::EMPTY)],
            &visible_item_changes,
            &virtual_item_changes,
            &[],
            None,
        );

        if slot < 19 {
            host.send_stat_update();
        }
    }
}

pub(crate) async fn generate_item_loot_template_entries_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    item_entry: u32,
) -> Vec<LootEntry> {
    let mut loot_items = Vec::new();
    let mut frames = Vec::new();
    let rows = host
        .lifecycle()
        .load_loot_template_rows_like_cpp(LootTemplateTable::Item, item_entry)
        .await;
    let condition_references = host
        .lifecycle()
        .load_loot_template_condition_reference_rows_like_cpp(&rows)
        .await;
    frames.push(LootTemplateFrame {
        rows,
        condition_references,
        index: 0,
        group_id: 0,
        groups_enqueued: false,
    });

    let mut rng = host.core_mut().represented_runtime_subrng_like_cpp();
    let mut processed_frames = 0u32;
    while let Some(mut frame) = frames.pop() {
        if frame.group_id != 0 {
            let addon_metadata =
                load_item_template_addon_loot_metadata_for_rows_like_cpp(host, &frame.rows).await;
            if let Some(row) = roll_group_loot_row_like_cpp(
                &frame.rows,
                frame.group_id,
                |item_id| host.item_storage_template(item_id).is_some(),
                |row| {
                    let metadata = addon_metadata
                        .get(&row.item_id)
                        .copied()
                        .unwrap_or_default();
                    item_loot_allowed_for_player_like_cpp_representable(
                        host,
                        row.item_id,
                        row.needs_quest,
                        metadata,
                        &row.conditions,
                        &frame.condition_references,
                    )
                },
                |item_id| host.item_drop_rate_like_cpp(item_id),
                &mut rng,
            ) {
                let metadata = addon_metadata
                    .get(&row.item_id)
                    .copied()
                    .unwrap_or_default();
                let flags = loot_entry_flags_for_row_like_cpp(host, &row, metadata);
                add_loot_template_row_item_like_cpp(
                    &mut loot_items,
                    &row,
                    flags,
                    |item_id| {
                        host.item_storage_template(item_id)
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
                    host.loot_drop_rates_like_cpp().item_referenced,
                    &mut rng,
                )
            {
                continue;
            }

            let reference_rows = host
                .lifecycle()
                .load_loot_template_rows_like_cpp(LootTemplateTable::Reference, row.reference)
                .await;
            let reference_condition_references = host
                .lifecycle()
                .load_loot_template_condition_reference_rows_like_cpp(&reference_rows)
                .await;
            let max_count = referenced_loot_max_count_like_cpp(
                row.max_count,
                host.loot_drop_rates_like_cpp().item_referenced_amount,
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

        let addon_metadata =
            load_item_template_addon_loot_metadata_like_cpp(host, row.item_id).await;
        if !loot_template_plain_row_can_roll_like_cpp(
            row.item_id,
            row.chance,
            row.needs_quest,
            row.loot_mode,
            row.min_count,
            row.max_count,
            host.item_storage_template(row.item_id).is_some(),
            item_loot_allowed_for_player_like_cpp_representable(
                host,
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
                host.item_drop_rate_like_cpp(row.item_id),
                &mut rng,
            )
        {
            continue;
        }
        let flags = loot_entry_flags_for_row_like_cpp(host, &row, addon_metadata);
        add_loot_template_row_item_like_cpp(
            &mut loot_items,
            &row,
            flags,
            |item_id| {
                host.item_storage_template(item_id)
                    .map(|template| template.max_stack_size)
                    .unwrap_or(1)
            },
            &mut rng,
        );
    }

    loot_items
}

pub async fn load_item_template_addon_money_loot_like_cpp<H: LootSessionHostLikeCpp>(
    host: &H,
    item_entry: u32,
) -> (u32, u32) {
    let Some(port) = host.item_template_addon_catalog_persistence_port_like_cpp() else {
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

pub async fn load_item_template_addon_loot_metadata_like_cpp<H: LootSessionHostLikeCpp>(
    host: &H,
    item_entry: u32,
) -> ItemTemplateAddonLootMetadataLikeCpp {
    let Some(port) = host.item_template_addon_catalog_persistence_port_like_cpp() else {
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

pub(crate) async fn load_item_template_addon_loot_metadata_for_rows_like_cpp<
    H: LootSessionHostLikeCpp,
>(
    host: &H,
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
            load_item_template_addon_loot_metadata_like_cpp(host, item_id).await,
        );
    }

    metadata
}

// Moved from the World `session/player_items/items.rs` (#1363 slice 12c).
/// Remove a fully-looted runtime item after its DB rows were deleted.
pub fn remove_fully_looted_runtime_item<H: OpenItemHostLikeCpp>(
    host: &mut H,
    bag: u8,
    slot: u8,
    item_guid: ObjectGuid,
) {
    if bag == INVENTORY_SLOT_BAG_0
        && host
            .inventory()
            .resolved_inventory_item_like_cpp(host.hub_ref(), slot)
            .is_some_and(|item| item.guid == item_guid)
    {
        let (state, mut hub) = host.split_inventory_mut();
        state.remove_inventory_item_like_cpp(&mut hub, slot);
    }
    let (state, mut hub) = host.split_inventory_mut();
    state.remove_inventory_item_object(&mut hub, item_guid);
    host.sync_player_registry_state_like_cpp();
}
