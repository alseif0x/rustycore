// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The login hydration steps only the continue-login coordinator calls
//! (#1363 slice 13c): `Player::LoadFromDB`'s `knownTitles`/`chosenTitle`
//! load, `Player::_LoadGroup`'s group-leader flag, the login known-spell
//! projection after the account collections, and the legacy backpack-size
//! compatibility. Moved from the World `session/persistence/load.rs`,
//! `session/social/group.rs`, `handlers/character/account/collections.rs`
//! and `handlers/character/mod.rs` verbatim except for the mechanical
//! `self` -> `host` rewrite. The former `#[cfg(test)]` titles fixture step
//! runs only in the World test build (`test_fixtures_ref_like_cpp`).

use std::collections::BTreeSet;

use wow_entities::INVENTORY_DEFAULT_SIZE;
use wow_world_character::CharacterLoginHostLikeCpp;
use wow_world_loot_ops::LootSessionHostLikeCpp;
use wow_world_spell::{RepresentedPlayerSpellStateLikeCpp, SpellbookHostLikeCpp};

/// C++ `PLAYER_FLAGS_GROUP_LEADER` (moved from the World session root).
pub const PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP: u32 = 0x0000_0001;

/// C++ `Player::LoadFromDB` parses `knownTitles` as 32-bit words and
/// validates `chosenTitle` against `Player::HasTitle` before setting it.
pub fn load_represented_character_titles_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
    known_titles: &str,
    chosen_title: u32,
) {
    let mut known_title_ids = BTreeSet::new();
    for (word_index, token) in known_titles.split_whitespace().enumerate() {
        let word = token.parse::<u64>().unwrap_or(0) & u64::from(u32::MAX);
        for bit_index in 0..32_u32 {
            if (word & (1_u64 << bit_index)) != 0 {
                known_title_ids.insert((word_index as u32) * 32 + bit_index);
            }
        }
    }

    let chosen_title = if chosen_title != 0 && !known_title_ids.contains(&chosen_title) {
        0
    } else {
        chosen_title
    };
    let chosen_title = i32::try_from(chosen_title).unwrap_or(0);
    let _canonical = host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            player.replace_known_titles_like_cpp(known_title_ids.clone());
            player.set_chosen_title_like_cpp(chosen_title);
        })
        .is_some();
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && !_canonical
        && host.core().player_handle_like_cpp.is_none()
    {
        host.quest_state_mut()
            .fixture_replace_represented_known_titles_like_cpp(known_title_ids);
        host.quest_state_mut()
            .fixture_set_represented_chosen_title_like_cpp(chosen_title);
    }
}
/// C++ `Player::_LoadGroup` sets `PLAYER_FLAGS_GROUP_LEADER` when the
/// loaded group leader matches the player, and removes it otherwise.
pub fn apply_represented_group_leader_flag_like_cpp<H: LootSessionHostLikeCpp>(
    host: &mut H,
) -> bool {
    let Some(player_guid) = host.player_guid() else {
        return false;
    };

    let is_group_leader = host
        .resolved_group_guid_like_cpp()
        .and_then(|group_guid| {
            host.core()
                .directory
                .group_registry
                .as_ref()
                .and_then(|registry| registry.get(&group_guid).map(|group| group.leader_guid))
        })
        .is_some_and(|leader_guid| leader_guid == player_guid);

    let updated = host
        .core()
        .mutate_canonical_player_like_cpp(|player| {
            if is_group_leader {
                player.set_player_flag(PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP);
            } else {
                player.remove_player_flag(PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP);
            }
        })
        .is_some();

    if updated {
        host.sync_player_registry_state_like_cpp();
    }

    updated
}
pub fn login_known_spells_after_account_collections_like_cpp<
    H: CharacterLoginHostLikeCpp + SpellbookHostLikeCpp,
>(
    host: &H,
) -> Vec<i32> {
    // C++ `Player::HasSpell` includes inactive, non-disabled rows, while
    // `Player::SendKnownSpells` publishes only active rows. Prefer the
    // complete PlayerSpellMap when available so the internal mirror can
    // retain lower ranks without leaking them into the login packet.
    let mut spells = wow_world_spell::complete_represented_player_spell_rows_like_cpp(host)
        .map(|rows| {
            rows.values()
                .filter(|spell| {
                    spell.state != RepresentedPlayerSpellStateLikeCpp::Removed
                        && spell.active
                        && !spell.disabled
                })
                .map(|spell| spell.spell_id)
                .collect()
        })
        .unwrap_or_else(|| wow_world_spell::known_spells_like_cpp(host).to_vec());
    let (state, hub) = host.split_lifecycle_ref();
    for mount in state.account_mount_rows_like_cpp(hub) {
        if !spells.contains(&mount.spell_id) {
            spells.push(mount.spell_id);
        }
    }
    spells
}
pub fn loaded_inventory_slot_count_with_legacy_rust_compat(saved_slots: u8) -> u8 {
    // C++ loads the saved value directly, but TrinityCore's schema defaults
    // inventorySlots to the base backpack size. Older RustyCore builds
    // explicitly inserted zero before this field was wired; keep those
    // already-created characters playable without an out-of-band migration.
    if saved_slots == 0 {
        INVENTORY_DEFAULT_SIZE
    } else {
        saved_slots
    }
}
