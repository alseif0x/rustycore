// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Restore the persisted transport attachment during character login.

use crate::character_login_support::CharacterLoginLocationLikeCpp;
use crate::login_session::login_sequence::resolve_persisted_transport_login_like_cpp;
use crate::{CharacterLoginHostLikeCpp, seed_login_location_zone_area_like_cpp};
use tracing::warn;
use wow_core::{ObjectGuid, Position};
use wow_world_lifecycle::login_transport::PersistedTransportLoginLikeCpp;

pub async fn restore_persisted_transport_for_login_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    guid: ObjectGuid,
    saved_transport_guid_low: u64,
    saved_map_id_for_transport: u16,
    saved_transport_position: Position,
    saved_character_map_is_battleground: bool,
    login_homebind: CharacterLoginLocationLikeCpp,
    attached_controller: bool,
    map_id: &mut i32,
    zone: &mut i32,
    position: &mut Position,
) -> Option<PersistedTransportLoginLikeCpp> {
    if saved_transport_guid_low != 0 && !saved_character_map_is_battleground {
        if let Some(transport) = resolve_persisted_transport_login_like_cpp(
            host,
            saved_transport_guid_low,
            saved_map_id_for_transport,
            saved_transport_position,
        )
        .await
        {
            *map_id = i32::from(transport.map_id);
            *position = transport.world_position;
            seed_login_location_zone_area_like_cpp(
                host,
                zone,
                CharacterLoginLocationLikeCpp {
                    map_id: u32::from(transport.map_id),
                    bind_area_id: None,
                    position: transport.world_position,
                },
            );
            host.hub_mut()
                .set_player_map_position_like_cpp(transport.map_id, transport.world_position);
            host.set_player_transport_guid_like_cpp(Some(transport.guid));
            set_player_transport_position_like_cpp(host, Some(transport.offset));
            if attached_controller {
                let _ = host.ensure_canonical_world_map_for_current_player_like_cpp();
            }
            Some(transport)
        } else {
            warn!(
                player_guid = guid.counter(),
                transport_guid_low = saved_transport_guid_low,
                offset_x = saved_transport_position.x,
                offset_y = saved_transport_position.y,
                offset_z = saved_transport_position.z,
                offset_o = saved_transport_position.orientation,
                "invalid persisted transport login state; relocated to homebind like C++ Player::LoadFromDB"
            );
            let homebind_map_id = u16::try_from(login_homebind.map_id)
                .expect("validated character login homebind map ID");
            *map_id = i32::from(homebind_map_id);
            *position = login_homebind.position;
            seed_login_location_zone_area_like_cpp(host, zone, login_homebind);
            host.hub_mut()
                .set_player_map_position_like_cpp(homebind_map_id, login_homebind.position);
            host.set_player_transport_guid_like_cpp(None);
            set_player_transport_position_like_cpp(host, None);
            if attached_controller {
                let _ = host.ensure_canonical_world_map_for_current_player_like_cpp();
            }
            None
        }
    } else {
        host.set_player_transport_guid_like_cpp(None);
        set_player_transport_position_like_cpp(host, None);
        None
    }
}

// The login transport-offset write (#1363 slice 12d; moved from the World
// `session/movement/state.rs`).
pub fn set_player_transport_position_like_cpp<H: CharacterLoginHostLikeCpp + ?Sized>(
    host: &mut H,
    position: Option<Position>,
) {
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some() && host.core().player_handle_like_cpp.is_none() {
        if let (Some(state), Some(position)) = (
            host.hub_mut()
                .fixtures
                .vehicles
                .player_transport_login_state_like_cpp
                .as_mut(),
            position,
        ) {
            state.info.x = position.x;
            state.info.y = position.y;
            state.info.z = position.z;
            state.info.o = position.orientation;
        }
        return;
    }
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        if let Some(position) = position {
            player.set_transport_position_like_cpp(position);
        }
    });
}
