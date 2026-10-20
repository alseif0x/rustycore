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
    CreatureKillSnapshot, HookFamily, HookOutcome, LogoutReason, ModuleRegistry,
    NotificationEffect, PlayerLevelChangedSnapshot, PlayerLoginSnapshot, PlayerLogoutSnapshot,
    PlayerMoneyChangedSnapshot, QuestAcceptedSnapshot, QuestRewardedSnapshot, SpellCastSnapshot,
};
use wow_world_application::{PlayerLoginHookEventLikeCpp, PlayerLogoutHookEventLikeCpp};
use wow_world_core::session::PacketPublicationAccessLikeCpp;
use wow_world_entities::CreatureKillHookEventLikeCpp;
use wow_world_lifecycle::FinalizationMode;
use wow_world_quest::{
    PlayerLevelChangedHookEventLikeCpp, PlayerMoneyChangedHookEventLikeCpp,
    QuestAcceptedHookEventLikeCpp, QuestModuleHookPortLikeCpp, QuestRewardedHookEventLikeCpp,
};
use wow_world_spell::SpellCastHookEventLikeCpp;

/// The session's handle on the linked-module registry.
pub(crate) struct SessionModuleHooksLikeCpp {
    registry: Arc<ModuleRegistry>,
    /// Whether the session factory already installed the process registry.
    installed: bool,
}

impl Default for SessionModuleHooksLikeCpp {
    /// The zero-module registry: every port returns before building anything.
    fn default() -> Self {
        Self {
            registry: Arc::new(ModuleRegistry::new()),
            installed: false,
        }
    }
}

impl SessionModuleHooksLikeCpp {
    /// Install the process registry. Composition does this exactly once per
    /// session; a second install would silently swap the modules a running
    /// session dispatches to, so it is refused (and asserted in debug).
    pub(crate) fn install_like_cpp(&mut self, registry: Arc<ModuleRegistry>) -> bool {
        debug_assert!(
            !self.installed,
            "the linked-module registry is installed once per session"
        );
        if self.installed {
            warn!("ignoring a second linked-module registry install for this session");
            return false;
        }
        self.registry = registry;
        self.installed = true;
        true
    }

    /// Whether any enabled module offers `family`: the ports' interest query,
    /// asked before an event is built.
    pub(crate) fn wants_like_cpp(&self, family: HookFamily) -> bool {
        self.registry.handles(family)
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
        publication: PacketPublicationAccessLikeCpp<'_>,
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
        apply_notifications_like_cpp(publication, "player.login", outcome);
    }

    /// `player.logout`: C++ `ScriptMgr::OnPlayerLogout`
    /// (`WorldSession.cpp:655`); AzerothCore `PlayerScript::OnPlayerLogout`
    /// (`PlayerScript.h:338`). The session is closing, so modules observe
    /// only; the outcome can carry no effect to apply.
    pub(crate) fn player_logout_like_cpp(&self, event: &PlayerLogoutHookEventLikeCpp) {
        if !self.registry.handles(HookFamily::Player) {
            return;
        }
        let reason = match event.mode {
            FinalizationMode::CharacterSelection => LogoutReason::CharacterSelection,
            FinalizationMode::TimedLogout => LogoutReason::TimedLogout,
            FinalizationMode::Disconnect => LogoutReason::Disconnect,
        };
        let snapshot = PlayerLogoutSnapshot::new(event.guid, event.account_id, reason);
        let outcome = self.registry.dispatch_player_logout(&snapshot);
        log_disabled_like_cpp("player.logout", &outcome.disabled);
    }

    /// `player.quest_accepted`: AzerothCore `OnPlayerQuestAccept`
    /// (`PlayerQuest.cpp:426`, `PlayerScript.h:750`).
    pub(crate) fn quest_accepted_like_cpp(
        &self,
        publication: PacketPublicationAccessLikeCpp<'_>,
        event: &QuestAcceptedHookEventLikeCpp,
    ) {
        if !self.registry.handles(HookFamily::Player) {
            return;
        }
        let snapshot = QuestAcceptedSnapshot::new(event.guid, event.quest_id, event.slot);
        let outcome = self.registry.dispatch_quest_accepted(&snapshot);
        apply_notifications_like_cpp(publication, "player.quest_accepted", outcome);
    }
}

impl SessionModuleHooksLikeCpp {
    /// `player.creature_kill`: TrinityCore `OnCreatureKill` (`Unit.cpp:10755`);
    /// AzerothCore `OnPlayerCreatureKill` (`Unit.cpp:14298`, `PlayerScript.h:254`).
    pub(crate) fn creature_kill_like_cpp(
        &self,
        publication: PacketPublicationAccessLikeCpp<'_>,
        event: &CreatureKillHookEventLikeCpp,
    ) {
        if !self.registry.handles(HookFamily::Player) {
            return;
        }
        let snapshot = CreatureKillSnapshot::new(
            event.guid,
            event.creature_guid,
            event.creature_entry,
            event.creature_level,
            event.map_id,
        );
        let outcome = self.registry.dispatch_creature_kill(&snapshot);
        apply_notifications_like_cpp(publication, "player.creature_kill", outcome);
    }

    /// `player.spell_cast`: TrinityCore `OnPlayerSpellCast` (`Spell.cpp:3671`);
    /// AzerothCore `PlayerScript.h:326`. Self messages only (#754).
    pub(crate) fn spell_cast_like_cpp(
        &self,
        publication: PacketPublicationAccessLikeCpp<'_>,
        event: &SpellCastHookEventLikeCpp,
    ) {
        if !self.registry.handles(HookFamily::Player) {
            return;
        }
        let snapshot = SpellCastSnapshot::new(
            event.guid,
            event.spell_id,
            event.cast_id,
            event.target_guid,
            event.triggered,
        );
        let outcome = self.registry.dispatch_spell_cast(&snapshot);
        apply_notifications_like_cpp(publication, "player.spell_cast", outcome);
    }
}

/// The quest and progression context port (#1444 H1).
impl QuestModuleHookPortLikeCpp for SessionModuleHooksLikeCpp {
    fn player_hooks_wanted_like_cpp(&self) -> bool {
        self.wants_like_cpp(HookFamily::Player)
    }

    /// `player.level_changed`: AzerothCore `OnPlayerLevelChanged`
    /// (`Player.cpp:2585`, `PlayerScript.h:263`).
    fn player_level_changed_like_cpp(
        &self,
        publication: PacketPublicationAccessLikeCpp<'_>,
        event: &PlayerLevelChangedHookEventLikeCpp,
    ) {
        if !self.registry.handles(HookFamily::Player) {
            return;
        }
        let snapshot =
            PlayerLevelChangedSnapshot::new(event.guid, event.old_level, event.new_level);
        let outcome = self.registry.dispatch_player_level_changed(&snapshot);
        apply_notifications_like_cpp(publication, "player.level_changed", outcome);
    }

    /// `player.money_changed`: AzerothCore `OnPlayerMoneyChanged`
    /// (`Player.cpp:11688`, `PlayerScript.h:282`), observation only.
    fn player_money_changed_like_cpp(
        &self,
        publication: PacketPublicationAccessLikeCpp<'_>,
        event: &PlayerMoneyChangedHookEventLikeCpp,
    ) {
        if !self.registry.handles(HookFamily::Player) {
            return;
        }
        let snapshot =
            PlayerMoneyChangedSnapshot::new(event.guid, event.old_money, event.new_money);
        let outcome = self.registry.dispatch_player_money_changed(&snapshot);
        apply_notifications_like_cpp(publication, "player.money_changed", outcome);
    }

    /// `player.quest_rewarded`: AzerothCore `OnPlayerCompleteQuest`
    /// (`PlayerQuest.cpp:903`, `PlayerScript.h:245`).
    fn quest_rewarded_like_cpp(
        &self,
        publication: PacketPublicationAccessLikeCpp<'_>,
        event: &QuestRewardedHookEventLikeCpp,
    ) {
        if !self.registry.handles(HookFamily::Player) {
            return;
        }
        let snapshot = QuestRewardedSnapshot::new(
            event.guid,
            event.quest_id,
            event.quest_giver,
            event.choice_item_id,
            event.reward_xp,
            event.money_before,
            event.money_after,
        );
        let outcome = self.registry.dispatch_quest_rewarded(&snapshot);
        apply_notifications_like_cpp(publication, "player.quest_rewarded", outcome);
    }
}

fn log_disabled_like_cpp(hook: &'static str, disabled: &[wow_module_api::ModuleId]) {
    for module in disabled {
        warn!(%module, hook, "linked module panicked; disabled for the rest of the process");
    }
}

/// Apply a validated notification batch to the session's own player.
///
/// The batch is all or nothing: a validation error applies none of it. A
/// module a panic disabled is logged once, here, with the hook that hit it.
fn apply_notifications_like_cpp(
    publication: PacketPublicationAccessLikeCpp<'_>,
    hook: &'static str,
    outcome: HookOutcome<NotificationEffect>,
) {
    log_disabled_like_cpp(hook, &outcome.disabled);
    match outcome.effects {
        Ok(effects) => {
            for (module, effect) in effects.iter() {
                match effect {
                    NotificationEffect::SendSystemMessageSelf { text } => {
                        debug!(%module, hook, "module system message");
                        publication.send_system_message_like_cpp(text);
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
