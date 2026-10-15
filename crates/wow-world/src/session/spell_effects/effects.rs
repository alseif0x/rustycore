//! Remaining represented effect operations owned by this responsibility.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged. The server-triggered cast, the
//! transmog-set and kill-credit effects moved to `wow_world_spell` (#1363
//! slice 9d); the homebind write and the farsight seer consumer with its
//! viewpoint create moved there too (#1363 slice 12b2).

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_nearby_candidate_meets_implicit_conditions_like_cpp(
        &self,
        candidate: &WorldObject,
        candidate_unit_snapshot: Option<wow_conditions::ConditionUnitSnapshot>,
        implicit_conditions: Option<&[wow_data::Condition]>,
        caster_object: Option<&WorldObject>,
        player_unit_snapshot: wow_conditions::ConditionUnitSnapshot,
        player_snapshot: wow_conditions::ConditionPlayerSnapshot,
        player_condition_store: Option<&wow_data::PlayerConditionStore>,
        player_condition_context: Option<&RepresentedPlayerConditionContextLikeCpp>,
        area_table_store: Option<&wow_data::AreaTableStore>,
    ) -> bool {
        let Some(conditions) = implicit_conditions.filter(|conditions| !conditions.is_empty())
        else {
            return true;
        };
        let Some(condition_store) = self.catalogs.condition_store.as_ref() else {
            return false;
        };

        // C++ `WorldObjectSpellTargetCheck` builds `ConditionSourceInfo(nullptr, caster)`
        // and then assigns the tested target to condition slot 0.
        let mut source_info =
            wow_conditions::ConditionSourceInfo::from_targets(Some(candidate), caster_object, None);
        if let Some(snapshot) = candidate_unit_snapshot {
            source_info.set_unit_target_snapshot(0, snapshot);
        }
        source_info.set_unit_target_snapshot(1, player_unit_snapshot);
        source_info.set_player_target_snapshot(1, player_snapshot);
        if let (Some(store), Some(context)) = (player_condition_store, player_condition_context) {
            source_info.set_player_condition_store(store);
            if let Some(context) = context.as_context(self) {
                source_info.set_player_condition_context(1, context);
            }
        }

        wow_conditions::is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions,
            condition_store.as_ref(),
            |condition, source_info| {
                wow_conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |area_id, required_area_id| {
                        area_table_store.is_some_and(|store| {
                            store.is_in_area_like_cpp(area_id, required_area_id)
                        })
                    },
                )
                .value()
                .unwrap_or(false)
            },
        )
    }
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
