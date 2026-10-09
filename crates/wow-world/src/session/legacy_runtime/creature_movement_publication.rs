//! Canonical-gated publication of one global legacy creature movement tick.
//!
//! #1263 F6-8B. The movement tick drives each creature's canonical runtime
//! state — `CreatureRuntimeLikeCpp`: `Unit::movespline`, the persistent
//! generators and `Unit::i_motionMaster` — and captures one frame per creature
//! while the legacy map write guard is held. This module owns the second half
//! of that tick: it transports each frame's representation through the shared
//! canonical admission gate and publishes the frame's events **only** for a
//! representation the canonical incarnation applied. A frame the gate refuses
//! or defers therefore decides nothing; the legacy copy is not a second
//! publication authority.
//!
//! The legacy-only configuration (no canonical manager) has no canonical
//! application to gate on and keeps publishing by name.

use super::*;

/// One creature's movement frame from one global movement tick.
///
/// Captured while the legacy map write guard is held so the canonical gate is
/// never re-entered from inside that guard. The published event order is the
/// tick's established order: the reached-home values update first, then the
/// movement packet.
pub(in crate::session) struct CreatureMovementFrameLikeCpp {
    pub(in crate::session) map_id: u16,
    pub(in crate::session) instance_id: u32,
    pub(in crate::session) guid: ObjectGuid,
    /// Transported representation offered to the canonical incarnation.
    pub(in crate::session) snapshot: wow_entities::Creature,
    /// Position the spline was launched from, read from the creature's canonical
    /// runtime state after this frame's `Unit::Update`.
    pub(in crate::session) source_position: Position,
    pub(in crate::session) visibility_range: f32,
    /// Reached-home `Unit::Update` values update, when the frame restored health.
    pub(in crate::session) home_health_restore: Option<crate::map_manager::RuntimeEvent>,
    /// `MonsterMove`/`MonsterMoveStop` bytes built from the spline this frame
    /// launched into the creature's canonical runtime state.
    pub(in crate::session) movement_packet: Option<Vec<u8>>,
}

/// Transport every captured frame through the canonical admission gate and
/// publish the accepted frames into `outcome`.
///
/// `canonical_syncs` stays an **attempt** count: one increment for every frame
/// processed while a canonical manager exists, including refusals.
pub(in crate::session) fn publish_creature_movement_frames_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    frames: Vec<CreatureMovementFrameLikeCpp>,
    outcome: &mut LegacyCreatureMovementTickOutcomeLikeCpp,
) {
    use crate::map_manager::{RecipientRule, RuntimeEvent};

    for frame in frames {
        let applied = match canonical_map_manager {
            Some(canonical) => {
                let expected_legacy_authority = frame.snapshot.loot_authority_like_cpp().clone();
                let expected_legacy_stamp = expected_legacy_authority.stamp_like_cpp();
                outcome.canonical_syncs += 1;
                // F6-7 R7b-2b: the shared map-level gate admits the transported
                // representation against the current canonical incarnation
                // before applying it, and rebinds the legacy alias only for an
                // applied snapshot.
                sync_admitted_creature_representation_on_map_like_cpp(
                    canonical,
                    Some(legacy_map_manager),
                    frame.map_id,
                    frame.instance_id,
                    frame.snapshot,
                    &expected_legacy_authority,
                    expected_legacy_stamp,
                )
            }
            // Legacy-only runtime: there is no canonical application to gate the
            // publication on, so this configuration is preserved by name.
            None => true,
        };
        if !applied {
            // The canonical incarnation refused or deferred this representation,
            // so the legacy copy must not publish a movement projection the
            // canonical authority did not accept.
            continue;
        }
        if let Some(event) = frame.home_health_restore {
            outcome.plan.events.push(event);
        }
        if let Some(packet_bytes) = frame.movement_packet {
            outcome.movement_packets += 1;
            outcome.plan.events.push(RuntimeEvent {
                source_guid: frame.guid,
                recipients: RecipientRule::NearbyVisible {
                    source_guid: frame.guid,
                    map_id: frame.map_id,
                    instance_id: frame.instance_id,
                    source_position: frame.source_position,
                    range: frame.visibility_range,
                    required_3d: false,
                },
                packet_bytes,
            });
        }
    }
}
