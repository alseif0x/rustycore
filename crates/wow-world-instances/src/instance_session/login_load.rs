// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The explored-zones and difficulty login hydration (#1363 slice 13c).
//!
//! C++ `Player::LoadFromDB` loads `exploredZones` and applies
//! `CheckLoaded*DifficultyID` to the saved difficulties. Moved from the World
//! `session/persistence/load.rs` verbatim except for the mechanical
//! `self` -> `host` rewrite. The difficulty load was a `LifecycleCx` method;
//! it now borrows the same two fields, the instance state and the hub, through
//! `split_instances_mut`. The former `#[cfg(test)]` explored-zones fixture
//! step runs only in the World test build (`test_fixtures_ref_like_cpp`).

use wow_entities::parse_explored_zones_db_string_like_cpp;

use super::{InstanceSessionHostLikeCpp, player_explored_zones_snapshot_like_cpp};
use crate::{
    DIFFICULTY_10_N_LIKE_CPP, DIFFICULTY_NORMAL_LIKE_CPP, DIFFICULTY_NORMAL_RAID_LIKE_CPP,
};

pub fn load_represented_explored_zones_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    input: &str,
) -> usize {
    let blocks = parse_explored_zones_db_string_like_cpp(input);
    let Some(previous) = player_explored_zones_snapshot_like_cpp(host) else {
        return 0;
    };
    let changed = previous != blocks;

    let canonical = host
        .core()
        .mutate_canonical_player_like_cpp(|player| {
            let applied = player.set_explored_zones_blocks_like_cpp(&blocks);
            (applied > 0).then(|| player.values_update(true))
        })
        .flatten();
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        host.split_instances_mut()
            .0
            .replace_represented_explored_zones_for_test_like_cpp(blocks);
    }
    if let Some(update) = canonical {
        host.core().send_player_values_update_like_cpp(&update);
    }

    if changed {
        blocks.iter().filter(|block| **block != 0).count()
    } else {
        0
    }
}
/// C++ `Player::LoadFromDB` applies `CheckLoaded*DifficultyID` to the raw
/// `characters` columns before any login packets are sent.
pub fn load_represented_player_difficulties_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &mut H,
    dungeon_difficulty_id: u32,
    raid_difficulty_id: u32,
    legacy_raid_difficulty_id: u32,
) {
    let (instances, mut hub) = host.split_instances_mut();
    let Some(store) = hub.catalogs.difficulty_store() else {
        let _ = instances.replace_player_difficulty_preferences_like_cpp(
            &mut hub,
            DIFFICULTY_NORMAL_LIKE_CPP,
            DIFFICULTY_NORMAL_RAID_LIKE_CPP,
            DIFFICULTY_10_N_LIKE_CPP,
        );
        return;
    };

    let dungeon_difficulty_id =
        store.check_loaded_dungeon_difficulty_id_like_cpp(dungeon_difficulty_id);
    let raid_difficulty_id = store.check_loaded_raid_difficulty_id_like_cpp(raid_difficulty_id);
    let legacy_raid_difficulty_id =
        store.check_loaded_legacy_raid_difficulty_id_like_cpp(legacy_raid_difficulty_id);

    let _ = instances.replace_player_difficulty_preferences_like_cpp(
        &mut hub,
        dungeon_difficulty_id,
        raid_difficulty_id,
        legacy_raid_difficulty_id,
    );
}
