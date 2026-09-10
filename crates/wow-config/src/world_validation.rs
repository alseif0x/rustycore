//! World validation packets.
//!
//! Separated from lib.rs under #693.

use crate::*;

/// `MaxPrimaryTradeSkill` default from C++ `World.cpp` and
/// `worldserver.conf.dist`.
pub const DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP: u8 = 2;

/// Documented inclusive upper bound for `MaxPrimaryTradeSkill`.
pub const MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP: u8 = 11;

pub(super) fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

pub(super) fn apply_world_config_validations(values: &mut WorldConfigSet) {
    const MAX_LEVEL: u32 = 123;
    const MAX_PLAYER_NAME: u32 = 12;
    const MAX_PET_NAME: u32 = 12;
    const MAX_CHARTER_NAME: u32 = 24;
    const MAX_CHARACTERS_PER_REALM: u32 = 200;
    const MIN_GRID_DELAY: u32 = 60_000;
    const MIN_MAP_UPDATE_DELAY: u32 = 1;
    const MAX_START_MONEY: u32 = 0x7fff_ffff - 1;
    const GUILD_NEWSLOG_MAX_RECORDS: u32 = 250;
    const GUILD_EVENTLOG_MAX_RECORDS: u32 = 100;
    const GUILD_BANKLOG_MAX_RECORDS: u32 = 25;
    const BAN_CHARACTER: u32 = 1;
    const BAN_IP: u32 = 2;
    const BAN_ACCOUNT: u32 = 0;

    int_outside_to(values, "CONFIG_COMPRESSION", 1, 9, 1);
    int_outside_to(values, "CONFIG_AUCTION_SEARCH_DELAY", 100, 10_000, 300);
    int_outside_to(
        values,
        "CONFIG_AUCTION_TAINTED_SEARCH_DELAY",
        100,
        10_000,
        3_000,
    );

    if values
        .get_bool("CONFIG_GRID_UNLOAD")
        .zip(values.get_bool("CONFIG_BASEMAP_LOAD_GRIDS"))
        .is_some_and(|(grid_unload, load_grids)| grid_unload && load_grids)
    {
        values.set_bool("CONFIG_BASEMAP_LOAD_GRIDS", false);
    }

    if values
        .get_bool("CONFIG_GRID_UNLOAD")
        .zip(values.get_bool("CONFIG_INSTANCEMAP_LOAD_GRIDS"))
        .is_some_and(|(grid_unload, load_grids)| grid_unload && load_grids)
    {
        values.set_bool("CONFIG_INSTANCEMAP_LOAD_GRIDS", false);
    }

    int_above_to(values, "CONFIG_MIN_LEVEL_STAT_SAVE", MAX_LEVEL, 0);
    int_below_to(
        values,
        "CONFIG_INTERVAL_GRIDCLEAN",
        MIN_GRID_DELAY,
        MIN_GRID_DELAY,
    );
    int_below_to(
        values,
        "CONFIG_INTERVAL_MAPUPDATE",
        MIN_MAP_UPDATE_DELAY,
        MIN_MAP_UPDATE_DELAY,
    );
    int_divide_by(values, "CONFIG_SOCKET_TIMEOUTTIME", 1_000);
    int_divide_by(values, "CONFIG_SOCKET_TIMEOUTTIME_ACTIVE", 1_000);

    for name in [
        "CONFIG_MIN_QUEST_SCALED_XP_RATIO",
        "CONFIG_MIN_CREATURE_SCALED_XP_RATIO",
        "CONFIG_MIN_DISCOVERED_SCALED_XP_RATIO",
    ] {
        int_above_to(values, name, 100, 0);
    }

    int_outside_to(values, "CONFIG_MIN_PLAYER_NAME", 1, MAX_PLAYER_NAME, 2);
    int_outside_to(values, "CONFIG_MIN_CHARTER_NAME", 1, MAX_CHARTER_NAME, 2);
    int_outside_to(values, "CONFIG_MIN_PET_NAME", 1, MAX_PET_NAME, 2);
    int_outside_to(
        values,
        "CONFIG_CHARACTERS_PER_REALM",
        1,
        MAX_CHARACTERS_PER_REALM,
        MAX_CHARACTERS_PER_REALM,
    );

    if let (Some(account), Some(realm)) = (
        values.get_int("CONFIG_CHARACTERS_PER_ACCOUNT"),
        values.get_int("CONFIG_CHARACTERS_PER_REALM"),
    ) {
        if account < realm {
            values.set_int("CONFIG_CHARACTERS_PER_ACCOUNT", realm);
        }
    }

    if let Some(value) = values.get_int("CONFIG_CHARACTER_CREATING_EVOKERS_PER_REALM") {
        if signed_i32(value) < 0 || value > 10 {
            values.set_int("CONFIG_CHARACTER_CREATING_EVOKERS_PER_REALM", 1);
        }
    }

    if let Some(value) = values.get_int("CONFIG_SKIP_CINEMATICS") {
        if signed_i32(value) < 0 || value > 2 {
            values.set_int("CONFIG_SKIP_CINEMATICS", 0);
        }
    }

    int_above_to(values, "CONFIG_MAX_PLAYER_LEVEL", MAX_LEVEL, MAX_LEVEL);
    for name in [
        "CONFIG_START_PLAYER_LEVEL",
        "CONFIG_START_DEATH_KNIGHT_PLAYER_LEVEL",
        "CONFIG_START_DEMON_HUNTER_PLAYER_LEVEL",
        "CONFIG_START_EVOKER_PLAYER_LEVEL",
        "CONFIG_START_ALLIED_RACE_LEVEL",
    ] {
        clamp_start_level(values, name);
    }

    if let Some(value) = values.get_int("CONFIG_START_PLAYER_MONEY") {
        if signed_i32(value) < 0 {
            values.set_int("CONFIG_START_PLAYER_MONEY", 0);
        } else if value > MAX_START_MONEY {
            values.set_int("CONFIG_START_PLAYER_MONEY", MAX_START_MONEY);
        }
    }

    int_above_to(values, "CONFIG_CURRENCY_RESET_HOUR", 23, 3);
    int_above_to(values, "CONFIG_CURRENCY_RESET_DAY", 6, 3);
    if let Some(value) = values.get_int("CONFIG_CURRENCY_RESET_INTERVAL") {
        if signed_i32(value) <= 0 {
            values.set_int("CONFIG_CURRENCY_RESET_INTERVAL", 7);
        }
    }

    if let (Some(raf_level), Some(max_level)) = (
        values.get_int("CONFIG_MAX_RECRUIT_A_FRIEND_BONUS_PLAYER_LEVEL"),
        values.get_int("CONFIG_MAX_PLAYER_LEVEL"),
    ) {
        if raf_level > max_level {
            values.set_int("CONFIG_MAX_RECRUIT_A_FRIEND_BONUS_PLAYER_LEVEL", 85);
        }
    }

    int_above_to(values, "CONFIG_DAILY_QUEST_RESET_TIME_HOUR", 23, 3);
    int_above_to(values, "CONFIG_WEEKLY_QUEST_RESET_TIME_WDAY", 6, 3);
    // `worldserver.conf.dist` documents 0..11. The target C++ reads the
    // value without validating it, but Rust must reject negative values
    // (represented as wrapped u32 here) before downstream u8 conversion.
    int_outside_to(
        values,
        "CONFIG_MAX_PRIMARY_TRADE_SKILL",
        0,
        u32::from(MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP),
        u32::from(DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP),
    );
    int_above_to(values, "CONFIG_MIN_PETITION_SIGNS", 4, 4);

    if let (Some(gm_level), Some(start_level)) = (
        values.get_int("CONFIG_START_GM_LEVEL"),
        values.get_int("CONFIG_START_PLAYER_LEVEL"),
    ) {
        if gm_level < start_level {
            values.set_int("CONFIG_START_GM_LEVEL", start_level);
        } else if gm_level > MAX_LEVEL {
            values.set_int("CONFIG_START_GM_LEVEL", MAX_LEVEL);
        }
    }

    int_above_to(values, "CONFIG_CLEAN_OLD_MAIL_TIME", 23, 4);
    int_signed_below_or_equal_to(values, "CONFIG_UPTIME_UPDATE", 0, 10);
    int_signed_below_or_equal_to(values, "CONFIG_LOGDB_CLEARINTERVAL", 0, 10);

    if let Some(value) = values.get_int("CONFIG_MAX_OVERSPEED_PINGS") {
        if value != 0 && value < 2 {
            values.set_int("CONFIG_MAX_OVERSPEED_PINGS", 2);
        }
    }

    int_above_to(
        values,
        "CONFIG_QUEST_LOW_LEVEL_HIDE_DIFF",
        MAX_LEVEL,
        MAX_LEVEL,
    );
    int_above_to(
        values,
        "CONFIG_QUEST_HIGH_LEVEL_HIDE_DIFF",
        MAX_LEVEL,
        MAX_LEVEL,
    );
    int_above_to(values, "CONFIG_RANDOM_BG_RESET_HOUR", 23, 6);
    int_above_to(values, "CONFIG_CALENDAR_DELETE_OLD_EVENTS_HOUR", 23, 6);
    int_above_to(values, "CONFIG_GUILD_RESET_HOUR", 23, 6);
    int_outside_to(values, "CONFIG_BATTLEGROUND_REPORT_AFK", 1, 9, 3);
    int_above_to(
        values,
        "CONFIG_GUILD_NEWS_LOG_COUNT",
        GUILD_NEWSLOG_MAX_RECORDS,
        GUILD_NEWSLOG_MAX_RECORDS,
    );
    int_above_to(
        values,
        "CONFIG_GUILD_EVENT_LOG_COUNT",
        GUILD_EVENTLOG_MAX_RECORDS,
        GUILD_EVENTLOG_MAX_RECORDS,
    );
    int_above_to(
        values,
        "CONFIG_GUILD_BANK_EVENT_LOG_COUNT",
        GUILD_BANKLOG_MAX_RECORDS,
        GUILD_BANKLOG_MAX_RECORDS,
    );

    if let (Some(above), Some(max_level)) = (
        values.get_int("CONFIG_NO_GRAY_AGGRO_ABOVE"),
        values.get_int("CONFIG_MAX_PLAYER_LEVEL"),
    ) {
        if above > max_level {
            values.set_int("CONFIG_NO_GRAY_AGGRO_ABOVE", max_level);
        }
    }

    if let (Some(below), Some(max_level)) = (
        values.get_int("CONFIG_NO_GRAY_AGGRO_BELOW"),
        values.get_int("CONFIG_MAX_PLAYER_LEVEL"),
    ) {
        if below > max_level {
            values.set_int("CONFIG_NO_GRAY_AGGRO_BELOW", max_level);
        }
    }

    if let (Some(above), Some(below)) = (
        values.get_int("CONFIG_NO_GRAY_AGGRO_ABOVE"),
        values.get_int("CONFIG_NO_GRAY_AGGRO_BELOW"),
    ) {
        if above > 0 && above < below {
            values.set_int("CONFIG_NO_GRAY_AGGRO_BELOW", above);
        }
    }

    int_above_to(values, "CONFIG_RESPAWN_DYNAMICMODE", 1, 0);
    int_above_to(
        values,
        "CONFIG_RESPAWN_GUIDWARNLEVEL",
        16_777_215,
        12_000_000,
    );
    int_above_to(
        values,
        "CONFIG_RESPAWN_GUIDALERTLEVEL",
        16_777_215,
        16_000_000,
    );
    int_above_to(values, "CONFIG_RESPAWN_RESTARTQUIETTIME", 23, 3);
    float_below_to(values, "RATE_REPAIRCOST", 0.0, 0.0);
    float_below_to(values, "CONFIG_RESPAWN_DYNAMICRATE_CREATURE", 0.0, 10.0);
    float_below_to(values, "CONFIG_RESPAWN_DYNAMICRATE_GAMEOBJECT", 0.0, 10.0);
    int_below_to(values, "CONFIG_PVP_TOKEN_COUNT", 1, 1);

    if let Some(value) = values.get_int("CONFIG_PACKET_SPOOF_BANMODE") {
        if value == BAN_CHARACTER || value > BAN_IP {
            values.set_int("CONFIG_PACKET_SPOOF_BANMODE", BAN_ACCOUNT);
        }
    }
}

pub(super) fn signed_i32(value: u32) -> i32 {
    value as i32
}

fn int_above_to(values: &mut WorldConfigSet, enum_name: &str, max: u32, replacement: u32) {
    if values.get_int(enum_name).is_some_and(|value| value > max) {
        values.set_int(enum_name, replacement);
    }
}

fn int_below_to(values: &mut WorldConfigSet, enum_name: &str, min: u32, replacement: u32) {
    if values.get_int(enum_name).is_some_and(|value| value < min) {
        values.set_int(enum_name, replacement);
    }
}

fn int_outside_to(
    values: &mut WorldConfigSet,
    enum_name: &str,
    min: u32,
    max: u32,
    replacement: u32,
) {
    if values
        .get_int(enum_name)
        .is_some_and(|value| value < min || value > max)
    {
        values.set_int(enum_name, replacement);
    }
}

fn int_signed_below_or_equal_to(
    values: &mut WorldConfigSet,
    enum_name: &str,
    threshold: i32,
    replacement: u32,
) {
    if values
        .get_int(enum_name)
        .is_some_and(|value| signed_i32(value) <= threshold)
    {
        values.set_int(enum_name, replacement);
    }
}

fn int_divide_by(values: &mut WorldConfigSet, enum_name: &str, divisor: u32) {
    if let Some(value) = values.get_int(enum_name) {
        values.set_int(enum_name, value / divisor);
    }
}

fn float_below_to(values: &mut WorldConfigSet, enum_name: &str, min: f32, replacement: f32) {
    if values.get_float(enum_name).is_some_and(|value| value < min) {
        values.set_float(enum_name, replacement);
    }
}

fn clamp_start_level(values: &mut WorldConfigSet, enum_name: &str) {
    let Some(value) = values.get_int(enum_name) else {
        return;
    };
    let Some(max_level) = values.get_int("CONFIG_MAX_PLAYER_LEVEL") else {
        return;
    };

    if value < 1 {
        values.set_int(enum_name, 1);
    } else if value > max_level {
        values.set_int(enum_name, max_level);
    }
}
