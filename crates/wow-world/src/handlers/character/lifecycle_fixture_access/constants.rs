//! Feature-gated forwards to the real Character operation; no replacement logic.

use super::*;

pub const WORLDSTATE_ANY_MAP_FOR_TEST: i32 = crate::handlers::character::WORLDSTATE_ANY_MAP_LIKE_CPP;
pub const CHAR_CREATE_ERROR_FOR_TEST: u8 = crate::handlers::character::CHAR_CREATE_ERROR_LIKE_CPP;
pub const CLASS_HUNTER_FOR_TEST: u8 = crate::handlers::character::CLASS_HUNTER_LIKE_CPP;
pub const PLAYER_FLAGS_GHOST_FOR_TEST: u32 = crate::handlers::character::PLAYER_FLAGS_GHOST_LIKE_CPP;
pub const AT_LOGIN_RENAME_FOR_TEST: u16 = crate::handlers::character::AT_LOGIN_RENAME_LIKE_CPP;
pub const AT_LOGIN_CUSTOMIZE_FOR_TEST: u16 = crate::handlers::character::AT_LOGIN_CUSTOMIZE_LIKE_CPP;
pub const AT_LOGIN_FIRST_FOR_TEST: u16 = crate::handlers::character::AT_LOGIN_FIRST_LIKE_CPP;
pub const AT_LOGIN_CHANGE_FACTION_FOR_TEST: u16 = crate::handlers::character::AT_LOGIN_CHANGE_FACTION_LIKE_CPP;
pub const AT_LOGIN_CHANGE_RACE_FOR_TEST: u16 = crate::handlers::character::AT_LOGIN_CHANGE_RACE_LIKE_CPP;
pub const AT_LOGIN_RESURRECT_FOR_TEST: u16 = crate::handlers::character::AT_LOGIN_RESURRECT_LIKE_CPP;
pub const CHARACTER_FLAG_GHOST_FOR_TEST: u32 = crate::handlers::character::CHARACTER_FLAG_GHOST_LIKE_CPP;
pub const CHARACTER_FLAG_RENAME_FOR_TEST: u32 = crate::handlers::character::CHARACTER_FLAG_RENAME_LIKE_CPP;
pub const CHARACTER_FLAG_LOCKED_BY_BILLING_FOR_TEST: u32 = crate::handlers::character::CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP;
pub const CHARACTER_FLAG_DECLINED_FOR_TEST: u32 = crate::handlers::character::CHARACTER_FLAG_DECLINED_LIKE_CPP;
pub const CHAR_CUSTOMIZE_FLAG_CUSTOMIZE_FOR_TEST: u32 = crate::handlers::character::CHAR_CUSTOMIZE_FLAG_CUSTOMIZE_LIKE_CPP;
pub const CHAR_CUSTOMIZE_FLAG_FACTION_FOR_TEST: u32 = crate::handlers::character::CHAR_CUSTOMIZE_FLAG_FACTION_LIKE_CPP;
pub const CHAR_CUSTOMIZE_FLAG_RACE_FOR_TEST: u32 = crate::handlers::character::CHAR_CUSTOMIZE_FLAG_RACE_LIKE_CPP;
pub const GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_FOR_TEST: u8 = crate::handlers::character::GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT_LIKE_CPP;
