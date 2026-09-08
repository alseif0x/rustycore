// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Durable Session mailbox rails.
//!
//! The creature-runtime rail keeps authoritative combat transitions in
//! committed FIFO order under bounded memory, and the loot-money rail fences
//! durable persistence against admission, logout and unknown-commit outcomes.
//! Both survive general-queue backpressure, so they never silently drop work.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use super::protocol::{
    ApplyCreatureMeleeDamageLikeCppCommand, ApplyPlayerMeleeResultLikeCppCommand,
    CreatureAttackStartLikeCppCommand, CreatureAttackStopLikeCppCommand,
    ReconcilePvpCombatExpiryLikeCppCommand, SendCreatureSpellCastIfVisibleLikeCppCommand,
    SendIfVisibleLikeCppCommand, SendPlayerSpellIfVisibleLikeCppCommand, SessionCommand,
};

/// Retained FIFO handoff for committed map-owned creature transitions and
/// Player cast publication decisions. Retention is in memory, not database durability.
///
/// The bounded general-purpose session queue may legitimately reject visual
/// fanout under backpressure. These commands cannot be dropped, but the global
/// map tick also cannot wait for a stalled session. C++ publishes every melee
/// swing and attack transition in order, so this rail retains each committed
/// event until the owning session drains it. A session that cannot drain a
/// bounded backlog is marked desynchronized and disconnected instead of
/// silently losing authoritative events or growing memory without limit.
pub const MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP: usize = 4_096;

#[derive(Default)]
pub struct DurableCreatureRuntimeCommandsLikeCpp {
    commands: VecDeque<SessionCommand>,
    overflowed: bool,
    deferred_visibility: Option<wow_map::PlayerVisibilityRefreshIntentLikeCpp>,
}

impl DurableCreatureRuntimeCommandsLikeCpp {
    pub fn publish_player_spell_if_visible_like_cpp(
        &mut self,
        command: SendPlayerSpellIfVisibleLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::SendPlayerSpellIfVisibleLikeCpp(command))
    }

    fn publish_like_cpp(&mut self, command: SessionCommand) -> bool {
        if self.commands.len() >= MAX_DURABLE_CREATURE_RUNTIME_COMMANDS_LIKE_CPP {
            self.overflowed = true;
            return false;
        }
        self.commands.push_back(command);
        true
    }

    pub fn publish_attack_start_like_cpp(
        &mut self,
        command: CreatureAttackStartLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::CreatureAttackStartLikeCpp(command))
    }

    pub fn publish_attack_stop_like_cpp(
        &mut self,
        command: CreatureAttackStopLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::CreatureAttackStopLikeCpp(command))
    }

    pub fn publish_pvp_combat_expiry_like_cpp(
        &mut self,
        command: ReconcilePvpCombatExpiryLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::ReconcilePvpCombatExpiryLikeCpp(command))
    }

    pub fn publish_melee_damage_like_cpp(
        &mut self,
        command: ApplyCreatureMeleeDamageLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command))
    }

    /// Publish one map-owned player auto-attack resolution.
    ///
    /// Durable, not the bounded rail: a dropped result is a lost kill, and with
    /// it the loot and the experience. The bounded rail is allowed to shed a
    /// visibility refresh; it is not allowed to shed a death (#28).
    pub fn publish_player_melee_result_like_cpp(
        &mut self,
        command: ApplyPlayerMeleeResultLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::ApplyPlayerMeleeResultLikeCpp(command))
    }

    pub fn publish_send_if_visible_like_cpp(
        &mut self,
        command: SendIfVisibleLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::SendIfVisibleLikeCpp(command))
    }

    /// Publish START+GO as one queue element so capacity checks and session
    /// drains cannot observe only one half of a committed spell cast.
    pub fn publish_creature_spell_cast_if_visible_like_cpp(
        &mut self,
        command: SendCreatureSpellCastIfVisibleLikeCppCommand,
    ) -> bool {
        self.publish_like_cpp(SessionCommand::SendCreatureSpellCastIfVisibleLikeCpp(
            command,
        ))
    }

    /// One map-selected obligation, independent of the bounded general queue.
    /// The directory validates incarnation before publishing; residence ordering
    /// additionally prevents a delayed producer from replacing newer work.
    pub(crate) fn retain_deferred_visibility_like_cpp(
        &mut self,
        intent: wow_map::PlayerVisibilityRefreshIntentLikeCpp,
    ) {
        if self.deferred_visibility.is_some_and(|pending| {
            pending.handle() == intent.handle()
                && pending.residence_revision() > intent.residence_revision()
        }) {
            return;
        }
        self.deferred_visibility = Some(intent);
    }

    pub fn drain_like_cpp(&mut self) -> Vec<SessionCommand> {
        let mut commands: Vec<_> = self.commands.drain(..).collect();
        if let Some(intent) = self.deferred_visibility.take() {
            // Preserve the committed transition prefix. Visibility must run
            // before presentation packets, not before their state application.
            let first_visible = commands
                .iter()
                .position(SessionCommand::is_visibility_gated_like_cpp)
                .unwrap_or(commands.len());
            commands.insert(
                first_visible,
                SessionCommand::RefreshDeferredPlayerVisibilityLikeCpp(intent),
            );
        }
        commands
    }

    pub fn take_overflowed_and_discard_like_cpp(&mut self) -> bool {
        let overflowed = std::mem::take(&mut self.overflowed);
        if overflowed {
            self.commands.clear();
            self.deferred_visibility = None;
        }
        overflowed
    }
}
