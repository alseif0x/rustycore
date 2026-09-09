//! Recorded represented gameobject overrides layered over canonical state.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    #[allow(dead_code)]
    pub(crate) fn record_represented_gameobject_zone_area_like_cpp(
        &mut self,
        guid: ObjectGuid,
        zone_id: u32,
        area_id: u32,
    ) {
        let state = self
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        state.zone_id = Some(zone_id);
        state.area_id = Some(area_id);
    }
    pub(crate) fn record_represented_gameobject_lock_id_like_cpp(
        &mut self,
        guid: ObjectGuid,
        lock_id: u32,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .lock_id = (lock_id != 0).then_some(lock_id);
    }
    pub(crate) fn record_represented_gameobject_override_like_cpp(
        &mut self,
        guid: ObjectGuid,
        flags: u32,
        faction_template: u32,
        override_source_known: bool,
    ) {
        let state = self
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        state.gameobject_flags = flags;
        state.gameobject_override_flags = override_source_known.then_some(flags);
        state.faction_template = Some(faction_template);
    }
    pub(crate) fn record_represented_gameobject_display_model_like_cpp(
        &mut self,
        guid: ObjectGuid,
        display_id: u32,
        scale: f32,
        rotation: [f32; 4],
    ) {
        let state = self
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        state.display_id = (display_id != 0).then_some(display_id);
        state.scale = scale;
        state.rotation = rotation;
    }
    pub(crate) fn record_represented_gameobject_anim_progress_like_cpp(
        &mut self,
        guid: ObjectGuid,
        anim_progress: u8,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .go_anim_progress = anim_progress;
    }
    #[allow(dead_code)]
    pub(crate) fn record_represented_gameobject_owner_guid_like_cpp(
        &mut self,
        guid: ObjectGuid,
        owner_guid: ObjectGuid,
    ) {
        let owner_guid = (!owner_guid.is_empty()).then_some(owner_guid);
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .owner_guid = owner_guid;

        let Some(owner_guid) = owner_guid else {
            return;
        };
        let Some(map_key) =
            self.canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))
        else {
            return;
        };
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        let Some(map) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return;
        };
        if let Some(game_object) = map.map_mut().get_typed_game_object_mut(guid) {
            game_object.set_created_by(owner_guid);
        }
    }
    #[allow(dead_code)]
    pub(crate) fn record_represented_gameobject_spell_id_like_cpp(
        &mut self,
        guid: ObjectGuid,
        spell_id: u32,
    ) {
        self.set_canonical_gameobject_spell_id_like_cpp(guid, spell_id);
    }
    pub(crate) fn record_represented_gameobject_db_phase_shift_like_cpp(
        &mut self,
        guid: ObjectGuid,
        map_id: u16,
        phase_use_flags: u8,
        phase_id: u16,
        phase_group_id: u32,
        terrain_swap_map: i32,
    ) {
        let (phase_shift, _) = self.db_spawn_phase_shift_like_cpp(
            map_id,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
        );
        self.record_represented_gameobject_phase_shift_like_cpp(guid, phase_shift);
    }
    pub(crate) fn record_represented_gameobject_phase_shift_like_cpp(
        &mut self,
        guid: ObjectGuid,
        phase_shift: PhaseShift,
    ) {
        self.represented_gameobject_phase_shifts
            .insert(guid, phase_shift);
    }
}
