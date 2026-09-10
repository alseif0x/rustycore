//! Store packets.
//!
//! Separated from lib.rs under #693.

use crate::*;

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
pub(super) struct ConfigStore {
    pub(super) values: HashMap<String, ConfigEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ConfigEntry {
    original_key: String,
    value: String,
}

impl ConfigStore {
    /// Parse the full text content of a `.conf` file into the store.
    pub(super) fn parse(&mut self, content: &str) -> Result<(), ConfigError> {
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

    pub(super) fn get(&self, key: &str) -> Option<&str> {
        self.values
            .get(&key.to_ascii_lowercase())
            .map(|entry| entry.value.as_str())
    }

    fn override_with_env_variables(&mut self) -> Vec<String> {
        self.override_with_env_provider(|key| {
            env::var_os(key).map(|value| value.to_string_lossy().into_owned())
        })
    }

    pub(super) fn override_with_env_provider<F>(&mut self, mut provider: F) -> Vec<String>
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

    pub(super) fn database_info_default(&self, name: &str, default: DatabaseInfo) -> DatabaseInfo {
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
pub(super) fn env_key_for_ini_key(key: &str) -> String {
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

pub(super) static CONFIG: Lazy<RwLock<ConfigStore>> =
    Lazy::new(|| RwLock::new(ConfigStore::default()));

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
pub fn get_value<T: FromStr + 'static>(key: &str) -> Option<T> {
    let store = CONFIG.read();
    store.get(key).and_then(parse_config_value::<T>)
}

/// Retrieve a configuration value parsed as `T`, falling back to `default`
/// when the key is absent or unparsable.
pub fn get_value_default<T: FromStr + 'static>(key: &str, default: T) -> T {
    get_value(key).unwrap_or(default)
}

fn parse_config_value<T: FromStr + 'static>(raw: &str) -> Option<T> {
    if TypeId::of::<T>() == TypeId::of::<bool>() {
        let value = parse_config_bool(raw)?;
        let boxed: Box<dyn Any> = Box::new(value);
        return boxed.downcast::<T>().ok().map(|value| *value);
    }

    raw.parse::<T>().ok()
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

pub(super) fn parse_config_bool(raw: &str) -> Option<bool> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
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
