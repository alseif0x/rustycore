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

use super::protocol::*;
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
        self.session_command_tx.clone()
    }

    pub(crate) fn drain_session_commands(&self) -> Vec<SessionCommand> {
        let durable_commands = self
            .durable_creature_runtime_commands_like_cpp
            .lock()
            .map(|mut pending| pending.drain_like_cpp())
            .unwrap_or_default();
        // Drain the bounded general rail before the first durable presentation
        // packet so a pending visibility refresh can run first. The rails do
        // not yet share an enqueue ordinal, so this is not a global cross-rail
        // ordering guarantee. The spell pair itself still occupies one durable
        // command and therefore cannot be split by this merge.
        let first_visible = durable_commands
            .iter()
            .position(|command| {
                matches!(
                    command,
                    SessionCommand::SendIfVisibleLikeCpp(_)
                        | SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(_)
                        | SessionCommand::SendRealmIfVisibleLikeCpp(_)
                        | SessionCommand::SendRealmIfVisibleFromLegacySourceLikeCpp(_)
                )
            })
            .unwrap_or(durable_commands.len());
        let mut commands = durable_commands;
        let deferred_durable_suffix = commands.split_off(first_visible);
        while let Ok(command) = self.session_command_rx.try_recv() {
            commands.push(command);
        }
        commands.extend(deferred_durable_suffix);
        commands
    }

    pub(crate) fn take_durable_creature_runtime_overflow_like_cpp(&self) -> bool {
        self.durable_creature_runtime_commands_like_cpp
            .lock()
            .map(|mut pending| pending.take_overflowed_and_discard_like_cpp())
            .unwrap_or(true)
    }

    pub(crate) async fn process_represented_session_commands_with_catalogs_like_cpp(
        &mut self,
        catalogs: &SessionHandlerCatalogsLikeCpp,
    ) {
        self.apply_pending_durable_item_loot_completions_with_generator_like_cpp(
            catalogs.id_generators.item.as_ref(),
        )
        .await;
        let creature_runtime_overflowed = self.take_durable_creature_runtime_overflow_like_cpp();
        if creature_runtime_overflowed {
            self.kick(
                "authoritative creature runtime command backlog overflowed; disconnecting desynchronized session",
            );
            return;
        }
        let commands = self.drain_session_commands();
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
