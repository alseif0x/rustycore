// Explicit application fixtures; no implicit canonical owner is installed.

pub use std::sync::Arc;
pub use wow_core::{ObjectGuid, Position};
pub use wow_world::session::*;
pub use wow_world::test_fixtures::*;

pub use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
pub use std::sync::Mutex;
pub use wow_constants::rest::*;
pub use wow_constants::unit::*;
pub use wow_constants::{PowerType, ServerOpcodes};
pub use wow_data::*;
pub use wow_packet::WorldPacket;
pub use wow_packet::packets::{character::*, misc::*, spell::*, update::*};
pub use wow_persistence::*;
pub use wow_world::test_fixtures::{
    AT_LOGIN_CHANGE_FACTION_FOR_TEST as AT_LOGIN_CHANGE_FACTION_LIKE_CPP,
    AT_LOGIN_CHANGE_RACE_FOR_TEST as AT_LOGIN_CHANGE_RACE_LIKE_CPP,
    AT_LOGIN_CUSTOMIZE_FOR_TEST as AT_LOGIN_CUSTOMIZE_LIKE_CPP,
    AT_LOGIN_FIRST_FOR_TEST as AT_LOGIN_FIRST_LIKE_CPP,
    AT_LOGIN_RENAME_FOR_TEST as AT_LOGIN_RENAME_LIKE_CPP,
    AT_LOGIN_RESURRECT_FOR_TEST as AT_LOGIN_RESURRECT_LIKE_CPP,
    CHAR_CREATE_ERROR_FOR_TEST as CHAR_CREATE_ERROR_LIKE_CPP,
    CHAR_CUSTOMIZE_FLAG_CUSTOMIZE_FOR_TEST as CHAR_CUSTOMIZE_FLAG_CUSTOMIZE_LIKE_CPP,
    CHAR_CUSTOMIZE_FLAG_FACTION_FOR_TEST as CHAR_CUSTOMIZE_FLAG_FACTION_LIKE_CPP,
    CHAR_CUSTOMIZE_FLAG_RACE_FOR_TEST as CHAR_CUSTOMIZE_FLAG_RACE_LIKE_CPP,
    CHARACTER_FLAG_DECLINED_FOR_TEST as CHARACTER_FLAG_DECLINED_LIKE_CPP,
    CHARACTER_FLAG_GHOST_FOR_TEST as CHARACTER_FLAG_GHOST_LIKE_CPP,
    CHARACTER_FLAG_LOCKED_BY_BILLING_FOR_TEST as CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP,
    CHARACTER_FLAG_RENAME_FOR_TEST as CHARACTER_FLAG_RENAME_LIKE_CPP,
    CLASS_HUNTER_FOR_TEST as CLASS_HUNTER_LIKE_CPP,
    CharacterBattlegroundLoginDataForTest as CharacterBattlegroundLoginDataLikeCpp,
    CharacterLoginLocationForTest as CharacterLoginLocationLikeCpp,
    EnumCharacterFlagsForTest as EnumCharacterFlagsLikeCpp,
    GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_FOR_TEST as GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP,
    LoadedMapCorpseRowForTest as LoadedMapCorpseRowLikeCpp,
    LoginWorldStateTemplateForTest as LoginWorldStateTemplateLikeCpp,
    MapCorpseLoadOutcomeForTest as MapCorpseLoadOutcomeLikeCpp,
    MapTransportCreateForTest as MapTransportCreateLikeCpp,
    PLAYER_FLAGS_GHOST_FOR_TEST as PLAYER_FLAGS_GHOST_LIKE_CPP,
    PersistedTransportLoginForTest as PersistedTransportLoginLikeCpp,
    TransportCreatePositionForTest as TransportCreatePositionLikeCpp,
    WORLDSTATE_ANY_MAP_FOR_TEST as WORLDSTATE_ANY_MAP_LIKE_CPP,
    apply_pvp_season_world_states_for_test as apply_pvp_season_world_states_like_cpp,
    battleground_login_fallback_location_for_test as battleground_login_fallback_location_like_cpp,
    build_initial_world_states_for_test as build_initial_world_states_like_cpp,
    compose_init_self_create_blocks_for_test as compose_init_self_create_blocks_like_cpp,
    default_character_power1_for_test as default_character_power1_like_cpp,
    default_display_id_for_test as default_display_id,
    default_graveyard_safe_loc_ids_for_race_for_test as default_graveyard_safe_loc_ids_for_race_like_cpp,
    enum_character_flags_for_test as enum_character_flags_like_cpp,
    enum_character_pet_data_for_test as enum_character_pet_data_like_cpp,
    first_login_creation_homebind_for_test as first_login_creation_homebind_like_cpp,
    initial_character_rest_state_for_test as initial_character_rest_state_like_cpp,
    login_bind_point_update_for_test as login_bind_point_update_like_cpp,
    login_location_zone_area_for_test as login_location_zone_area_like_cpp,
    materialize_loaded_map_corpses_for_test as materialize_loaded_map_corpses_like_cpp,
    motd_lines_for_test as motd_lines_like_cpp,
    restored_saved_health_for_test as restored_saved_health_like_cpp,
    start_position_for_test as start_position, start_zone_for_test as start_zone,
    transport_route_contains_saved_map_for_test as transport_route_contains_saved_map_like_cpp,
    usable_character_homebind_for_test as usable_character_homebind_like_cpp,
    validate_persisted_transport_login_for_test as validate_persisted_transport_login_like_cpp,
    zone_and_area_from_area_id_for_test as zone_and_area_from_area_id_like_cpp,
};

mod constructors;
mod persistence;
pub use constructors::session;
pub use constructors::*;
pub use persistence::{RecordingPortLikeCpp, session_with_port};

pub use wow_world::test_fixtures::insert_character_fixture_player_into_canonical_map as insert_session_player_into_canonical_map_like_cpp;
pub type SharedCanonicalMapManager = wow_world::SharedCanonicalMapManager;

pub const GLOBAL_CACHE_MASK_LIKE_CPP: u32 = WorldSession::CHARACTER_GLOBAL_CACHE_MASK_FOR_TEST;
pub const PER_CHARACTER_CACHE_MASK_LIKE_CPP: u32 =
    WorldSession::CHARACTER_PER_CHARACTER_CACHE_MASK_FOR_TEST;
pub const ALL_ACCOUNT_DATA_CACHE_MASK_LIKE_CPP: u32 =
    WorldSession::CHARACTER_ALL_ACCOUNT_DATA_CACHE_MASK_FOR_TEST;
pub const REST_FLAG_IN_TAVERN_LIKE_CPP: u32 = WorldSession::CHARACTER_REST_FLAG_IN_TAVERN_FOR_TEST;
pub const REST_FLAG_IN_CITY_LIKE_CPP: u32 = WorldSession::CHARACTER_REST_FLAG_IN_CITY_FOR_TEST;
pub const REST_FLAG_IN_FACTION_AREA_LIKE_CPP: u32 =
    WorldSession::CHARACTER_REST_FLAG_IN_FACTION_AREA_FOR_TEST;
pub const PLAYER_FLAGS_RESTING_LIKE_CPP: u32 =
    WorldSession::CHARACTER_PLAYER_FLAGS_RESTING_FOR_TEST;
pub const PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP: u32 =
    WorldSession::CHARACTER_PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_FOR_TEST;
