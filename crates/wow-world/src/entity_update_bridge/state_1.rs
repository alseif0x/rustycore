//! Entity update bridge state definitions, part 1 of 2.
//!
//! Separated from the entity_update_bridge.rs root under #662. Behaviour is preserved.

use super::*;

pub(super) const VISIBLE_ITEM_FULL_UPDATE_MASK: u32 = 0x0F;

pub fn corpse_create_data_from_entity_like_cpp(corpse: &Corpse) -> CorpseCreateData {
    let world = corpse.world();
    let object = world.object().object_data_values();
    let data = corpse.data();
    CorpseCreateData {
        guid: world.guid(),
        entry_id: u32::try_from(object.entry_id).unwrap_or(0),
        object_dynamic_flags: object.dynamic_flags,
        scale: object.scale,
        position: world.position(),
        corpse_dynamic_flags: data.dynamic_flags,
        owner: data.owner,
        party_guid: data.party_guid,
        guild_guid: data.guild_guid,
        display_id: data.display_id,
        items: data.items,
        race_id: data.race_id,
        sex: data.sex,
        class: data.class,
        customizations: data
            .customizations
            .iter()
            .map(|customization| ChrCustomizationChoiceValuesUpdate {
                option_id: customization.option_id,
                choice_id: customization.choice_id,
            })
            .collect(),
        flags: data.flags,
        faction_template: data.faction_template,
    }
}

pub fn scene_object_create_data_from_entity_like_cpp(
    scene_object: &SceneObject,
) -> SceneObjectCreateData {
    let world = scene_object.world();
    let object = world.object().object_data_values();
    let data = scene_object.data();
    SceneObjectCreateData {
        guid: world.guid(),
        entry_id: u32::try_from(object.entry_id).unwrap_or(0),
        dynamic_flags: object.dynamic_flags,
        scale: object.scale,
        position: scene_object.stationary_position(),
        script_package_id: data.script_package_id,
        rnd_seed_val: data.rnd_seed_val,
        created_by: data.created_by,
        scene_type: data.scene_type,
    }
}

pub fn conversation_create_data_from_entity_like_cpp(
    conversation: &Conversation,
    receiver_locale: &str,
) -> ConversationCreateData {
    let world = conversation.world();
    let object = world.object().object_data_values();
    let data = conversation.data();
    let locale = wow_data::locale_index_like_cpp(receiver_locale);
    ConversationCreateData {
        guid: world.guid(),
        entry_id: u32::try_from(object.entry_id).unwrap_or(0),
        dynamic_flags: object.dynamic_flags,
        scale: object.scale,
        position: conversation.stationary_position(),
        texture_kit_id: conversation.texture_kit_id(),
        lines: data
            .lines
            .iter()
            .map(|line| ConversationLineValuesUpdate {
                conversation_line_id: line.conversation_line_id,
                start_time: conversation
                    .line_start_time(locale as u8, line.conversation_line_id)
                    .map_or(line.start_time, |start_time| start_time as u32),
                ui_camera_id: line.ui_camera_id,
                actor_index: line.actor_index,
                flags: line.flags,
            })
            .collect(),
        actors: data
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
        // C++ initializes one end time per locale to zero and sends the
        // receiver's slot directly, not the aggregate update-field value.
        last_line_end_time: conversation.last_line_end_time(locale).unwrap_or_default(),
    }
}

pub fn player_values_update_to_packet(
    update: &PlayerValuesUpdate,
) -> Option<PlayerDataValuesDeltaUpdate> {
    let mut packet_update = PlayerDataValuesDeltaUpdate {
        changed_object_type_mask: 0,
        ..Default::default()
    };

    if let Some(player_data) = &update.player_data {
        packet_update.changed_object_type_mask |= 1 << TYPEID_PLAYER;
        copy_player_data_update(player_data, &mut packet_update);
    }

    if let Some(object_data) = &update.object_data {
        packet_update.changed_object_type_mask |= 1 << TYPEID_OBJECT;
        packet_update.object_data = Some(object_data_update_to_packet(object_data));
    }

    if let Some(unit_data) = &update.unit_data {
        packet_update.changed_object_type_mask |= 1 << TYPEID_UNIT;
        packet_update.unit_data = Some(unit_data_update_to_packet(unit_data));
    }

    if let Some(active_player_data) = &update.active_player_data {
        packet_update.changed_object_type_mask |= 1 << TYPEID_ACTIVE_PLAYER;
        packet_update.active_player_data =
            Some(active_player_data_update_to_packet(active_player_data));
    }

    (packet_update.changed_object_type_mask != 0).then_some(packet_update)
}

pub fn player_values_update_to_update_object(
    guid: wow_core::ObjectGuid,
    map_id: u16,
    update: &PlayerValuesUpdate,
) -> Option<UpdateObject> {
    player_values_update_to_packet(update)
        .map(|packet_update| UpdateObject::full_player_values_update(guid, map_id, packet_update))
}

pub fn object_values_update_to_packet(
    update: &ObjectDataUpdate,
    changed_object_type_mask: u32,
) -> ObjectDataValuesUpdate {
    object_data_update_to_packet_with_type_mask(update, changed_object_type_mask)
}

pub fn object_values_update_to_update_object(
    guid: wow_core::ObjectGuid,
    map_id: u16,
    update: &ObjectDataUpdate,
    changed_object_type_mask: u32,
) -> UpdateObject {
    UpdateObject::object_values_update(
        guid,
        map_id,
        object_values_update_to_packet(update, changed_object_type_mask),
    )
}

pub fn unit_values_update_to_packet(
    update: &UnitValuesUpdate,
) -> Option<UnitDataValuesDeltaUpdate> {
    let mut packet_update = update
        .unit_data
        .as_ref()
        .map(unit_data_update_to_packet)
        .unwrap_or_default();
    packet_update.changed_object_type_mask = update.changed_object_type_mask;
    if let Some(object_data) = &update.object_data {
        packet_update.object_data = Some(object_data_update_to_packet(object_data));
    }
    update.has_data().then_some(packet_update)
}

pub fn unit_values_update_to_update_object(
    guid: wow_core::ObjectGuid,
    map_id: u16,
    update: &UnitValuesUpdate,
) -> Option<UpdateObject> {
    unit_values_update_to_packet(update)
        .map(|packet_update| UpdateObject::unit_values_update(guid, map_id, packet_update))
}

pub fn item_values_update_to_packet(
    update: &ItemValuesUpdate,
) -> Option<ItemDataValuesDeltaUpdate> {
    let mut packet_update = update
        .item_data
        .as_ref()
        .map(item_data_update_to_packet)
        .unwrap_or_else(empty_item_values_update);
    packet_update.changed_object_type_mask = update.changed_object_type_mask;
    if let Some(object_data) = &update.object_data {
        packet_update.object_data = Some(object_data_update_to_packet(object_data));
    }
    update.has_data().then_some(packet_update)
}

pub fn item_values_update_to_update_object(
    guid: wow_core::ObjectGuid,
    map_id: u16,
    update: &ItemValuesUpdate,
) -> Option<UpdateObject> {
    item_values_update_to_packet(update)
        .map(|packet_update| UpdateObject::full_item_values_update(guid, map_id, packet_update))
}

pub fn bag_values_update_to_packet(update: &BagValuesUpdate) -> Option<ContainerDataValuesUpdate> {
    let mut packet_update = if let Some(container_data) = &update.container_data {
        container_data_update_to_packet(container_data)
    } else {
        ContainerDataValuesUpdate {
            changed_object_type_mask: update.changed_object_type_mask,
            object_data: None,
            item_data: None,
            container_data_mask: 0,
            num_slots: 0,
            slots: [wow_core::ObjectGuid::EMPTY; 36],
        }
    };
    packet_update.changed_object_type_mask = update.changed_object_type_mask;
    if let Some(object_data) = &update.object_data {
        packet_update.object_data = Some(object_data_update_to_packet(object_data));
    }
    if let Some(item_data) = &update.item_data {
        packet_update.item_data = Some(item_data_update_to_packet(item_data));
    }
    update.has_data().then_some(packet_update)
}

pub fn bag_values_update_to_update_object(
    guid: wow_core::ObjectGuid,
    map_id: u16,
    update: &BagValuesUpdate,
) -> Option<UpdateObject> {
    bag_values_update_to_packet(update)
        .map(|packet_update| UpdateObject::container_values_update(guid, map_id, packet_update))
}

pub fn game_object_values_update_to_packet(
    update: &GameObjectValuesUpdate,
) -> Option<GameObjectDataValuesUpdate> {
    let mut packet_update = update
        .game_object_data
        .as_ref()
        .map(game_object_data_update_to_packet)
        .unwrap_or_else(empty_game_object_values_update);
    packet_update.changed_object_type_mask = update.changed_object_type_mask;
    if let Some(object_data) = &update.object_data {
        packet_update.object_data = Some(object_data_update_to_packet(object_data));
    }
    update.has_data().then_some(packet_update)
}

pub fn game_object_values_update_to_update_object(
    guid: wow_core::ObjectGuid,
    map_id: u16,
    update: &GameObjectValuesUpdate,
) -> Option<UpdateObject> {
    game_object_values_update_to_packet(update)
        .map(|packet_update| UpdateObject::game_object_values_update(guid, map_id, packet_update))
}

pub fn dynamic_object_values_update_to_packet(
    update: &DynamicObjectValuesUpdate,
) -> Option<DynamicObjectDataValuesUpdate> {
    let mut packet_update = update
        .dynamic_object_data
        .as_ref()
        .map(dynamic_object_data_update_to_packet)
        .unwrap_or_else(empty_dynamic_object_values_update);
    packet_update.changed_object_type_mask = update.changed_object_type_mask;
    if let Some(object_data) = &update.object_data {
        packet_update.object_data = Some(object_data_update_to_packet(object_data));
    }
    update.has_data().then_some(packet_update)
}

pub fn dynamic_object_values_update_to_update_object(
    guid: wow_core::ObjectGuid,
    map_id: u16,
    update: &DynamicObjectValuesUpdate,
) -> Option<UpdateObject> {
    dynamic_object_values_update_to_packet(update).map(|packet_update| {
        UpdateObject::dynamic_object_values_update(guid, map_id, packet_update)
    })
}

pub fn corpse_values_update_to_packet(
    update: &CorpseValuesUpdate,
) -> Option<CorpseDataValuesUpdate> {
    let mut packet_update = update
        .corpse_data
        .as_ref()
        .map(corpse_data_update_to_packet)
        .unwrap_or_else(empty_corpse_values_update);
    packet_update.changed_object_type_mask = update.changed_object_type_mask;
    if let Some(object_data) = &update.object_data {
        packet_update.object_data = Some(object_data_update_to_packet(object_data));
    }
    update.has_data().then_some(packet_update)
}

pub fn corpse_values_update_to_update_object(
    guid: wow_core::ObjectGuid,
    map_id: u16,
    update: &CorpseValuesUpdate,
) -> Option<UpdateObject> {
    corpse_values_update_to_packet(update)
        .map(|packet_update| UpdateObject::corpse_values_update(guid, map_id, packet_update))
}

pub fn area_trigger_values_update_to_packet(
    update: &AreaTriggerValuesUpdate,
) -> Option<AreaTriggerDataValuesUpdate> {
    let mut packet_update = update
        .area_trigger_data
        .as_ref()
        .map(area_trigger_data_update_to_packet)
        .unwrap_or_else(empty_area_trigger_values_update);
    packet_update.changed_object_type_mask = update.changed_object_type_mask;
    if let Some(object_data) = &update.object_data {
        packet_update.object_data = Some(object_data_update_to_packet(object_data));
    }
    update.has_data().then_some(packet_update)
}

pub fn area_trigger_values_update_to_update_object(
    guid: wow_core::ObjectGuid,
    map_id: u16,
    update: &AreaTriggerValuesUpdate,
) -> Option<UpdateObject> {
    area_trigger_values_update_to_packet(update)
        .map(|packet_update| UpdateObject::area_trigger_values_update(guid, map_id, packet_update))
}

pub fn area_trigger_create_data_from_entity_like_cpp(
    area_trigger: &wow_entities::AreaTrigger,
) -> AreaTriggerCreateData {
    let object = area_trigger.world().object();
    let object_data = object.object_data_values();
    let data = area_trigger.data();
    let create_properties_flags = area_trigger.create_properties_flags();
    let shape = area_trigger.shape();

    AreaTriggerCreateData {
        guid: object.guid(),
        entry_id: u32::try_from(object_data.entry_id).unwrap_or(0),
        dynamic_flags: object_data.dynamic_flags,
        scale: object_data.scale,
        position: area_trigger.stationary_position(),
        time_since_created_ms: area_trigger.time_since_created_ms(),
        roll_pitch_yaw: area_trigger.roll_pitch_yaw(),
        target_roll_pitch_yaw: area_trigger.target_roll_pitch_yaw(),
        create_properties_flags: create_properties_flags.flags,
        scale_curve_id: create_properties_flags.scale_curve_id,
        morph_curve_id: create_properties_flags.morph_curve_id,
        facing_curve_id: create_properties_flags.facing_curve_id,
        move_curve_id: create_properties_flags.move_curve_id,
        shape: AreaTriggerShapeCreateData {
            shape_type: area_trigger.shape_type() as u8,
            data: shape.data,
            polygon_vertices: shape
                .polygon_vertices
                .iter()
                .map(|position| AreaTriggerPosition2CreateData {
                    x: position.x,
                    y: position.y,
                })
                .collect(),
            polygon_vertices_target: shape
                .polygon_vertices_target
                .iter()
                .map(|position| AreaTriggerPosition2CreateData {
                    x: position.x,
                    y: position.y,
                })
                .collect(),
        },
        spline_points: area_trigger
            .spline_points()
            .iter()
            .map(|position| AreaTriggerPosition3CreateData {
                x: position.x,
                y: position.y,
                z: position.z,
            })
            .collect(),
        orbit: area_trigger
            .orbit_info()
            .map(|orbit| AreaTriggerOrbitCreateData {
                counter_clockwise: orbit.counter_clockwise,
                can_loop: orbit.can_loop,
                time_to_target: orbit.time_to_target,
                elapsed_time_for_movement: orbit.elapsed_time_for_movement,
                start_delay: orbit.start_delay,
                radius: orbit.radius,
                blend_from_radius: orbit.blend_from_radius,
                initial_angle: orbit.initial_angle,
                z_offset: orbit.z_offset,
                center: AreaTriggerPosition3CreateData {
                    x: area_trigger.stationary_position().x,
                    y: area_trigger.stationary_position().y,
                    z: area_trigger.stationary_position().z,
                },
            }),
        override_scale_curve: scale_curve_values_update(data.override_scale_curve),
        extra_scale_curve: scale_curve_values_update(data.extra_scale_curve),
        override_move_curve_x: scale_curve_values_update(data.override_move_curve_x),
        override_move_curve_y: scale_curve_values_update(data.override_move_curve_y),
        override_move_curve_z: scale_curve_values_update(data.override_move_curve_z),
        caster: data.caster,
        duration: data.duration,
        time_to_target: data.time_to_target,
        time_to_target_scale: data.time_to_target_scale,
        time_to_target_extra_scale: data.time_to_target_extra_scale,
        time_to_target_pos: data.time_to_target_pos,
        spell_id: data.spell_id,
        spell_for_visuals: data.spell_for_visuals,
        spell_visual_id: data.spell_visual_id,
        bounds_radius_2d: data.bounds_radius_2d,
        decal_properties_id: data.decal_properties_id,
        creating_effect_guid: data.creating_effect_guid,
        orbit_path_target: data.orbit_path_target,
        visual_anim: VisualAnimValuesUpdate {
            visual_anim_mask: 0x1F,
            field_c: data.visual_anim.field_c,
            animation_data_id: data.visual_anim.animation_data_id,
            anim_kit_id: data.visual_anim.anim_kit_id,
            anim_progress: data.visual_anim.anim_progress,
        },
    }
}

pub fn scene_object_values_update_to_packet(
    update: &SceneObjectValuesUpdate,
) -> Option<SceneObjectDataValuesUpdate> {
    let mut packet_update = update
        .scene_object_data
        .as_ref()
        .map(scene_object_data_update_to_packet)
        .unwrap_or_else(empty_scene_object_values_update);
    packet_update.changed_object_type_mask = update.changed_object_type_mask;
    if let Some(object_data) = &update.object_data {
        packet_update.object_data = Some(object_data_update_to_packet(object_data));
    }
    update.has_data().then_some(packet_update)
}

pub fn scene_object_values_update_to_update_object(
    guid: wow_core::ObjectGuid,
    map_id: u16,
    update: &SceneObjectValuesUpdate,
) -> Option<UpdateObject> {
    scene_object_values_update_to_packet(update)
        .map(|packet_update| UpdateObject::scene_object_values_update(guid, map_id, packet_update))
}

pub fn conversation_values_update_to_packet(
    update: &ConversationValuesUpdate,
) -> Option<ConversationDataValuesUpdate> {
    let mut packet_update = update
        .conversation_data
        .as_ref()
        .map(conversation_data_update_to_packet)
        .unwrap_or_else(empty_conversation_values_update);
    packet_update.changed_object_type_mask = update.changed_object_type_mask;
    if let Some(object_data) = &update.object_data {
        packet_update.object_data = Some(object_data_update_to_packet(object_data));
    }
    update.has_data().then_some(packet_update)
}

pub fn conversation_values_update_to_update_object(
    guid: wow_core::ObjectGuid,
    map_id: u16,
    update: &ConversationValuesUpdate,
) -> Option<UpdateObject> {
    conversation_values_update_to_packet(update)
        .map(|packet_update| UpdateObject::conversation_values_update(guid, map_id, packet_update))
}

pub(super) fn object_data_update_to_packet(update: &ObjectDataUpdate) -> ObjectDataValuesUpdate {
    object_data_update_to_packet_with_type_mask(update, 1 << TYPEID_OBJECT)
}

pub(super) fn object_data_update_to_packet_with_type_mask(
    update: &ObjectDataUpdate,
    changed_object_type_mask: u32,
) -> ObjectDataValuesUpdate {
    ObjectDataValuesUpdate {
        changed_object_type_mask,
        object_data_mask: update.mask.blocks().first().copied().unwrap_or(0),
        entry_id: update.values.entry_id,
        dynamic_flags: update.values.dynamic_flags,
        scale: update.values.scale,
    }
}

pub(super) fn copy_player_data_update(
    update: &PlayerDataUpdate,
    packet_update: &mut PlayerDataValuesDeltaUpdate,
) {
    copy_mask_blocks(update.mask.blocks(), &mut packet_update.player_data_mask);
    packet_update.loot_target_guid = update.values.loot_target_guid;
    packet_update.player_flags = update.values.player_flags;
    packet_update.player_flags_ex = update.values.player_flags_ex;
    packet_update.party_type = update.values.party_type;
    packet_update.num_bank_slots = update.values.num_bank_slots;
    packet_update.native_sex = update.values.native_sex;
    packet_update.inebriation = update.values.inebriation;
    packet_update.player_title = update.values.player_title;
    packet_update.current_spec_id = update.values.current_spec_id;
    packet_update.current_battle_pet_breed_quality = update.values.current_battle_pet_breed_quality;
    packet_update.honor_level = update.values.honor_level;

    for (dst, src) in packet_update
        .visible_items
        .iter_mut()
        .zip(update.values.visible_items.iter())
    {
        *dst = VisibleItemValuesUpdate {
            visible_item_mask: VISIBLE_ITEM_FULL_UPDATE_MASK,
            item_id: src.item_id,
            appearance_mod_id: src.item_appearance_mod_id,
            item_visual: src.item_visual,
        };
    }
}

pub(super) fn unit_data_update_to_packet(update: &UnitDataUpdate) -> UnitDataValuesDeltaUpdate {
    let mut packet_update = UnitDataValuesDeltaUpdate::default();
    copy_mask_blocks(update.mask.blocks(), &mut packet_update.unit_data_mask);
    packet_update.health = update.values.health.min(i64::MAX as u64) as i64;
    packet_update.max_health = update.values.max_health.min(i64::MAX as u64) as i64;
    packet_update.display_id = update.values.display_id;
    packet_update.critter = update.values.critter;
    packet_update.battle_pet_companion_guid = update.values.battle_pet_companion_guid;
    packet_update.battle_pet_companion_name_timestamp =
        update.values.battle_pet_companion_name_timestamp;
    packet_update.target = update.values.target;
    packet_update.race = update.values.race;
    packet_update.class_id = update.values.class_id;
    packet_update.player_class_id = update.values.player_class_id;
    packet_update.sex = update.values.sex;
    packet_update.display_power = update.values.display_power;
    packet_update.level = update.values.level;
    packet_update.faction_template = update.values.faction_template;
    packet_update.flags = update.values.flags;
    packet_update.flags2 = update.values.flags2;
    packet_update.flags3 = update.values.flags3;
    packet_update.npc_flags = update.values.npc_flags;
    packet_update.bounding_radius = update.values.bounding_radius;
    packet_update.combat_reach = update.values.combat_reach;
    packet_update.display_scale = update.values.display_scale;
    packet_update.native_display_id = update.values.native_display_id;
    packet_update.native_display_scale = update.values.native_display_scale;
    packet_update.mount_display_id = update.values.mount_display_id;
    packet_update.stand_state = update.values.stand_state;
    packet_update.vis_flags = update.values.vis_flags;
    packet_update.anim_tier = update.values.anim_tier;
    packet_update.sheathe_state = update.values.sheathe_state;
    packet_update.pvp_flags = update.values.pvp_flags;
    packet_update.pet_flags = update.values.pet_flags;
    packet_update.shapeshift_form = update.values.shapeshift_form;
    packet_update.mod_casting_speed = update.values.mod_casting_speed;
    packet_update.mod_spell_haste = update.values.mod_spell_haste;
    packet_update.mod_haste = update.values.mod_haste;
    packet_update.mod_ranged_haste = update.values.mod_ranged_haste;
    packet_update.mod_haste_regen = update.values.mod_haste_regen;
    packet_update.mod_time_rate = update.values.mod_time_rate;
    packet_update.emote_state = update.values.emote_state;
    packet_update.hover_height = update.values.hover_height;
    packet_update.wild_battle_pet_level = update.values.wild_battle_pet_level;
    packet_update.base_mana = update.values.base_mana;
    packet_update.power = update.values.power;
    packet_update.max_power = update.values.max_power;

    for (dst, src) in packet_update
        .virtual_items
        .iter_mut()
        .zip(update.values.virtual_items.iter())
    {
        *dst = VisibleItemValuesUpdate {
            visible_item_mask: VISIBLE_ITEM_FULL_UPDATE_MASK,
            item_id: src.item_id,
            appearance_mod_id: src.item_appearance_mod_id,
            item_visual: src.item_visual,
        };
    }

    packet_update
}

pub(super) fn item_data_update_to_packet(update: &ItemDataUpdate) -> ItemDataValuesDeltaUpdate {
    let mut spell_charges = [0; 5];
    spell_charges.copy_from_slice(&update.values.spell_charges);

    let mut enchantments = [ItemEnchantmentValuesUpdate::default(); 13];
    for (dst, src) in enchantments
        .iter_mut()
        .zip(update.values.enchantments.iter())
    {
        *dst = ItemEnchantmentValuesUpdate {
            item_enchantment_mask: 0x1F,
            id: src.id,
            duration: src.duration,
            charges: src.charges,
            field_a: src.field_a,
            field_b: src.field_b,
        };
    }

    let modifiers = ItemModListValuesUpdate {
        item_mod_list_mask: 0x01,
        values: update
            .values
            .modifiers
            .iter()
            .enumerate()
            .map(|(index, value)| ItemModValuesUpdate {
                value: *value as i32,
                item_mod_type: index as u8,
            })
            .collect(),
        values_update_mask: None,
    };

    ItemDataValuesDeltaUpdate {
        changed_object_type_mask: 1 << TYPEID_ITEM,
        item_data_mask: mask_to_u64(update.mask.blocks()),
        artifact_powers: update
            .values
            .artifact_powers
            .iter()
            .map(
                |power| wow_packet::packets::update::ArtifactPowerValuesUpdate {
                    artifact_power_id: power.artifact_power_id,
                    purchased_rank: power.purchased_rank,
                    current_rank_with_bonus: power.current_rank_with_bonus,
                },
            )
            .collect(),
        artifact_powers_update_mask: None,
        gems: update
            .values
            .gems
            .iter()
            .map(|gem| {
                let mut bonus_list_ids = [0; 16];
                for (dst, src) in bonus_list_ids.iter_mut().zip(gem.bonus_list_ids.iter()) {
                    *dst = *src;
                }
                SocketedGemValuesUpdate {
                    socketed_gem_mask: 0x07,
                    item_id: gem.item_id,
                    context: gem.context,
                    bonus_list_ids,
                }
            })
            .collect(),
        gems_update_mask: None,
        owner: update.values.owner,
        contained_in: update.values.contained_in,
        creator: update.values.creator,
        gift_creator: update.values.gift_creator,
        stack_count: update.values.stack_count,
        expiration: update.values.expiration,
        dynamic_flags: update.values.dynamic_flags,
        property_seed: update.values.property_seed,
        random_properties_id: update.values.random_properties_id,
        durability: update.values.durability,
        max_durability: update.values.max_durability,
        create_played_time: update.values.create_played_time,
        context: update.values.context,
        create_time: update.values.create_time,
        artifact_xp: update.values.artifact_xp,
        item_appearance_mod_id: update.values.item_appearance_mod_id,
        modifiers,
        dynamic_flags2: update.values.dynamic_flags2,
        item_bonus_key: ItemBonusKeyValuesUpdate {
            item_id: update.values.item_bonus_key.item_id,
            bonus_list_ids: update.values.item_bonus_key.bonus_list_ids.clone(),
        },
        debug_item_level: update.values.debug_item_level,
        spell_charges,
        enchantments,
        ..empty_item_values_update()
    }
}

pub(super) fn empty_item_values_update() -> ItemDataValuesDeltaUpdate {
    ItemDataValuesDeltaUpdate {
        changed_object_type_mask: 0,
        object_data: None,
        item_data_mask: 0,
        artifact_powers: Vec::new(),
        artifact_powers_update_mask: None,
        gems: Vec::new(),
        gems_update_mask: None,
        owner: wow_core::ObjectGuid::EMPTY,
        contained_in: wow_core::ObjectGuid::EMPTY,
        creator: wow_core::ObjectGuid::EMPTY,
        gift_creator: wow_core::ObjectGuid::EMPTY,
        stack_count: 0,
        expiration: 0,
        dynamic_flags: 0,
        property_seed: 0,
        random_properties_id: 0,
        durability: 0,
        max_durability: 0,
        create_played_time: 0,
        context: 0,
        create_time: 0,
        artifact_xp: 0,
        item_appearance_mod_id: 0,
        modifiers: ItemModListValuesUpdate {
            item_mod_list_mask: 0,
            values: Vec::new(),
            values_update_mask: None,
        },
        dynamic_flags2: 0,
        item_bonus_key: ItemBonusKeyValuesUpdate::default(),
        debug_item_level: 0,
        spell_charges: [0; 5],
        enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
    }
}

pub(super) fn container_data_update_to_packet(
    update: &ContainerDataUpdate,
) -> ContainerDataValuesUpdate {
    ContainerDataValuesUpdate {
        changed_object_type_mask: 1 << TYPEID_CONTAINER,
        object_data: None,
        item_data: None,
        container_data_mask: mask_to_u64(update.mask.blocks()),
        num_slots: update.values.num_slots,
        slots: update.values.slots,
    }
}

pub(super) fn game_object_data_update_to_packet(
    update: &GameObjectDataUpdate,
) -> GameObjectDataValuesUpdate {
    GameObjectDataValuesUpdate {
        changed_object_type_mask: 1 << TYPEID_GAME_OBJECT,
        object_data: None,
        game_object_data_mask: update.mask.blocks().first().copied().unwrap_or(0),
        state_world_effect_ids: Vec::new(),
        enable_doodad_sets: Vec::new(),
        enable_doodad_sets_update_mask: None,
        world_effects: Vec::new(),
        world_effects_update_mask: None,
        display_id: update.values.display_id,
        spell_visual_id: 0,
        state_spell_visual_id: 0,
        spawn_tracking_state_anim_id: 0,
        spawn_tracking_state_anim_kit_id: 0,
        created_by: update.values.created_by,
        guild_guid: wow_core::ObjectGuid::EMPTY,
        flags: update.values.flags,
        parent_rotation: [0.0; 4],
        faction_template: update.values.faction_template,
        level: update.values.level,
        state: update.values.state,
        type_id: update.values.type_id,
        percent_health: update.values.percent_health,
        art_kit: update.values.art_kit,
        custom_param: update.values.custom_param,
    }
}

pub(super) fn empty_game_object_values_update() -> GameObjectDataValuesUpdate {
    GameObjectDataValuesUpdate {
        changed_object_type_mask: 0,
        object_data: None,
        game_object_data_mask: 0,
        state_world_effect_ids: Vec::new(),
        enable_doodad_sets: Vec::new(),
        enable_doodad_sets_update_mask: None,
        world_effects: Vec::new(),
        world_effects_update_mask: None,
        display_id: 0,
        spell_visual_id: 0,
        state_spell_visual_id: 0,
        spawn_tracking_state_anim_id: 0,
        spawn_tracking_state_anim_kit_id: 0,
        created_by: wow_core::ObjectGuid::EMPTY,
        guild_guid: wow_core::ObjectGuid::EMPTY,
        flags: 0,
        parent_rotation: [0.0; 4],
        faction_template: 0,
        level: 0,
        state: 0,
        type_id: 0,
        percent_health: 0,
        art_kit: 0,
        custom_param: 0,
    }
}

pub(super) fn dynamic_object_data_update_to_packet(
    update: &DynamicObjectDataUpdate,
) -> DynamicObjectDataValuesUpdate {
    DynamicObjectDataValuesUpdate {
        changed_object_type_mask: 1 << TYPEID_DYNAMIC_OBJECT,
        object_data: None,
        dynamic_object_data_mask: update.mask.blocks().first().copied().unwrap_or(0),
        caster: update.values.caster,
        dynamic_object_type: update.values.dynamic_object_type,
        spell_visual_id: update.values.spell_visual_id,
        spell_id: update.values.spell_id,
        radius: update.values.radius,
        cast_time_ms: update.values.cast_time_ms,
    }
}

pub(super) fn empty_dynamic_object_values_update() -> DynamicObjectDataValuesUpdate {
    DynamicObjectDataValuesUpdate {
        changed_object_type_mask: 0,
        object_data: None,
        dynamic_object_data_mask: 0,
        caster: wow_core::ObjectGuid::EMPTY,
        dynamic_object_type: 0,
        spell_visual_id: 0,
        spell_id: 0,
        radius: 0.0,
        cast_time_ms: 0,
    }
}

pub(super) fn corpse_data_update_to_packet(update: &CorpseDataUpdate) -> CorpseDataValuesUpdate {
    CorpseDataValuesUpdate {
        changed_object_type_mask: 1 << TYPEID_CORPSE,
        object_data: None,
        corpse_data_mask: update.mask.blocks().first().copied().unwrap_or(0),
        customizations: update
            .values
            .customizations
            .iter()
            .map(|customization| ChrCustomizationChoiceValuesUpdate {
                option_id: customization.option_id,
                choice_id: customization.choice_id,
            })
            .collect(),
        // `Corpse::set_customizations` replaces the complete C++ dynamic
        // field, so a present field bit publishes every current element.
        customizations_update_mask: None,
        dynamic_flags: update.values.dynamic_flags,
        owner: update.values.owner,
        party_guid: update.values.party_guid,
        guild_guid: update.values.guild_guid,
        display_id: update.values.display_id,
        race_id: update.values.race_id,
        sex: update.values.sex,
        class: update.values.class,
        flags: update.values.flags,
        faction_template: update.values.faction_template,
        items: update.values.items,
    }
}

pub(super) fn empty_corpse_values_update() -> CorpseDataValuesUpdate {
    CorpseDataValuesUpdate {
        changed_object_type_mask: 0,
        object_data: None,
        corpse_data_mask: 0,
        customizations: Vec::new(),
        customizations_update_mask: None,
        dynamic_flags: 0,
        owner: wow_core::ObjectGuid::EMPTY,
        party_guid: wow_core::ObjectGuid::EMPTY,
        guild_guid: wow_core::ObjectGuid::EMPTY,
        display_id: 0,
        race_id: 0,
        sex: 0,
        class: 0,
        flags: 0,
        faction_template: 0,
        items: [0; 19],
    }
}

pub(super) fn area_trigger_data_update_to_packet(
    update: &AreaTriggerDataUpdate,
) -> AreaTriggerDataValuesUpdate {
    AreaTriggerDataValuesUpdate {
        changed_object_type_mask: 1 << TYPEID_AREA_TRIGGER,
        object_data: None,
        area_trigger_data_mask: update.mask.blocks().first().copied().unwrap_or(0),
        override_scale_curve: scale_curve_values_update(update.values.override_scale_curve),
        extra_scale_curve: scale_curve_values_update(update.values.extra_scale_curve),
        override_move_curve_x: scale_curve_values_update(update.values.override_move_curve_x),
        override_move_curve_y: scale_curve_values_update(update.values.override_move_curve_y),
        override_move_curve_z: scale_curve_values_update(update.values.override_move_curve_z),
        caster: update.values.caster,
        duration: update.values.duration,
        time_to_target: update.values.time_to_target,
        time_to_target_scale: update.values.time_to_target_scale,
        time_to_target_extra_scale: update.values.time_to_target_extra_scale,
        time_to_target_pos: update.values.time_to_target_pos,
        spell_id: update.values.spell_id,
        spell_for_visuals: update.values.spell_for_visuals,
        spell_visual_id: update.values.spell_visual_id,
        bounds_radius_2d: update.values.bounds_radius_2d,
        decal_properties_id: update.values.decal_properties_id,
        creating_effect_guid: update.values.creating_effect_guid,
        orbit_path_target: update.values.orbit_path_target,
        visual_anim: VisualAnimValuesUpdate {
            visual_anim_mask: 0x1F,
            field_c: update.values.visual_anim.field_c,
            animation_data_id: update.values.visual_anim.animation_data_id,
            anim_kit_id: update.values.visual_anim.anim_kit_id,
            anim_progress: update.values.visual_anim.anim_progress,
        },
    }
}
