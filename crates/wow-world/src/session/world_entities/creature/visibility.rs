//! Existing creature visibility/admission selectors; no new runtime producer.
use super::*;

pub(crate) fn creature_message_to_set_target_allows_like_cpp(
    creature: &crate::map_manager::WorldCreature,
    source_is_visible_like_cpp: bool,
    player_map_id: u32,
    player_instance_id: u32,
    player_position: &wow_core::Position,
    player_phase_shift: &wow_entities::PhaseShift,
    required_3d: bool,
) -> bool {
    if !source_is_visible_like_cpp {
        return false;
    }
    creature_message_source_allows(
        &creature.capture_message_source(),
        source_is_visible_like_cpp,
        player_map_id,
        player_instance_id,
        player_position,
        player_phase_shift,
        required_3d,
    )
}

fn creature_message_source_allows(
    creature: &wow_map::CreatureMessageSourceFacts,
    source_is_visible_like_cpp: bool,
    player_map_id: u32,
    player_instance_id: u32,
    player_position: &wow_core::Position,
    player_phase_shift: &wow_entities::PhaseShift,
    required_3d: bool,
) -> bool {
    if !source_is_visible_like_cpp {
        return false;
    }
    if creature.map_id() != player_map_id || creature.instance_id() != player_instance_id {
        return false;
    }
    if !player_phase_shift.can_see(creature.phase_shift()) {
        return false;
    }

    let range = creature.visibility_range();
    if required_3d {
        wow_core::position_is_in_dist_strict_3d_like_cpp(
            &creature.position(),
            player_position,
            range,
        )
    } else {
        wow_core::position_is_in_dist_strict_2d_like_cpp(
            &creature.position(),
            player_position,
            range,
        )
    }
}

// Keep the borrowed compatibility source until the original late selector.
// By-GUID queries have already captured their owned facts under the Map guard.
enum CreatureMessageSource<'a> {
    Legacy(&'a crate::map_manager::WorldCreature),
    Captured(&'a wow_map::CreatureMessageSourceFacts),
}

impl WorldSession {
    pub(crate) fn active_world_creature_guids_for_update_like_cpp(&self) -> Vec<ObjectGuid> {
        self.active_world_creature_guids_with_fixture_phase(false)
    }

    pub(crate) fn active_world_creature_guids_with_fixture_phase(&self, fixture_phase: bool) -> Vec<ObjectGuid> {
        let Some(player_position) = self.player_position_like_cpp() else {
            return Vec::new();
        };
        let Some(manager) = &self.map_manager else {
            return Vec::new();
        };
        let Some(player_phase_shift) = self.represented_player_phase_shift_like_cpp().or_else(|| {
            (fixture_phase && self.player_handle_like_cpp.is_none())
                .then(PhaseShift::default)
        }) else {
            return Vec::new();
        };
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .active_creature_guids_for_player_update_like_cpp(
                map_id,
                instance_id,
                player_position,
                &player_phase_shift,
            )
    }

    pub(in crate::session) fn represented_can_see_or_detect_creature_facts(
        &self,
        creature: &wow_map::CreatureVisibilityCandidate,
    ) -> bool {
        let expected = wow_map::MapKey::new(
            u32::from(self.player_map_id_like_cpp()),
            creature.create().instance_id(),
        );
        if self.current_canonical_player_map_key_like_cpp() != Some(expected) {
            return false;
        }
        self.with_owned_player_like_cpp(|player| {
            player.unit().can_see_or_detect_target(
                creature.visibility_target(),
                false,
                true,
                false,
            )
        })
        .unwrap_or(false)
    }

    pub(in crate::session) fn represented_can_receive_creature_message_to_set_like_cpp(
        &self,
        guid: ObjectGuid,
        creature: &crate::map_manager::WorldCreature,
        required_3d: bool,
    ) -> bool {
        self.can_receive_creature_message_source(
            guid,
            CreatureMessageSource::Legacy(creature),
            required_3d,
        )
    }

    fn can_receive_creature_message_source(
        &self,
        guid: ObjectGuid,
        source: CreatureMessageSource<'_>,
        required_3d: bool,
    ) -> bool {
        if !self.client_visible_guids_like_cpp.contains(&guid) {
            return false;
        }

        let Some(player_position) = self.player_position_like_cpp() else {
            return false;
        };
        let player_map_id = u32::from(self.player_map_id_like_cpp());
        let player_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let Some(player_phase_shift) = self.represented_player_phase_shift_like_cpp() else {
            return false;
        };

        match source {
            CreatureMessageSource::Legacy(creature) => creature_message_to_set_target_allows_like_cpp(
                creature,
                true,
                player_map_id,
                player_instance_id,
                &player_position,
                &player_phase_shift,
                required_3d,
            ),
            CreatureMessageSource::Captured(creature) => creature_message_source_allows(
                creature,
                true,
                player_map_id,
                player_instance_id,
                &player_position,
                &player_phase_shift,
                required_3d,
            ),
        }
    }

    pub(crate) fn represented_can_receive_creature_message_to_set_by_guid_like_cpp(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        required_3d: bool,
    ) -> Option<bool> {
        self.represented_can_receive_creature_message_to_set_by_guid_with_legacy_fallback_like_cpp(
            guid,
            map_id,
            instance_id,
            required_3d,
            false,
        )
    }

    pub(crate) fn represented_can_receive_creature_message_to_set_by_guid_with_legacy_fallback_like_cpp(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        required_3d: bool,
        allow_legacy_fallback: bool,
    ) -> Option<bool> {
        if let Some(manager) = &self.canonical_map_manager {
            let creature = {
                let manager = manager.lock().ok()?;
                manager
                    .find_map(u32::from(map_id), instance_id)
                    .and_then(|map| map.map().capture_compatible_creature_message_source(guid))
            };
            if let Some(creature) = creature {
                if creature.map_id() != u32::from(map_id) || creature.instance_id() != instance_id {
                    return Some(false);
                }
                return Some(
                    self.can_receive_creature_message_source(
                        guid,
                        CreatureMessageSource::Captured(&creature),
                        required_3d,
                    ),
                );
            }
            if !allow_legacy_fallback {
                return None;
            }
        }

        // The two map managers coexist during the incremental runtime
        // migration. A canonical manager being installed does not prove that
        // it owns a source already validated from the legacy map. Only the
        // provenance-marked command path may cross this fallback boundary.
        let creature = {
            let manager = self.map_manager.as_ref()?;
            manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .find_creature(map_id, instance_id, guid)
                .map(crate::map_manager::WorldCreature::capture_message_source)
        }?;
        Some(
            self.can_receive_creature_message_source(
                guid,
                CreatureMessageSource::Captured(&creature),
                required_3d,
            ),
        )
    }
}
