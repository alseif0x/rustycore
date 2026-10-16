// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented login-load steps over the session (#1363 slice 12b4): the
//! trait-config authority load, the group membership and difficulty restore
//! and the void-storage load. Moved verbatim from the World
//! `session/persistence/{load,load_authority}.rs`; the canonical owner of each
//! piece of state is unchanged.

use crate::CharacterLoginHostLikeCpp;
use wow_entities::PlayerVoidStorageItemLikeCpp as RepresentedVoidStorageItemLikeCpp;
use wow_social::group::group_guid_by_db_store_id_like_cpp;
use wow_world_spell::SpellbookHostLikeCpp;

pub fn begin_represented_trait_config_authority_load_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
) {
    let _ = wow_world_spell::begin_represented_trait_config_load_like_cpp(host);
    host.core()
        .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
}

pub fn complete_represented_trait_config_authority_load_like_cpp<H: SpellbookHostLikeCpp>(
    host: &mut H,
    configs: impl IntoIterator<Item = (i32, i32, i32, i32)>,
    entries_empty: bool,
) -> bool {
    host.core()
        .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    let configs = configs.into_iter().collect();
    let result = wow_world_spell::complete_represented_trait_config_rows_like_cpp(
        host,
        configs,
        entries_empty,
    );
    if result == Some(false) {
        // Invalid input has reset the source proof. Keep the previous
        // post-reset invalidation outside the exclusive Player access.
        host.core()
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }
    result.unwrap_or(false)
}

pub fn mark_represented_void_storage_loaded_like_cpp<H: CharacterLoginHostLikeCpp>(host: &mut H) {
    let (state, mut hub) = host.split_inventory_mut();
    state.mark_represented_void_storage_loaded_like_cpp(&mut hub)
}

/// Match C++ `Player::LoadFromDB`: locked characters do not consume the
/// prepared void-storage result, but still own a coherent empty vault that
/// can be unlocked and saved during this session.
pub fn prepare_represented_void_storage_login_load_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
) -> bool {
    {
        let (state, mut hub) = host.split_inventory_mut();
        state.clear_represented_void_storage_like_cpp(&mut hub)
    };
    let should_load_rows = host.void_storage_is_unlocked_like_cpp();
    if !should_load_rows {
        mark_represented_void_storage_loaded_like_cpp(host);
    }
    should_load_rows
}

pub fn load_represented_void_storage_row_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    slot: u8,
    item: RepresentedVoidStorageItemLikeCpp,
) -> bool {
    let slot_index = usize::from(slot);
    if item.item_id == 0
        || slot_index >= wow_packet::packets::void_storage::VOID_STORAGE_MAX_SLOT_LIKE_CPP
        || host
            .hub_ref()
            .catalogs
            .item_storage_template(item.item_entry)
            .is_none()
    {
        return false;
    }
    let item_entry = item.item_entry;
    let canonical = host.core().with_owned_player_mut_like_cpp(|player| {
        player.load_void_storage_item_like_cpp(slot, item.clone())
    });
    let inserted = match canonical {
        Some(inserted) => inserted,
        None => {
            #[cfg(any(test, feature = "test-fixtures"))]
            {
                if host.test_fixtures_ref_like_cpp().is_some()
                    && host.core().player_handle_like_cpp.is_none()
                {
                    host.split_inventory_mut()
                        .0
                        .insert_loaded_void_storage_item_for_test_like_cpp(slot_index, &item)
                } else {
                    false
                }
            }
            #[cfg(not(any(test, feature = "test-fixtures")))]
            {
                false
            }
        }
    };
    if !inserted {
        return false;
    }
    // C++ `_LoadVoidStorage` initializes `BonusData` from the void item
    // instance and calls `CollectionMgr::AddItemAppearance`; this DB shape
    // only carries the fixed-level modifier, so the effective appearance
    // modifier remains the template default zero.
    let _ = wow_world_inventory::add_item_appearance_for_item_like_cpp(host, item_entry, 0);
    true
}

/// C++ `Player::_LoadGroup` overwrites the loaded player difficulties with
/// the current group values because the leader may change them while the
/// member is offline.
pub fn load_represented_group_difficulties_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
) -> bool {
    let (Some(group_guid), Some(group_registry)) = (
        host.resolved_group_guid_like_cpp(),
        host.core().directory.group_registry.as_ref(),
    ) else {
        return false;
    };

    let preferences = {
        let Some(group) = group_registry.get(&group_guid) else {
            return false;
        };
        (
            group.dungeon_difficulty_id,
            group.raid_difficulty_id,
            group.legacy_raid_difficulty_id,
        )
    };

    host.replace_player_difficulty_preferences_like_cpp(preferences.0, preferences.1, preferences.2)
}

/// C++ `Player::_LoadGroup` resolves `CHAR_SEL_GROUP_MEMBER.guid` through
/// `sGroupMgr->GetGroupByDbStoreId` before attaching the player to the
/// already-loaded group.
pub fn load_represented_group_by_db_store_id_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    db_store_id: u32,
) -> bool {
    let Some(group_registry) = host.core().directory.group_registry.as_ref() else {
        let _ = host.set_owned_player_group_like_cpp(None);
        return false;
    };
    let Some(group_guid) = group_guid_by_db_store_id_like_cpp(db_store_id) else {
        let _ = host.set_owned_player_group_like_cpp(None);
        return false;
    };
    if !group_registry.contains_key(&group_guid) {
        let _ = host.set_owned_player_group_like_cpp(None);
        return false;
    }

    let Some(player_guid) = host.player_guid() else {
        return false;
    };
    let Some(subgroup) = group_registry.get(&group_guid).and_then(|group| {
        group
            .member_slot_like_cpp(player_guid)
            .map(|slot| slot.subgroup)
    }) else {
        let _ = host.set_owned_player_group_like_cpp(None);
        return false;
    };
    if !host.set_owned_player_group_like_cpp(Some((group_guid, subgroup))) {
        return false;
    }
    load_represented_group_difficulties_like_cpp(host)
}
