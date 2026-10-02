//! Entity update bridge state definitions, part 2 of 2.
//!
//! Separated from the entity_update_bridge.rs root under #662. Behaviour is preserved.

use super::*;

pub(super) fn empty_area_trigger_values_update() -> AreaTriggerDataValuesUpdate {
    AreaTriggerDataValuesUpdate {
        changed_object_type_mask: 0,
        object_data: None,
        area_trigger_data_mask: 0,
        override_scale_curve: scale_curve_values_update(wow_entities::ScaleCurveValues::default()),
        extra_scale_curve: scale_curve_values_update(wow_entities::ScaleCurveValues::default()),
        override_move_curve_x: scale_curve_values_update(wow_entities::ScaleCurveValues::default()),
        override_move_curve_y: scale_curve_values_update(wow_entities::ScaleCurveValues::default()),
        override_move_curve_z: scale_curve_values_update(wow_entities::ScaleCurveValues::default()),
        caster: wow_core::ObjectGuid::EMPTY,
        duration: 0,
        time_to_target: 0,
        time_to_target_scale: 0,
        time_to_target_extra_scale: 0,
        time_to_target_pos: 0,
        spell_id: 0,
        spell_for_visuals: 0,
        spell_visual_id: 0,
        bounds_radius_2d: 0.0,
        decal_properties_id: 0,
        creating_effect_guid: wow_core::ObjectGuid::EMPTY,
        orbit_path_target: wow_core::ObjectGuid::EMPTY,
        visual_anim: VisualAnimValuesUpdate {
            visual_anim_mask: 0,
            field_c: false,
            animation_data_id: 0,
            anim_kit_id: 0,
            anim_progress: 0,
        },
    }
}

pub(super) fn scale_curve_values_update(
    values: wow_entities::ScaleCurveValues,
) -> ScaleCurveValuesUpdate {
    ScaleCurveValuesUpdate {
        scale_curve_mask: 0x0F,
        override_active: values.override_active,
        start_time_offset: values.start_time_offset,
        parameter_curve: values.parameter_curve,
        points: [(0.0, 0.0); 2],
    }
}

pub(super) fn scene_object_data_update_to_packet(
    update: &SceneObjectDataUpdate,
) -> SceneObjectDataValuesUpdate {
    SceneObjectDataValuesUpdate {
        changed_object_type_mask: 1 << TYPEID_SCENE_OBJECT,
        object_data: None,
        scene_object_data_mask: update.mask.blocks().first().copied().unwrap_or(0),
        script_package_id: update.values.script_package_id,
        rnd_seed_val: update.values.rnd_seed_val,
        created_by: update.values.created_by,
        scene_type: update.values.scene_type,
    }
}

pub(super) fn empty_scene_object_values_update() -> SceneObjectDataValuesUpdate {
    SceneObjectDataValuesUpdate {
        changed_object_type_mask: 0,
        object_data: None,
        scene_object_data_mask: 0,
        script_package_id: 0,
        rnd_seed_val: 0,
        created_by: wow_core::ObjectGuid::EMPTY,
        scene_type: 0,
    }
}

pub(super) fn conversation_data_update_to_packet(
    update: &ConversationDataUpdate,
) -> ConversationDataValuesUpdate {
    ConversationDataValuesUpdate {
        changed_object_type_mask: 1 << TYPEID_CONVERSATION,
        object_data: None,
        conversation_data_mask: update.mask.blocks().first().copied().unwrap_or(0),
        lines: update
            .values
            .lines
            .iter()
            .map(|line| ConversationLineValuesUpdate {
                conversation_line_id: line.conversation_line_id,
                start_time: line.start_time,
                ui_camera_id: line.ui_camera_id,
                actor_index: line.actor_index,
                flags: line.flags,
            })
            .collect(),
        actors: update
            .values
            .actors
            .iter()
            .map(|actor| ConversationActorValuesUpdate {
                actor_type: actor.actor_type,
                id: actor.id,
                creature_id: actor.creature_id,
                creature_display_info_id: actor.creature_display_info_id,
                actor_guid: actor.actor_guid,
            })
            .collect(),
        actor_update_mask: None,
        last_line_end_time: update.values.last_line_end_time,
    }
}

pub(super) fn empty_conversation_values_update() -> ConversationDataValuesUpdate {
    ConversationDataValuesUpdate {
        changed_object_type_mask: 0,
        object_data: None,
        conversation_data_mask: 0,
        lines: Vec::new(),
        actors: Vec::new(),
        actor_update_mask: None,
        last_line_end_time: 0,
    }
}

pub(super) fn active_player_data_update_to_packet(
    update: &ActivePlayerDataUpdate,
) -> PacketActivePlayerDataValuesUpdate {
    let mut packet_update = PacketActivePlayerDataValuesUpdate::default();
    copy_mask_blocks(
        update.mask.blocks(),
        &mut packet_update.active_player_data_mask,
    );
    packet_update.coinage = update.values.coinage;
    packet_update.xp = update.values.xp;
    packet_update.next_level_xp = update.values.next_level_xp;
    packet_update.character_points = update.values.character_points;
    packet_update.honor = update.values.honor;
    packet_update.honor_next_level = update.values.honor_next_level;
    packet_update.watched_faction_index = update.values.watched_faction_index;
    packet_update.scaling_player_level_delta = update.values.scaling_player_level_delta;
    packet_update.num_backpack_slots = update.values.num_backpack_slots;
    packet_update.farsight_object = update.values.farsight_object;
    packet_update.summoned_battle_pet_guid = update.values.summoned_battle_pet_guid;
    packet_update
        .inv_slots
        .copy_from_slice(&update.values.inv_slots);
    packet_update
        .explored_zones
        .copy_from_slice(&update.values.explored_zones);
    for ((dst, src), nested_mask) in packet_update
        .rest_info
        .iter_mut()
        .zip(update.values.rest_info.iter())
        .zip(update.rest_info_change_masks.iter())
    {
        *dst = RestInfoValuesUpdate {
            rest_info_mask: *nested_mask,
            threshold: src.threshold,
            state_id: src.state_id,
        };
    }
    packet_update.buyback_price = update.values.buyback_price;
    packet_update.buyback_timestamp = update.values.buyback_timestamp;
    packet_update.bank_bag_slot_flags = update.values.bank_bag_slot_flags;
    packet_update.heirlooms = update.values.heirlooms.clone();
    packet_update.heirlooms_update_mask = update.values.heirlooms_update_mask.clone();
    packet_update.heirloom_flags = update.values.heirloom_flags.clone();
    packet_update.heirloom_flags_update_mask = update.values.heirloom_flags_update_mask.clone();
    packet_update.toys = update.values.toys.clone();
    packet_update.toys_update_mask = update.values.toys_update_mask.clone();
    packet_update.transmog = update.values.transmog.clone();
    packet_update.transmog_update_mask = update.values.transmog_update_mask.clone();
    packet_update.conditional_transmog = update.values.conditional_transmog.clone();
    packet_update.conditional_transmog_update_mask =
        update.values.conditional_transmog_update_mask.clone();
    packet_update.quest_completed = update.values.quest_completed;
    packet_update
}

pub(super) fn mask_to_u64(blocks: &[u32]) -> u64 {
    blocks
        .iter()
        .take(2)
        .enumerate()
        .fold(0u64, |acc, (index, block)| {
            acc | ((*block as u64) << (index * 32))
        })
}

pub(super) fn copy_mask_blocks<const N: usize>(src: &[u32], dst: &mut [u32; N]) {
    let count = src.len().min(N);
    dst[..count].copy_from_slice(&src[..count]);
}
