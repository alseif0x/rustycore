// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character-name response values shared by character admission rules.

pub const RESPONSE_SUCCESS_LIKE_CPP: u8 = 0;
pub const CHAR_NAME_NO_NAME_LIKE_CPP: u8 = 92;
pub const CHAR_NAME_TOO_SHORT_LIKE_CPP: u8 = 93;
pub const CHAR_NAME_TOO_LONG_LIKE_CPP: u8 = 94;
pub const CHAR_NAME_INVALID_CHARACTER_LIKE_CPP: u8 = 95;

// C++ `AtLoginFlags`, `CharacterFlags`, `CharCustomizeFlags`, `PlayerFlags`
// and class ids used by character enumeration and creation.

pub const CLASS_HUNTER_LIKE_CPP: u8 = 3;
pub const CLASS_DEATH_KNIGHT_LIKE_CPP: u8 = 6;
pub const CLASS_WARLOCK_LIKE_CPP: u8 = 9;
pub const PLAYER_FLAGS_GHOST_LIKE_CPP: u32 = 0x0000_0010;
pub const AT_LOGIN_RENAME_LIKE_CPP: u16 = 0x001;
pub const AT_LOGIN_CUSTOMIZE_LIKE_CPP: u16 = 0x008;
pub const AT_LOGIN_FIRST_LIKE_CPP: u16 = 0x020;
pub const AT_LOGIN_CHANGE_FACTION_LIKE_CPP: u16 = 0x040;
pub const AT_LOGIN_CHANGE_RACE_LIKE_CPP: u16 = 0x080;
pub const AT_LOGIN_RESURRECT_LIKE_CPP: u16 = 0x100;
pub const CHARACTER_FLAG_LOCKED_FOR_TRANSFER_LIKE_CPP: u32 = 0x0000_0004;
pub const CHARACTER_FLAG_GHOST_LIKE_CPP: u32 = 0x0000_2000;
pub const CHARACTER_FLAG_RENAME_LIKE_CPP: u32 = 0x0000_4000;
pub const CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP: u32 = 0x0100_0000;
pub const CHARACTER_FLAG_DECLINED_LIKE_CPP: u32 = 0x0200_0000;
pub const CHAR_CUSTOMIZE_FLAG_CUSTOMIZE_LIKE_CPP: u32 = 0x0000_0001;
pub const CHAR_CUSTOMIZE_FLAG_FACTION_LIKE_CPP: u32 = 0x0001_0000;
pub const CHAR_CUSTOMIZE_FLAG_RACE_LIKE_CPP: u32 = 0x0010_0000;
