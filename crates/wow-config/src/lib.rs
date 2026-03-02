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
//! DataDir = "/path/to/data"
//! WorldServerPort = 8085
//! Rate.XP.Kill = 1.5
//! ```

use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur while loading or parsing a configuration file.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The configuration file could not be found or read.
    #[error("config file not found: {0}")]
    FileNotFound(String),

    /// A line in the configuration file could not be parsed.
    #[error("parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
}

// ---------------------------------------------------------------------------
// Internal config store
// ---------------------------------------------------------------------------

/// Internal configuration store.
///
/// Keys are stored in **lowercase** so that lookups are case-insensitive.
#[derive(Debug, Default)]
struct ConfigStore {
    values: HashMap<String, String>,
}

impl ConfigStore {
    /// Parse the full text content of a `.conf` file into the store.
    fn parse(&mut self, content: &str) -> Result<(), ConfigError> {
        self.values.clear();

        for (idx, raw_line) in content.lines().enumerate() {
            let line_number = idx + 1;
            let line = raw_line.trim();

            // Skip empty lines and full-line comments.
            if line.is_empty() || line.starts_with('#') {
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

            self.values.insert(key.to_ascii_lowercase(), value);
        }

        Ok(())
    }

    fn get(&self, key: &str) -> Option<&str> {
        self.values.get(&key.to_ascii_lowercase()).map(|s| s.as_str())
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

// ---------------------------------------------------------------------------
// Global singleton
// ---------------------------------------------------------------------------

static CONFIG: Lazy<RwLock<ConfigStore>> = Lazy::new(|| RwLock::new(ConfigStore::default()));

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
        let store = parse(r#"DataDir = "/path/to/data""#);
        assert_eq!(store.get("DataDir"), Some("/path/to/data"));
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
        load_config_from_str("TestKey = 42\nGreeting = \"hello world\"")
            .expect("load failed");

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
        assert_eq!(
            store.get("ConnString"),
            Some("server=localhost;port=3306")
        );
    }

    // -- Quoted value containing hash ---------------------------------------

    #[test]
    fn test_quoted_value_with_hash() {
        let store = parse(r##"Color = "#FF0000""##);
        assert_eq!(store.get("Color"), Some("#FF0000"));
    }
}
