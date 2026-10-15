// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented item appearance collection (#1363 slice 12b1; moved from the
//! World `session/player_items/{appearance,items}.rs` and
//! `session/item_modifiers.rs`): bounded `CollectionMgr::OnItemAdded`,
//! `AddItemAppearance` and `CanAddAppearance`.

use wow_constants::{
    InventoryType, ItemClass, ItemFlags2, ItemFlags3, ItemQuality, ItemSubClassArmor,
    ItemSubClassWeapon,
};
use wow_world_core::session::{
    player_class_mask_for_transmog_like_cpp, player_team_id_for_race_cpp,
};

use crate::VendorSessionHostLikeCpp;

/// Bounded C++ `CollectionMgr::OnItemAdded`.
pub fn on_item_added_to_collection_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    item: &wow_entities::Item,
) -> Vec<wow_entities::PlayerValuesUpdate> {
    let item_id = item.object().entry();
    let mut updates = Vec::new();

    if host
        .hub_ref()
        .catalogs
        .heirloom_store
        .as_ref()
        .and_then(|store| store.get_by_item_id_like_cpp(item_id))
        .is_some()
        && ({
            let (state, mut hub) = host.split_lifecycle_mut();
            state.add_account_heirloom_like_cpp(&mut hub, item_id, 0)
        })
        && let Some(update) = host
            .hub_mut()
            .add_player_heirloom_dynamic_fields_like_cpp(item_id, 0)
    {
        updates.push(update);
    }

    if let Some(update) = add_item_appearance_for_runtime_item_like_cpp(host, item) {
        updates.push(update);
    }

    updates
}

/// Bounded C++ `CollectionMgr::AddItemAppearance(uint32, uint32)`.
///
/// This resolves `ItemModifiedAppearance` by `(item_id, appearance_mod_id)` like
/// `sDB2Manager.GetItemModifiedAppearance` and runs the represented
/// `CanAddAppearance` item-template/proficiency/quality gate, including the
/// learned `Player::GetWeaponProficiency` mask.
pub fn add_item_appearance_for_item_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    item_id: u32,
    appearance_mod_id: u32,
) -> Option<wow_entities::PlayerValuesUpdate> {
    let item_modified_appearance_id = host
        .hub_ref()
        .catalogs
        .item_modified_appearance_for_item(item_id, appearance_mod_id)?;
    if !can_add_item_appearance_represented_like_cpp(host, item_modified_appearance_id) {
        return None;
    }
    {
        let (state, mut hub) = host.split_inventory_mut();
        state.add_item_appearance_like_cpp(&mut hub, item_modified_appearance_id)
    }
}

/// Bounded C++ `CollectionMgr::AddItemAppearance(Item*)`.
pub fn add_item_appearance_for_runtime_item_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &mut H,
    item: &wow_entities::Item,
) -> Option<wow_entities::PlayerValuesUpdate> {
    if !item.is_soul_bound() {
        return None;
    }

    let item_id = item.object().entry();
    let appearance_mod_id = item.visible_appearance_mod_id(0, |appearance_id| {
        host.hub_ref()
            .catalogs
            .item_modified_appearance_ref(appearance_id)
    });
    let item_modified_appearance_id = host
        .hub_ref()
        .catalogs
        .item_modified_appearance_for_item(item_id, u32::from(appearance_mod_id))?;
    if !can_add_item_appearance_represented_like_cpp(host, item_modified_appearance_id) {
        return None;
    }

    if item.is_bop_tradeable() || item.is_refundable() {
        return {
            let (state, mut hub) = host.split_inventory_mut();
            state.add_temporary_item_appearance_like_cpp(
                &mut hub,
                item_modified_appearance_id,
                item.object().guid(),
            )
        };
    }

    {
        let (state, mut hub) = host.split_inventory_mut();
        state.add_item_appearance_like_cpp(&mut hub, item_modified_appearance_id)
    }
}

/// Bounded C++ `CollectionMgr::CanAddAppearance`.
///
/// This covers the DB2/template, represented `CanUseItem` class/proficiency,
/// transmog source, quality/flags, item class/subclass/inventory and duplicate
/// gates that Rust currently represents. Full `Player::CanUseItem` remains
/// wider than this helper.
pub fn can_add_item_appearance_represented_like_cpp<H: VendorSessionHostLikeCpp>(
    host: &H,
    item_modified_appearance_id: u32,
) -> bool {
    let Some(item_modified_appearance) = host
        .hub_ref()
        .catalogs
        .items
        .modified_appearance_store
        .as_ref()
        .and_then(|store| store.get(item_modified_appearance_id))
    else {
        return false;
    };

    if matches!(item_modified_appearance.transmog_source_type_enum, 6 | 9) {
        return false;
    }

    let Ok(item_id) = u32::try_from(item_modified_appearance.item_id) else {
        return false;
    };
    let Some(search_template) = host
        .hub_ref()
        .catalogs
        .items
        .search_name_store
        .as_ref()
        .and_then(|store| store.get(item_id))
    else {
        return false;
    };

    let Some(item_record) = host
        .hub_ref()
        .catalogs
        .items
        .store
        .as_ref()
        .and_then(|store| store.get(item_id))
    else {
        return false;
    };
    let Some(sparse_template) = host
        .hub_ref()
        .catalogs
        .items
        .stats_store
        .as_ref()
        .and_then(|store| store.sparse_template(item_id))
    else {
        return false;
    };
    let Some(template) = host.hub_ref().catalogs.item_storage_template(item_id) else {
        return false;
    };
    if host.player_guid().is_none() {
        return false;
    }

    // C++ `CollectionMgr::CanAddAppearance` first runs
    // `Player::CanUseItem(ItemTemplate const*)` (`Player.cpp:11069-11125`).
    // The represented template gates are the internal/faction flags, the
    // allowable class/race masks and the required skill, spell and level
    // checks; the holiday, reputation, learning-effect and artifact
    // specialization gates remain separate slices.
    let use_flags2 = sparse_template.flags[1];
    if (use_flags2 & ItemFlags2::InternalItem as u32) != 0 {
        return false;
    }
    let player_team = player_team_id_for_race_cpp(host.hub_ref().player_race_like_cpp());
    if (use_flags2 & ItemFlags2::FactionHorde as u32) != 0
        && player_team != wow_entities::TEAM_HORDE_ID
    {
        return false;
    }
    if (use_flags2 & ItemFlags2::FactionAlliance as u32) != 0
        && player_team != wow_entities::TEAM_ALLIANCE_ID
    {
        return false;
    }
    let player_race_mask = host
        .hub_ref()
        .player_race_like_cpp()
        .checked_sub(1)
        .and_then(|shift| 1i64.checked_shl(u32::from(shift)))
        .unwrap_or(0);
    if search_template.allowable_race != 0
        && (search_template.allowable_race & player_race_mask) == 0
    {
        return false;
    }
    if search_template.required_level > 0
        && host.hub_ref().player_level_like_cpp()
            < u8::try_from(search_template.required_level).unwrap_or(u8::MAX)
    {
        return false;
    }
    if search_template.required_skill != 0 {
        let Some(skill_value) = u16::try_from(search_template.required_skill)
            .ok()
            .and_then(|skill| host.hub_ref().resolved_player_skill_value_like_cpp(skill))
        else {
            return false;
        };
        if u32::from(skill_value) < u32::from(search_template.required_skill_rank) {
            return false;
        }
    }
    if search_template.required_ability != 0
        && !i32::try_from(search_template.required_ability)
            .ok()
            .is_some_and(|spell_id| {
                host.item_appearance_known_spells_like_cpp()
                    .contains(&spell_id)
            })
    {
        return false;
    }
    if sparse_template.required_reputation_faction != 0 {
        let required_rank =
            u32::try_from(sparse_template.required_reputation_rank.max(0)).unwrap_or(0);
        if ({
            let (state, hub) = host.split_inventory_ref();
            state.represented_item_reputation_rank_like_cpp(
                hub,
                u32::from(sparse_template.required_reputation_faction),
            )
        })
        .unwrap_or(0)
            < required_rank
        {
            return false;
        }
    }
    // C++ `CanUseItem` learning-effect pair (`Player.cpp:11110-11113`): a
    // recipe, mount or pet item whose second effect is already known cannot
    // be used again.
    let effect_spell_ids = host
        .hub_ref()
        .catalogs
        .represented_item_effect_spell_ids_like_cpp(item_id);
    if let (Some((_, first)), Some((_, second))) =
        (effect_spell_ids.first(), effect_spell_ids.get(1))
        && matches!(*first, 483 | 55_884)
        && i32::try_from(*second).ok().is_some_and(|spell_id| {
            host.item_appearance_known_spells_like_cpp()
                .contains(&spell_id)
        })
    {
        return false;
    }

    let player_class_mask =
        player_class_mask_for_transmog_like_cpp(host.hub_ref().player_class_like_cpp());
    if sparse_template.allowable_class != 0
        && (u32::try_from(sparse_template.allowable_class).unwrap_or(0) & player_class_mask) == 0
    {
        return false;
    }

    let flags2 = sparse_template.flags[1];
    let flags3 = sparse_template.flags[2];
    if (flags2 & ItemFlags2::NoSourceForItemVisual as u32) != 0 {
        return false;
    }
    let Some(quality) = host.hub_ref().catalogs.item_template_quality(item_id) else {
        return false;
    };
    if quality == ItemQuality::Artifact as i8 {
        return false;
    }

    match template.class_id {
        ItemClass::Weapon => {
            let subclass = u32::from(item_record.subclass_id);
            if subclass >= 32 {
                return false;
            }
            // C++ `CollectionMgr::CanAddAppearance` reads the learned
            // `Player::GetWeaponProficiency` mask, not the class default
            // the client receives at creation.
            let weapon_proficiency = host
                .core()
                .represented_player_weapon_proficiency_like_cpp()
                .unwrap_or(0);
            if (weapon_proficiency & (1_u32 << subclass)) == 0 {
                return false;
            }
            if matches!(
                subclass,
                x if x == ItemSubClassWeapon::Exotic as u32
                    || x == ItemSubClassWeapon::Exotic2 as u32
                    || x == ItemSubClassWeapon::Miscellaneous as u32
                    || x == ItemSubClassWeapon::Thrown as u32
                    || x == ItemSubClassWeapon::Spear as u32
                    || x == ItemSubClassWeapon::FishingPole as u32
            ) {
                return false;
            }
        }
        ItemClass::Armor => {
            let subclass = u32::from(item_record.subclass_id);
            match template.inventory_type {
                InventoryType::Body
                | InventoryType::Shield
                | InventoryType::Cloak
                | InventoryType::Tabard
                | InventoryType::Holdable => {}
                InventoryType::Head
                | InventoryType::Shoulders
                | InventoryType::Chest
                | InventoryType::Waist
                | InventoryType::Legs
                | InventoryType::Feet
                | InventoryType::Wrists
                | InventoryType::Hands
                | InventoryType::Robe => {
                    if subclass == ItemSubClassArmor::Miscellaneous as u32 {
                        return false;
                    }
                }
                _ => return false,
            }

            if template.inventory_type != InventoryType::Cloak
                && (player_class_by_armor_subclass_like_cpp(subclass) & player_class_mask) == 0
            {
                return false;
            }
        }
        _ => return false,
    }

    if quality < ItemQuality::Uncommon as i8
        && ((flags2 & ItemFlags2::IgnoreQualityForItemVisualSource as u32) == 0
            || (flags3 & ItemFlags3::ActsAsTransmogHiddenVisualOption as u32) == 0)
    {
        return false;
    }

    host.hub_ref()
        .player_collection_state_snapshot_like_cpp()
        .is_some_and(|collections| {
            !collections
                .item_appearances_like_cpp()
                .contains(&item_modified_appearance_id)
        })
}

fn player_class_by_armor_subclass_like_cpp(subclass: u32) -> u32 {
    match subclass {
        x if x == ItemSubClassArmor::Miscellaneous as u32 => 0x0FFF,
        x if x == ItemSubClassArmor::Cloth as u32 => {
            (1 << (5 - 1)) | (1 << (8 - 1)) | (1 << (9 - 1))
        }
        x if x == ItemSubClassArmor::Leather as u32 => {
            (1 << (4 - 1)) | (1 << (10 - 1)) | (1 << (11 - 1)) | (1 << (12 - 1))
        }
        x if x == ItemSubClassArmor::Mail as u32 => (1 << (3 - 1)) | (1 << (7 - 1)),
        x if x == ItemSubClassArmor::Plate as u32 => {
            (1 << (1 - 1)) | (1 << (2 - 1)) | (1 << (6 - 1))
        }
        x if x == ItemSubClassArmor::Cosmetic as u32 => 0x0FFF,
        x if x == ItemSubClassArmor::Shield as u32 => {
            (1 << (1 - 1)) | (1 << (2 - 1)) | (1 << (7 - 1))
        }
        x if x == ItemSubClassArmor::Libram as u32 => 1 << (2 - 1),
        x if x == ItemSubClassArmor::Idol as u32 => 1 << (11 - 1),
        x if x == ItemSubClassArmor::Totem as u32 => 1 << (7 - 1),
        x if x == ItemSubClassArmor::Sigil as u32 => 1 << (6 - 1),
        x if x == ItemSubClassArmor::Relic as u32 => {
            (1 << (2 - 1)) | (1 << (6 - 1)) | (1 << (7 - 1)) | (1 << (11 - 1))
        }
        _ => 0,
    }
}
