// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Module identity, descriptors and the explicit registrar.

use std::collections::BTreeMap;
use std::fmt;

use crate::effect::PlayerLoginEffects;
use crate::hook::{PlayerLoginModule, PlayerLoginSnapshot};

/// A validated module identifier.
///
/// Deliberately narrow so an id can be logged, ordered and compared without
/// escaping: lowercase ASCII letters, digits, `_` and `.`, starting with a
/// letter. Rejecting at construction means the registry never holds one that
/// cannot be rendered.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModuleId(String);

impl ModuleId {
    pub fn new(raw: &str) -> Result<Self, ModuleRegistrationError> {
        if raw.is_empty() || raw.len() > 64 {
            return Err(ModuleRegistrationError::InvalidId {
                id: raw.to_owned(),
                reason: "length must be 1..=64",
            });
        }
        let mut chars = raw.chars();
        let first = chars.next().expect("non-empty");
        if !first.is_ascii_lowercase() {
            return Err(ModuleRegistrationError::InvalidId {
                id: raw.to_owned(),
                reason: "must start with a lowercase ASCII letter",
            });
        }
        if !raw
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
        {
            return Err(ModuleRegistrationError::InvalidId {
                id: raw.to_owned(),
                reason: "only lowercase ASCII letters, digits, '_' and '.' are allowed",
            });
        }
        Ok(Self(raw.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Semantic-looking module version. No compatibility rule is implied yet.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModuleVersion {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl fmt::Display for ModuleVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// What a module declares about itself at registration time.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModuleDescriptor {
    pub id: ModuleId,
    pub version: ModuleVersion,
    pub display_name: String,
}

impl ModuleDescriptor {
    pub fn new(
        id: &str,
        version: ModuleVersion,
        display_name: &str,
    ) -> Result<Self, ModuleRegistrationError> {
        let id = ModuleId::new(id)?;
        if display_name.trim().is_empty() || display_name.len() > 128 {
            return Err(ModuleRegistrationError::InvalidDescriptor {
                id: id.to_string(),
                reason: "display name must be non-blank and at most 128 bytes",
            });
        }
        Ok(Self {
            id,
            version,
            display_name: display_name.to_owned(),
        })
    }
}

/// Why a registration was refused.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleRegistrationError {
    InvalidId { id: String, reason: &'static str },
    InvalidDescriptor { id: String, reason: &'static str },
    DuplicateId { id: String },
}

impl fmt::Display for ModuleRegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId { id, reason } => write!(f, "invalid module id {id:?}: {reason}"),
            Self::InvalidDescriptor { id, reason } => {
                write!(f, "invalid descriptor for module {id}: {reason}")
            }
            Self::DuplicateId { id } => write!(f, "module {id} is already registered"),
        }
    }
}

impl std::error::Error for ModuleRegistrationError {}

/// The registered trusted modules.
///
/// Ordering is deterministic: modules run in `ModuleId` order, never in
/// registration order, so a build that links the same set always dispatches
/// the same sequence.
#[derive(Default)]
pub struct ModuleRegistry {
    modules: BTreeMap<ModuleId, (ModuleDescriptor, Box<dyn PlayerLoginModule>)>,
}

impl ModuleRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_player_login_module(
        &mut self,
        descriptor: ModuleDescriptor,
        module: Box<dyn PlayerLoginModule>,
    ) -> Result<(), ModuleRegistrationError> {
        if self.modules.contains_key(&descriptor.id) {
            return Err(ModuleRegistrationError::DuplicateId {
                id: descriptor.id.to_string(),
            });
        }
        self.modules
            .insert(descriptor.id.clone(), (descriptor, module));
        Ok(())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    #[must_use]
    pub fn descriptors(&self) -> Vec<&ModuleDescriptor> {
        self.modules.values().map(|(d, _)| d).collect()
    }

    /// Run every registered login module and return the validated batch.
    ///
    /// The batch is validated as a whole before the caller applies anything,
    /// so an invalid effect from one module discards the entire login batch
    /// rather than leaving a partial result. With no modules registered this
    /// allocates nothing observable and returns an empty batch, which is the
    /// zero-module no-op path.
    pub fn dispatch_player_login(
        &self,
        snapshot: &PlayerLoginSnapshot,
    ) -> Result<PlayerLoginEffects, crate::effect::ModuleEffectError> {
        let mut effects = PlayerLoginEffects::default();
        for (id, (_, module)) in &self.modules {
            module.on_player_login(snapshot, &mut effects.scoped_for(id.clone()));
        }
        effects.validated()
    }
}
