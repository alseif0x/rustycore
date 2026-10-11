// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1363 slice 1: production-dead `combat_phases` entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl AdmittedCreatureCombatPhasesOutcomeLikeCpp {
    /// The deferred respawn DB statements, in the order they are submitted to
    /// the respawn writer.
    #[must_use]
    pub(crate) fn deferred_respawn_db_mutations_like_cpp(
        &self,
    ) -> &[wow_persistence::RespawnPersistenceMutationLikeCpp] {
        &self.lifecycle.respawn_db_mutations
    }
    /// The deferred visibility refresh the lifecycle phase drives: one
    /// `RefreshVisibleWorldCreaturesLikeCpp` per affected map, sent to every
    /// in-world session on that map and instance.
    #[must_use]
    pub(crate) fn deferred_visibility_refreshes_like_cpp(
        &self,
    ) -> Vec<crate::session::mailbox::RefreshVisibleWorldCreaturesLikeCppCommand> {
        self.lifecycle
            .refresh_map_keys
            .iter()
            .map(|&(map_id, instance_id)| {
                crate::session::mailbox::RefreshVisibleWorldCreaturesLikeCppCommand {
                    map_id,
                    instance_id,
                }
            })
            .collect()
    }
    /// The deferred plan events, in phase order (movement, aggro, spell, then
    /// melee).
    #[must_use]
    pub(crate) fn deferred_plan_events_like_cpp(&self) -> Vec<RuntimeEvent> {
        self.movement
            .plan
            .events
            .iter()
            .chain(&self.aggro.plan.events)
            .chain(&self.spell.plan.events)
            .chain(&self.melee.plan.events)
            .cloned()
            .collect()
    }
}
