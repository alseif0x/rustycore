// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The account-mount login partial rows, moved from `wow-world`
//! `session/lifecycle_ops/operations.rs` (#1363 final cleanup).

use wow_packet::packets::misc::AccountMount;

use crate::CharacterLoginHostLikeCpp;

/// Account mounts for the per-mount `CollectionMgr::LoadMounts` login
/// publications. C++ keeps every valid DB2 row in the collection, but
/// suppresses this partial update when the mount's PlayerCondition fails.
pub fn account_mount_login_partial_rows_like_cpp<H: CharacterLoginHostLikeCpp + ?Sized>(
    host: &H,
) -> Vec<AccountMount> {
    let (lifecycle, hub) = host.split_lifecycle_ref();
    lifecycle
        .account_mount_rows_like_cpp(hub)
        .into_iter()
        .filter(|mount| {
            let Ok(spell_id) = u32::try_from(mount.spell_id) else {
                return false;
            };
            host.hub_ref()
                .catalogs
                .mount_store
                .as_ref()
                .and_then(|store| store.get_by_source_spell_id_like_cpp(spell_id))
                .is_none_or(|entry| {
                    host.represented_meets_player_condition_id_like_cpp(entry.player_condition_id)
                })
        })
        .collect()
}
