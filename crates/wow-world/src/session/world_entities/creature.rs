//! Represented creature state owned by the Session boundary.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) async fn talked_to_creature_like_cpp(
        &mut self,
        creature_entry: u32,
        creature_guid: wow_core::ObjectGuid,
    ) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.talked_to_creature_with_generator_like_cpp(
            generator.as_ref(),
            creature_entry,
            creature_guid,
        )
        .await;
    }
    /// Present a health transition already committed by the canonical map
    /// owner without replaying the delayed value into canonical state.
    ///
    /// The command revision is a presentation high-water mark. The health and
    /// death tuple itself is always reread from the current canonical Player,
    /// so a heal, later hit, death, or resurrection that won after the queued
    /// swing cannot be rolled back by session delivery.
    pub(crate) fn present_committed_creature_melee_health_like_cpp(
        &mut self,
        committed_revision: u64,
    ) -> Option<u64> {
        if committed_revision == 0
            || committed_revision
                <= self
                    .view
                    .last_presented_creature_melee_health_state_revision_like_cpp
        {
            return None;
        }

        let canonical = self.core.with_owned_player_like_cpp(|player| {
            (
                player.unit().health_state_revision_like_cpp(),
                player.unit().data().health,
                player.unit().data().max_health,
                player.unit().is_alive(),
            )
        });
        #[cfg(test)]
        let canonical = canonical.or_else(|| {
            if self.core.player_handle_like_cpp.is_some() {
                return None;
            }
            self.core.mutate_canonical_player_like_cpp(|player| {
                (
                    player.unit().health_state_revision_like_cpp(),
                    player.unit().data().health,
                    player.unit().data().max_health,
                    player.unit().is_alive(),
                )
            })
        });
        let (canonical_revision, canonical_health, _canonical_max_health, _canonical_alive) =
            canonical?;
        if canonical_revision < committed_revision {
            return None;
        }

        #[cfg(test)]
        {
            self.fixtures.combat.player_max_health_like_cpp =
                _canonical_max_health.clamp(1, u64::from(u32::MAX)) as u32;
            self.fixtures.combat.player_health_like_cpp = canonical_health
                .min(u64::from(self.fixtures.combat.player_max_health_like_cpp))
                as u32;
            self.fixtures.combat.player_alive_like_cpp =
                _canonical_alive && self.fixtures.combat.player_health_like_cpp > 0;
        }
        self.view
            .last_presented_creature_melee_health_state_revision_like_cpp = committed_revision;
        self.sync_player_registry_state_like_cpp();
        Some(canonical_health)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/world_entities/creature/f3_shims.rs"]
mod f3_shims;
