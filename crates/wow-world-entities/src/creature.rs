use wow_core::ObjectGuid;
use wow_world_core::map_manager::WorldCreature;
use wow_world_core::session::{HubMut, HubRef};

use crate::WorldEntitiesState;

pub fn creature_message_to_set_target_allows_like_cpp(
    creature: &WorldCreature,
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

    let range = creature.visibility_range_like_cpp();
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

impl WorldEntitiesState {
    pub fn represented_can_see_or_detect_world_creature_like_cpp(
        &self,
        hub: HubRef<'_>,
        creature: &WorldCreature,
    ) -> bool {
        let expected = wow_map::MapKey::new(
            u32::from(hub.core.player_map_id_like_cpp()),
            creature.instance_id(),
        );
        if hub.core.current_canonical_player_map_key_like_cpp() != Some(expected) {
            return false;
        }
        hub.core
            .with_owned_player_like_cpp(|player| {
                player.unit().can_see_or_detect_unit_like_cpp(
                    creature.creature.unit(),
                    false,
                    true,
                    false,
                )
            })
            .unwrap_or(false)
    }
}

impl WorldEntitiesState {
    /// The canonical creature aura slot of one `(spell, caster)` application.
    pub fn canonical_creature_aura_slot_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        target_guid: ObjectGuid,
        spell_id: u32,
        caster_guid: ObjectGuid,
    ) -> Option<u8> {
        hub.core
            .mutate_canonical_creature_by_guid_like_cpp(target_guid, |creature| {
                creature
                    .unit()
                    .subsystems()
                    .auras
                    .applied_auras
                    .iter()
                    .find(|aura| aura.spell_id == spell_id && aura.caster_guid == caster_guid)
                    .map(|aura| aura.slot)
            })
            .flatten()
    }
}
