// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Load the login-time battleground, homebind, and guild admission rows.

use crate::CharacterLoginHostLikeCpp;
use crate::character_login_support::CharacterBattlegroundLoginDataLikeCpp;
use crate::character_login_support::CharacterLoginLocationLikeCpp;
use std::sync::Arc;
use tracing::warn;
use wow_core::ObjectGuid;
use wow_core::Position;

pub struct LoginAdmissionDataLikeCpp {
    pub saved_character_map_is_battleground: bool,
    pub battleground_login_data: Option<CharacterBattlegroundLoginDataLikeCpp>,
    pub homebind: Option<CharacterLoginLocationLikeCpp>,
    pub guild_id: Option<u64>,
}

pub async fn load_login_admission_data_like_cpp<H: CharacterLoginHostLikeCpp>(
    host: &mut H,
    player_lifecycle_port: &Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
    guid: ObjectGuid,
    saved_map_id: i32,
) -> Option<LoginAdmissionDataLikeCpp> {
    let saved_character_map_is_battleground = host
        .hub_ref()
        .catalogs
        .map_store()
        .and_then(|store| store.get(saved_map_id as u32))
        .is_some_and(|entry| entry.is_battleground_or_arena());
    let battleground_login_data = if saved_character_map_is_battleground {
        match player_lifecycle_port
            .load_login_admission_like_cpp(
                wow_persistence::PlayerLoginAdmissionLoadRequestLikeCpp::BattlegroundLocation {
                    player_guid: guid.counter() as u64,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAdmissionLoadedLikeCpp::BattlegroundLocation(row),
            ) => row.map(|row| CharacterBattlegroundLoginDataLikeCpp {
                entry_point: CharacterLoginLocationLikeCpp {
                    map_id: u32::from(row.map_id.unwrap_or(u16::MAX)),
                    bind_area_id: None,
                    position: Position::new(
                        row.x.unwrap_or(f32::NAN),
                        row.y.unwrap_or(f32::NAN),
                        row.z.unwrap_or(f32::NAN),
                        row.orientation.unwrap_or(f32::NAN),
                    ),
                },
            }),
            wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    player_guid = guid.counter(),
                    %reason,
                    "failed to load character_battleground_data like C++ Player::_LoadBGData"
                );
                None
            }
            _ => {
                warn!(
                    player_guid = guid.counter(),
                    "unexpected battleground-location lifecycle result"
                );
                None
            }
        }
    } else {
        None
    };
    let homebind = match player_lifecycle_port
        .load_login_admission_like_cpp(
            wow_persistence::PlayerLoginAdmissionLoadRequestLikeCpp::HomebindLocation {
                player_guid: guid.counter() as u64,
            },
        )
        .await
    {
        wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Loaded(
            wow_persistence::PlayerLoginAdmissionLoadedLikeCpp::HomebindLocation(row),
        ) => row.map(|row| CharacterLoginLocationLikeCpp {
            map_id: u32::from(row.map_id.unwrap_or(u16::MAX)),
            bind_area_id: Some(u32::from(row.area_id.unwrap_or(0))),
            position: Position::new(
                row.x.unwrap_or(f32::NAN),
                row.y.unwrap_or(f32::NAN),
                row.z.unwrap_or(f32::NAN),
                row.orientation.unwrap_or(f32::NAN),
            ),
        }),
        wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                player_guid = guid.counter(),
                %reason,
                "failed to load character homebind like C++ Player::_LoadHomeBind"
            );
            host.core_mut()
                .kick("WorldSession::HandlePlayerLogin Player::_LoadHomeBind query failed");
            return None;
        }
        _ => {
            warn!(
                player_guid = guid.counter(),
                "unexpected homebind-location lifecycle result"
            );
            host.core_mut()
                .kick("WorldSession::HandlePlayerLogin Player::_LoadHomeBind query failed");
            return None;
        }
    };
    let guild_id = match player_lifecycle_port
        .load_login_admission_like_cpp(
            wow_persistence::PlayerLoginAdmissionLoadRequestLikeCpp::GuildMembership {
                player_guid: guid.counter() as u64,
            },
        )
        .await
    {
        wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Loaded(
            wow_persistence::PlayerLoginAdmissionLoadedLikeCpp::GuildMembership(rows),
        ) if rows.is_empty() => Some(0),
        wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Loaded(
            wow_persistence::PlayerLoginAdmissionLoadedLikeCpp::GuildMembership(rows),
        ) if rows.len() == 1 => {
            if rows[0].guild_id.is_none() {
                warn!(
                    player_guid = guid.counter(),
                    "Keeping guild membership authority incomplete: malformed row"
                );
            }
            rows[0].guild_id
        }
        wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Loaded(
            wow_persistence::PlayerLoginAdmissionLoadedLikeCpp::GuildMembership(_),
        ) => {
            warn!(
                player_guid = guid.counter(),
                "Keeping guild membership authority incomplete: duplicate rows"
            );
            None
        }
        wow_persistence::PlayerLoginAdmissionLoadOutcomeLikeCpp::Failed { reason } => {
            warn!(
                player_guid = guid.counter(),
                %reason,
                "Failed to load guild membership for player login"
            );
            None
        }
        _ => {
            warn!(
                player_guid = guid.counter(),
                "unexpected guild-membership lifecycle result"
            );
            None
        }
    };

    Some(LoginAdmissionDataLikeCpp {
        saved_character_map_is_battleground,
        battleground_login_data,
        homebind,
        guild_id,
    })
}
