// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `Player::CheckAreaExploreAndOutdoor` and the explored-zones snapshot it
//! reads (#1363 slice 11b; moved from the World
//! `session/world_state/operations.rs`).

use wow_ai::max_level_for_expansion_like_cpp;
use wow_core::ObjectGuid;
use wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP;
use wow_world_core::session::ProgressionCatalogsLikeCpp;

use super::InstanceSessionHostLikeCpp;

pub fn player_explored_zones_snapshot_like_cpp<H: InstanceSessionHostLikeCpp>(
    host: &H,
) -> Option<[u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP]> {
    let canonical = host
        .core()
        .with_owned_player_like_cpp(|player| *player.explored_zones_blocks_like_cpp());
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && canonical.is_none()
        && host.core().player_handle_like_cpp.is_none()
    {
        return Some(
            *host
                .split_instances_ref()
                .0
                .represented_explored_zones_for_test_like_cpp(),
        );
    }
    canonical
}
/// Represented C++ `Player::CheckAreaExploreAndOutdoor` discovery branch.
///
/// This slice covers `AreaTableEntry::AreaBit`, `AddExploredZones`, the player-values update,
/// `CriteriaType::RevealWorldMapOverlay`, the exploration XP branch, and the
/// `CONFIG_VMAP_INDOOR_CHECK` aura-removal branch when a represented
/// `WorldObject::IsOutdoors()` value is available. Terrain/VMAP ownership of
/// the outdoors state remains a map-runtime gap.
pub async fn check_area_explore_and_outdoor_represented_with_catalogs_like_cpp<
    H: InstanceSessionHostLikeCpp,
>(
    host: &mut H,
    progression: &ProgressionCatalogsLikeCpp,
    area_id: u32,
) -> bool {
    if host.hub_ref().resolved_player_is_alive_like_cpp() != Some(true) {
        return false;
    }

    if host.hub_ref().resolved_is_in_taxi_flight_like_cpp() != Some(false) {
        return false;
    }

    wow_world_spell::remove_indoor_outdoor_auras_for_current_position_represented_like_cpp(host);

    if area_id == 0 {
        return false;
    }

    let Some(area_entry) = host
        .hub_ref()
        .catalogs
        .area_table_store
        .as_ref()
        .and_then(|store| store.get(area_id))
        .copied()
    else {
        return false;
    };

    let Some((offset, mask)) =
        area_entry.explored_zone_bit_like_cpp(PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP)
    else {
        return false;
    };

    let Some(explored_zones) = player_explored_zones_snapshot_like_cpp(host) else {
        return false;
    };
    if explored_zones[offset] & mask != 0 {
        return false;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() {
        host.split_instances_mut()
            .0
            .record_represented_reveal_world_map_overlay_criteria_for_test_like_cpp(area_id);
    }

    if let Some(update) = host.core().mutate_canonical_player_like_cpp(|player| {
        player.add_explored_zones_like_cpp(offset, mask);
        player.values_update(true)
    }) {
        host.core().send_player_values_update_like_cpp(&update);
    }

    if area_entry.exploration_level > 0 {
        use wow_packet::packets::misc::ExplorationExperience;

        let max_level =
            max_level_for_expansion_like_cpp(host.core().realm_policy.server_expansion_like_cpp);
        let xp = if host.hub_ref().player_level_like_cpp() >= max_level {
            0
        } else {
            progression
                .exploration_base_xp
                .exploration_xp_reward_like_cpp(
                    host.hub_ref().player_level_like_cpp(),
                    area_entry.exploration_level,
                    progression.exploration_xp_rate,
                    progression.min_discovered_scaled_xp_ratio,
                )
        };

        if xp != 0 {
            host.give_xp(xp, ObjectGuid::EMPTY, 1.0).await;
        }
        host.send_packet(&ExplorationExperience {
            area_id: area_id as i32,
            experience: xp as i32,
        });
    }

    true
}
