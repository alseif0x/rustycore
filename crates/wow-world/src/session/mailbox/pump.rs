// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session mailbox pump.
//!
//! One consumer — the owning session task — drains both rails and hands each
//! committed command to gameplay. Draining order is deliberate: the durable
//! creature rail is presented first up to its first visibility-gated packet so
//! a pending refresh can run before it, then the bounded general rail, then the
//! deferred durable suffix. An overflowed durable backlog disconnects the
//! desynchronized session rather than dropping authoritative transitions.
//!
//! #368 moved what a command *does* to [`crate::session_commands`]: this file
//! owns the queue, not the gameplay. That took the methods this module names in
//! `handlers/` from 31 to one — `apply_pending_durable_item_loot_completions_like_cpp`,
//! a four-line loot wrapper the pump must call *before* the overflow check so a
//! session about to be disconnected still lands its committed loot. Hiding that
//! last call behind a second one-implementation wrapper would buy a cleaner
//! count and a worse boundary; the honest seam is the loot rail draining the way
//! the command rails now do, which is loot's own work and not this issue's.

use super::SessionCommand;
use crate::session::{SessionHandlerCatalogsLikeCpp, SessionState, WorldSession};

impl WorldSession {
    /// Clone the C++-style cross-session command channel for this active
    /// session.
    ///
    /// Worldserver-level registries use this as the Rust equivalent of holding
    /// a `WorldSession*` in `World::m_sessions` for commands such as
    /// `World::KickAll`; session state is still mutated only by the session
    /// task when it drains the channel.
    pub fn session_command_tx(&self) -> flume::Sender<SessionCommand> {
        self.core.session_command_tx.clone()
    }

    pub(crate) async fn process_represented_session_commands_with_catalogs_like_cpp(
        &mut self,
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) {
        self.apply_pending_durable_item_loot_completions_with_generator_like_cpp(
            catalogs.id_generators.item.as_ref(),
        )
        .await;
        let creature_runtime_overflowed =
            self.core.take_durable_creature_runtime_overflow_like_cpp();
        if creature_runtime_overflowed {
            self.kick(
                "authoritative creature runtime command backlog overflowed; disconnecting desynchronized session",
            );
            return;
        }
        let commands = self.core.drain_session_commands();
        for command in commands {
            self.apply_session_command_with_catalogs_like_cpp(catalogs, command)
                .await;
        }
        self.flush_pending_visibility_refresh_with_catalogs_like_cpp(
            catalogs.creature_spawns.as_ref(),
        )
        .await;
    }

    #[cfg(test)]
    pub(crate) async fn process_represented_session_commands_like_cpp(&mut self) {
        let catalogs = self.session_handler_catalogs_for_test_like_cpp();
        self.process_represented_session_commands_with_catalogs_like_cpp(&catalogs)
            .await;
    }
}
