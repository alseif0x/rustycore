//! World registry packets.
//!
//! Separated from lib.rs under #693.

use crate::*;

const WORLD_CONFIG_REGISTRY_TSV: &str =
    include_str!("../../../docs/migration/inventory/cpp-world-config-registry.tsv");

static WORLD_CONFIG_REGISTRY: Lazy<Vec<WorldConfigEntry>> = Lazy::new(parse_world_config_registry);

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

    pub(super) fn set_bool(&mut self, enum_name: &str, value: bool) {
        if let Some(slot @ WorldConfigValue::Bool(_)) = self.values.get_mut(enum_name) {
            *slot = WorldConfigValue::Bool(value);
        }
    }

    pub(super) fn set_float(&mut self, enum_name: &str, value: f32) {
        if let Some(slot @ WorldConfigValue::Float(_)) = self.values.get_mut(enum_name) {
            *slot = WorldConfigValue::Float(value);
        }
    }

    pub(super) fn set_int(&mut self, enum_name: &str, value: u32) {
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
