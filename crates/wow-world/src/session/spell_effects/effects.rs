//! Remaining represented effect operations owned by this responsibility.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The server-triggered cast, the
//! transmog-set and kill-credit effects moved to `wow_world_spell` (#1363
//! slice 9d); the homebind write and the farsight seer consumer with its
//! viewpoint create moved there too (#1363 slice 12b2), and the nearby-entry
//! implicit-condition check with its destination search (#1363 slice 12b2b).

use super::*;

impl WorldSession {
    /// C++ `Spell::EffectForceDeselect`.
    ///
    /// Represented boundary: packet construction and effect evidence only.
    /// C++ delivers `SMSG_BREAK_TARGET` and `SMSG_CLEAR_TARGET` to hostile
    /// visible clients around the caster, then stops attacker pets without
    /// threat lists. The hostile visible-set fanout and attacker traversal are
    /// deferred to the live visibility/combat runtime.
    pub(in crate::session) fn apply_force_deselect_effect_like_cpp(&mut self) -> bool {
        let Some(caster_guid) = self.player_guid() else {
            return false;
        };

        #[cfg(test)]
        {
            use wow_packet::ServerPacket;
            let break_target_packet_bytes = wow_packet::packets::combat::BreakTarget {
                unit_guid: caster_guid,
            }
            .to_bytes();
            let clear_target_packet_bytes =
                wow_packet::packets::spell::ClearTarget { guid: caster_guid }.to_bytes();

            self.social
                .record_represented_force_deselect_for_test_like_cpp(
                    RepresentedForceDeselectLikeCpp {
                        caster_guid,
                        visibility_range_yards: DEFAULT_VISIBILITY_DISTANCE_YARDS_LIKE_CPP,
                        break_target_packet_bytes,
                        clear_target_packet_bytes,
                        hostile_visible_fanout_unrepresented: true,
                        attacker_pet_attack_stop_unrepresented: true,
                    },
                );
        }
        #[cfg(not(test))]
        let _ = caster_guid;
        true
    }
}
