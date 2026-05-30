//! WoW server `.conf` file parser.
//!
//! Parses configuration files that use the `Key = Value` format found in
//! TrinityCore/RustyCore `.conf.dist` files. Provides a global singleton
//! [`ConfigMgr`] for application-wide configuration access.
//!
//! # Format
//!
//! ```text
//! # This is a comment
//! DataDir = "/home/server/data"
//! WorldServerPort = 8085
//! Rate.XP.Kill = 1.5
//! ```

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const WORLD_CONFIG_REGISTRY_TSV: &str =
    include_str!("../../../docs/migration/inventory/cpp-world-config-registry.tsv");

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur while loading or parsing a configuration file.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The configuration file could not be found or read.
    #[error("config file not found: {0}")]
    FileNotFound(String),

    /// None of the requested configuration files could be loaded.
    #[error("no config file found; tried: {0}")]
    NoConfigFile(String),

    /// A line in the configuration file could not be parsed.
    #[error("parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },

    /// A database connection string could not be parsed.
    #[error("invalid {key}: {message}")]
    InvalidDatabaseInfo { key: String, message: String },
}

// ---------------------------------------------------------------------------
// Internal config store
// ---------------------------------------------------------------------------

/// Internal configuration store.
///
/// Keys are stored in **lowercase** so that lookups are case-insensitive.
#[derive(Debug, Default)]
struct ConfigStore {
    values: HashMap<String, ConfigEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConfigEntry {
    original_key: String,
    value: String,
}

impl ConfigStore {
    /// Parse the full text content of a `.conf` file into the store.
    fn parse(&mut self, content: &str) -> Result<(), ConfigError> {
        self.values.clear();
        self.merge(content)
    }

    /// Merge the full text content of a `.conf` file into the store.
    fn merge(&mut self, content: &str) -> Result<(), ConfigError> {
        for (idx, raw_line) in content.lines().enumerate() {
            let line_number = idx + 1;
            let line = raw_line.trim();

            // Skip empty lines and full-line comments.
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // TrinityCore uses Boost.PropertyTree INI parsing and then flattens
            // the single top-level section (`[worldserver]` / `[bnetserver]`)
            // with `fullTree.begin()->second`.
            if line.starts_with('[') && line.ends_with(']') {
                continue;
            }

            // Find the first '=' to split key and value.
            let Some(eq_pos) = line.find('=') else {
                return Err(ConfigError::ParseError {
                    line: line_number,
                    message: format!("expected '=' in: {line}"),
                });
            };

            let key = line[..eq_pos].trim();
            if key.is_empty() {
                return Err(ConfigError::ParseError {
                    line: line_number,
                    message: "empty key".to_string(),
                });
            }

            let raw_value = line[eq_pos + 1..].trim();
            let value = parse_value(raw_value);

            self.values.insert(
                key.to_ascii_lowercase(),
                ConfigEntry {
                    original_key: key.to_string(),
                    value,
                },
            );
        }

        Ok(())
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.values
            .get(&key.to_ascii_lowercase())
            .map(|entry| entry.value.as_str())
    }

    fn override_with_env_variables(&mut self) -> Vec<String> {
        self.override_with_env_provider(|key| {
            env::var_os(key).map(|value| value.to_string_lossy().into_owned())
        })
    }

    fn override_with_env_provider<F>(&mut self, mut provider: F) -> Vec<String>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let mut overridden_keys = Vec::new();

        for entry in self.values.values_mut() {
            let Some(env_value) = provider(&env_key_for_ini_key(&entry.original_key)) else {
                continue;
            };

            entry.value = env_value;
            overridden_keys.push(entry.original_key.clone());
        }

        overridden_keys
    }

    fn database_info_default(&self, name: &str, default: DatabaseInfo) -> DatabaseInfo {
        let key = format!("{name}DatabaseInfo");
        if let Some(value) = self.get(&key) {
            return parse_database_info(&key, value).unwrap_or(default);
        }

        default
    }
}

/// Extract the actual value from the raw right-hand side of a config line.
///
/// Handles:
/// - Quoted strings: `"some value"` -> `some value` (content between quotes)
/// - Unquoted values with optional inline comments: `123 # a comment` -> `123`
fn parse_value(raw: &str) -> String {
    if raw.starts_with('"') {
        // Find the closing quote.
        if let Some(end) = raw[1..].find('"') {
            return raw[1..=end].to_string();
        }
        // No closing quote found -- treat the rest (minus the opening quote)
        // as the value, stripping an inline comment if present.
        return strip_inline_comment(&raw[1..]).to_string();
    }

    strip_inline_comment(raw).to_string()
}

/// Remove an inline `# comment` from an unquoted value and trim whitespace.
fn strip_inline_comment(s: &str) -> &str {
    match s.find('#') {
        Some(pos) => s[..pos].trim(),
        None => s.trim(),
    }
}

/// Converts an ini key to the TrinityCore `TC_*` environment variable name.
///
/// This mirrors `IniKeyToEnvVarKey` in C++
/// `/src/common/Configuration/Config.cpp`.
fn env_key_for_ini_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    let mut result = String::from("TC_");

    for (idx, curr) in chars.iter().copied().enumerate() {
        if matches!(curr, ' ' | '.' | '-') {
            result.push('_');
            continue;
        }

        if let Some(next) = chars.get(idx + 1).copied() {
            let next_is_upper = next.is_ascii_uppercase();

            if !curr.is_ascii_uppercase() && next_is_upper {
                result.push(curr.to_ascii_uppercase());
                result.push('_');
                continue;
            }

            let curr_is_numeric = curr.is_ascii_digit();
            let next_is_numeric = next.is_ascii_digit();

            if !curr_is_numeric && next_is_numeric {
                result.push(curr.to_ascii_uppercase());
                result.push('_');
                continue;
            }

            if curr_is_numeric && !next_is_numeric {
                result.push(curr.to_ascii_uppercase());
                result.push('_');
                continue;
            }
        }

        result.push(curr.to_ascii_uppercase());
    }

    result
}

fn collect_conf_files(dir: &Path) -> Result<Vec<PathBuf>, ConfigError> {
    if !dir.exists() || !dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut pending = vec![dir.to_path_buf()];
    let mut files = Vec::new();

    while let Some(path) = pending.pop() {
        let entries = fs::read_dir(&path)
            .map_err(|_| ConfigError::FileNotFound(path.display().to_string()))?;

        for entry in entries {
            let entry = entry.map_err(|_| ConfigError::FileNotFound(path.display().to_string()))?;
            let entry_path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|_| ConfigError::FileNotFound(entry_path.display().to_string()))?;

            if file_type.is_dir() {
                pending.push(entry_path);
            } else if file_type.is_file() && entry_path.extension().is_some_and(|ext| ext == "conf")
            {
                files.push(entry_path);
            }
        }
    }

    files.sort();
    Ok(files)
}

// ---------------------------------------------------------------------------
// Global singleton
// ---------------------------------------------------------------------------

static CONFIG: Lazy<RwLock<ConfigStore>> = Lazy::new(|| RwLock::new(ConfigStore::default()));
static WORLD_CONFIG_REGISTRY: Lazy<Vec<WorldConfigEntry>> = Lazy::new(parse_world_config_registry);

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Load and parse a `.conf` file, replacing any previously loaded
/// configuration.
///
/// # Errors
///
/// Returns [`ConfigError::FileNotFound`] if the file cannot be read, or
/// [`ConfigError::ParseError`] if the content is malformed.
pub fn load_config(path: &str) -> Result<(), ConfigError> {
    let content =
        fs::read_to_string(path).map_err(|_| ConfigError::FileNotFound(path.to_string()))?;

    let mut store = CONFIG.write();
    store.parse(&content)
}

/// Report produced by canonical config startup loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadReport {
    /// The initial config file that was loaded.
    pub initial_file: String,
    /// Index of the successful initial file in the candidate list.
    pub candidate_index: usize,
    /// Additional `.conf` overlay files loaded from the config dir.
    pub loaded_files: Vec<String>,
    /// Keys overridden by `TC_*` environment variables.
    pub overridden_keys: Vec<String>,
}

/// Load the first readable initial config, then merge additional `.conf`
/// files from `config_dir`, then apply `TC_*` environment overrides.
///
/// This follows the C++ startup order:
/// `LoadInitial` -> `LoadAdditionalDir` -> `OverrideWithEnvVariablesIfAny`.
pub fn load_config_with_fallbacks(
    config_candidates: &[&str],
    config_dir: &str,
) -> Result<LoadReport, ConfigError> {
    let Some((candidate_index, initial_file, content)) = config_candidates
        .iter()
        .enumerate()
        .find_map(|(idx, path)| {
            fs::read_to_string(path)
                .ok()
                .map(|content| (idx, *path, content))
        })
    else {
        return Err(ConfigError::NoConfigFile(config_candidates.join(", ")));
    };

    let mut store = ConfigStore::default();
    store.parse(&content)?;

    let mut loaded_files = Vec::new();
    for path in collect_conf_files(Path::new(config_dir))? {
        let content = fs::read_to_string(&path)
            .map_err(|_| ConfigError::FileNotFound(path.display().to_string()))?;
        store.merge(&content)?;
        loaded_files.push(path.display().to_string());
    }

    let overridden_keys = store.override_with_env_variables();

    let mut global = CONFIG.write();
    *global = store;

    Ok(LoadReport {
        initial_file: initial_file.to_string(),
        candidate_index,
        loaded_files,
        overridden_keys,
    })
}

/// Retrieve a configuration value parsed as `T`.
///
/// Returns `None` when the key is absent **or** the value cannot be parsed
/// into `T`.
pub fn get_value<T: FromStr>(key: &str) -> Option<T> {
    let store = CONFIG.read();
    store.get(key).and_then(|v| v.parse::<T>().ok())
}

/// Retrieve a configuration value parsed as `T`, falling back to `default`
/// when the key is absent or unparsable.
pub fn get_value_default<T: FromStr>(key: &str, default: T) -> T {
    get_value(key).unwrap_or(default)
}

/// Retrieve a string value, returning `default` when the key is absent.
pub fn get_string_default(key: &str, default: &str) -> String {
    let store = CONFIG.read();
    store
        .get(key)
        .map(|s| s.to_string())
        .unwrap_or_else(|| default.to_string())
}

/// Parsed TrinityCore `*DatabaseInfo` semicolon connection string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseInfo {
    pub host: String,
    pub port_or_socket: String,
    pub username: String,
    pub password: String,
    pub database: String,
    pub ssl: bool,
}

impl DatabaseInfo {
    pub fn new(host: &str, port: u16, username: &str, password: &str, database: &str) -> Self {
        Self {
            host: host.to_string(),
            port_or_socket: port.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            database: database.to_string(),
            ssl: false,
        }
    }
}

/// Parse a C++ TrinityCore database info value:
/// `host;port_or_socket;username;password;database[;ssl]`.
pub fn parse_database_info(key: &str, value: &str) -> Result<DatabaseInfo, ConfigError> {
    let parts: Vec<&str> = value.split(';').collect();
    if parts.len() < 5 {
        return Err(ConfigError::InvalidDatabaseInfo {
            key: key.to_string(),
            message: "expected host;port;username;password;database".to_string(),
        });
    }

    if parts[1].is_empty() {
        return Err(ConfigError::InvalidDatabaseInfo {
            key: key.to_string(),
            message: "empty port_or_socket".to_string(),
        });
    }

    Ok(DatabaseInfo {
        host: parts[0].to_string(),
        port_or_socket: parts[1].to_string(),
        username: parts[2].to_string(),
        password: parts[3].to_string(),
        database: parts[4].to_string(),
        ssl: parts
            .get(5)
            .is_some_and(|value| value.eq_ignore_ascii_case("ssl")),
    })
}

/// Read `{name}DatabaseInfo` using the C++ semicolon schema.
pub fn get_database_info_default(name: &str, default: DatabaseInfo) -> DatabaseInfo {
    let store = CONFIG.read();
    store.database_info_default(name, default)
}

/// TrinityCore `World*Configs` value group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorldConfigKind {
    Bool,
    Float,
    Int,
    Int64,
}

/// Typed value for one `World*Configs` entry.
#[derive(Debug, Clone, PartialEq)]
pub enum WorldConfigValue {
    Bool(bool),
    Float(f32),
    Int(u32),
    Int64(u64),
}

/// One row from the C++ `World*Configs` registry.
#[derive(Debug, Clone, PartialEq)]
pub struct WorldConfigEntry {
    pub kind: WorldConfigKind,
    pub enum_name: String,
    pub key: Option<String>,
    pub cxx_ref: Option<String>,
    pub default_expr: Option<String>,
    pub default_value: Option<WorldConfigValue>,
}

/// Loaded `World*Configs` values, indexed by C++ enum name.
#[derive(Debug, Clone, Default)]
pub struct WorldConfigSet {
    values: HashMap<String, WorldConfigValue>,
}

impl WorldConfigSet {
    pub fn get(&self, enum_name: &str) -> Option<&WorldConfigValue> {
        self.values.get(enum_name)
    }

    pub fn get_bool(&self, enum_name: &str) -> Option<bool> {
        match self.get(enum_name) {
            Some(WorldConfigValue::Bool(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn get_float(&self, enum_name: &str) -> Option<f32> {
        match self.get(enum_name) {
            Some(WorldConfigValue::Float(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn get_int(&self, enum_name: &str) -> Option<u32> {
        match self.get(enum_name) {
            Some(WorldConfigValue::Int(value)) => Some(*value),
            _ => None,
        }
    }

    pub fn get_int64(&self, enum_name: &str) -> Option<u64> {
        match self.get(enum_name) {
            Some(WorldConfigValue::Int64(value)) => Some(*value),
            _ => None,
        }
    }

    fn set_bool(&mut self, enum_name: &str, value: bool) {
        if let Some(slot @ WorldConfigValue::Bool(_)) = self.values.get_mut(enum_name) {
            *slot = WorldConfigValue::Bool(value);
        }
    }

    fn set_float(&mut self, enum_name: &str, value: f32) {
        if let Some(slot @ WorldConfigValue::Float(_)) = self.values.get_mut(enum_name) {
            *slot = WorldConfigValue::Float(value);
        }
    }

    fn set_int(&mut self, enum_name: &str, value: u32) {
        if let Some(slot @ WorldConfigValue::Int(_)) = self.values.get_mut(enum_name) {
            *slot = WorldConfigValue::Int(value);
        }
    }
}

/// Canonical registry rows for C++ `WorldBoolConfigs`,
/// `WorldFloatConfigs`, `WorldIntConfigs`, and `WorldInt64Configs`.
pub fn world_config_registry() -> &'static [WorldConfigEntry] {
    &WORLD_CONFIG_REGISTRY
}

/// Resolve all world config values from the loaded config store.
///
/// Missing config keys use the C++ default expression from
/// `cpp-world-config-registry.tsv`. Rows without a literal C++ load remain
/// absent until their C++ initialization is ported explicitly.
pub fn load_world_config_values() -> WorldConfigSet {
    let store = CONFIG.read();
    let mut values = HashMap::new();

    for entry in world_config_registry() {
        let Some(value) = resolve_world_config_entry(entry, &store) else {
            continue;
        };

        values.insert(entry.enum_name.clone(), value);
    }

    let mut set = WorldConfigSet { values };
    apply_world_config_validations(&mut set);
    set
}

fn resolve_world_config_entry(
    entry: &WorldConfigEntry,
    store: &ConfigStore,
) -> Option<WorldConfigValue> {
    if entry.enum_name == "CONFIG_CLIENTCACHE_VERSION" {
        return store
            .get("ClientCacheVersion")
            .and_then(|raw| parse_world_config_value(WorldConfigKind::Int, raw))
            .and_then(|value| match value {
                WorldConfigValue::Int(value) if signed_i32(value) > 0 => {
                    Some(WorldConfigValue::Int(value))
                }
                _ => None,
            });
    }

    let configured = entry
        .key
        .as_deref()
        .and_then(|key| store.get(key))
        .and_then(|raw| parse_world_config_value(entry.kind, raw));

    configured.or_else(|| entry.default_value.clone())
}

fn parse_world_config_registry() -> Vec<WorldConfigEntry> {
    WORLD_CONFIG_REGISTRY_TSV
        .lines()
        .skip(1)
        .filter_map(parse_world_config_registry_row)
        .collect()
}

fn parse_world_config_registry_row(row: &str) -> Option<WorldConfigEntry> {
    let columns: Vec<&str> = row.split('\t').collect();
    if columns.len() < 9 {
        return None;
    }

    let kind = match columns[0] {
        "Bool" => WorldConfigKind::Bool,
        "Float" => WorldConfigKind::Float,
        "Int" => WorldConfigKind::Int,
        "Int64" => WorldConfigKind::Int64,
        _ => return None,
    };

    let key = non_empty(columns[3]).map(ToOwned::to_owned);
    let cxx_ref = non_empty(columns[5]).map(ToOwned::to_owned);
    let default_expr = non_empty(columns[6]).map(ToOwned::to_owned);
    let default_value = default_expr
        .as_deref()
        .and_then(|expr| parse_world_default_expr(kind, expr));

    Some(WorldConfigEntry {
        kind,
        enum_name: columns[1].to_string(),
        key,
        cxx_ref,
        default_expr,
        default_value,
    })
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

fn parse_world_config_value(kind: WorldConfigKind, raw: &str) -> Option<WorldConfigValue> {
    match kind {
        WorldConfigKind::Bool => parse_config_bool(raw).map(WorldConfigValue::Bool),
        WorldConfigKind::Float => raw.parse::<f32>().ok().map(WorldConfigValue::Float),
        WorldConfigKind::Int => raw
            .parse::<i32>()
            .ok()
            .map(|value| WorldConfigValue::Int(value as u32)),
        WorldConfigKind::Int64 => raw.parse::<u64>().ok().map(WorldConfigValue::Int64),
    }
}

fn apply_world_config_validations(values: &mut WorldConfigSet) {
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

fn signed_i32(value: u32) -> i32 {
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

fn parse_config_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn parse_world_default_expr(kind: WorldConfigKind, expr: &str) -> Option<WorldConfigValue> {
    match kind {
        WorldConfigKind::Bool => parse_config_bool(expr).map(WorldConfigValue::Bool),
        WorldConfigKind::Float => parse_world_float_expr(expr).map(WorldConfigValue::Float),
        WorldConfigKind::Int => {
            eval_world_int_expr(expr).map(|value| WorldConfigValue::Int(value as u32))
        }
        WorldConfigKind::Int64 => {
            eval_world_int_expr(expr).map(|value| WorldConfigValue::Int64(value as u64))
        }
    }
}

fn parse_world_float_expr(expr: &str) -> Option<f32> {
    expr.trim().trim_end_matches('f').parse::<f32>().ok()
}

fn eval_world_int_expr(expr: &str) -> Option<i64> {
    let normalized = expr
        .replace("(uint32)", "")
        .replace('(', "")
        .replace(')', "");

    normalized
        .split('*')
        .map(|part| eval_world_int_atom(part.trim()))
        .try_fold(1_i64, |acc, value| value.map(|value| acc * value))
}

fn eval_world_int_atom(atom: &str) -> Option<i64> {
    match atom {
        "MINUTE" => Some(60),
        "IN_MILLISECONDS" => Some(1000),
        "HOUR" => Some(3600),
        "DEFAULT_MAX_LEVEL" => Some(80),
        "CURRENT_EXPANSION" => Some(2),
        "HARDCODED_DEVELOPMENT_REALM_CATEGORY_ID" => Some(1),
        "SEC_ADMINISTRATOR" => Some(3),
        "SEC_CONSOLE" => Some(4),
        "GUILD_BANKLOG_MAX_RECORDS" => Some(25),
        "GUILD_EVENTLOG_MAX_RECORDS" => Some(100),
        "GUILD_NEWSLOG_MAX_RECORDS" => Some(250),
        "WorldSession::DosProtection::POLICY_KICK" => Some(1),
        "BAN_ACCOUNT" => Some(0),
        _ => atom.parse::<i64>().ok(),
    }
}

// ---------------------------------------------------------------------------
// Internal helper for tests -- load from string instead of file
// ---------------------------------------------------------------------------

/// Load configuration from a raw string (useful for testing).
#[doc(hidden)]
pub fn load_config_from_str(content: &str) -> Result<(), ConfigError> {
    let mut store = CONFIG.write();
    store.parse(content)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard};

    static TEST_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    fn global_config_lock() -> MutexGuard<'static, ()> {
        TEST_LOCK.lock().expect("test lock poisoned")
    }

    /// Helper: create an isolated `ConfigStore` and parse into it so tests
    /// do not interfere with the global singleton.
    fn parse(content: &str) -> ConfigStore {
        let mut store = ConfigStore::default();
        store.parse(content).expect("parse failed");
        store
    }

    // -- Parsing basics -----------------------------------------------------

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

    // -- Comments -----------------------------------------------------------

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

    // -- Empty / whitespace lines -------------------------------------------

    #[test]
    fn test_empty_lines_ignored() {
        let content = "\n\n  \nKey = val\n\n";
        let store = parse(content);
        assert_eq!(store.get("Key"), Some("val"));
        assert_eq!(store.values.len(), 1);
    }

    // -- Case-insensitive lookup --------------------------------------------

    #[test]
    fn test_case_insensitive_lookup() {
        let store = parse("DataDir = /data");
        assert_eq!(store.get("datadir"), Some("/data"));
        assert_eq!(store.get("DATADIR"), Some("/data"));
        assert_eq!(store.get("DataDir"), Some("/data"));
    }

    // -- Numeric parsing ----------------------------------------------------

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

    // -- Defaults -----------------------------------------------------------

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

    // -- Global API round-trip ----------------------------------------------

    #[test]
    fn test_global_load_and_get() {
        let _guard = global_config_lock();
        load_config_from_str("TestKey = 42\nGreeting = \"hello world\"").expect("load failed");

        assert_eq!(get_value::<i32>("TestKey"), Some(42));
        assert_eq!(get_value::<String>("Greeting"), Some("hello world".into()));
        assert_eq!(get_string_default("Greeting", ""), "hello world");
    }

    // -- Error paths --------------------------------------------------------

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

    // -- Multiple keys ------------------------------------------------------

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

    // -- Overwrite on reload ------------------------------------------------

    #[test]
    fn test_reload_replaces_values() {
        let mut store = ConfigStore::default();
        store.parse("Key = old").unwrap();
        assert_eq!(store.get("Key"), Some("old"));

        store.parse("Key = new").unwrap();
        assert_eq!(store.get("Key"), Some("new"));
    }

    // -- Value with equals sign ---------------------------------------------

    #[test]
    fn test_value_containing_equals() {
        let store = parse(r#"ConnString = "server=localhost;port=3306""#);
        assert_eq!(store.get("ConnString"), Some("server=localhost;port=3306"));
    }

    // -- Quoted value containing hash ---------------------------------------

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
        assert_eq!(registry.len(), 340);
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
            37
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
MaxGroupXPDistance = 120.5
WorldServerPort = 8088
CharacterCreating.Disabled.RaceMask = 12
"#,
        )
        .expect("load failed");

        let values = load_world_config_values();
        assert_eq!(values.get_bool("CONFIG_ADDON_CHANNEL"), Some(false));
        assert_eq!(values.get_float("CONFIG_GROUP_XP_DISTANCE"), Some(120.5));
        assert_eq!(values.get_int("CONFIG_PORT_WORLD"), Some(8088));
        assert_eq!(
            values.get_int64("CONFIG_CHARACTER_CREATING_DISABLED_RACEMASK"),
            Some(12)
        );
        assert_eq!(values.get_bool("CONFIG_ENABLE_MMAPS"), Some(true));
        assert_eq!(values.get_int("CONFIG_INTERVAL_SAVE"), Some(900_000));
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

    fn unique_temp_dir(name: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "rustycore_wow_config_{name}_{}_{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));

        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temp dir failed");
        path
    }
}
