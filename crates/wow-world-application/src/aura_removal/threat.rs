// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::RemovedAuraLikeCpp;
use wow_world_core::session::PlayerAuraRemovalAccessLikeCpp;

/// Run after transform removal and before mount/control and AuraUpdate.
/// The consumer flag preserves World's test-only threat fixture fallback.
pub(super) fn remove_threat_phase_like_cpp(
    player: &mut PlayerAuraRemovalAccessLikeCpp<'_>,
    removed: &RemovedAuraLikeCpp,
    consumer_test: bool,
) {
    let aura = &removed.aura;
    player.remove_player_threat_aura_for_consumer_like_cpp(
        aura.spell_id,
        aura.caster_guid,
        aura.slot,
        aura.effect_mask,
        consumer_test,
    );
}
