//! CLI, configuration resolution, and startup policy.

use super::super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorldServerCliLikeCpp {
    pub(crate) config_file: Option<PathBuf>,
    pub(crate) config_dir: PathBuf,
    pub(crate) update_databases_only: bool,
    pub(crate) show_version: bool,
    pub(crate) show_help: bool,
}

impl WorldServerCliLikeCpp {
    pub(crate) fn parse_from(args: impl IntoIterator<Item = String>) -> Self {
        let mut cli = Self::default();
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => cli.show_help = true,
                "--version" | "-v" => cli.show_version = true,
                "--update-databases-only" | "-u" => cli.update_databases_only = true,
                "--config" | "-c" => {
                    if let Some(value) = args.next() {
                        cli.config_file = Some(PathBuf::from(value));
                    }
                }
                "--config-dir" | "-cd" => {
                    if let Some(value) = args.next() {
                        cli.config_dir = PathBuf::from(value);
                    }
                }
                _ => {
                    if let Some(value) = arg.strip_prefix("--config=") {
                        cli.config_file = Some(PathBuf::from(value));
                    } else if let Some(value) = arg.strip_prefix("--config-dir=") {
                        cli.config_dir = PathBuf::from(value);
                    }
                }
            }
        }

        cli
    }
}

impl Default for WorldServerCliLikeCpp {
    fn default() -> Self {
        Self {
            config_file: None,
            config_dir: PathBuf::from(WORLD_CONFIG_DIR),
            update_databases_only: false,
            show_version: false,
            show_help: false,
        }
    }
}

pub(crate) fn worldserver_cli_help_like_cpp() -> &'static str {
    "Allowed options:\n  -h [ --help ]                  print usage message\n  -v [ --version ]               print version build info\n  -c [ --config ] <arg>          use <arg> as configuration file\n  -cd [ --config-dir ] <arg>     use <arg> as directory with additional config files\n  -u [ --update-databases-only ] updates databases only\n"
}

pub(crate) fn worldserver_full_version_like_cpp() -> String {
    let revision = worldserver_revision_like_cpp();
    format!(
        "RustyCore World Server {} (rev {revision})",
        env!("CARGO_PKG_VERSION")
    )
}

pub(crate) fn worldserver_revision_like_cpp() -> &'static str {
    option_env!("GIT_HASH")
        .or(option_env!("VERGEN_GIT_SHA"))
        .unwrap_or("unknown")
}

pub(crate) fn load_world_config(cli: &WorldServerCliLikeCpp) -> Result<LoadReport> {
    let config_dir = cli.config_dir.to_string_lossy();
    if let Some(config_file) = &cli.config_file {
        let config_file = config_file.to_string_lossy();
        return load_world_config_from(&[config_file.as_ref()], config_dir.as_ref());
    }

    load_world_config_from(WORLD_CONFIG_CANDIDATES, config_dir.as_ref())
}

pub(crate) fn load_world_config_from(
    config_candidates: &[&str],
    config_dir: &str,
) -> Result<LoadReport> {
    let loaded_config = wow_config::load_config_with_fallbacks(config_candidates, config_dir)
        .context("Failed to load worldserver.conf")?;

    if loaded_config.candidate_index > 1 {
        tracing::warn!(
            config = %loaded_config.initial_file,
            "Using legacy Rust config filename; prefer worldserver.conf"
        );
    }

    Ok(loaded_config)
}

pub(crate) fn log_database_target_like_cpp(kind: &str, info: &DatabaseInfo) {
    info!(
        database_kind = kind,
        host = %info.host,
        port_or_socket = %info.port_or_socket,
        database = %info.database,
        "Connecting to database"
    );
}

pub(crate) fn log_startup_banner_like_cpp(config_report: &LoadReport) {
    info!("{}", worldserver_full_version_like_cpp());
    info!(
        config = %config_report.initial_file,
        "Using configuration file"
    );
    for loaded_file in &config_report.loaded_files {
        info!(config = %loaded_file, "Using additional configuration file");
    }
    for overridden_key in &config_report.overridden_keys {
        info!(
            key = %overridden_key,
            "Configuration field was overridden with environment variable"
        );
    }
    info!(
        tls_backend = "rustls",
        rustls = "0.23",
        tokio_rustls = "0.26",
        sqlx = "0.8",
        "Using Rust dependency versions"
    );
}

pub(crate) fn database_pool_size_like_cpp(name: &str) -> u32 {
    let worker_threads =
        database_thread_count_like_cpp(&format!("{name}Database.WorkerThreads"), 1);
    let synch_threads = database_thread_count_like_cpp(&format!("{name}Database.SynchThreads"), 1);
    worker_threads + synch_threads
}

pub(crate) fn updates_auto_setup_enabled_like_cpp() -> bool {
    let auto_setup = wow_config::get_string_default("Updates.AutoSetup", "1");
    auto_setup != "0" && !auto_setup.eq_ignore_ascii_case("false")
}

pub(crate) fn updates_database_mask_like_cpp() -> u32 {
    wow_config::get_value_default("Updates.EnableDatabases", DATABASE_MASK_ALL_LIKE_CPP)
}

pub(crate) fn updates_enabled_for_database_like_cpp(update_mask: u32, database_flag: u32) -> bool {
    update_mask & database_flag != 0
}

pub(crate) fn database_auto_create_enabled_like_cpp(
    auto_setup: bool,
    update_mask: u32,
    database_flag: u32,
) -> bool {
    auto_setup && updates_enabled_for_database_like_cpp(update_mask, database_flag)
}

pub(crate) fn database_thread_count_like_cpp(key: &str, default: u32) -> u32 {
    let value = wow_config::get_value_default::<u32>(key, default);
    if !(1..=32).contains(&value) {
        warn!("{key}={value} is outside 1..32; using {default}");
        return default;
    }
    value
}

pub(crate) fn legacy_creature_global_runtime_enabled_from_config_like_cpp() -> bool {
    wow_config::get_value::<u8>(RUSTYCORE_LEGACY_CREATURE_GLOBAL_RUNTIME_CONFIG)
        .map(|value| value != 0)
        .unwrap_or(true)
}

pub(crate) fn realm_id_like_cpp() -> Result<u16> {
    let Some(realm_id) = wow_config::get_value::<u16>("RealmID") else {
        bail!("Realm ID not defined in configuration file");
    };
    if realm_id == 0 {
        bail!("Realm ID not defined in configuration file");
    }
    Ok(realm_id)
}

pub(crate) fn world_config_u16(configs: &WorldConfigSet, enum_name: &str, default: u16) -> u16 {
    configs
        .get_int(enum_name)
        .map(|value| value as u16)
        .unwrap_or(default)
}

pub(crate) fn world_config_u8(configs: &WorldConfigSet, enum_name: &str, default: u8) -> u8 {
    configs
        .get_int(enum_name)
        .map(|value| value as u8)
        .unwrap_or(default)
}

pub(crate) fn max_primary_trade_skills_like_cpp(configs: &WorldConfigSet) -> u8 {
    configs
        .get_int("CONFIG_MAX_PRIMARY_TRADE_SKILL")
        .filter(|configured| {
            *configured <= u32::from(wow_config::MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP)
        })
        .and_then(|configured| u8::try_from(configured).ok())
        .unwrap_or(wow_config::DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP)
}

pub(crate) fn world_config_u32(configs: &WorldConfigSet, enum_name: &str, default: u32) -> u32 {
    configs
        .get_int(enum_name)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(default)
}

pub(crate) fn world_config_f32(configs: &WorldConfigSet, enum_name: &str, default: f32) -> f32 {
    configs.get_float(enum_name).unwrap_or(default)
}

pub(crate) fn world_config_bool(configs: &WorldConfigSet, enum_name: &str, default: bool) -> bool {
    configs.get_bool(enum_name).unwrap_or(default)
}

pub(crate) fn declined_names_used_for_realm_category_like_cpp(
    configured: bool,
    realm_zone: u32,
    categories: &wow_data::CfgCategoriesStore,
) -> bool {
    configured
        || categories.get(realm_zone).is_some_and(|category| {
            category.create_charset_mask & CFG_CATEGORIES_CHARSET_RUSSIAN_LIKE_CPP != 0
        })
}

/// Mirrors C++ `World::LoadConfigSettings`: Russian realm categories always
/// enable declined names, regardless of the explicit `DeclinedNames` value.
pub(crate) fn declined_names_used_like_cpp(
    configs: &WorldConfigSet,
    categories: &wow_data::CfgCategoriesStore,
) -> bool {
    declined_names_used_for_realm_category_like_cpp(
        world_config_bool(configs, "CONFIG_DECLINED_NAMES_USED", false),
        world_config_u32(
            configs,
            "CONFIG_REALM_ZONE",
            HARDCODED_DEVELOPMENT_REALM_CATEGORY_ID_LIKE_CPP,
        ),
        categories,
    )
}

pub(crate) fn min_world_update_time_ms_like_cpp() -> u32 {
    wow_config::get_value_default("MinWorldUpdateTime", 1_u32)
}

pub(crate) fn max_core_stuck_time_secs_like_cpp() -> u32 {
    wow_config::get_value_default("MaxCoreStuckTime", 60_u32)
}

pub(crate) fn max_core_stuck_time_ms_like_cpp() -> u32 {
    max_core_stuck_time_secs_like_cpp().wrapping_mul(1_000)
}

pub(crate) fn max_skill_value_like_cpp(configs: &WorldConfigSet) -> u32 {
    let max_player_level = u32::from(world_config_u8(configs, "CONFIG_MAX_PLAYER_LEVEL", 80));
    if max_player_level > 60 {
        300 + ((max_player_level - 60) * 75) / 10
    } else {
        max_player_level * 5
    }
}

pub(crate) fn mmap_runtime_config_like_cpp(
    configs: &WorldConfigSet,
    disabled_map_ids: HashSet<u32>,
) -> MMapRuntimeConfigLikeCpp {
    MMapRuntimeConfigLikeCpp {
        data_dir: wow_config::get_string_default("DataDir", "./Data"),
        enabled: world_config_bool(configs, "CONFIG_ENABLE_MMAPS", true),
        disabled_map_ids,
    }
}

pub(crate) async fn load_disable_mgr_like_cpp(
    world_db: &WorldDatabase,
    map_store: &wow_data::MapStore,
    map_difficulty_store: &wow_data::MapDifficultyStore,
    spell_store: &wow_data::SpellStore,
    quest_store: &wow_data::quest::QuestStore,
    criteria_store: &wow_data::Db2IdStore,
    battlemaster_list_store: &wow_data::Db2IdStore,
) -> Result<wow_data::DisableMgrLikeCpp> {
    let (disable_mgr, _) = wow_data::DisableMgrLikeCpp::load_like_cpp(
        world_db,
        wow_data::DisableMgrRefsLikeCpp {
            map_store: Some(map_store),
            map_difficulty_store: Some(map_difficulty_store),
            spell_store: Some(spell_store),
            quest_store: Some(quest_store),
            criteria_store: Some(criteria_store),
            battlemaster_list_store: Some(battlemaster_list_store),
            ..Default::default()
        },
    )
    .await
    .context("Failed to query C++ disables")?;

    Ok(disable_mgr)
}

pub(crate) fn loot_drop_rates_like_cpp(configs: &WorldConfigSet) -> LootDropRatesLikeCpp {
    LootDropRatesLikeCpp {
        item_poor: world_config_f32(configs, "RATE_DROP_ITEM_POOR", 1.0),
        item_normal: world_config_f32(configs, "RATE_DROP_ITEM_NORMAL", 1.0),
        item_uncommon: world_config_f32(configs, "RATE_DROP_ITEM_UNCOMMON", 1.0),
        item_rare: world_config_f32(configs, "RATE_DROP_ITEM_RARE", 1.0),
        item_epic: world_config_f32(configs, "RATE_DROP_ITEM_EPIC", 1.0),
        item_legendary: world_config_f32(configs, "RATE_DROP_ITEM_LEGENDARY", 1.0),
        item_artifact: world_config_f32(configs, "RATE_DROP_ITEM_ARTIFACT", 1.0),
        item_referenced: world_config_f32(configs, "RATE_DROP_ITEM_REFERENCED", 1.0),
        item_referenced_amount: world_config_f32(configs, "RATE_DROP_ITEM_REFERENCED_AMOUNT", 1.0),
        money: world_config_f32(configs, "RATE_DROP_MONEY", 1.0),
        corpse_decay_looted: world_config_f32(configs, "RATE_CORPSE_DECAY_LOOTED", 0.5),
    }
}

pub(crate) fn reputation_rates_like_cpp(configs: &WorldConfigSet) -> ReputationRatesLikeCpp {
    ReputationRatesLikeCpp {
        gain: world_config_f32(configs, "RATE_REPUTATION_GAIN", 1.0),
        low_level_kill: world_config_f32(configs, "RATE_REPUTATION_LOWLEVEL_KILL", 1.0),
        low_level_quest: world_config_f32(configs, "RATE_REPUTATION_LOWLEVEL_QUEST", 1.0),
        recruit_a_friend_bonus: world_config_f32(
            configs,
            "RATE_REPUTATION_RECRUIT_A_FRIEND_BONUS",
            0.1,
        ),
        recruit_a_friend_distance: world_config_f32(
            configs,
            "CONFIG_MAX_RECRUIT_A_FRIEND_DISTANCE",
            100.0,
        ),
    }
}

pub(crate) fn repair_cost_rate_like_cpp(configs: &WorldConfigSet) -> f32 {
    world_config_f32(configs, "RATE_REPAIRCOST", 1.0).max(0.0)
}

pub(crate) fn reset_schedule_like_cpp(configs: &WorldConfigSet) -> ResetSchedule {
    ResetSchedule {
        hour: world_config_u8(configs, "CONFIG_RESET_SCHEDULE_HOUR", 8),
        week_day: world_config_u8(configs, "CONFIG_RESET_SCHEDULE_WEEK_DAY", 2),
    }
}
