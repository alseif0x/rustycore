use super::*;

impl<'a> GameObjectUseValues<'a> {
    // Preparation stays after the original clamp and before slot inspection.
    // This keeps the caller's entry point without storing a Session or context.
    #[allow(clippy::too_many_arguments)]
    pub fn use_chair(
        player_guid: ObjectGuid,
        player_position: Position,
        gameobject_position: Position,
        gameobject_size: f32,
        source: ChairUseSource,
        prepare: impl FnOnce() -> Self,
    ) -> Option<ChairPlacement> {
        let slot_count = source.chair_slots.max(1).min(5);
        let values = prepare();
        if values.chair_slots.is_empty() {
            *values.chair_slots = vec![None; slot_count as usize];
        }

        let orthogonal_orientation = gameobject_position.orientation + std::f32::consts::PI * 0.5;
        let mut nearest_slot = None;
        let mut lowest_dist = f32::MAX;
        let mut nearest_position = gameobject_position;
        for slot in 0..values.chair_slots.len() {
            if values.chair_slots[slot].is_some() {
                continue;
            }

            let relative_distance =
                (gameobject_size * slot as f32) - (gameobject_size * (slot_count - 1) as f32 / 2.0);
            let candidate = Position::new(
                gameobject_position.x + relative_distance * orthogonal_orientation.cos(),
                gameobject_position.y + relative_distance * orthogonal_orientation.sin(),
                gameobject_position.z,
                gameobject_position.orientation,
            );
            let dist = player_position.distance_2d(&candidate);
            if dist <= lowest_dist {
                nearest_slot = Some(slot);
                lowest_dist = dist;
                nearest_position = candidate;
            }
        }

        let Some(slot) = nearest_slot else {
            return None;
        };

        values.chair_slots[slot] = Some(player_guid);
        let stand_state = 4_u32.saturating_add(source.chair_height);
        Some(ChairPlacement {
            slot: slot as u32,
            teleport_position: nearest_position,
            raw_stand_state: stand_state,
            trigger_event: (source.triggered_event_id != 0).then_some(source.triggered_event_id),
        })
    }
}
