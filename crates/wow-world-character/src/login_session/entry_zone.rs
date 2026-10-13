use crate::CharacterLoginHostLikeCpp;
use tracing::{info, warn};
use wow_core::Position;

/// Applies the currently represented post-add zone/rest effects without publishing packets.
/// Terrain reads are synchronous file I/O, outside Player/map guards. This is not
/// the complete C++ UpdateZone operation (Player.cpp:7298,7356).
/// False preserves the missing seeded-state early exit; true is not terrain authority proof.
pub fn apply_post_add_zone_from_terrain_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    map_id: i32,
    position: &Position,
) -> bool {
    let terrain_area_authority_complete =
        wow_world_core::terrain::terrain_grid_area_id_for_position_like_cpp(
            &host
                .hub_ref()
                .config
                .mmap_runtime_config_like_cpp()
                .data_dir,
            map_id as u32,
            position.x,
            position.y,
        )
        .is_ok_and(|area_id| area_id.is_some_and(|area_id| area_id != 0));

    match wow_world_core::terrain::zone_and_area_for_position_like_cpp(
        &host
            .hub_ref()
            .config
            .mmap_runtime_config_like_cpp()
            .data_dir,
        map_id as u32,
        position.x,
        position.y,
        host.hub_ref()
            .catalogs
            .area_table_store()
            .map(|store| store.as_ref()),
        |map_id| {
            host.hub_ref()
                .catalogs
                .map_store()
                .as_deref()
                .map(|store| u32::from(store.area_table_id_like_cpp(map_id)))
                .unwrap_or(0)
        },
    ) {
        Ok((resolved_zone_id, resolved_area_id)) => {
            host.update_zone_represented_without_rest_update_packet_like_cpp(
                resolved_zone_id,
                resolved_area_id,
            );
            host.set_player_zone_area_authority_complete_like_cpp(
                terrain_area_authority_complete && resolved_zone_id != 0 && resolved_area_id != 0,
            );
            info!(
                map_id,
                x = position.x,
                y = position.y,
                zone_id = resolved_zone_id,
                area_id = resolved_area_id,
                "Resolved player zone/area like C++ terrain before InitWorldStates"
            );
        }
        Err(error) => {
            warn!(
                map_id,
                x = position.x,
                y = position.y,
                %error,
                "failed to resolve C++ terrain zone/area before InitWorldStates; using DB-seeded zone/area"
            );
            let Some((seeded_zone_id, seeded_area_id)) = host.hub_ref().player_zone_area_like_cpp()
            else {
                return false;
            };
            host.update_zone_represented_without_rest_update_packet_like_cpp(
                seeded_zone_id,
                seeded_area_id,
            );
            host.set_player_zone_area_authority_complete_like_cpp(false);
        }
    }
    true
}
