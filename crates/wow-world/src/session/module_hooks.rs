// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The World adapter for trusted linked modules (#1445 H0).
//!
//! Domain crates never see `wow-module-api`. They raise a plain domain event
//! through a hook port, a provided host-trait method whose default does
//! nothing; `WorldSession` overrides the port and lands here. This adapter is
//! the only place that converts an event into a public snapshot, dispatches it
//! in `ModuleId` order, and applies the validated batch through the canonical
//! owner, so the public contract stays in one crate and one file.
//!
//! The registry is the process-owned, immutable `Arc` the compositor built.
//! The session factory installs it once at construction; nothing replaces or
//! mutates it afterwards (only a panicking module's disabled flag changes).
//!
//! Every dispatch runs with no split borrow, map guard or await in scope: the
//! ports are called where the domain already released its borrows, and this
//! adapter takes no lock of its own.

use std::sync::Arc;

use tracing::{debug, warn};
use wow_module_api::{
    HookFamily, HookOutcome, ModuleRegistry, NotificationEffect, PlayerLoginSnapshot,
};
use wow_world_application::PlayerLoginHookEventLikeCpp;
use wow_world_core::session::SessionCore;

/// The session's handle on the linked-module registry.
pub(crate) struct SessionModuleHooksLikeCpp {
    registry: Arc<ModuleRegistry>,
}

impl Default for SessionModuleHooksLikeCpp {
    /// The zero-module registry: every port returns before building anything.
    fn default() -> Self {
        Self::new(Arc::new(ModuleRegistry::new()))
    }
}

impl SessionModuleHooksLikeCpp {
    pub(crate) fn new(registry: Arc<ModuleRegistry>) -> Self {
        Self { registry }
    }

    #[cfg(test)]
    pub(crate) fn registry_like_cpp(&self) -> &Arc<ModuleRegistry> {
        &self.registry
    }

    /// `player.login`: C++ `ScriptMgr::OnPlayerLogin`
    /// (`CharacterHandler.cpp:1390`); AzerothCore
    /// `PlayerScript::OnPlayerLogin` (`PlayerScript.h:332`).
    ///
    /// A rejected batch is logged and the login continues: a module must not
    /// be able to fail a player's login.
    pub(crate) fn player_login_like_cpp(
        &self,
        core: &SessionCore,
        event: &PlayerLoginHookEventLikeCpp,
    ) {
        if !self.registry.handles(HookFamily::PlayerLogin) {
            return;
        }
        let snapshot = PlayerLoginSnapshot {
            guid: event.guid,
            name: event.name.clone(),
            race: event.race,
            class: event.class,
            level: event.level,
            map_id: event.map_id,
            first_login: event.first_login,
        };
        let outcome = self.registry.dispatch_player_login_outcome(&snapshot);
        apply_notifications_like_cpp(core, "player.login", outcome);
    }
}

/// Apply a validated notification batch to the session's own player.
///
/// The batch is all or nothing: a validation error applies none of it. A
/// module a panic disabled is logged once, here, with the hook that hit it.
fn apply_notifications_like_cpp(
    core: &SessionCore,
    hook: &'static str,
    outcome: HookOutcome<NotificationEffect>,
) {
    for module in &outcome.disabled {
        warn!(%module, hook, "linked module panicked; disabled for the rest of the process");
    }
    match outcome.effects {
        Ok(effects) => {
            for (module, effect) in effects.iter() {
                match effect {
                    NotificationEffect::SendSystemMessageSelf { text } => {
                        debug!(%module, hook, "module system message");
                        core.send_system_message_like_cpp(text);
                    }
                }
            }
        }
        Err(error) => {
            warn!(%error, hook, "module effect batch rejected; no effect applied");
        }
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/module_hooks.rs"]
mod tests;
