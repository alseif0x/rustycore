// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Typed hook effects and their all-or-nothing validation.
//!
//! Every hook family returns its effects as one batch, recorded per module in
//! dispatch order. The owner validates the whole batch before applying any of
//! it, so one invalid effect discards the batch instead of half-applying it.

use std::fmt;

use crate::registry::ModuleId;

/// C++ `ChatHandler::PSendSysMessage` carries one line at a time; the server
/// splits on newline before sending. Keep the limit well inside that so a
/// module cannot smuggle a multi-kilobyte packet through the hook.
pub(crate) const MAX_SYSTEM_MESSAGE_BYTES: usize = 255;

/// A notification effect: what a confirmed-result hook may request.
///
/// Only self-targeted messaging exists. The variant carries no target: the
/// owner always applies it to the player whose action raised the hook, so a
/// module cannot address anyone else.
///
/// Source API 1 matched this enum exhaustively as `PlayerLoginEffect`, so it
/// keeps that shape; a new kind of effect gets its own family type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NotificationEffect {
    SendSystemMessageSelf { text: String },
}

/// The source API 1 name of [`NotificationEffect`].
pub type PlayerLoginEffect = NotificationEffect;

/// Why a batch was rejected.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
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

/// Validation one effect family applies to each of its entries.
pub(crate) trait ValidatedEffect {
    fn validate(&self, module: &ModuleId) -> Result<(), ModuleEffectError>;
}

/// Shared rule for every self-addressed system message.
pub(crate) fn validate_system_message(
    module: &ModuleId,
    text: &str,
) -> Result<(), ModuleEffectError> {
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
    // C++ splits on '\n' before sending, so it is the one control character a
    // message may legitimately contain.
    if text.chars().any(|c| c.is_control() && c != '\n') {
        return Err(ModuleEffectError::MessageHasControlCharacters {
            module: module.to_string(),
        });
    }
    Ok(())
}

impl ValidatedEffect for NotificationEffect {
    fn validate(&self, module: &ModuleId) -> Result<(), ModuleEffectError> {
        match self {
            Self::SendSystemMessageSelf { text } => validate_system_message(module, text),
        }
    }
}

/// The batch one dispatch produced, in deterministic module order.
#[derive(Debug, Eq, PartialEq)]
pub struct EffectBatch<E> {
    entries: Vec<(ModuleId, E)>,
}

impl<E> Default for EffectBatch<E> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

/// The source API 1 name of a notification batch.
pub type PlayerLoginEffects = EffectBatch<NotificationEffect>;

impl<E> EffectBatch<E> {
    pub(crate) fn push(&mut self, module: ModuleId, effect: E) {
        self.entries.push((module, effect));
    }

    /// Drop everything recorded after `len`: the effects a failed callback
    /// queued before it panicked.
    pub(crate) fn truncate(&mut self, len: usize) {
        self.entries.truncate(len);
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
    pub fn iter(&self) -> impl Iterator<Item = (&ModuleId, &E)> {
        self.entries.iter().map(|(m, e)| (m, e))
    }
}

/// Validate the whole batch. Nothing is applied unless every effect passes.
pub(crate) fn validated<E: ValidatedEffect>(
    batch: EffectBatch<E>,
) -> Result<EffectBatch<E>, ModuleEffectError> {
    for (module, effect) in &batch.entries {
        effect.validate(module)?;
    }
    Ok(batch)
}

/// The handle one module writes notification effects through, so every effect
/// records its author.
pub struct ScopedEffects<'a> {
    batch: &'a mut PlayerLoginEffects,
    module: ModuleId,
}

impl<'a> ScopedEffects<'a> {
    pub(crate) fn new(batch: &'a mut PlayerLoginEffects, module: ModuleId) -> Self {
        Self { batch, module }
    }

    /// Queue a system message to the player whose action raised the hook.
    pub fn send_system_message_self(&mut self, text: impl Into<String>) {
        self.batch.push(
            self.module.clone(),
            NotificationEffect::SendSystemMessageSelf { text: text.into() },
        );
    }
}

/// What one dispatch returns to the owner.
///
/// `effects` is the validated batch, or the first validation error, in which
/// case nothing may be applied. `disabled` names every module whose callback
/// panicked during this dispatch: the effects it queued in that callback were
/// discarded and the registry disabled it for the rest of the process.
#[derive(Debug)]
#[must_use]
pub struct HookOutcome<E> {
    pub effects: Result<EffectBatch<E>, ModuleEffectError>,
    pub disabled: Vec<ModuleId>,
}
