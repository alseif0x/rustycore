// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Namespaced, typed module configuration.
//!
//! A module never reads a file. The composition step validates the operator's
//! overrides against the module's declared defaults, embeds the result, and
//! hands each module only its own namespace at registration time. What the
//! module keeps is an immutable snapshot, so a login callback does no I/O and
//! cannot observe a mid-flight change.
//!
//! This module deliberately parses nothing: `wow-module-api` has an empty
//! external dependency allowlist, so the TOML lives on the composition side
//! and only typed values cross the boundary.

use std::collections::BTreeMap;
use std::fmt;

use crate::registry::ModuleId;

/// A configuration value, restricted to what a module option may be.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleConfigValue {
    Bool(bool),
    Integer(i64),
    Text(String),
}

impl ModuleConfigValue {
    fn type_name(&self) -> &'static str {
        match self {
            Self::Bool(_) => "boolean",
            Self::Integer(_) => "integer",
            Self::Text(_) => "string",
        }
    }

    /// Canonical rendering used for the digest. Deterministic by construction:
    /// no floats, no map iteration order, no locale.
    fn canonical(&self) -> String {
        match self {
            Self::Bool(v) => format!("b:{v}"),
            Self::Integer(v) => format!("i:{v}"),
            Self::Text(v) => format!("s:{}:{v}", v.len()),
        }
    }
}

/// Why a module refused its configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleConfigError {
    UnknownField {
        module: String,
        key: String,
    },
    MissingField {
        module: String,
        key: String,
    },
    WrongType {
        module: String,
        key: String,
        expected: &'static str,
        found: &'static str,
    },
    InvalidValue {
        module: String,
        key: String,
        reason: String,
    },
}

impl fmt::Display for ModuleConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownField { module, key } => write!(
                f,
                "module {module}: unknown configuration key {key:?}; remove it or check the \
                 module's documented options"
            ),
            Self::MissingField { module, key } => {
                write!(
                    f,
                    "module {module}: required configuration key {key:?} is missing"
                )
            }
            Self::WrongType {
                module,
                key,
                expected,
                found,
            } => write!(
                f,
                "module {module}: configuration key {key:?} must be a {expected}, found a {found}"
            ),
            Self::InvalidValue {
                module,
                key,
                reason,
            } => {
                write!(
                    f,
                    "module {module}: configuration key {key:?} is invalid: {reason}"
                )
            }
        }
    }
}

impl std::error::Error for ModuleConfigError {}

/// One module's namespaced configuration.
///
/// Keys are namespaced by construction: a module is only ever handed the tree
/// built for its own [`ModuleId`], so two modules cannot collide.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ModuleConfig {
    module: String,
    values: BTreeMap<String, ModuleConfigValue>,
    /// Fixed at construction, because the digest must describe what the module
    /// was *given*. Computing it from `values` would change as options are
    /// read and would report an empty configuration once the module finished.
    digest: String,
}

impl ModuleConfig {
    #[must_use]
    pub fn new(module: &ModuleId, values: BTreeMap<String, ModuleConfigValue>) -> Self {
        let digest = Self::compute_digest(&values);
        Self {
            module: module.to_string(),
            values,
            digest,
        }
    }

    /// A deterministic digest of the exact configuration this module received.
    ///
    /// Stable across runs and machines: the map is ordered, values render
    /// canonically, and no float or timestamp participates.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    fn compute_digest(values: &BTreeMap<String, ModuleConfigValue>) -> String {
        let mut state: u64 = 0xcbf2_9ce4_8422_2325;
        for (key, value) in values {
            for byte in key
                .as_bytes()
                .iter()
                .chain(b"=")
                .chain(value.canonical().as_bytes())
                .chain(b";")
            {
                state ^= u64::from(*byte);
                state = state.wrapping_mul(0x0000_0100_0000_01b3);
            }
        }
        format!("fnv1a64:{state:016x}")
    }

    pub fn bool(&mut self, key: &str) -> Result<Option<bool>, ModuleConfigError> {
        match self.take(key) {
            None => Ok(None),
            Some(ModuleConfigValue::Bool(v)) => Ok(Some(v)),
            Some(other) => Err(self.wrong_type(key, "boolean", &other)),
        }
    }

    pub fn integer(&mut self, key: &str) -> Result<Option<i64>, ModuleConfigError> {
        match self.take(key) {
            None => Ok(None),
            Some(ModuleConfigValue::Integer(v)) => Ok(Some(v)),
            Some(other) => Err(self.wrong_type(key, "integer", &other)),
        }
    }

    pub fn text(&mut self, key: &str) -> Result<Option<String>, ModuleConfigError> {
        match self.take(key) {
            None => Ok(None),
            Some(ModuleConfigValue::Text(v)) => Ok(Some(v)),
            Some(other) => Err(self.wrong_type(key, "string", &other)),
        }
    }

    /// Reject a value the type system cannot: an empty or oversized string, a
    /// negative count, and so on.
    pub fn invalid(&self, key: &str, reason: impl Into<String>) -> ModuleConfigError {
        ModuleConfigError::InvalidValue {
            module: self.module.clone(),
            key: key.to_owned(),
            reason: reason.into(),
        }
    }

    pub fn missing(&self, key: &str) -> ModuleConfigError {
        ModuleConfigError::MissingField {
            module: self.module.clone(),
            key: key.to_owned(),
        }
    }

    /// Every key the module did not read is an error, not a silent typo.
    ///
    /// A module calls this once it has taken the options it knows about, so a
    /// misspelled operator key fails activation instead of being ignored.
    pub fn finish(self) -> Result<(), ModuleConfigError> {
        match self.values.keys().next() {
            None => Ok(()),
            Some(key) => Err(ModuleConfigError::UnknownField {
                module: self.module,
                key: key.clone(),
            }),
        }
    }

    fn take(&mut self, key: &str) -> Option<ModuleConfigValue> {
        self.values.remove(key)
    }

    fn wrong_type(
        &self,
        key: &str,
        expected: &'static str,
        found: &ModuleConfigValue,
    ) -> ModuleConfigError {
        ModuleConfigError::WrongType {
            module: self.module.clone(),
            key: key.to_owned(),
            expected,
            found: found.type_name(),
        }
    }
}
