//! Feature-gated forwards to the real Character operation; no replacement logic.

use super::*;

use crate::handlers::character::login_support as original;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterLoginLocationForTest {
    pub map_id: u32,
    pub bind_area_id: Option<u32>,
    pub position: Position,
}

impl From<CharacterLoginLocationForTest> for original::CharacterLoginLocationLikeCpp {
    fn from(value: CharacterLoginLocationForTest) -> Self {
        Self {
            map_id: value.map_id,
            bind_area_id: value.bind_area_id,
            position: value.position,
        }
    }
}

impl From<original::CharacterLoginLocationLikeCpp> for CharacterLoginLocationForTest {
    fn from(value: original::CharacterLoginLocationLikeCpp) -> Self {
        Self {
            map_id: value.map_id,
            bind_area_id: value.bind_area_id,
            position: value.position,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CharacterBattlegroundLoginDataForTest {
    pub entry_point: CharacterLoginLocationForTest,
}

impl From<CharacterBattlegroundLoginDataForTest> for original::CharacterBattlegroundLoginDataLikeCpp {
    fn from(value: CharacterBattlegroundLoginDataForTest) -> Self {
        Self {
            entry_point: value.entry_point.into(),
        }
    }
}

impl From<original::CharacterBattlegroundLoginDataLikeCpp> for CharacterBattlegroundLoginDataForTest {
    fn from(value: original::CharacterBattlegroundLoginDataLikeCpp) -> Self {
        Self {
            entry_point: value.entry_point.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginWorldStateTemplateForTest {
    pub id: i32,
    pub default_value: i32,
    pub map_ids: BTreeSet<i32>,
    pub area_ids: BTreeSet<u32>,
}

impl From<LoginWorldStateTemplateForTest> for original::LoginWorldStateTemplateLikeCpp {
    fn from(value: LoginWorldStateTemplateForTest) -> Self {
        Self {
            id: value.id,
            default_value: value.default_value,
            map_ids: value.map_ids,
            area_ids: value.area_ids,
        }
    }
}

impl From<original::LoginWorldStateTemplateLikeCpp> for LoginWorldStateTemplateForTest {
    fn from(value: original::LoginWorldStateTemplateLikeCpp) -> Self {
        Self {
            id: value.id,
            default_value: value.default_value,
            map_ids: value.map_ids,
            area_ids: value.area_ids,
        }
    }
}

pub fn usable_character_homebind_for_test(
    location: CharacterLoginLocationForTest,
    map_store: Option<&wow_data::MapStore>,
    session_expansion: u8,
) -> bool {
    crate::handlers::character::login_support::usable_character_homebind_like_cpp(location.into(), map_store, session_expansion)
}

pub fn default_graveyard_safe_loc_ids_for_race_for_test(
    race: u8,
) -> [Option<u32>; 2] {
    crate::handlers::character::login_support::default_graveyard_safe_loc_ids_for_race_like_cpp(race)
}

pub fn first_login_creation_homebind_for_test(
    player_create_info: PlayerCreateInfoLikeCpp,
    create_mode: u8,
) -> Option<CharacterLoginLocationForTest> {
    crate::handlers::character::login_support::first_login_creation_homebind_like_cpp(player_create_info, create_mode).map(Into::into)
}

pub fn zone_and_area_from_area_id_for_test(
    area_id: u32,
    area_store: Option<&wow_data::AreaTableStore>,
) -> (u32, u32) {
    crate::handlers::character::login_support::zone_and_area_from_area_id_like_cpp(area_id, area_store)
}

pub fn login_location_zone_area_for_test(
    location: CharacterLoginLocationForTest,
    resolve_terrain: impl FnOnce(u32, Position) -> std::io::Result<(u32, u32)>,
) -> std::io::Result<(u32, u32)> {
    crate::handlers::character::login_support::login_location_zone_area_like_cpp(location.into(), resolve_terrain)
}

pub fn login_bind_point_update_for_test(
    homebind: CharacterLoginLocationForTest,
) -> BindPointUpdate {
    crate::handlers::character::login_support::login_bind_point_update_like_cpp(homebind.into())
}

pub fn battleground_login_fallback_location_for_test(
    battleground_data: Option<CharacterBattlegroundLoginDataForTest>,
    homebind: Option<CharacterLoginLocationForTest>,
    map_store: Option<&wow_data::MapStore>,
) -> Option<CharacterLoginLocationForTest> {
    crate::handlers::character::login_support::battleground_login_fallback_location_like_cpp(battleground_data.map(Into::into), homebind.map(Into::into), map_store).map(Into::into)
}

pub fn build_initial_world_states_for_test(
    templates: impl IntoIterator<Item = LoginWorldStateTemplateForTest>,
    saved_values: impl IntoIterator<Item = (i32, i32)>,
    map_id: i32,
    player_area_id: u32,
    area_store: Option<&wow_data::AreaTableStore>,
) -> Vec<(i32, i32)> {
    crate::handlers::character::login_support::build_initial_world_states_like_cpp(templates.into_iter().map(Into::into), saved_values, map_id, player_area_id, area_store)
}

pub fn apply_pvp_season_world_states_for_test(
    states: &mut Vec<(i32, i32)>,
    arena_season_id: i32,
    arena_season_in_progress: bool,
)  {
    crate::handlers::character::login_support::apply_pvp_season_world_states_like_cpp(states, arena_season_id, arena_season_in_progress)
}
