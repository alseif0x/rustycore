//! Configuration loader regressions.
//!
//! Moved out of lib.rs under #683; every test is unchanged.

use super::*;

#[test]
fn test_basic_key_value() {
    let store = parse("WorldServerPort = 8085");
    assert_eq!(store.get("WorldServerPort"), Some("8085"));
}

#[test]
fn test_quoted_string_value() {
    let store = parse(r#"DataDir = "/home/server/data""#);
    assert_eq!(store.get("DataDir"), Some("/home/server/data"));
}

#[test]
fn test_quoted_string_with_spaces() {
    let store = parse(r#"Motd = "Welcome to the server!""#);
    assert_eq!(store.get("Motd"), Some("Welcome to the server!"));
}

#[test]
fn test_empty_quoted_string() {
    let store = parse(r#"Empty = """#);
    assert_eq!(store.get("Empty"), Some(""));
}

#[test]
fn test_full_line_comment_ignored() {
    let store = parse("# this is a comment\nPort = 3724");
    assert_eq!(store.get("Port"), Some("3724"));
    // Only one entry in the map.
    assert_eq!(store.values.len(), 1);
}

#[test]
fn test_inline_comment_stripped() {
    let store = parse("Port = 3724 # default bnet port");
    assert_eq!(store.get("Port"), Some("3724"));
}

#[test]
fn test_inline_comment_with_quoted_value() {
    // The inline comment is outside the quotes, so the value is just
    // the quoted content.
    let store = parse(r#"DataDir = "/data" # path to data"#);
    assert_eq!(store.get("DataDir"), Some("/data"));
}

#[test]
fn test_empty_lines_ignored() {
    let content = "\n\n  \nKey = val\n\n";
    let store = parse(content);
    assert_eq!(store.get("Key"), Some("val"));
    assert_eq!(store.values.len(), 1);
}

#[test]
fn test_case_insensitive_lookup() {
    let store = parse("DataDir = /data");
    assert_eq!(store.get("datadir"), Some("/data"));
    assert_eq!(store.get("DATADIR"), Some("/data"));
    assert_eq!(store.get("DataDir"), Some("/data"));
}

#[test]
fn test_parse_integer() {
    let store = parse("Port = 8085");
    let val: u16 = store.get("Port").unwrap().parse().unwrap();
    assert_eq!(val, 8085);
}

#[test]
fn test_parse_float() {
    let store = parse("Rate.XP.Kill = 1.5");
    let val: f64 = store.get("Rate.XP.Kill").unwrap().parse().unwrap();
    assert!((val - 1.5).abs() < f64::EPSILON);
}

#[test]
fn test_get_value_default_missing_key() {
    // Use the global API with a key we know does not exist.
    let val: i32 = get_value_default("__nonexistent_key_42__", 99);
    assert_eq!(val, 99);
}

#[test]
fn test_get_string_default_missing_key() {
    let val = get_string_default("__nonexistent_key_43__", "fallback");
    assert_eq!(val, "fallback");
}

#[test]
fn test_global_load_and_get() {
    let _guard = global_config_lock();
    load_config_from_str("TestKey = 42\nGreeting = \"hello world\"").expect("load failed");

    assert_eq!(get_value::<i32>("TestKey"), Some(42));
    assert_eq!(get_value::<String>("Greeting"), Some("hello world".into()));
    assert_eq!(get_string_default("Greeting", ""), "hello world");
}

#[test]
fn test_global_bool_values_accept_cpp_numeric_literals() {
    let _guard = global_config_lock();
    load_config_from_str(
        r#"
Enabled = 1
Disabled = 0
TrueText = true
FalseText = false
YesText = yes
NoText = no
OnText = on
OffText = off
BadBool = maybe
"#,
    )
    .expect("load failed");

    assert_eq!(get_value::<bool>("Enabled"), Some(true));
    assert_eq!(get_value::<bool>("Disabled"), Some(false));
    assert_eq!(get_value::<bool>("TrueText"), Some(true));
    assert_eq!(get_value::<bool>("FalseText"), Some(false));
    assert_eq!(get_value::<bool>("YesText"), Some(true));
    assert_eq!(get_value::<bool>("NoText"), Some(false));
    assert_eq!(get_value::<bool>("OnText"), Some(true));
    assert_eq!(get_value::<bool>("OffText"), Some(false));
    assert_eq!(get_value::<bool>("BadBool"), None);
    assert!(get_value_default("Enabled", false));
    assert!(!get_value_default("Disabled", true));
    assert!(get_value_default("BadBool", true));
}

#[test]
fn test_file_not_found() {
    let err = load_config("/tmp/__does_not_exist_12345__.conf");
    assert!(err.is_err());
    match err.unwrap_err() {
        ConfigError::FileNotFound(p) => {
            assert!(p.contains("__does_not_exist_12345__"));
        }
        other => panic!("expected FileNotFound, got: {other:?}"),
    }
}

#[test]
fn test_parse_error_no_equals() {
    let mut store = ConfigStore::default();
    let result = store.parse("this line has no equals sign");
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::ParseError { line, .. } => assert_eq!(line, 1),
        other => panic!("expected ParseError, got: {other:?}"),
    }
}

#[test]
fn test_parse_error_empty_key() {
    let mut store = ConfigStore::default();
    let result = store.parse(" = value");
    assert!(result.is_err());
    match result.unwrap_err() {
        ConfigError::ParseError { line, message } => {
            assert_eq!(line, 1);
            assert!(message.contains("empty key"));
        }
        other => panic!("expected ParseError, got: {other:?}"),
    }
}

#[test]
fn test_multiple_keys() {
    let content = r#"
# Server settings
WorldServerPort = 8085
DataDir = "/opt/wow/data"
Rate.XP.Kill = 2.0
LogLevel = 3
"#;
    let store = parse(content);
    assert_eq!(store.get("WorldServerPort"), Some("8085"));
    assert_eq!(store.get("DataDir"), Some("/opt/wow/data"));
    assert_eq!(store.get("Rate.XP.Kill"), Some("2.0"));
    assert_eq!(store.get("LogLevel"), Some("3"));
}

#[test]
fn test_single_section_header_is_flattened_like_cpp() {
    let store = parse(
        r#"
################################################
# Trinity Core World Server configuration file #
################################################
[worldserver]

WorldServerPort = 8085
LoginDatabaseInfo = "127.0.0.1;3306;trinity;trinity;auth"
"#,
    );

    assert_eq!(store.get("WorldServerPort"), Some("8085"));
    assert_eq!(
        store.database_info_default(
            "Login",
            DatabaseInfo::new("fallback", 1, "fallback", "fallback", "fallback"),
        ),
        DatabaseInfo {
            host: "127.0.0.1".to_string(),
            port_or_socket: "3306".to_string(),
            username: "trinity".to_string(),
            password: "trinity".to_string(),
            database: "auth".to_string(),
            ssl: false,
        }
    );
}

#[test]
fn test_bnet_section_header_is_flattened_like_cpp() {
    let store = parse(
        r#"
[bnetserver]
BattlenetPort = 1119
LoginREST.Port = 8081
"#,
    );

    assert_eq!(store.get("BattlenetPort"), Some("1119"));
    assert_eq!(store.get("LoginREST.Port"), Some("8081"));
}

#[test]
fn test_reload_replaces_values() {
    let mut store = ConfigStore::default();
    store.parse("Key = old").unwrap();
    assert_eq!(store.get("Key"), Some("old"));

    store.parse("Key = new").unwrap();
    assert_eq!(store.get("Key"), Some("new"));
}

#[test]
fn test_value_containing_equals() {
    let store = parse(r#"ConnString = "server=localhost;port=3306""#);
    assert_eq!(store.get("ConnString"), Some("server=localhost;port=3306"));
}

#[test]
fn test_quoted_value_with_hash() {
    let store = parse(r##"Color = "#FF0000""##);
    assert_eq!(store.get("Color"), Some("#FF0000"));
}

#[test]
fn test_database_info_semicolon_parser() {
    let info = parse_database_info(
        "LoginDatabaseInfo",
        "127.0.0.1;3306;trinity;trinity;auth;ssl",
    )
    .expect("db info should parse");

    assert_eq!(info.host, "127.0.0.1");
    assert_eq!(info.port_or_socket, "3306");
    assert_eq!(info.username, "trinity");
    assert_eq!(info.password, "trinity");
    assert_eq!(info.database, "auth");
    assert!(info.ssl);
}

#[test]
fn test_database_info_accepts_unix_socket_like_cpp() {
    let info = parse_database_info(
        "WorldDatabaseInfo",
        ".;/var/run/mysqld/mysqld.sock;trinity;trinity;world",
    )
    .expect("db info should parse");

    assert_eq!(info.host, ".");
    assert_eq!(info.port_or_socket, "/var/run/mysqld/mysqld.sock");
    assert_eq!(info.database, "world");
}

#[test]
fn test_get_database_info_uses_canonical_key_only() {
    let store = parse(
        r#"
LoginDatabaseInfo = "127.0.0.1;3306;trinity;trinity;auth"
LoginDatabaseInfo.Host = "legacy"
"#,
    );

    let info = store.database_info_default(
        "Login",
        DatabaseInfo::new("fallback", 1, "fallback", "fallback", "fallback"),
    );

    assert_eq!(info.host, "127.0.0.1");
    assert_eq!(info.port_or_socket, "3306");
    assert_eq!(info.username, "trinity");
    assert_eq!(info.password, "trinity");
    assert_eq!(info.database, "auth");
}

#[test]
fn test_get_database_info_ignores_legacy_split_subkeys() {
    let store = parse(
        r#"
LoginDatabaseInfo.Host = "127.0.0.2"
LoginDatabaseInfo.Port = 3307
LoginDatabaseInfo.Username = "legacy_user"
LoginDatabaseInfo.Password = "legacy_pass"
LoginDatabaseInfo.Database = "legacy_auth"
"#,
    );

    let info = store.database_info_default(
        "Login",
        DatabaseInfo::new("fallback", 1, "fallback", "fallback", "fallback"),
    );

    assert_eq!(info.host, "fallback");
    assert_eq!(info.port_or_socket, "1");
    assert_eq!(info.username, "fallback");
    assert_eq!(info.password, "fallback");
    assert_eq!(info.database, "fallback");
}

#[test]
fn test_env_key_for_ini_key_matches_trinity_examples() {
    assert_eq!(env_key_for_ini_key("SomeConfig"), "TC_SOME_CONFIG");
    assert_eq!(
        env_key_for_ini_key("myNestedConfig.opt1"),
        "TC_MY_NESTED_CONFIG_OPT_1"
    );
    assert_eq!(
        env_key_for_ini_key("LogDB.Opt.ClearTime"),
        "TC_LOG_DB_OPT_CLEAR_TIME"
    );
}

#[test]
fn test_env_override_provider_overrides_scalar_keys() {
    let mut store = parse("WorldServerPort = 8085\n");
    let overridden = store.override_with_env_provider(|key| {
        (key == "TC_WORLD_SERVER_PORT").then(|| "9100".to_string())
    });

    assert_eq!(overridden, vec!["WorldServerPort"]);
    assert_eq!(store.get("WorldServerPort"), Some("9100"));
}

#[test]
fn test_load_config_with_fallbacks_loads_dir_overlays() {
    let _guard = global_config_lock();
    let root = unique_temp_dir("load_config_with_fallbacks");
    let conf_dir = root.join("worldserver.conf.d");
    fs::create_dir_all(conf_dir.join("nested")).expect("mkdir failed");

    let primary = root.join("worldserver.conf");
    let overlay = conf_dir.join("nested").join("override.conf");
    let ignored = conf_dir.join("ignored.txt");

    fs::write(
        &primary,
        r#"
WorldServerPort = 8085
LoginDatabaseInfo = "127.0.0.1;3306;trinity;trinity;auth"
"#,
    )
    .expect("write primary failed");
    fs::write(&overlay, "WorldServerPort = 9000\n").expect("write overlay failed");
    fs::write(&ignored, "WorldServerPort = 1\n").expect("write ignored failed");

    let report = load_config_with_fallbacks(
        &[primary.to_str().expect("utf8 path")],
        conf_dir.to_str().expect("utf8 path"),
    )
    .expect("config should load");

    assert_eq!(report.candidate_index, 0);
    assert_eq!(report.loaded_files.len(), 1);
    assert_eq!(get_value::<u16>("WorldServerPort"), Some(9000));

    let info = get_database_info_default(
        "Login",
        DatabaseInfo::new("fallback", 1, "fallback", "fallback", "fallback"),
    );
    assert_eq!(info.database, "auth");

    fs::remove_dir_all(root).expect("cleanup failed");
}

#[test]
fn test_load_config_with_fallbacks_uses_lowercase_before_legacy_name() {
    let _guard = global_config_lock();
    let root = unique_temp_dir("load_config_candidate_order");
    let lower = root.join("bnetserver.conf");
    let legacy = root.join("BNetServer.conf");

    fs::write(&lower, "BattlenetPort = 1119\n").expect("write lower failed");
    fs::write(&legacy, "BattlenetPort = 2222\n").expect("write legacy failed");

    let report = load_config_with_fallbacks(
        &[
            lower.to_str().expect("utf8 path"),
            legacy.to_str().expect("utf8 path"),
        ],
        root.join("bnetserver.conf.d").to_str().expect("utf8 path"),
    )
    .expect("config should load");

    assert_eq!(report.candidate_index, 0);
    assert_eq!(get_value::<u16>("BattlenetPort"), Some(1119));

    fs::remove_dir_all(root).expect("cleanup failed");
}

#[test]
fn test_world_config_registry_covers_cpp_inventory() {
    let registry = world_config_registry();
    assert_eq!(registry.len(), 346);
    assert_eq!(
        registry
            .iter()
            .filter(|entry| entry.kind == WorldConfigKind::Bool)
            .count(),
        97
    );
    assert_eq!(
        registry
            .iter()
            .filter(|entry| entry.kind == WorldConfigKind::Float)
            .count(),
        43
    );
    assert_eq!(
        registry
            .iter()
            .filter(|entry| entry.kind == WorldConfigKind::Int)
            .count(),
        205
    );
    assert_eq!(
        registry
            .iter()
            .filter(|entry| entry.kind == WorldConfigKind::Int64)
            .count(),
        1
    );
    let creature_aggro = registry
        .iter()
        .find(|entry| entry.enum_name == "RATE_CREATURE_AGGRO")
        .expect("RATE_CREATURE_AGGRO must be represented");
    assert_eq!(creature_aggro.key.as_deref(), Some("Rate.Creature.Aggro"));
    assert_eq!(
        creature_aggro.default_value.as_ref(),
        Some(&WorldConfigValue::Float(1.0))
    );
    let xp_explore = registry
        .iter()
        .find(|entry| entry.enum_name == "RATE_XP_EXPLORE")
        .expect("RATE_XP_EXPLORE must be represented");
    assert_eq!(xp_explore.key.as_deref(), Some("Rate.XP.Explore"));
    assert_eq!(
        xp_explore.default_value.as_ref(),
        Some(&WorldConfigValue::Float(1.0))
    );
    for (enum_name, key) in [
        ("RATE_REST_INGAME", "Rate.Rest.InGame"),
        (
            "RATE_REST_OFFLINE_IN_TAVERN_OR_CITY",
            "Rate.Rest.Offline.InTavernOrCity",
        ),
        (
            "RATE_REST_OFFLINE_IN_WILDERNESS",
            "Rate.Rest.Offline.InWilderness",
        ),
    ] {
        let entry = registry
            .iter()
            .find(|entry| entry.enum_name == enum_name)
            .expect("rest rate must be represented");
        assert_eq!(entry.key.as_deref(), Some(key));
        assert_eq!(
            entry.default_value.as_ref(),
            Some(&WorldConfigValue::Float(1.0))
        );
    }
}

#[test]
fn test_world_config_registry_resolves_cpp_symbolic_defaults() {
    let registry = world_config_registry();

    assert_eq!(
        registry
            .iter()
            .find(|entry| entry.enum_name == "CONFIG_INTERVAL_SAVE")
            .and_then(|entry| entry.default_value.as_ref()),
        Some(&WorldConfigValue::Int(900_000))
    );
    assert_eq!(
        registry
            .iter()
            .find(|entry| entry.enum_name == "CONFIG_EXPANSION")
            .and_then(|entry| entry.default_value.as_ref()),
        Some(&WorldConfigValue::Int(2))
    );
    assert_eq!(
        registry
            .iter()
            .find(|entry| entry.enum_name == "CONFIG_PACKET_SPOOF_POLICY")
            .and_then(|entry| entry.default_value.as_ref()),
        Some(&WorldConfigValue::Int(1))
    );
    assert_eq!(
        registry
            .iter()
            .find(|entry| entry.enum_name == "CONFIG_GUILD_NEWS_LOG_COUNT")
            .and_then(|entry| entry.default_value.as_ref()),
        Some(&WorldConfigValue::Int(250))
    );
}

#[test]
fn test_world_config_registry_tracks_rows_without_literal_cpp_load() {
    let registry = world_config_registry();
    let missing_defaults: Vec<_> = registry
        .iter()
        .filter(|entry| entry.default_value.is_none())
        .map(|entry| entry.enum_name.as_str())
        .collect();

    assert_eq!(
        missing_defaults,
        vec![
            "CONFIG_CURRENCY_START_APEXIS_CRYSTALS",
            "CONFIG_CURRENCY_MAX_APEXIS_CRYSTALS",
            "CONFIG_CURRENCY_START_JUSTICE_POINTS",
            "CONFIG_CURRENCY_MAX_JUSTICE_POINTS",
            "CONFIG_INSTANT_LOGOUT",
            "CONFIG_PLAYER_ALLOW_COMMANDS",
            "CONFIG_CLIENTCACHE_VERSION",
        ]
    );
}

#[test]
fn test_load_world_config_values_uses_config_and_defaults() {
    let _guard = global_config_lock();
    load_config_from_str(
        r#"
AddonChannel = 0
Support.Enabled = 0
Support.BugsEnabled = 1
Support.ComplaintsEnabled = 1
Support.SuggestionsEnabled = 1
MaxGroupXPDistance = 120.5
WorldServerPort = 8088
CharacterCreating.Disabled.RaceMask = 12
Rate.Rest.InGame = 2.5
Rate.Rest.Offline.InTavernOrCity = 3.5
Rate.Rest.Offline.InWilderness = 0.5
"#,
    )
    .expect("load failed");

    let values = load_world_config_values();
    assert_eq!(values.get_bool("CONFIG_ADDON_CHANNEL"), Some(false));
    assert_eq!(values.get_bool("CONFIG_SUPPORT_ENABLED"), Some(false));
    assert_eq!(values.get_bool("CONFIG_SUPPORT_BUGS_ENABLED"), Some(true));
    assert_eq!(
        values.get_bool("CONFIG_SUPPORT_COMPLAINTS_ENABLED"),
        Some(true)
    );
    assert_eq!(
        values.get_bool("CONFIG_SUPPORT_SUGGESTIONS_ENABLED"),
        Some(true)
    );
    assert_eq!(values.get_float("CONFIG_GROUP_XP_DISTANCE"), Some(120.5));
    assert_eq!(values.get_int("CONFIG_PORT_WORLD"), Some(8088));
    assert_eq!(
        values.get_int64("CONFIG_CHARACTER_CREATING_DISABLED_RACEMASK"),
        Some(12)
    );
    assert_eq!(values.get_bool("CONFIG_ENABLE_MMAPS"), Some(true));
    assert_eq!(
        values.get_bool("CONFIG_SUPPORT_TICKETS_ENABLED"),
        Some(false)
    );
    assert_eq!(values.get_int("CONFIG_INTERVAL_SAVE"), Some(900_000));
    assert_eq!(values.get_float("RATE_REST_INGAME"), Some(2.5));
    assert_eq!(
        values.get_float("RATE_REST_OFFLINE_IN_TAVERN_OR_CITY"),
        Some(3.5)
    );
    assert_eq!(
        values.get_float("RATE_REST_OFFLINE_IN_WILDERNESS"),
        Some(0.5)
    );
    assert_eq!(values.get_int("CONFIG_MAX_PRIMARY_TRADE_SKILL"), Some(2));
}

#[test]
fn test_load_world_config_values_applies_cpp_validations() {
    let _guard = global_config_lock();
    load_config_from_str(
        r#"
Compression = 99
Auction.SearchDelay = 50
Auction.TaintedSearchDelay = 20000
GridUnload = 1
BaseMapLoadAllGrids = 1
InstanceMapLoadAllGrids = 1
PlayerSave.Stats.MinLevel = 124
GridCleanUpDelay = 1
MapUpdateInterval = 0
SocketTimeOutTime = 900000
SocketTimeOutTimeActive = 60000
MinQuestScaledXPRatio = 101
MinCreatureScaledXPRatio = 101
MinDiscoveredScaledXPRatio = 101
MinPlayerName = 13
MinCharterName = 25
MinPetName = 0
CharactersPerRealm = 0
CharactersPerAccount = 60
CharacterCreating.EvokersPerRealm = 11
SkipCinematics = 3
MaxPlayerLevel = 90
StartPlayerLevel = 0
StartDeathKnightPlayerLevel = 200
StartDemonHunterPlayerLevel = 200
StartEvokerPlayerLevel = 200
StartAlliedRacePlayerLevel = 200
StartPlayerMoney = 2147483647
Currency.ResetHour = 24
Currency.ResetDay = 7
Currency.ResetInterval = 0
RecruitAFriend.MaxLevel = 91
Quests.DailyResetTime = 24
Quests.WeeklyResetWDay = 7
MaxPrimaryTradeSkill = 12
MinPetitionSigns = 5
GM.StartLevel = 0
CleanOldMailTime = 24
UpdateUptimeInterval = 0
LogDB.Opt.ClearInterval = 0
MaxOverspeedPings = 1
Quests.LowLevelHideDiff = 124
Quests.HighLevelHideDiff = 124
Battleground.Random.ResetHour = 24
Calendar.DeleteOldEventsHour = 24
Guild.ResetHour = 24
Battleground.ReportAFK = 10
Guild.NewsLogRecordsCount = 251
Guild.EventLogRecordsCount = 101
Guild.BankEventLogRecordsCount = 26
NoGrayAggro.Above = 80
NoGrayAggro.Below = 90
Respawn.DynamicMode = 2
Respawn.GuidWarnLevel = 16777216
Respawn.GuidAlertLevel = 16777216
Respawn.RestartQuietTime = 24
Respawn.DynamicRateCreature = -1.0
Respawn.DynamicRateGameObject = -1.0
PvPToken.ItemCount = 0
PacketSpoof.BanMode = 1
"#,
    )
    .expect("load failed");

    let values = load_world_config_values();
    assert_eq!(values.get_int("CONFIG_COMPRESSION"), Some(1));
    assert_eq!(values.get_int("CONFIG_AUCTION_SEARCH_DELAY"), Some(300));
    assert_eq!(
        values.get_int("CONFIG_AUCTION_TAINTED_SEARCH_DELAY"),
        Some(3_000)
    );
    assert_eq!(values.get_bool("CONFIG_BASEMAP_LOAD_GRIDS"), Some(false));
    assert_eq!(
        values.get_bool("CONFIG_INSTANCEMAP_LOAD_GRIDS"),
        Some(false)
    );
    assert_eq!(values.get_int("CONFIG_MIN_LEVEL_STAT_SAVE"), Some(0));
    assert_eq!(values.get_int("CONFIG_INTERVAL_GRIDCLEAN"), Some(60_000));
    assert_eq!(values.get_int("CONFIG_INTERVAL_MAPUPDATE"), Some(1));
    assert_eq!(values.get_int("CONFIG_SOCKET_TIMEOUTTIME"), Some(900));
    assert_eq!(values.get_int("CONFIG_SOCKET_TIMEOUTTIME_ACTIVE"), Some(60));
    assert_eq!(values.get_int("CONFIG_MIN_QUEST_SCALED_XP_RATIO"), Some(0));
    assert_eq!(
        values.get_int("CONFIG_MIN_CREATURE_SCALED_XP_RATIO"),
        Some(0)
    );
    assert_eq!(
        values.get_int("CONFIG_MIN_DISCOVERED_SCALED_XP_RATIO"),
        Some(0)
    );
    assert_eq!(values.get_int("CONFIG_MIN_PLAYER_NAME"), Some(2));
    assert_eq!(values.get_int("CONFIG_MIN_CHARTER_NAME"), Some(2));
    assert_eq!(values.get_int("CONFIG_MIN_PET_NAME"), Some(2));
    assert_eq!(values.get_int("CONFIG_CHARACTERS_PER_REALM"), Some(200));
    assert_eq!(values.get_int("CONFIG_CHARACTERS_PER_ACCOUNT"), Some(200));
    assert_eq!(
        values.get_int("CONFIG_CHARACTER_CREATING_EVOKERS_PER_REALM"),
        Some(1)
    );
    assert_eq!(values.get_int("CONFIG_SKIP_CINEMATICS"), Some(0));
    assert_eq!(values.get_int("CONFIG_MAX_PLAYER_LEVEL"), Some(90));
    assert_eq!(values.get_int("CONFIG_START_PLAYER_LEVEL"), Some(1));
    assert_eq!(
        values.get_int("CONFIG_START_DEATH_KNIGHT_PLAYER_LEVEL"),
        Some(90)
    );
    assert_eq!(
        values.get_int("CONFIG_START_DEMON_HUNTER_PLAYER_LEVEL"),
        Some(90)
    );
    assert_eq!(values.get_int("CONFIG_START_EVOKER_PLAYER_LEVEL"), Some(90));
    assert_eq!(values.get_int("CONFIG_START_ALLIED_RACE_LEVEL"), Some(90));
    assert_eq!(
        values.get_int("CONFIG_START_PLAYER_MONEY"),
        Some(2_147_483_646)
    );
    assert_eq!(values.get_int("CONFIG_CURRENCY_RESET_HOUR"), Some(3));
    assert_eq!(values.get_int("CONFIG_CURRENCY_RESET_DAY"), Some(3));
    assert_eq!(values.get_int("CONFIG_CURRENCY_RESET_INTERVAL"), Some(7));
    assert_eq!(
        values.get_int("CONFIG_MAX_RECRUIT_A_FRIEND_BONUS_PLAYER_LEVEL"),
        Some(85)
    );
    assert_eq!(
        values.get_int("CONFIG_DAILY_QUEST_RESET_TIME_HOUR"),
        Some(3)
    );
    assert_eq!(
        values.get_int("CONFIG_WEEKLY_QUEST_RESET_TIME_WDAY"),
        Some(3)
    );
    assert_eq!(values.get_int("CONFIG_MAX_PRIMARY_TRADE_SKILL"), Some(2));
    assert_eq!(values.get_int("CONFIG_MIN_PETITION_SIGNS"), Some(4));
    assert_eq!(values.get_int("CONFIG_START_GM_LEVEL"), Some(1));
    assert_eq!(values.get_int("CONFIG_CLEAN_OLD_MAIL_TIME"), Some(4));
    assert_eq!(values.get_int("CONFIG_UPTIME_UPDATE"), Some(10));
    assert_eq!(values.get_int("CONFIG_LOGDB_CLEARINTERVAL"), Some(10));
    assert_eq!(values.get_int("CONFIG_MAX_OVERSPEED_PINGS"), Some(2));
    assert_eq!(
        values.get_int("CONFIG_QUEST_LOW_LEVEL_HIDE_DIFF"),
        Some(123)
    );
    assert_eq!(
        values.get_int("CONFIG_QUEST_HIGH_LEVEL_HIDE_DIFF"),
        Some(123)
    );
    assert_eq!(values.get_int("CONFIG_RANDOM_BG_RESET_HOUR"), Some(6));
    assert_eq!(
        values.get_int("CONFIG_CALENDAR_DELETE_OLD_EVENTS_HOUR"),
        Some(6)
    );
    assert_eq!(values.get_int("CONFIG_GUILD_RESET_HOUR"), Some(6));
    assert_eq!(values.get_int("CONFIG_BATTLEGROUND_REPORT_AFK"), Some(3));
    assert_eq!(values.get_int("CONFIG_GUILD_NEWS_LOG_COUNT"), Some(250));
    assert_eq!(values.get_int("CONFIG_GUILD_EVENT_LOG_COUNT"), Some(100));
    assert_eq!(
        values.get_int("CONFIG_GUILD_BANK_EVENT_LOG_COUNT"),
        Some(25)
    );
    assert_eq!(values.get_int("CONFIG_NO_GRAY_AGGRO_ABOVE"), Some(80));
    assert_eq!(values.get_int("CONFIG_NO_GRAY_AGGRO_BELOW"), Some(80));
    assert_eq!(values.get_int("CONFIG_RESPAWN_DYNAMICMODE"), Some(0));
    assert_eq!(
        values.get_int("CONFIG_RESPAWN_GUIDWARNLEVEL"),
        Some(12_000_000)
    );
    assert_eq!(
        values.get_int("CONFIG_RESPAWN_GUIDALERTLEVEL"),
        Some(16_000_000)
    );
    assert_eq!(values.get_int("CONFIG_RESPAWN_RESTARTQUIETTIME"), Some(3));
    assert_eq!(
        values.get_float("CONFIG_RESPAWN_DYNAMICRATE_CREATURE"),
        Some(10.0)
    );
    assert_eq!(
        values.get_float("CONFIG_RESPAWN_DYNAMICRATE_GAMEOBJECT"),
        Some(10.0)
    );
    assert_eq!(values.get_int("CONFIG_PVP_TOKEN_COUNT"), Some(1));
    assert_eq!(values.get_int("CONFIG_PACKET_SPOOF_BANMODE"), Some(0));
}

#[test]
fn test_max_primary_trade_skill_accepts_documented_range_and_repairs_negative() {
    let _guard = global_config_lock();

    for (configured, expected) in [("0", 0), ("1", 1), ("2", 2), ("11", 11), ("-1", 2)] {
        load_config_from_str(&format!("MaxPrimaryTradeSkill = {configured}\n"))
            .expect("load failed");
        assert_eq!(
            load_world_config_values().get_int("CONFIG_MAX_PRIMARY_TRADE_SKILL"),
            Some(expected),
            "configured value {configured}"
        );
    }
}

#[test]
fn test_load_world_config_values_handles_cpp_signed_int_edges() {
    let _guard = global_config_lock();
    load_config_from_str(
        r#"
Quests.LowLevelHideDiff = -1
ClientCacheVersion = 77
"#,
    )
    .expect("load failed");

    let values = load_world_config_values();
    assert_eq!(
        values.get_int("CONFIG_QUEST_LOW_LEVEL_HIDE_DIFF"),
        Some(123)
    );
    assert_eq!(values.get_int("CONFIG_CLIENTCACHE_VERSION"), Some(77));

    load_config_from_str("ClientCacheVersion = -1").expect("load failed");
    let values = load_world_config_values();
    assert_eq!(values.get_int("CONFIG_CLIENTCACHE_VERSION"), None);
}
