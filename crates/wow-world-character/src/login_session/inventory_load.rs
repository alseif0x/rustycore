// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Restore the canonical inventory and item-create snapshot during login.

use crate::CharacterLoginHostLikeCpp;
use crate::login_session::world_entry::is_represented_bag_slot;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;
use wow_constants::{InventoryType, ItemContext, ItemFieldFlags, ItemUpdateState};
use wow_core::ObjectGuid;
use wow_entities::PlayerInventoryItem as InventoryItem;
use wow_entities::PlayerVoidStorageItemLikeCpp as RepresentedVoidStorageItemLikeCpp;
use wow_world_inventory::loaded_item_support::{
    apply_loaded_item_instance_fields_like_cpp, apply_loaded_item_storage_mutable_fields_like_cpp,
    loaded_item_effective_enchantments_like_cpp, loaded_item_enchantments_like_cpp,
    loaded_item_random_properties_like_cpp,
    loaded_item_slot_applies_equipped_enchantments_like_cpp,
    loaded_socketed_gem_create_updates_like_cpp, loaded_socketed_gems_like_cpp,
};
use wow_world_inventory::vendor::{LoadedItemRefundDecision, loaded_item_refund_decision};

mod carried_proof;
mod item_payload;

pub struct LoginInventorySnapshotLikeCpp {
    pub visible_items: [(i32, u16, u16); 19],
    pub inv_slots: [ObjectGuid; 141],
    pub item_creates: Vec<wow_packet::packets::update::ItemCreateData>,
    pub loaded_equipped_item_guids: Vec<ObjectGuid>,
    pub loaded_item_time_updates: Vec<wow_entities::PlayerItemTimeUpdate>,
    pub loaded_non_equipped_enchantment_updates: Vec<wow_entities::PlayerEnchantTimeUpdate>,
}

pub async fn load_inventory_for_login_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
    guid: ObjectGuid,
    realm_id: u16,
) -> Option<LoginInventorySnapshotLikeCpp> {
    // Load equipped items for visible display + inventory objects
    let mut visible_items = [(0i32, 0u16, 0u16); 19];
    let mut inv_slots = [ObjectGuid::EMPTY; 141];
    let mut item_creates: Vec<wow_packet::packets::update::ItemCreateData> = Vec::new();
    let mut login_bag_create_index_by_slot: HashMap<u8, usize> = HashMap::new();
    let mut loaded_inventory_item_guids: Vec<ObjectGuid> = Vec::new();
    let mut loaded_equipped_item_guids: Vec<ObjectGuid> = Vec::new();
    let loading_owner = host.hub_ref().core.player_handle_like_cpp;
    let mut equipment_loaded = false;
    let mut bags_loaded = false;
    let mut carried_rows_coherent = true;
    let mut item_save_inputs_complete = true;
    let mut item_repair_outcomes_applied = true;
    let mut tradeable_item_registry_complete = true;
    {
        let (state, mut hub) = host.split_inventory_mut();
        state.clear_inventory_items_and_objects_like_cpp(&mut hub)
    };
    if !host.clear_player_currencies_like_cpp() {
        warn!("canonical Player currency owner unavailable during login");
        return None;
    }
    {
        {
            let (state, mut hub) = host.split_inventory_mut();
            state.begin_player_equipment_inventory_authority_load_like_cpp(&mut hub)
        };
        let mut refund_cleanup_actions = Vec::new();
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentInventory {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentInventory(rows),
            ) => {
                equipment_loaded = true;
                for row in rows {
                    let slot = row.slot;
                    let item = row.item;
                    let item_entry = item.item_entry;
                    let item_db_guid = item.item_db_guid;
                    let item_count = item.count;
                    let item_context_value =
                        <ItemContext as num_traits::FromPrimitive>::from_u8(item.context);
                    let item_context_is_representable = item_context_value.is_some();
                    let item_context = item_context_value.unwrap_or(ItemContext::None);
                    let item_enchantment_values =
                        loaded_item_enchantments_like_cpp(&item.enchantments);
                    let item_enchantments_are_representable = item.enchantments.trim().is_empty()
                        || item_enchantment_values.is_some();
                    let item_gem_ids_are_representable =
                        item.gems.iter().all(|(item_id, _, _)| *item_id >= 0);
                    let storage_template =
                        host.hub_ref().catalogs.item_storage_template(item_entry);
                    let row_save_inputs_are_known = {
                        let catalogs = &host.hub_ref().catalogs;
                        item_payload::row_save_inputs_known_like_cpp(
                            &item,
                            item_context_is_representable,
                            item_enchantments_are_representable,
                            item_gem_ids_are_representable,
                            storage_template.is_some(),
                            catalogs,
                        )
                    };
                    item_save_inputs_complete &= row_save_inputs_are_known;
                    let row_guid = ObjectGuid::create_item(realm_id, item_db_guid as i64);
                    carried_rows_coherent &= item_entry > 0
                        && item_db_guid > 0
                        && row_guid.counter() as u64 == item_db_guid
                        && item_count > 0
                        && inv_slots
                            .get(slot as usize)
                            .is_some_and(|guid| guid.is_empty())
                        && !loaded_inventory_item_guids.contains(&row_guid);
                    let item_durability = item.durability;
                    let item_flags = item.flags;
                    let item_played_time = item.played_time;
                    let item_expiration = item.expiration;
                    let item_spell_charges = item.spell_charges;
                    let random_properties = loaded_item_random_properties_like_cpp(
                        item.random_properties_id,
                        item.random_properties_seed,
                        host.hub_ref()
                            .catalogs
                            .item_random_properties_store()
                            .map(|store| store.as_ref()),
                        host.hub_ref()
                            .catalogs
                            .item_random_suffix_store()
                            .map(|store| store.as_ref()),
                    );
                    let random_properties_id =
                        random_properties.map(|random| random.id).unwrap_or(0);
                    let random_properties_seed =
                        random_properties.map(|random| random.seed).unwrap_or(0);
                    let socketed_gems = loaded_socketed_gems_like_cpp(item.gems);
                    let socketed_gem_create_updates =
                        loaded_socketed_gem_create_updates_like_cpp(&socketed_gems);
                    let item_create_enchantments = loaded_item_effective_enchantments_like_cpp(
                        item_enchantment_values.as_ref(),
                        random_properties_id,
                        host.hub_ref()
                            .catalogs
                            .item_random_properties_store()
                            .map(|store| store.as_ref()),
                        host.hub_ref()
                            .catalogs
                            .item_random_suffix_store()
                            .map(|store| store.as_ref()),
                    );
                    item_save_inputs_complete &= {
                        let catalogs = &host.hub_ref().catalogs;
                        item_payload::effective_enchantment_catalog_is_known_like_cpp(
                            &item_create_enchantments,
                            catalogs,
                        )
                    };
                    let refund_decision = loaded_item_refund_decision(
                        item_flags,
                        item_played_time,
                        item.paid_money,
                        item.paid_extended_cost,
                    );
                    if item_entry > 0 && (slot as usize) < 141 {
                        let item_max_durability = host
                            .hub_ref()
                            .catalogs
                            .item_template_max_durability(item_entry)
                            .max(item_durability);
                        let item_guid = ObjectGuid::create_item(realm_id, item_db_guid as i64);
                        let stored_flags = match refund_decision {
                            LoadedItemRefundDecision::Clear { new_flags } => {
                                refund_cleanup_actions.push(
                                    wow_persistence::PlayerLoginItemRepairActionLikeCpp::ClearRefundable {
                                        item_guid: item_db_guid,
                                        new_flags,
                                    },
                                );
                                new_flags
                            }
                            LoadedItemRefundDecision::None
                            | LoadedItemRefundDecision::Valid { .. } => item_flags,
                        };
                        inv_slots[slot as usize] = item_guid;
                        let inventory_type = storage_template
                            .as_ref()
                            .map(|template| template.inventory_type as u8)
                            .filter(|&inventory_type| {
                                inventory_type != InventoryType::NonEquip as u8
                            })
                            .or_else(|| {
                                if slot < 19 {
                                    slot_to_inventory_type(slot)
                                } else {
                                    None
                                }
                            });
                        let is_bag_container = inventory_type == Some(InventoryType::Bag as u8);
                        if is_represented_bag_slot(slot) || is_bag_container {
                            carried_rows_coherent &= is_bag_container
                                && storage_template.as_ref().is_some_and(|template| {
                                    template.container_slots > 0 && template.container_slots <= 36
                                });
                        }
                        let container_slots = if is_bag_container {
                            storage_template
                                .as_ref()
                                .map(|template| u32::from(template.container_slots))
                                .unwrap_or(0)
                                .min(36)
                        } else {
                            0
                        };
                        let create_index = item_creates.len();
                        item_creates.push(wow_packet::packets::update::ItemCreateData {
                            item_guid,
                            entry_id: item_entry as i32,
                            owner_guid: guid,
                            contained_in: guid,
                            stack_count: item_count,
                            dynamic_flags: stored_flags,
                            durability: item_durability,
                            max_durability: item_max_durability,
                            random_properties_seed,
                            random_properties_id,
                            enchantments: item_create_enchantments,
                            gems: socketed_gem_create_updates,
                            context: item_context as u8,
                            container_slots,
                            container_item_guids: [ObjectGuid::EMPTY; 36],
                        });
                        if container_slots > 0 {
                            login_bag_create_index_by_slot.insert(slot, create_index);
                        }
                        let inventory_item = InventoryItem {
                            guid: item_guid,
                            entry_id: item_entry,
                            db_guid: item_db_guid,
                            inventory_type,
                        };
                        if wow_entities::is_buyback_slot(slot) {
                            {
                                let (state, mut hub) = host.split_inventory_mut();
                                state.insert_buyback_item_like_cpp(&mut hub, slot, inventory_item)
                            };
                        } else {
                            host.insert_inventory_item_like_cpp(slot, inventory_item);
                        }
                        let mut item_object = host.make_inventory_item_object(
                            item_guid,
                            item_entry,
                            guid,
                            item_count,
                            item_durability,
                            item_context,
                            slot,
                        );
                        item_object.set_create_played_time(item_played_time);
                        let template_expiration = host
                            .hub_ref()
                            .catalogs
                            .item_stats_store()
                            .and_then(|store| store.duration_in_inventory(item_entry))
                            .unwrap_or(0);
                        let effect_count = host
                            .hub_ref()
                            .catalogs
                            .item_effect_count_like_cpp(item_entry);
                        let expiration_needs_save =
                            apply_loaded_item_storage_mutable_fields_like_cpp(
                                &mut item_object,
                                item_expiration,
                                template_expiration,
                                &item_spell_charges,
                                effect_count,
                            );
                        apply_loaded_item_instance_fields_like_cpp(
                            &mut item_object,
                            &item_create_enchantments,
                            random_properties,
                        );
                        item_object.set_gems(socketed_gems);
                        item_object
                            .replace_all_item_flags(ItemFieldFlags::from_bits_retain(stored_flags));
                        if let LoadedItemRefundDecision::Valid {
                            paid_money,
                            paid_extended_cost,
                        } = refund_decision
                        {
                            item_object.set_refund_recipient(guid);
                            item_object.set_paid_money(paid_money);
                            item_object.set_paid_extended_cost(u32::from(paid_extended_cost));
                        }
                        let bop_allowed_players = item_payload::hydrate_complete_payload_like_cpp(
                            &mut item_object,
                            item.complete_payload,
                            item_flags,
                            realm_id,
                        );
                        let bop_registry_complete =
                            item_payload::apply_bop_trade_metadata_like_cpp(
                                &mut item_object,
                                bop_allowed_players,
                                item_flags,
                                storage_template.as_ref().map(|template| template.max_stack_size),
                                realm_id,
                                |loaded_item| {
                                    let loading_owner_is_current = loading_owner.is_some()
                                        && host.hub_ref().core.player_handle_like_cpp
                                            == loading_owner;
                                    item_payload::register_tradeable_item_like_cpp(
                                        host,
                                        guid,
                                        loaded_item,
                                        loading_owner_is_current,
                                    )
                                },
                            );
                        tradeable_item_registry_complete &= bop_registry_complete;
                        if expiration_needs_save {
                            refund_cleanup_actions.push(
                                wow_persistence::PlayerLoginItemRepairActionLikeCpp::NormalizeOnLoad {
                                    item_guid: item_db_guid,
                                    expiration: item_object.data().expiration,
                                    flags: item_object.item_flags_bits(),
                                    durability: item_object.data().durability,
                                },
                            );
                        }
                        item_creates[create_index].dynamic_flags = item_object.item_flags_bits();
                        host.apply_loaded_inventory_item_collection_hooks_like_cpp(&item_object);
                        item_object.set_state(ItemUpdateState::Unchanged);
                        let visible_item_fields = ((slot as usize) < 19).then(|| {
                            let (s, h) = host.split_inventory_ref();
                            s.loaded_inventory_item_visible_fields_like_cpp(h, &item_object)
                        });
                        {
                            let (state, mut hub) = host.split_inventory_mut();
                            state.insert_inventory_item_object(&mut hub, item_object)
                        };
                        loaded_inventory_item_guids.push(item_guid);
                        if loaded_item_slot_applies_equipped_enchantments_like_cpp(slot) {
                            loaded_equipped_item_guids.push(item_guid);
                        }
                        // Slots 0-18 also populate VisibleItems for character model
                        if let Some(fields) = visible_item_fields {
                            visible_items[slot as usize] = fields;
                        }
                    }
                }
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load equipment for {:?}: {}", guid, reason);
            }
            _ => unreachable!("equipment request returned a different row family"),
        }
        if !refund_cleanup_actions.is_empty() {
            let outcome = player_lifecycle_port
                .persist_login_item_repairs_like_cpp(
                    wow_persistence::PlayerLoginItemRepairRequestLikeCpp {
                        actions: refund_cleanup_actions,
                    },
                )
                .await;
            item_repair_outcomes_applied &= outcome.is_applied();
            if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
            {
                warn!(
                    "Failed to clean expired/missing item refund metadata for {:?}: {}",
                    guid, reason
                );
            }
        }

        // ── Load represented bag contents (nested items) ──
        // C++ `Player::_LoadInventory` loads child rows after their top-level
        // bag rows. `character_inventory.bag` stores the bag item GUID, so the
        // query joins back to the represented bag row and returns its top-level slot.
        {
            let mut bag_load_fix_actions = Vec::new();
            match player_lifecycle_port
                .load_login_auxiliary_like_cpp(
                    wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::BagInventory {
                        player_guid: guid.counter() as u64,
                    },
                )
                .await
            {
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                    wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::BagInventory(rows),
                ) => {
                    bags_loaded = true;
                    for row in rows {
                        let bag_slot = row.bag_slot;
                        let inner_slot = row.inner_slot;
                        let item = row.item;
                        let item_entry = item.item_entry;
                        let item_db_guid = item.item_db_guid;
                        let item_count = item.count;
                        let item_context_value =
                            <ItemContext as num_traits::FromPrimitive>::from_u8(item.context);
                        let item_context_is_representable = item_context_value.is_some();
                        let item_context = item_context_value.unwrap_or(ItemContext::None);
                        let item_enchantment_values =
                            loaded_item_enchantments_like_cpp(&item.enchantments);
                        let item_enchantments_are_representable = item.enchantments.trim().is_empty()
                            || item_enchantment_values.is_some();
                        let item_gem_ids_are_representable =
                            item.gems.iter().all(|(item_id, _, _)| *item_id >= 0);
                        let storage_template =
                            host.hub_ref().catalogs.item_storage_template(item_entry);
                        let row_save_inputs_are_known = {
                            let catalogs = &host.hub_ref().catalogs;
                            item_payload::row_save_inputs_known_like_cpp(
                                &item,
                                item_context_is_representable,
                                item_enchantments_are_representable,
                                item_gem_ids_are_representable,
                                storage_template.is_some(),
                                catalogs,
                            )
                        };
                        item_save_inputs_complete &= row_save_inputs_are_known;
                        let row_guid = ObjectGuid::create_item(realm_id, item_db_guid as i64);
                        carried_rows_coherent &= item_entry > 0
                            && item_db_guid > 0
                            && row_guid.counter() as u64 == item_db_guid
                            && item_count > 0
                            && !loaded_inventory_item_guids.contains(&row_guid)
                            && login_bag_create_index_by_slot.get(&bag_slot).is_some_and(
                                |&index| {
                                    let bag = &item_creates[index];
                                    u32::from(inner_slot) < bag.container_slots
                                        && bag.container_item_guids[inner_slot as usize].is_empty()
                                },
                            );
                        let item_durability = item.durability;
                        let item_flags = item.flags;
                        let item_played_time = item.played_time;
                        let item_expiration = item.expiration;
                        let item_spell_charges = item.spell_charges;
                        let random_properties = loaded_item_random_properties_like_cpp(
                            item.random_properties_id,
                            item.random_properties_seed,
                            host.hub_ref()
                                .catalogs
                                .item_random_properties_store()
                                .map(|store| store.as_ref()),
                            host.hub_ref()
                                .catalogs
                                .item_random_suffix_store()
                                .map(|store| store.as_ref()),
                        );
                        let random_properties_id =
                            random_properties.map(|random| random.id).unwrap_or(0);
                        let random_properties_seed =
                            random_properties.map(|random| random.seed).unwrap_or(0);
                        let socketed_gems = loaded_socketed_gems_like_cpp(item.gems);
                        let socketed_gem_create_updates =
                            loaded_socketed_gem_create_updates_like_cpp(&socketed_gems);
                        let item_create_enchantments = loaded_item_effective_enchantments_like_cpp(
                            item_enchantment_values.as_ref(),
                            random_properties_id,
                            host.hub_ref()
                                .catalogs
                                .item_random_properties_store()
                                .map(|store| store.as_ref()),
                            host.hub_ref()
                                .catalogs
                                .item_random_suffix_store()
                                .map(|store| store.as_ref()),
                        );
                        item_save_inputs_complete &= {
                            let catalogs = &host.hub_ref().catalogs;
                            item_payload::effective_enchantment_catalog_is_known_like_cpp(
                                &item_create_enchantments,
                                catalogs,
                            )
                        };
                        let refund_decision = loaded_item_refund_decision(
                            item_flags,
                            item_played_time,
                            item.paid_money,
                            item.paid_extended_cost,
                        );
                        if item_entry > 0 && is_represented_bag_slot(bag_slot) {
                            if let Some(bag_item_guid) = ({
                                let (state, hub) = host.split_inventory_ref();
                                state.resolved_inventory_item_like_cpp(hub, bag_slot)
                            })
                            .map(|bag_item| bag_item.guid)
                            {
                                let item_guid =
                                    ObjectGuid::create_item(realm_id, item_db_guid as i64);
                                let stored_flags = match refund_decision {
                                    LoadedItemRefundDecision::Clear { new_flags } => {
                                        bag_load_fix_actions.push(
                                            wow_persistence::PlayerLoginItemRepairActionLikeCpp::ClearRefundable {
                                                item_guid: item_db_guid,
                                                new_flags,
                                            },
                                        );
                                        new_flags
                                    }
                                    LoadedItemRefundDecision::None
                                    | LoadedItemRefundDecision::Valid { .. } => item_flags,
                                };
                                let item_max_durability = host
                                    .hub_ref()
                                    .catalogs
                                    .item_template_max_durability(item_entry)
                                    .max(item_durability);
                                let mut item_object = host.make_inventory_item_object(
                                    item_guid,
                                    item_entry,
                                    guid,
                                    item_count,
                                    item_durability,
                                    item_context,
                                    inner_slot,
                                );
                                item_object.set_create_played_time(item_played_time);
                                let template_expiration = host
                                    .hub_ref()
                                    .catalogs
                                    .item_stats_store()
                                    .and_then(|store| store.duration_in_inventory(item_entry))
                                    .unwrap_or(0);
                                let effect_count = host
                                    .hub_ref()
                                    .catalogs
                                    .item_effect_count_like_cpp(item_entry);
                                let expiration_needs_save =
                                    apply_loaded_item_storage_mutable_fields_like_cpp(
                                        &mut item_object,
                                        item_expiration,
                                        template_expiration,
                                        &item_spell_charges,
                                        effect_count,
                                    );
                                apply_loaded_item_instance_fields_like_cpp(
                                    &mut item_object,
                                    &item_create_enchantments,
                                    random_properties,
                                );
                                item_object.set_gems(socketed_gems);
                                item_object.replace_all_item_flags(
                                    ItemFieldFlags::from_bits_retain(stored_flags),
                                );
                                if let LoadedItemRefundDecision::Valid {
                                    paid_money,
                                    paid_extended_cost,
                                } = refund_decision
                                {
                                    item_object.set_refund_recipient(guid);
                                    item_object.set_paid_money(paid_money);
                                    item_object
                                        .set_paid_extended_cost(u32::from(paid_extended_cost));
                                }
                                item_object.set_container_guid_and_slot(bag_item_guid, bag_slot);
                                let bop_allowed_players =
                                    item_payload::hydrate_complete_payload_like_cpp(
                                        &mut item_object,
                                        item.complete_payload,
                                        item_flags,
                                        realm_id,
                                    );
                                let bop_registry_complete =
                                    item_payload::apply_bop_trade_metadata_like_cpp(
                                        &mut item_object,
                                        bop_allowed_players,
                                        item_flags,
                                        storage_template
                                            .as_ref()
                                            .map(|template| template.max_stack_size),
                                        realm_id,
                                        |loaded_item| {
                                            let loading_owner_is_current = loading_owner.is_some()
                                                && host.hub_ref().core.player_handle_like_cpp
                                                    == loading_owner;
                                            item_payload::register_tradeable_item_like_cpp(
                                                host,
                                                guid,
                                                loaded_item,
                                                loading_owner_is_current,
                                            )
                                        },
                                    );
                                tradeable_item_registry_complete &= bop_registry_complete;
                                if expiration_needs_save {
                                    bag_load_fix_actions.push(
                                        wow_persistence::PlayerLoginItemRepairActionLikeCpp::NormalizeOnLoad {
                                            item_guid: item_db_guid,
                                            expiration: item_object.data().expiration,
                                            flags: item_object.item_flags_bits(),
                                            durability: item_object.data().durability,
                                        },
                                    );
                                }
                                host.apply_loaded_inventory_item_collection_hooks_like_cpp(
                                    &item_object,
                                );
                                item_object.set_state(ItemUpdateState::Unchanged);
                                if let Some(&create_index) =
                                    login_bag_create_index_by_slot.get(&bag_slot)
                                {
                                    if (inner_slot as usize) < 36 {
                                        item_creates[create_index].container_item_guids
                                            [inner_slot as usize] = item_guid;
                                    }
                                }
                                item_creates.push(wow_packet::packets::update::ItemCreateData {
                                    item_guid,
                                    entry_id: item_entry as i32,
                                    owner_guid: guid,
                                    contained_in: bag_item_guid,
                                    stack_count: item_count,
                                    dynamic_flags: item_object.item_flags_bits(),
                                    durability: item_durability,
                                    max_durability: item_max_durability,
                                    random_properties_seed,
                                    random_properties_id,
                                    enchantments: item_create_enchantments,
                                    gems: socketed_gem_create_updates,
                                    context: item_context as u8,
                                    container_slots: 0,
                                    container_item_guids: [ObjectGuid::EMPTY; 36],
                                });
                                {
                                    let (state, mut hub) = host.split_inventory_mut();
                                    state.insert_inventory_item_object(&mut hub, item_object)
                                };
                                loaded_inventory_item_guids.push(item_guid);
                            } else {
                                warn!(
                                    "Skipping bag content {:?}/{} for {:?}: missing represented bag slot {}",
                                    ObjectGuid::create_item(realm_id, item_db_guid as i64),
                                    inner_slot,
                                    guid,
                                    bag_slot
                                );
                            }
                        }
                    }
                }
                wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                    warn!("Failed to load bag contents for {:?}: {}", guid, reason);
                }
                _ => unreachable!("bag inventory request returned a different row family"),
            }
            if !bag_load_fix_actions.is_empty() {
                let outcome = player_lifecycle_port
                    .persist_login_item_repairs_like_cpp(
                        wow_persistence::PlayerLoginItemRepairRequestLikeCpp {
                            actions: bag_load_fix_actions,
                        },
                    )
                    .await;
                item_repair_outcomes_applied &= outcome.is_applied();
                if let wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
                | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } = outcome
                {
                    warn!(
                        "Failed to normalize loaded bag item state for {:?}: {}",
                        guid, reason
                    );
                }
            }
        }

        // inventory_type is now loaded from the canonical ItemTemplate bridge.
        // No SQL cache needed.
    }
    // ── Load void storage ──
    // C++ `Player::LoadFromDB` calls `_LoadVoidStorage` only when the
    // already-loaded player flags say the vault is unlocked. A locked
    // character starts with coherent empty storage even if stale rows
    // exist in CharacterDB.
    if crate::prepare_represented_void_storage_login_load_like_cpp(host) {
        match player_lifecycle_port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::VoidStorage {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::VoidStorage(rows),
            ) => {
                for row in rows {
                    let context =
                        void_storage_login_context_like_cpp(row.random_properties_id, row.context);
                    let creator_guid = if row.creator_guid == 0 {
                        ObjectGuid::EMPTY
                    } else {
                        ObjectGuid::create_player(realm_id, row.creator_guid as i64)
                    };
                    let loaded = crate::load_represented_void_storage_row_like_cpp(
                        host,
                        row.slot,
                        RepresentedVoidStorageItemLikeCpp {
                            item_id: row.item_id,
                            item_entry: row.item_entry,
                            creator_guid,
                            fixed_scaling_level: row.fixed_scaling_level,
                            random_properties_id: row.random_properties_id,
                            random_properties_seed: row.random_properties_seed,
                            context,
                        },
                    );
                    if !loaded {
                        warn!(
                            player_guid = guid.counter(),
                            item_id = row.item_id,
                            item_entry = row.item_entry,
                            slot = row.slot,
                            "Player::_LoadVoidStorage skipped an invalid row like C++"
                        );
                    }
                }
                {
                    let (state, mut hub) = host.split_inventory_mut();
                    state.mark_represented_void_storage_loaded_like_cpp(&mut hub)
                };
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!("Failed to load void storage for {:?}: {}", guid, reason);
            }
            _ => unreachable!("void-storage request returned a different row family"),
        }
    }

    // ── Load equipment sets / transmog outfits ──
    // C++ `Player::_LoadEquipmentSets` and `_LoadTransmogOutfits` rebuild
    // one shared `_equipmentSets` container before `SendEquipmentSetList`.
    {
        let (s, mut h) = host.split_inventory_mut();
        s.clear_represented_equipment_sets_like_cpp(&mut h)
    };
    let mut equipment_sets_loaded = true;
    match player_lifecycle_port
        .load_login_auxiliary_like_cpp(
            wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::EquipmentSets {
                player_guid: guid.counter() as u64,
            },
        )
        .await
    {
        wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentSets(rows),
        ) => {
            for row in rows {
                let mut pieces =
                    [ObjectGuid::EMPTY; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
                for (piece, item_low_guid) in pieces.iter_mut().zip(row.item_low_guids.into_iter())
                {
                    if item_low_guid != 0 {
                        *piece = ObjectGuid::create_item(realm_id, item_low_guid as i64);
                    }
                }
                host.load_represented_equipment_set_row_like_cpp(
                    row.set_guid,
                    u32::from(row.set_id),
                    row.name,
                    row.icon,
                    row.ignore_mask,
                    row.assigned_spec_index,
                    pieces,
                );
            }
        }
        wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
            equipment_sets_loaded = false;
            warn!("Failed to load equipment sets for {:?}: {}", guid, reason);
        }
        _ => unreachable!("equipment-set request returned a different row family"),
    }
    match player_lifecycle_port
        .load_login_auxiliary_like_cpp(
            wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::TransmogOutfits {
                player_guid: guid.counter() as u64,
            },
        )
        .await
    {
        wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
            wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::TransmogOutfits(rows),
        ) => {
            for row in rows {
                let mut appearances = [0; wow_packet::packets::misc::EQUIPMENT_SET_SLOTS_LIKE_CPP];
                for (appearance, loaded) in appearances.iter_mut().zip(row.appearances.into_iter())
                {
                    *appearance = loaded;
                }
                {
                    let (s, mut h) = host.split_inventory_mut();
                    s.load_represented_transmog_outfit_row_like_cpp(
                        &mut h,
                        row.set_guid,
                        u32::from(row.set_id),
                        row.name,
                        row.icon,
                        row.ignore_mask,
                        appearances,
                        row.enchants,
                    )
                };
            }
        }
        wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
            equipment_sets_loaded = false;
            warn!("Failed to load transmog outfits for {:?}: {}", guid, reason);
        }
        _ => unreachable!("transmog-outfit request returned a different row family"),
    }
    if equipment_sets_loaded {
        {
            let (state, mut hub) = host.split_inventory_mut();
            state.mark_represented_equipment_sets_loaded_like_cpp(&mut hub)
        };
    }

    let (loaded_item_time_updates, loaded_non_equipped_enchantment_updates) = host
        .register_loaded_inventory_item_duration_refs_like_cpp(
            &loaded_inventory_item_guids,
            &loaded_equipped_item_guids,
        );

    if equipment_loaded && bags_loaded && carried_rows_coherent && loading_owner.is_some() {
        let (_, mut hub) = host.split_inventory_mut();
        if hub.core.player_handle_like_cpp == loading_owner {
            let _ = hub.core.mutate_canonical_player_like_cpp(|player| {
                if player.guid() != guid {
                    return;
                }
                let carried_load_finished =
                    carried_proof::finish_carried_load_like_cpp(player, &inv_slots, &item_creates);
                if carried_load_finished
                    && item_save_inputs_complete
                    && item_repair_outcomes_applied
                    && tradeable_item_registry_complete
                {
                    player
                        .inventory_runtime_mut_like_cpp()
                        .set_equipment_inventory_authority_complete_like_cpp(true);
                }
            });
        }
    }

    Some(LoginInventorySnapshotLikeCpp {
        visible_items,
        inv_slots,
        item_creates,
        loaded_equipped_item_guids,
        loaded_item_time_updates,
        loaded_non_equipped_enchantment_updates,
    })
}

pub fn void_storage_login_context_like_cpp(
    random_properties_id: i32,
    _selected_context_column: u8,
) -> u8 {
    // Audited 3.4.3 `Player::_LoadVoidStorage` constructs ItemContext from
    // fields[5] even though CHAR_SEL_CHAR_VOID_STORAGE selects `context` as
    // fields[7]. Keep that executable C++ behavior; the unused argument makes
    // the query/implementation mismatch explicit instead of hiding column 7.
    random_properties_id as u8
}

fn slot_to_inventory_type(slot: u8) -> Option<u8> {
    match slot {
        0 => Some(1),        // Head
        1 => Some(2),        // Neck
        2 => Some(3),        // Shoulders
        3 => Some(4),        // Body (Shirt)
        4 => Some(5),        // Chest
        5 => Some(6),        // Waist
        6 => Some(7),        // Legs
        7 => Some(8),        // Feet
        8 => Some(9),        // Wrists
        9 => Some(10),       // Hands
        10 | 11 => Some(11), // Finger (Ring)
        12 | 13 => Some(12), // Trinket
        14 => Some(16),      // Cloak
        15 => Some(21),      // MainHand (WeaponMainHand)
        16 => Some(22),      // OffHand (WeaponOffHand)
        17 => Some(15),      // Ranged
        18 => Some(19),      // Tabard
        _ => None,
    }
}
