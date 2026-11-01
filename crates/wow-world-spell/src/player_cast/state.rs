//! Session cast execution snapshots and pending-cast views.

use crate::SessionSpellState;
use std::time::Instant;
use wow_entities::{
    CastExecutionStateLikeCpp,
    PendingSpellCastRequestLikeCpp as RepresentedPendingSpellCastRequestLikeCpp, SpellCastState,
};
use wow_world_core::session::HubRef;

impl SessionSpellState {
    pub(crate) fn with_cast_execution_like_cpp<R>(
        &self,
        hub: HubRef<'_>,
        f: impl FnOnce(&CastExecutionStateLikeCpp) -> R,
    ) -> Option<R> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            return Some(f(&CastExecutionStateLikeCpp {
                active: self.active_spell_cast.clone(),
                last_cast_time: self.last_spell_cast_time,
                last_cast_time_per_spell: self.last_spell_cast_time_per_spell.clone(),
            }));
        }
        hub.core
            .with_owned_player_like_cpp(|player| f(&player.unit().subsystems().spells.execution))
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mutate_cast_execution_fixture_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut CastExecutionStateLikeCpp) -> R,
    ) -> Option<R> {
        let mut state = CastExecutionStateLikeCpp {
            active: self.active_spell_cast.clone(),
            last_cast_time: self.last_spell_cast_time,
            last_cast_time_per_spell: self.last_spell_cast_time_per_spell.clone(),
        };
        let result = f(&mut state);
        self.active_spell_cast = state.active;
        self.last_spell_cast_time = state.last_cast_time;
        self.last_spell_cast_time_per_spell = state.last_cast_time_per_spell;
        Some(result)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn mutate_pending_spell_cast_fixture_like_cpp<R>(
        &mut self,
        f: impl FnOnce(&mut Option<RepresentedPendingSpellCastRequestLikeCpp>) -> R,
    ) -> R {
        f(&mut self.represented_pending_spell_cast_request_like_cpp)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn active_spell_cast_snapshot_like_cpp(&self, hub: HubRef<'_>) -> Option<SpellCastState> {
        self.with_cast_execution_like_cpp(hub, |state| state.active.clone())
            .flatten()
    }

    pub fn last_spell_cast_time_like_cpp(&self, hub: HubRef<'_>) -> Option<Option<Instant>> {
        self.with_cast_execution_like_cpp(hub, |state| state.last_cast_time)
    }

    pub fn spell_last_cast_time_like_cpp(
        &self,
        hub: HubRef<'_>,
        spell_id: i32,
    ) -> Option<Option<Instant>> {
        self.with_cast_execution_like_cpp(hub, |state| {
            state.last_cast_time_per_spell.get(&spell_id).copied()
        })
    }

    pub fn pending_spell_cast_snapshot_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<Option<RepresentedPendingSpellCastRequestLikeCpp>> {
        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            return Some(self.represented_pending_spell_cast_request_like_cpp.clone());
        }
        hub.core
            .with_owned_player_like_cpp(wow_entities::Player::pending_spell_cast_snapshot_like_cpp)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn pending_spell_cast_for_test_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<RepresentedPendingSpellCastRequestLikeCpp> {
        self.pending_spell_cast_snapshot_like_cpp(hub).flatten()
    }

    pub fn remaining_global_cooldown_ms_like_cpp(
        &self,
        hub: HubRef<'_>,
        spell_info: &wow_data::SpellInfo,
    ) -> Option<u32> {
        self.with_cast_execution_like_cpp(hub, |state| {
            state.remaining_global_cooldown_ms(spell_info.cooldown_ms)
        })
    }

    pub fn remaining_active_spell_cast_ms_like_cpp(&self, hub: HubRef<'_>) -> Option<u32> {
        self.with_cast_execution_like_cpp(hub, CastExecutionStateLikeCpp::remaining_cast_ms)
    }
}
