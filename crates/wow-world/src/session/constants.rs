//! Session-level gameplay constants retained at their existing root paths.

pub(super) const REST_FLAG_IN_TAVERN_LIKE_CPP: u32 = 0x1;
pub(super) const REST_FLAG_IN_CITY_LIKE_CPP: u32 = 0x2;
pub(super) const REST_FLAG_IN_FACTION_AREA_LIKE_CPP: u32 = 0x4;
// C++ `RestMgr::SetRestBonus`: `float(next_level_xp) * 1.5f / 2`.
#[cfg(any(test, feature = "test-fixtures"))]
pub(super) const REST_BONUS_MAX_NEXT_LEVEL_XP_FACTOR_LIKE_CPP: f32 = 1.5 / 2.0;
pub(super) const REST_OFFLINE_WILDERNESS_BUBBLE_LIKE_CPP: f32 = 0.031;
pub(super) const REST_OFFLINE_TAVERN_OR_CITY_BUBBLE_LIKE_CPP: f32 = 0.125;
pub(super) const REST_ONLINE_INGAME_BUBBLE_LIKE_CPP: f32 = 0.125;
pub(super) const DIFFICULTY_NORMAL_LIKE_CPP: u32 = 1;
pub(super) const DIFFICULTY_NORMAL_RAID_LIKE_CPP: u32 = 14;
pub(super) const DIFFICULTY_10_N_LIKE_CPP: u32 = 3;
pub(super) const MAP_INSTANCE_LIKE_CPP: u8 = 1;
pub(super) const MAP_RAID_LIKE_CPP: u8 = 2;
pub(crate) const PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP: u32 = 0x0000_0100;
pub(super) const PLAYER_FLAGS_IN_PVP_LIKE_CPP: u32 = 0x0000_0200;
pub(super) const PLAYER_FLAGS_TAXI_BENCHMARK_LIKE_CPP: u32 = 0x0002_0000;
pub(super) const PLAYER_FLAGS_PVP_TIMER_LIKE_CPP: u32 = 0x0004_0000;
pub(super) const PLAYER_FLAGS_AUTO_DECLINE_GUILD_LIKE_CPP: u32 = 0x0800_0000;
pub(super) const SPELL_PVP_RULES_ENABLED_LIKE_CPP: i32 = 134_735;
pub(super) const LANG_RESET_SPELLS_LIKE_CPP: u32 = 215;
pub(super) const LANG_RESET_TALENTS_LIKE_CPP: u32 = 216;
pub(super) const LANG_RESET_SPELLS_TEXT_LIKE_CPP: &str = "Your spells have been reset.";
pub(super) const LANG_RESET_TALENTS_TEXT_LIKE_CPP: &str = "Your talents have been reset.";
pub(crate) const TRADE_STATUS_PLAYER_BUSY_LIKE_CPP: u8 = 0;
pub(super) const PLAYER_LOCAL_FLAG_WAR_MODE_LIKE_CPP: u32 = 0x0000_0800;
pub(super) const AREA_FLAG_ENEMIES_PVP_FLAGGED_LIKE_CPP: u32 = 0x0000_0010;
pub(super) const AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP: u32 = 0x0000_0080;
pub(super) const AREA_FLAG_CONTESTED_LIKE_CPP: u32 = 0x0004_0000;
pub(super) const AREA_FLAG_COMBAT_ZONE_LIKE_CPP: u32 = 0x0100_0000;
pub(super) const CURRENCY_DB_UNUSED_FLAGS_LIKE_CPP: u8 = 0x13;
pub(crate) type TeleportToOptionsLikeCpp = u32;
pub(crate) const TELE_TO_NONE_LIKE_CPP: TeleportToOptionsLikeCpp = 0x00;
#[allow(dead_code)]
pub(crate) const TELE_TO_GM_MODE_LIKE_CPP: TeleportToOptionsLikeCpp = 0x01;
#[allow(dead_code)]
pub(crate) const TELE_TO_NOT_LEAVE_TRANSPORT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x02;
#[allow(dead_code)]
pub(crate) const TELE_TO_NOT_LEAVE_COMBAT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x04;
#[allow(dead_code)]
pub(crate) const TELE_TO_NOT_UNSUMMON_PET_LIKE_CPP: TeleportToOptionsLikeCpp = 0x08;
pub(crate) const TELE_TO_SPELL_LIKE_CPP: TeleportToOptionsLikeCpp = 0x10;
#[allow(dead_code)]
pub(crate) const TELE_TO_TRANSPORT_TELEPORT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x20;
#[allow(dead_code)]
pub(crate) const TELE_REVIVE_AT_TELEPORT_LIKE_CPP: TeleportToOptionsLikeCpp = 0x40;
pub(crate) const TELE_TO_SEAMLESS_LIKE_CPP: TeleportToOptionsLikeCpp = 0x80;
pub(super) const ATTACK_DISPLAY_DELAY_LIKE_CPP_MS: u32 = 200;
pub(super) const DEFAULT_PLAYER_COMBAT_REACH_LIKE_CPP: f32 = 1.5;
pub(super) const MIN_MELEE_REACH_LIKE_CPP: f32 = 2.0;
pub(super) const NOMINAL_MELEE_RANGE_LIKE_CPP: f32 = 5.0;
pub(super) const SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_LIKE_CPP: u32 = 0x0000_0010;
pub(super) const SUMMON_PROPERTIES_ONLY_VISIBLE_TO_SUMMONER_GROUP_LIKE_CPP: u32 = 0x0001_0000;

pub(super) const QUEST_MENU_ICON_TURN_IN_LIKE_CPP: u8 = 0;
pub(super) const QUEST_MENU_ICON_AVAILABLE_LIKE_CPP: u8 = 2;
pub(super) const QUEST_MENU_ICON_COMPLETE_LIKE_CPP: u8 = 4;

pub(super) const PACKET_SPOOF_BAN_REASON_LIKE_CPP: &str = "DOS (Packet Flooding/Spoofing";
pub(super) const PACKET_SPOOF_BAN_AUTHOR_LIKE_CPP: &str = "Server: AutoDOS";

// First-login standing and faction reputations.
pub(super) const FIRST_LOGIN_START_REPUTATION_STANDING_LIKE_CPP: i32 = 42_999;
pub(super) const FIRST_LOGIN_START_REPUTATION_COMMON_FACTIONS_LIKE_CPP: &[u32] = &[
    942, 935, 936, 1011, 970, 967, 989, 932, 934, 1038, 1077, 1106, 1104, 1090, 1098, 1156, 1073,
    1105, 1119, 1091,
];
pub(super) const FIRST_LOGIN_START_REPUTATION_ALLIANCE_FACTIONS_LIKE_CPP: &[u32] = &[
    72, 47, 69, 930, 730, 978, 54, 946, 1037, 1068, 1126, 1094, 1050,
];
pub(super) const FIRST_LOGIN_START_REPUTATION_HORDE_FACTIONS_LIKE_CPP: &[u32] = &[
    76, 68, 81, 911, 729, 941, 530, 947, 1052, 1067, 1124, 1064, 1085,
];
