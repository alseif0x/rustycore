// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Typed login effects and their all-or-nothing validation.

use std::fmt;

use crate::registry::ModuleId;

/// C++ `ChatHandler::PSendSysMessage` carries one line at a time; the server
/// splits on newline before sending. Keep the limit well inside that so a
/// module cannot smuggle a multi-kilobyte packet through the hook.
const MAX_SYSTEM_MESSAGE_BYTES: usize = 255;

/// An effect a login module may request.
///
/// Only self-targeted messaging exists today. The variant carries no target:
/// the owner always applies it to the player whose login raised the hook, so a
/// module cannot address anyone else.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlayerLoginEffect {
    SendSystemMessageSelf { text: String },
}

/// Why a batch was rejected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleEffectError {
    EmptyMessage { module: String },
    MessageTooLong { module: String, bytes: usize },
    MessageHasControlCharacters { module: String },
}

impl fmt::Display for ModuleEffectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMessage { module } => {
                write!(f, "module {module} requested an empty system message")
            }
            Self::MessageTooLong { module, bytes } => write!(
                f,
                "module {module} requested a {bytes}-byte system message; the limit is \
                 {MAX_SYSTEM_MESSAGE_BYTES}"
            ),
            Self::MessageHasControlCharacters { module } => write!(
                f,
                "module {module} requested a system message containing control characters"
            ),
        }
    }
}

impl std::error::Error for ModuleEffectError {}

/// The batch a login dispatch produced, in deterministic module order.
#[derive(Debug, Default, Eq, PartialEq)]
pub struct PlayerLoginEffects {
    entries: Vec<(ModuleId, PlayerLoginEffect)>,
}

impl PlayerLoginEffects {
    pub(crate) fn scoped_for(&mut self, module: ModuleId) -> ScopedEffects<'_> {
        ScopedEffects {
            batch: self,
            module,
        }
    }

    /// Validate the whole batch. Nothing is applied unless every effect passes.
    pub(crate) fn validated(self) -> Result<Self, ModuleEffectError> {
        for (module, effect) in &self.entries {
            match effect {
                PlayerLoginEffect::SendSystemMessageSelf { text } => {
                    if text.is_empty() {
                        return Err(ModuleEffectError::EmptyMessage {
                            module: module.to_string(),
                        });
                    }
                    if text.len() > MAX_SYSTEM_MESSAGE_BYTES {
                        return Err(ModuleEffectError::MessageTooLong {
                            module: module.to_string(),
                            bytes: text.len(),
                        });
                    }
                    // C++ splits on '\n' before sending, so it is the one
                    // control character a message may legitimately contain.
                    if text.chars().any(|c| c.is_control() && c != '\n') {
                        return Err(ModuleEffectError::MessageHasControlCharacters {
                            module: module.to_string(),
                        });
                    }
                }
            }
        }
        Ok(self)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Iterate the validated effects in the order the owner must apply them.
    pub fn iter(&self) -> impl Iterator<Item = (&ModuleId, &PlayerLoginEffect)> {
        self.entries.iter().map(|(m, e)| (m, e))
    }
}

/// The handle one module writes through, so every effect records its author.
pub struct ScopedEffects<'a> {
    batch: &'a mut PlayerLoginEffects,
    module: ModuleId,
}

impl ScopedEffects<'_> {
    /// Queue a system message to the player whose login raised the hook.
    pub fn send_system_message_self(&mut self, text: impl Into<String>) {
        self.batch.entries.push((
            self.module.clone(),
            PlayerLoginEffect::SendSystemMessageSelf { text: text.into() },
        ));
    }
}
