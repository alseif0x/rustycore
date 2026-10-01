// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[cfg(test)]
    pub(in crate::session) fn mutate_player_unit_presentation_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        self.core.mutate_player_unit_presentation_like_cpp(mutate)
    }
    pub(crate) fn canonical_player_effective_combat_stats_like_cpp(
        &self,
    ) -> Option<wow_entities::PlayerEffectiveCombatStatsLikeCpp> {
        self.core.canonical_player_effective_combat_stats_like_cpp()
    }
    pub(in crate::session) fn with_owned_player_like_cpp<R>(
        &self,
        f: impl FnOnce(&Player) -> R,
    ) -> Option<R> {
        self.core.with_owned_player_like_cpp(f)
    }
    pub(crate) fn mutate_canonical_player_by_guid_like_cpp<R>(
        &self,
        guid: ObjectGuid,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        self.core.mutate_canonical_player_by_guid_like_cpp(guid, f)
    }
    pub(in crate::session) fn canonical_player_has_player_flag_like_cpp(
        &self,
        guid: ObjectGuid,
        flag: u32,
    ) -> Option<bool> {
        self.core
            .canonical_player_has_player_flag_like_cpp(guid, flag)
    }
    pub(in crate::session) fn canonical_player_snapshot_like_cpp<R>(
        &self,
        f: impl FnOnce(&Player) -> R,
    ) -> Option<R> {
        self.core.canonical_player_snapshot_like_cpp(f)
    }
    pub(in crate::session) fn with_owned_player_mut_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        self.core.with_owned_player_mut_like_cpp(f)
    }
    pub(crate) fn mutate_canonical_player_like_cpp<R>(
        &self,
        f: impl FnOnce(&mut Player) -> R,
    ) -> Option<R> {
        self.core.mutate_canonical_player_like_cpp(f)
    }
    pub(in crate::session) fn mutate_player_collection_state_like_cpp<R>(
        &mut self,
        mutate: impl FnOnce(&mut wow_entities::PlayerCollectionStateLikeCpp) -> R,
    ) -> Option<R> {
        crate::session::hub_mut(self).mutate_player_collection_state_like_cpp(mutate)
    }
}
