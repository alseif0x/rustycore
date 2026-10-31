// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Inventory composition for the reagent return in `Player::RemovePet`.

use wow_constants::{InventoryResult, InventoryType, ItemContext, ItemFieldFlags};
use wow_core::{ObjectGuid, ObjectGuidGenerator};
use wow_entities::{PlayerInventoryItem, SendNewItemArgs, SendNewItemTemplateRef};
use wow_packet::packets::update::{ItemCreateData, ItemEnchantmentValuesUpdate, UpdateObject};
use wow_world_entities::CreatureSpawnCatalogsLikeCpp;
use wow_world_inventory::VendorSessionHostLikeCpp;
use wow_world_lifecycle::PlayerMoneyCommitCancellationFenceLikeCpp;
use wow_world_quest::{QuestItemStorageHostLikeCpp, QuestSessionHostLikeCpp};
use wow_world_spell::PlayerCastHostLikeCpp;

use crate::TrainerSessionHostLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetReagentReturnOutcome {
    InventoryRejected(InventoryResult),
    BoundQuestObjective,
    Stored(ObjectGuid),
    Empty,
}

/// C++ Player.cpp:20883–20903,11166–11352 at the pinned target revision.
///
/// The enclosing pet removal owns the cancellation/retirement fence. An error
/// after a quest or item callback is not permission to replay this grant.
/// Items stay dirty in the canonical Player until its normal full save.
/// Unsupported item publication metadata is rejected before any mutation.
pub async fn return_pet_reagent_like_cpp<H>(
    host: &mut H,
    fence: &mut PlayerMoneyCommitCancellationFenceLikeCpp,
    item_guids: &ObjectGuidGenerator,
    creature_spawns: &CreatureSpawnCatalogsLikeCpp,
    entry_id: u32,
    count: u32,
) -> Result<PetReagentReturnOutcome, &'static str>
where
    H: QuestItemStorageHostLikeCpp
        + TrainerSessionHostLikeCpp
        + PlayerCastHostLikeCpp
        + VendorSessionHostLikeCpp,
{
    if count == 0 {
        return Ok(PetReagentReturnOutcome::Empty);
    }
    let owner = host
        .player_guid()
        .ok_or("pet reagent Player is unavailable")?;
    if host.core().with_owned_player_like_cpp(|player| {
        player
            .inventory_runtime_like_cpp()
            .equipment_inventory_authority_complete_like_cpp()
    }) != Some(true)
    {
        return Err("pet reagent inventory authority is unavailable");
    }
    let (result, destinations, _) =
        QuestItemStorageHostLikeCpp::plan_store_new_direct_inventory_item(host, entry_id, count)
            .ok_or("pet reagent storage admission is unavailable")?;
    if result != InventoryResult::Ok {
        // RemovePet deliberately sends no equip error for a rejected refund.
        return Ok(PetReagentReturnOutcome::InventoryRejected(result));
    }
    if destinations.is_empty() {
        return Ok(PetReagentReturnOutcome::Empty);
    }
    let template = host
        .hub_ref()
        .catalogs
        .item_storage_template(entry_id)
        .ok_or("pet reagent item template is unavailable")?;
    if template.inventory_type != InventoryType::NonEquip || template.container_slots != 0 {
        return Err("pet reagent requires unsupported equipment or container creation");
    }
    let expiration = host
        .hub_ref()
        .catalogs
        .items
        .stats_store
        .as_ref()
        .and_then(|store| store.duration_in_inventory(entry_id))
        .ok_or("pet reagent duration metadata is unavailable")?;
    let effects = host
        .hub_ref()
        .catalogs
        .items
        .effect_store
        .clone()
        .ok_or("pet reagent ItemEffect metadata is unavailable")?;
    if expiration != 0
        || effects
            .item_effects_for_item_id_like_cpp(entry_id)
            .iter()
            .any(|effect| effect.legacy_slot_index < 5 && effect.charges != 0)
    {
        return Err("pet reagent requires unsupported timed or charged item CREATE fields");
    }
    if destinations
        .iter()
        .any(|dest| (dest.pos >> 8) as u8 != wow_entities::INVENTORY_SLOT_BAG_0 as u8)
    {
        return Err("pet reagent bag placement requires deferred container publication");
    }
    if host.hub_ref().catalogs.quests.store.is_none()
        || !QuestSessionHostLikeCpp::player_quest_gameplay_snapshot_like_cpp(host)
            .is_some_and(|state| state.status_authority_complete_like_cpp())
    {
        return Err("pet reagent quest authority is unavailable");
    }
    let quest_log_item_id = reagent_quest_log_item_id(host, entry_id).await?;
    let total = destinations
        .iter()
        .try_fold(0u32, |sum, dest| sum.checked_add(dest.count))
        .ok_or("pet reagent destination count overflow")?;
    if total != count {
        return Err("pet reagent destination count does not match admission");
    }
    // Admission, item catalogs, addon metadata and count checks above are
    // read-only. From this callback onward quest state may have changed, so
    // cancellation must keep the enclosing pet-removal operation fenced.
    fence.arm_like_cpp();
    if let Some(bound) =
        wow_world_quest::apply_quest_source_item_bound_objective_preflight_with_generator_like_cpp(
            host,
            item_guids,
            entry_id,
            quest_log_item_id,
            total,
        )
        .await
    {
        if bound.no_grant {
            return Ok(PetReagentReturnOutcome::BoundQuestObjective);
        }
    }

    let mut last = None;
    for destination in destinations {
        if host.player_guid() != Some(owner) {
            return Err("pet reagent Player changed during a callback");
        }
        let bag = (destination.pos >> 8) as u8;
        let slot = destination.pos as u8;
        let in_world = host
            .core()
            .with_owned_player_like_cpp(|player| player.unit().world().object().is_in_world())
            .ok_or("pet reagent Player disappeared")?;
        let map_id = host.core().player_map_id_like_cpp();
        let guid = if let Some(existing) =
            QuestItemStorageHostLikeCpp::get_inventory_item_by_pos(host, bag, slot)
        {
            let (new_count, previous_flags, flags) = host
                .core()
                .with_owned_player_mut_like_cpp(|player| {
                    if player.guid() != owner {
                        return Err("pet reagent Player changed during a callback");
                    }
                    let inventory = player.inventory_runtime_mut_like_cpp();
                    if inventory.inventory_items().get(&slot).map(|item| item.guid)
                        != Some(existing.guid)
                    {
                        return Err("pet reagent destination changed during a callback");
                    }
                    let item = inventory
                        .item_objects_mut()
                        .get_mut(&existing.guid)
                        .ok_or("pet reagent stack object is unavailable")?;
                    if item.object().entry() != entry_id || item.owner_guid() != owner {
                        return Err("pet reagent destination changed during a callback");
                    }
                    let new_count = item
                        .count()
                        .checked_add(destination.count)
                        .filter(|count| *count <= template.max_stack_size)
                        .ok_or("pet reagent stack no longer has admitted capacity")?;
                    let previous_flags = item.item_flags_bits();
                    item.set_count(new_count);
                    item.set_bonding(template.bonding);
                    item.bind_if_stored(wow_entities::is_bag_pos(destination.pos));
                    Ok((new_count, previous_flags, item.item_flags_bits()))
                })
                .ok_or("pet reagent stack owner disappeared")??;
            if in_world {
                let update = if flags == previous_flags {
                    UpdateObject::item_stack_count_update(existing.guid, map_id, new_count)
                } else {
                    UpdateObject::item_stack_count_and_flags_update(
                        existing.guid,
                        map_id,
                        new_count,
                        flags,
                    )
                };
                host.core().send_packet(&update);
            }
            existing.guid
        } else {
            let (db_guid, guid) = host
                .core()
                .allocate_item_instance_guids_with_generator_like_cpp(item_guids, 1)
                .and_then(|mut ids| ids.pop())
                .ok_or("pet reagent item GUID allocation failed")?;
            let durability = host
                .hub_ref()
                .catalogs
                .item_template_max_durability(entry_id);
            let mut item = QuestItemStorageHostLikeCpp::make_inventory_item_object(
                host,
                guid,
                entry_id,
                owner,
                destination.count,
                durability,
                ItemContext::None,
                slot,
            );
            item.set_bonding(template.bonding);
            item.set_item_flag(ItemFieldFlags::NEW_ITEM);
            item.bind_if_stored(wow_entities::is_bag_pos(destination.pos));
            if in_world {
                item.object_mut().add_to_world();
            }
            let flags = item.item_flags_bits();
            host.core()
                .with_owned_player_mut_like_cpp(|player| {
                    if player.guid() != owner {
                        return Err("pet reagent Player changed during a callback");
                    }
                    let inventory = player.inventory_runtime_like_cpp();
                    if inventory.inventory_items().contains_key(&slot) {
                        return Err("pet reagent slot unexpectedly occupied");
                    }
                    if inventory.item_objects().contains_key(&guid) {
                        return Err("pet reagent GUID unexpectedly occupied");
                    }
                    let inventory = player.inventory_runtime_mut_like_cpp();
                    let _ = inventory.store_item_in_slot_like_cpp(
                        slot,
                        PlayerInventoryItem {
                            guid,
                            entry_id,
                            db_guid,
                            inventory_type: None,
                        },
                    );
                    let _ = inventory.store_item_object_like_cpp(item);
                    player.set_inv_slot(slot as usize, guid);
                    Ok(())
                })
                .ok_or("pet reagent inventory field owner disappeared")??;
            if in_world {
                host.core().send_packet(&UpdateObject::create_stored_items(
                    vec![ItemCreateData {
                        item_guid: guid,
                        entry_id: entry_id as i32,
                        owner_guid: owner,
                        contained_in: owner,
                        stack_count: destination.count,
                        dynamic_flags: flags,
                        durability,
                        max_durability: durability,
                        random_properties_seed: 0,
                        random_properties_id: 0,
                        enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                        gems: Vec::new(),
                        context: ItemContext::None as u8,
                        container_slots: 0,
                        container_item_guids: [ObjectGuid::EMPTY; 36],
                    }],
                    map_id,
                ));
            }
            guid
        };
        crate::apply_inventory_item_store_side_effects_like_cpp(host, bag, slot, guid);
        wow_world_spell::apply_inventory_item_obtain_spells_with_generator_like_cpp(
            host,
            item_guids,
            creature_spawns,
            guid,
        )
        .await?;
        last = Some(guid);
    }
    wow_world_quest::apply_quest_source_item_added_non_bound_objective_progress_with_generator_like_cpp(
        host, item_guids, entry_id, quest_log_item_id, total,
    ).await;
    // ObtainAnyItem/AcquireItem remain the existing PORT_PLAN L18 criteria gap.
    // Context NONE installs no bonus lists; Item.cpp:2169 initializes HasFixedLevel=false.
    let guid = last.ok_or("pet reagent stored no item")?;
    let item = QuestItemStorageHostLikeCpp::resolved_inventory_item_object_like_cpp(host, guid)
        .ok_or("pet reagent item disappeared during a callback")?;
    for update in VendorSessionHostLikeCpp::on_item_added_to_collection_like_cpp(host, &item) {
        host.core().send_player_values_update_like_cpp(&update);
    }
    if host
        .core()
        .with_owned_player_like_cpp(|player| player.unit().world().object().is_in_world())
        == Some(true)
    {
        let quantity_in_inventory =
            QuestSessionHostLikeCpp::represented_inventory_item_counts_like_cpp(host)
                .ok_or("pet reagent count authority is unavailable")?
                .get(&entry_id)
                .copied()
                .unwrap_or(0);
        let plan = host
            .core()
            .with_owned_player_like_cpp(|player| {
                player.send_new_item_plan(
                    player
                        .inventory_runtime_like_cpp()
                        .item_objects()
                        .get(&guid),
                    SendNewItemTemplateRef::new(quest_log_item_id, false),
                    SendNewItemArgs {
                        quantity_in_inventory,
                        ..SendNewItemArgs::new(count, true, false)
                    },
                )
            })
            .flatten()
            .ok_or("pet reagent notification owner or item disappeared")?;
        QuestItemStorageHostLikeCpp::send_new_item_plan(host, &plan);
    }
    Ok(PetReagentReturnOutcome::Stored(guid))
}

async fn reagent_quest_log_item_id<H: QuestItemStorageHostLikeCpp>(
    host: &mut H,
    entry: u32,
) -> Result<u32, &'static str> {
    if let Some(id) = host
        .hub_ref()
        .catalogs
        .item_template_addon_quest_log_item_id_like_cpp(entry)
    {
        return Ok(id);
    }
    let port = QuestSessionHostLikeCpp::lifecycle(host)
        .item_template_addon_catalog_persistence_port_like_cpp()
        .cloned()
        .ok_or("pet reagent item addon authority is unavailable")?;
    let id = match port
        .load_item_template_addon_loot_metadata_like_cpp(
            wow_persistence::ItemTemplateAddonCatalogRequestLikeCpp { item_entry: entry },
        )
        .await
    {
        wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Found(row) => {
            u32::try_from(row.quest_log_item_id)
                .map_err(|_| "invalid pet reagent quest item key")?
        }
        wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Missing => 0,
        wow_persistence::ItemTemplateAddonLootMetadataOutcomeLikeCpp::Failed { .. } => {
            return Err("pet reagent item addon load failed");
        }
    };
    host.catalogs_mut()
        .cache_item_template_addon_quest_log_item_id_like_cpp(entry, id);
    Ok(id)
}
