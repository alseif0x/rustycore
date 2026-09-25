// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared character-login location and initial world-state support.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    BindPointUpdate, PlayerCreateInfoLikeCpp, WORLDSTATE_ANY_MAP_LIKE_CPP, player_team_for_race_cpp,
};
use wow_core::Position;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct LoginWorldStateTemplateLikeCpp {
    pub(super) id: i32,
    pub(super) default_value: i32,
    pub(super) map_ids: BTreeSet<i32>,
    pub(super) area_ids: BTreeSet<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CharacterLoginLocationLikeCpp {
    pub(super) map_id: u32,
    /// C++ `m_homebindAreaId`, loaded from `character_homebind.zoneId`.
    /// This belongs to the bind packet and is distinct from the current
    /// terrain-derived zone/area. Battleground join positions leave it unset.
    pub(super) bind_area_id: Option<u32>,
    pub(super) position: Position,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CharacterBattlegroundLoginDataLikeCpp {
    pub(super) entry_point: CharacterLoginLocationLikeCpp,
}

pub(super) fn usable_character_login_location_like_cpp(
    location: CharacterLoginLocationLikeCpp,
    map_store: Option<&wow_data::MapStore>,
) -> bool {
    location.map_id != u32::from(u16::MAX)
        && u16::try_from(location.map_id).is_ok()
        && location.position.is_valid_map_coord_like_cpp()
        && map_store.is_some_and(|store| store.get(location.map_id).is_some())
}

pub(super) fn usable_character_homebind_like_cpp(
    location: CharacterLoginLocationLikeCpp,
    map_store: Option<&wow_data::MapStore>,
    session_expansion: u8,
) -> bool {
    usable_character_login_location_like_cpp(location, map_store)
        && location.bind_area_id.is_some()
        && map_store
            .and_then(|store| store.get(location.map_id))
            .is_some_and(|entry| {
                !entry.is_instanceable_like_cpp() && session_expansion >= entry.expansion_like_cpp()
            })
}

pub(super) fn default_graveyard_safe_loc_ids_for_race_like_cpp(race: u8) -> [Option<u32>; 2] {
    const RACE_PANDAREN_NEUTRAL_LIKE_CPP: u8 = 24;
    const WANDERING_ISLE_STARTING_GRAVEYARD_LIKE_CPP: u32 = 3295;

    [
        wow_data::GraveyardStore::default_graveyard_safe_loc_id_like_cpp(player_team_for_race_cpp(
            race,
        ) as u32),
        (race == RACE_PANDAREN_NEUTRAL_LIKE_CPP)
            .then_some(WANDERING_ISLE_STARTING_GRAVEYARD_LIKE_CPP),
    ]
}

pub(super) fn first_login_creation_homebind_like_cpp(
    player_create_info: PlayerCreateInfoLikeCpp,
    create_mode: u8,
) -> Option<CharacterLoginLocationLikeCpp> {
    let create_position = if create_mode == wow_data::PLAYER_CREATE_MODE_NPE_LIKE_CPP {
        player_create_info
            .create_position_npe
            .unwrap_or(player_create_info.create_position)
    } else {
        player_create_info.create_position
    };

    create_position
        .transport_guid
        .is_none()
        .then_some(CharacterLoginLocationLikeCpp {
            map_id: create_position.map_id,
            bind_area_id: None,
            position: create_position.position,
        })
}

pub(super) fn zone_and_area_from_area_id_like_cpp(
    area_id: u32,
    area_store: Option<&wow_data::AreaTableStore>,
) -> (u32, u32) {
    let zone_id = area_store
        .and_then(|store| store.get(area_id))
        .filter(|area| area.parent_area_id != 0 && area.is_subzone_like_cpp())
        .map(|area| u32::from(area.parent_area_id))
        .unwrap_or(area_id);
    (zone_id, area_id)
}

pub(super) fn login_location_zone_area_like_cpp(
    location: CharacterLoginLocationLikeCpp,
    resolve_terrain: impl FnOnce(u32, Position) -> std::io::Result<(u32, u32)>,
) -> std::io::Result<(u32, u32)> {
    resolve_terrain(location.map_id, location.position)
}

pub(super) fn login_bind_point_update_like_cpp(
    homebind: CharacterLoginLocationLikeCpp,
) -> BindPointUpdate {
    let bind_area_id = homebind
        .bind_area_id
        .expect("validated character homebind must have an area ID");
    BindPointUpdate {
        x: homebind.position.x,
        y: homebind.position.y,
        z: homebind.position.z,
        map_id: homebind.map_id,
        area_id: bind_area_id,
    }
}

pub(super) fn battleground_login_fallback_location_like_cpp(
    battleground_data: Option<CharacterBattlegroundLoginDataLikeCpp>,
    homebind: Option<CharacterLoginLocationLikeCpp>,
    map_store: Option<&wow_data::MapStore>,
) -> Option<CharacterLoginLocationLikeCpp> {
    battleground_data
        .map(|data| data.entry_point)
        .filter(|location| usable_character_login_location_like_cpp(*location, map_store))
        .or_else(|| {
            homebind
                .filter(|location| usable_character_login_location_like_cpp(*location, map_store))
        })
}

pub(super) fn parse_login_world_state_map_ids_like_cpp(
    map_ids_csv: &str,
    map_exists: impl Fn(i32) -> bool,
) -> BTreeSet<i32> {
    let mut map_ids = BTreeSet::new();
    for token in map_ids_csv.split(',').filter(|token| !token.is_empty()) {
        let Ok(map_id) = token.trim().parse::<i32>() else {
            continue;
        };
        if map_id != WORLDSTATE_ANY_MAP_LIKE_CPP && !map_exists(map_id) {
            continue;
        }
        map_ids.insert(map_id);
    }
    map_ids
}

pub(super) fn parse_login_world_state_area_ids_like_cpp(
    area_ids_csv: &str,
    map_ids: &BTreeSet<i32>,
    area_store: Option<&wow_data::AreaTableStore>,
) -> BTreeSet<u32> {
    let mut area_ids = BTreeSet::new();
    for token in area_ids_csv.split(',').filter(|token| !token.is_empty()) {
        let Ok(area_id) = token.trim().parse::<u32>() else {
            continue;
        };
        let Some(area) = area_store.and_then(|store| store.get(area_id)) else {
            continue;
        };
        if !map_ids.contains(&i32::from(area.continent_id)) {
            continue;
        }
        area_ids.insert(area_id);
    }
    area_ids
}

pub(super) fn build_initial_world_states_like_cpp(
    templates: impl IntoIterator<Item = LoginWorldStateTemplateLikeCpp>,
    saved_values: impl IntoIterator<Item = (i32, i32)>,
    map_id: i32,
    player_area_id: u32,
    area_store: Option<&wow_data::AreaTableStore>,
) -> Vec<(i32, i32)> {
    let mut template_by_id = BTreeMap::new();
    let mut realm_values = BTreeMap::new();
    let mut map_values_by_map: BTreeMap<i32, BTreeMap<i32, i32>> = BTreeMap::new();

    for template in templates {
        if template.map_ids.is_empty() {
            realm_values.insert(template.id, template.default_value);
        } else {
            for &template_map_id in &template.map_ids {
                map_values_by_map
                    .entry(template_map_id)
                    .or_default()
                    .insert(template.id, template.default_value);
            }
        }
        template_by_id.insert(template.id, template);
    }

    for (world_state_id, value) in saved_values {
        let Some(template) = template_by_id.get(&world_state_id) else {
            continue;
        };
        if template.map_ids.is_empty() {
            realm_values.insert(world_state_id, value);
        } else {
            for &template_map_id in &template.map_ids {
                map_values_by_map
                    .entry(template_map_id)
                    .or_default()
                    .insert(world_state_id, value);
            }
        }
    }

    let mut out = Vec::new();
    out.extend(realm_values);

    for lookup_map_id in [WORLDSTATE_ANY_MAP_LIKE_CPP, map_id] {
        let Some(values) = map_values_by_map.get(&lookup_map_id) else {
            continue;
        };
        for (&world_state_id, &value) in values {
            if let Some(template) = template_by_id.get(&world_state_id) {
                if !template.area_ids.is_empty()
                    && !template.area_ids.iter().any(|required_area_id| {
                        area_store.is_some_and(|store| {
                            store.is_in_area_like_cpp(player_area_id, *required_area_id)
                        })
                    })
                {
                    continue;
                }
            }
            out.push((world_state_id, value));
        }
    }

    out
}

/// Apply the realm-wide PvP-season world states the C++ `WorldStateMgr` seeds and
/// `FillInitialWorldStates` always sends (World.cpp:1363-1364, 2300-2301):
/// `WS_CURRENT_PVP_SEASON_ID` (3191) = `in_progress ? season_id : 0`, and
/// `WS_PREVIOUS_PVP_SEASON_ID` (3901) = `season_id - (in_progress ? 1 : 0)`
/// (SharedDefines.h:8081-8082). Overrides the value in place when the id is already
/// present (preserving order), else appends it. Rust previously shipped both as 0.
pub(super) fn apply_pvp_season_world_states_like_cpp(
    states: &mut Vec<(i32, i32)>,
    arena_season_id: i32,
    arena_season_in_progress: bool,
) {
    const WS_CURRENT_PVP_SEASON_ID: i32 = 3191;
    const WS_PREVIOUS_PVP_SEASON_ID: i32 = 3901;

    let current = if arena_season_in_progress {
        arena_season_id
    } else {
        0
    };
    let previous = arena_season_id - i32::from(arena_season_in_progress);

    for (id, value) in [
        (WS_CURRENT_PVP_SEASON_ID, current),
        (WS_PREVIOUS_PVP_SEASON_ID, previous),
    ] {
        if let Some(entry) = states.iter_mut().find(|(state_id, _)| *state_id == id) {
            entry.1 = value;
        } else {
            states.push((id, value));
        }
    }
}
