use wow_entities::{
    ActivePlayerDataUpdate, AreaTriggerDataUpdate, AreaTriggerValuesUpdate, BagValuesUpdate,
    ContainerDataUpdate, ConversationDataUpdate, ConversationValuesUpdate, CorpseDataUpdate,
    CorpseValuesUpdate, DynamicObjectDataUpdate, DynamicObjectValuesUpdate, GameObjectDataUpdate,
    GameObjectValuesUpdate, ItemDataUpdate, ItemValuesUpdate, ObjectDataUpdate, PlayerDataUpdate,
    PlayerValuesUpdate, SceneObjectDataUpdate, SceneObjectValuesUpdate, TYPEID_ACTIVE_PLAYER,
    TYPEID_AREA_TRIGGER, TYPEID_CONTAINER, TYPEID_CONVERSATION, TYPEID_CORPSE,
    TYPEID_DYNAMIC_OBJECT, TYPEID_GAME_OBJECT, TYPEID_ITEM, TYPEID_OBJECT, TYPEID_PLAYER,
    TYPEID_SCENE_OBJECT, TYPEID_UNIT, UnitDataUpdate, UnitValuesUpdate,
};
use wow_packet::packets::update::{
    ActivePlayerDataValuesUpdate as PacketActivePlayerDataValuesUpdate,
    AreaTriggerDataValuesUpdate, ContainerDataValuesUpdate, ConversationActorValuesUpdate,
    ConversationDataValuesUpdate, ConversationLineValuesUpdate, CorpseDataValuesUpdate,
    DynamicObjectDataValuesUpdate, GameObjectDataValuesUpdate, ItemBonusKeyValuesUpdate,
    ItemDataValuesDeltaUpdate, ItemEnchantmentValuesUpdate, ItemModListValuesUpdate,
    ItemModValuesUpdate, ObjectDataValuesUpdate, PlayerDataValuesDeltaUpdate,
    ScaleCurveValuesUpdate, SceneObjectDataValuesUpdate, SocketedGemValuesUpdate,
    UnitDataValuesDeltaUpdate, UpdateObject, VisibleItemValuesUpdate, VisualAnimValuesUpdate,
};

const VISIBLE_ITEM_FULL_UPDATE_MASK: u32 = 0x0F;

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

fn object_data_update_to_packet(update: &ObjectDataUpdate) -> ObjectDataValuesUpdate {
    object_data_update_to_packet_with_type_mask(update, 1 << TYPEID_OBJECT)
}

fn object_data_update_to_packet_with_type_mask(
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

fn copy_player_data_update(
    update: &PlayerDataUpdate,
    packet_update: &mut PlayerDataValuesDeltaUpdate,
) {
    copy_mask_blocks(update.mask.blocks(), &mut packet_update.player_data_mask);
    packet_update.loot_target_guid = update.values.loot_target_guid;
    packet_update.player_flags = update.values.player_flags;
    packet_update.player_flags_ex = update.values.player_flags_ex;
    packet_update.num_bank_slots = update.values.num_bank_slots;
    packet_update.native_sex = update.values.native_sex;
    packet_update.current_spec_id = update.values.current_spec_id;
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

fn unit_data_update_to_packet(update: &UnitDataUpdate) -> UnitDataValuesDeltaUpdate {
    let mut packet_update = UnitDataValuesDeltaUpdate::default();
    copy_mask_blocks(update.mask.blocks(), &mut packet_update.unit_data_mask);
    packet_update.health = update.values.health.min(i64::MAX as u64) as i64;
    packet_update.max_health = update.values.max_health.min(i64::MAX as u64) as i64;
    packet_update.display_id = update.values.display_id;
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

fn item_data_update_to_packet(update: &ItemDataUpdate) -> ItemDataValuesDeltaUpdate {
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

fn empty_item_values_update() -> ItemDataValuesDeltaUpdate {
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

fn container_data_update_to_packet(update: &ContainerDataUpdate) -> ContainerDataValuesUpdate {
    ContainerDataValuesUpdate {
        changed_object_type_mask: 1 << TYPEID_CONTAINER,
        object_data: None,
        item_data: None,
        container_data_mask: mask_to_u64(update.mask.blocks()),
        num_slots: update.values.num_slots,
        slots: update.values.slots,
    }
}

fn game_object_data_update_to_packet(update: &GameObjectDataUpdate) -> GameObjectDataValuesUpdate {
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

fn empty_game_object_values_update() -> GameObjectDataValuesUpdate {
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

fn dynamic_object_data_update_to_packet(
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

fn empty_dynamic_object_values_update() -> DynamicObjectDataValuesUpdate {
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

fn corpse_data_update_to_packet(update: &CorpseDataUpdate) -> CorpseDataValuesUpdate {
    CorpseDataValuesUpdate {
        changed_object_type_mask: 1 << TYPEID_CORPSE,
        object_data: None,
        corpse_data_mask: update.mask.blocks().first().copied().unwrap_or(0),
        customizations: Vec::new(),
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

fn empty_corpse_values_update() -> CorpseDataValuesUpdate {
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

fn area_trigger_data_update_to_packet(
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

fn empty_area_trigger_values_update() -> AreaTriggerDataValuesUpdate {
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

fn scale_curve_values_update(values: wow_entities::ScaleCurveValues) -> ScaleCurveValuesUpdate {
    ScaleCurveValuesUpdate {
        scale_curve_mask: 0x0F,
        override_active: values.override_active,
        start_time_offset: values.start_time_offset,
        parameter_curve: values.parameter_curve,
        points: [(0.0, 0.0); 2],
    }
}

fn scene_object_data_update_to_packet(
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

fn empty_scene_object_values_update() -> SceneObjectDataValuesUpdate {
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

fn conversation_data_update_to_packet(
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

fn empty_conversation_values_update() -> ConversationDataValuesUpdate {
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

fn active_player_data_update_to_packet(
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
    packet_update.num_backpack_slots = update.values.num_backpack_slots;
    packet_update.farsight_object = update.values.farsight_object;
    packet_update
        .inv_slots
        .copy_from_slice(&update.values.inv_slots);
    packet_update.buyback_price = update.values.buyback_price;
    packet_update.buyback_timestamp = update.values.buyback_timestamp;
    packet_update.transmog = update.values.transmog.clone();
    packet_update.transmog_update_mask = update.values.transmog_update_mask.clone();
    packet_update.conditional_transmog = update.values.conditional_transmog.clone();
    packet_update.conditional_transmog_update_mask =
        update.values.conditional_transmog_update_mask.clone();
    packet_update.quest_completed = update.values.quest_completed;
    packet_update
}

fn mask_to_u64(blocks: &[u32]) -> u64 {
    blocks
        .iter()
        .take(2)
        .enumerate()
        .fold(0u64, |acc, (index, block)| {
            acc | ((*block as u64) << (index * 32))
        })
}

fn copy_mask_blocks<const N: usize>(src: &[u32], dst: &mut [u32; N]) {
    let count = src.len().min(N);
    dst[..count].copy_from_slice(&src[..count]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use wow_core::ObjectGuid;
    use wow_entities::{
        ACTIVE_PLAYER_DATA_COINAGE_BIT, ACTIVE_PLAYER_DATA_HONOR_BIT,
        ACTIVE_PLAYER_DATA_HONOR_NEXT_LEVEL_BIT, ACTIVE_PLAYER_DATA_HONOR_PARENT_BIT,
        ACTIVE_PLAYER_DATA_PARENT_BIT, ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT,
        ACTIVE_PLAYER_DATA_QUEST_COMPLETED_PARENT_BIT, ACTIVE_PLAYER_DATA_TRANSMOG_BIT,
        ACTIVE_PLAYER_DATA_WATCHED_FACTION_INDEX_BIT, AREA_TRIGGER_DATA_DURATION_BIT,
        AREA_TRIGGER_DATA_PARENT_BIT, Bag, CONTAINER_DATA_NUM_SLOTS_BIT,
        CONVERSATION_DATA_LAST_LINE_END_TIME_BIT, CONVERSATION_DATA_PARENT_BIT,
        CORPSE_DATA_DISPLAY_ID_BIT, CORPSE_DATA_PARENT_BIT, Corpse, CorpseType,
        DYNAMIC_OBJECT_DATA_PARENT_BIT, DYNAMIC_OBJECT_DATA_RADIUS_BIT, DynamicObject,
        GAME_OBJECT_DATA_CREATED_BY_BIT, GAME_OBJECT_DATA_DISPLAY_ID_BIT,
        GAME_OBJECT_DATA_PARENT_BIT, GameObject, ITEM_DATA_STACK_COUNT_BIT, Item,
        PLAYER_DATA_FLAGS_BIT, PLAYER_DATA_HONOR_LEVEL_BIT, PLAYER_DATA_PARENT_BIT, Player,
        SCENE_OBJECT_DATA_PARENT_BIT, SCENE_OBJECT_DATA_SCRIPT_PACKAGE_ID_BIT, SceneObject,
        UNIT_DATA_HEALTH_BIT, UNIT_DATA_PARENT_BIT, UNIT_DATA_STAND_STATE_BIT,
        UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT, UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT, VisibleItemValues,
    };
    use wow_packet::ServerPacket;

    fn mask_has(mask: &[u32], bit: usize) -> bool {
        (mask[bit / 32] & (1 << (bit % 32))) != 0
    }

    fn mask_has_u64(mask: u64, bit: usize) -> bool {
        (mask & (1u64 << bit)) != 0
    }

    #[test]
    fn bridges_active_player_money_update_from_entity_mask() {
        let mut player = Player::new(Some(7), false);
        player.clear_data_changes();
        player.set_money(123_456);
        player.set_watched_faction_index_like_cpp(42);

        let update = player.values_update(true);
        let packet_update = player_values_update_to_packet(&update).unwrap();
        let active = packet_update.active_player_data.unwrap();

        assert_eq!(
            packet_update.changed_object_type_mask,
            1 << TYPEID_ACTIVE_PLAYER
        );
        assert!(mask_has(
            &active.active_player_data_mask,
            ACTIVE_PLAYER_DATA_PARENT_BIT
        ));
        assert!(mask_has(
            &active.active_player_data_mask,
            ACTIVE_PLAYER_DATA_COINAGE_BIT
        ));
        assert!(mask_has(
            &active.active_player_data_mask,
            ACTIVE_PLAYER_DATA_WATCHED_FACTION_INDEX_BIT
        ));
        assert_eq!(active.coinage, 123_456);
        assert_eq!(active.watched_faction_index, 42);
    }

    #[test]
    fn bridges_player_honor_fields_with_cpp_nested_masks() {
        let mut player = Player::new(Some(7), true);
        player.clear_data_changes();

        player.set_honor_level_like_cpp(3);
        player.set_honor_like_cpp(1_234);
        player.set_honor_next_level_like_cpp(8_800);

        let update = player.values_update(true);
        let packet_update = player_values_update_to_packet(&update).unwrap();
        let active = packet_update.active_player_data.unwrap();

        assert!(mask_has(
            &packet_update.player_data_mask,
            PLAYER_DATA_PARENT_BIT
        ));
        assert!(mask_has(
            &packet_update.player_data_mask,
            PLAYER_DATA_HONOR_LEVEL_BIT
        ));
        assert_eq!(packet_update.honor_level, 3);

        assert!(mask_has(
            &active.active_player_data_mask,
            ACTIVE_PLAYER_DATA_PARENT_BIT
        ));
        assert!(mask_has(
            &active.active_player_data_mask,
            ACTIVE_PLAYER_DATA_HONOR_PARENT_BIT
        ));
        assert!(mask_has(
            &active.active_player_data_mask,
            ACTIVE_PLAYER_DATA_HONOR_BIT
        ));
        assert!(mask_has(
            &active.active_player_data_mask,
            ACTIVE_PLAYER_DATA_HONOR_NEXT_LEVEL_BIT
        ));
        assert_eq!(active.honor, 1_234);
        assert_eq!(active.honor_next_level, 8_800);
    }

    #[test]
    fn bridges_active_player_transmog_dynamic_field_like_cpp() {
        let mut player = Player::new(Some(7), true);
        player.clear_data_changes();

        let slot = player.add_transmog_block_like_cpp(0);
        assert!(player.add_transmog_flag_like_cpp(slot, 1 << 5));
        player.add_conditional_transmog_like_cpp(65);

        let update = player.values_update(true);
        let packet_update = player_values_update_to_packet(&update).unwrap();
        let active = packet_update.active_player_data.unwrap();

        assert!(mask_has(
            &active.active_player_data_mask,
            ACTIVE_PLAYER_DATA_PARENT_BIT
        ));
        assert!(mask_has(
            &active.active_player_data_mask,
            ACTIVE_PLAYER_DATA_TRANSMOG_BIT
        ));
        assert_eq!(active.transmog, vec![1 << 5]);
        assert_eq!(active.transmog_update_mask, Some(vec![1]));
        assert!(mask_has(
            &active.active_player_data_mask,
            wow_entities::ACTIVE_PLAYER_DATA_CONDITIONAL_TRANSMOG_BIT
        ));
        assert_eq!(active.conditional_transmog, vec![65]);
        assert_eq!(active.conditional_transmog_update_mask, Some(vec![1]));
    }

    #[test]
    fn bridges_quest_completed_bitmap_from_active_player_data_like_cpp() {
        let mut player = Player::new(Some(7), false);
        player
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(ObjectGuid::create_uniq(0x65));
        player.clear_data_changes();

        assert!(player.set_quest_completed_bit_like_cpp(65, true));
        let update = player.values_update(true);
        let packet_update = player_values_update_to_packet(&update).unwrap();
        let active = packet_update.active_player_data.as_ref().unwrap();

        assert!(mask_has(
            &active.active_player_data_mask,
            ACTIVE_PLAYER_DATA_QUEST_COMPLETED_PARENT_BIT
        ));
        assert!(mask_has(
            &active.active_player_data_mask,
            ACTIVE_PLAYER_DATA_QUEST_COMPLETED_FIRST_BIT + 1
        ));
        assert_eq!(active.quest_completed[1], 1);

        let update_object =
            player_values_update_to_update_object(player.guid(), 571, &update).unwrap();
        assert!(!update_object.to_bytes().is_empty());
    }

    #[test]
    fn bridges_player_and_active_player_values_without_unit_bits() {
        let mut player = Player::new(Some(7), false);
        player.clear_data_changes();
        player.set_player_flag(0x20);
        player.set_visible_item_slot(
            0,
            Some(VisibleItemValues {
                item_id: 25,
                item_appearance_mod_id: 3,
                item_visual: 4,
            }),
        );
        player.set_money(42);

        let update = player.values_update(true);
        let packet_update = player_values_update_to_packet(&update).unwrap();

        assert_eq!(
            packet_update.changed_object_type_mask,
            (1 << TYPEID_PLAYER) | (1 << TYPEID_ACTIVE_PLAYER)
        );
        assert!(mask_has(
            &packet_update.player_data_mask,
            PLAYER_DATA_PARENT_BIT
        ));
        assert!(mask_has(
            &packet_update.player_data_mask,
            PLAYER_DATA_FLAGS_BIT
        ));
        assert_eq!(packet_update.player_flags, 0x20);
        assert_eq!(packet_update.visible_items[0].visible_item_mask, 0x0F);
        assert_eq!(packet_update.visible_items[0].item_id, 25);
        assert_eq!(
            packet_update.active_player_data.as_ref().unwrap().coinage,
            42
        );
    }

    #[test]
    fn builds_update_object_from_entity_player_values_update() {
        let mut player = Player::new(Some(7), false);
        player
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(ObjectGuid::create_uniq(0x42));
        player.clear_data_changes();
        player.set_money(1234);

        let update = player.values_update(true);
        let packet = player_values_update_to_update_object(player.guid(), 571, &update).unwrap();
        let bytes = packet.to_bytes();

        assert!(!bytes.is_empty());
        assert!(
            bytes
                .windows(1234u64.to_le_bytes().len())
                .any(|window| window == 1234u64.to_le_bytes())
        );
    }

    #[test]
    fn bridges_player_object_data_like_cpp_values_prefix() {
        let mut player = Player::new(Some(7), false);
        player.clear_data_changes();
        player.unit_mut().world_mut().object_mut().set_entry(42);

        let update = player.values_update(true);
        let packet_update = player_values_update_to_packet(&update).unwrap();
        let object_data = packet_update.object_data.unwrap();

        assert_eq!(packet_update.changed_object_type_mask, 1 << TYPEID_OBJECT);
        assert_eq!(object_data.changed_object_type_mask, 1 << TYPEID_OBJECT);
        assert_eq!(object_data.entry_id, 42);
    }

    #[test]
    fn bridges_unit_object_and_unit_values() {
        let mut unit = wow_entities::Unit::new(true);
        unit.world_mut().object_mut().set_entry(99);
        unit.set_max_health(123);
        unit.set_health(123);
        unit.set_stand_state_like_cpp(wow_constants::UnitStandStateType::Sit);
        unit.replace_all_vis_flags_like_cpp(0x12);
        unit.set_anim_tier_like_cpp(2);
        unit.set_sheath_like_cpp(wow_constants::SheathState::Ranged);
        unit.set_pvp_flag_like_cpp(wow_constants::UnitPvpFlags::PVP);
        unit.replace_all_pet_flags_like_cpp(0x03);
        unit.set_shapeshift_form_like_cpp(wow_constants::ShapeShiftForm::CatForm);
        unit.set_npc_flags_like_cpp(0x40);
        unit.set_npc_flags2_like_cpp(0x1);

        let update = unit.values_update();
        let packet_update = unit_values_update_to_packet(&update).unwrap();

        assert_eq!(
            packet_update.changed_object_type_mask,
            (1 << TYPEID_OBJECT) | (1 << TYPEID_UNIT)
        );
        assert_eq!(packet_update.object_data.unwrap().entry_id, 99);
        assert!(mask_has(
            &packet_update.unit_data_mask,
            UNIT_DATA_PARENT_BIT
        ));
        assert!(mask_has(
            &packet_update.unit_data_mask,
            UNIT_DATA_HEALTH_BIT
        ));
        assert!(mask_has(
            &packet_update.unit_data_mask,
            UNIT_DATA_STAND_STATE_BIT
        ));
        assert!(mask_has(
            &packet_update.unit_data_mask,
            wow_entities::UNIT_DATA_VIS_FLAGS_BIT
        ));
        assert!(mask_has(
            &packet_update.unit_data_mask,
            wow_entities::UNIT_DATA_ANIM_TIER_BIT
        ));
        assert!(mask_has(
            &packet_update.unit_data_mask,
            wow_entities::UNIT_DATA_SHEATHE_STATE_BIT
        ));
        assert!(mask_has(
            &packet_update.unit_data_mask,
            wow_entities::UNIT_DATA_PVP_FLAGS_BIT
        ));
        assert!(mask_has(
            &packet_update.unit_data_mask,
            wow_entities::UNIT_DATA_PET_FLAGS_BIT
        ));
        assert!(mask_has(
            &packet_update.unit_data_mask,
            wow_entities::UNIT_DATA_SHAPESHIFT_FORM_BIT
        ));
        assert!(mask_has(
            &packet_update.unit_data_mask,
            wow_entities::UNIT_DATA_NPC_FLAGS_PARENT_BIT
        ));
        assert!(mask_has(
            &packet_update.unit_data_mask,
            wow_entities::UNIT_DATA_NPC_FLAGS_FIRST_BIT
        ));
        assert!(mask_has(
            &packet_update.unit_data_mask,
            wow_entities::UNIT_DATA_NPC_FLAGS_FIRST_BIT + 1
        ));
        assert_eq!(packet_update.health, 123);
        assert_eq!(packet_update.npc_flags, [0x40, 0x1]);
        assert_eq!(
            packet_update.stand_state,
            wow_constants::UnitStandStateType::Sit as u8
        );
        assert_eq!(packet_update.vis_flags, 0x12);
        assert_eq!(packet_update.anim_tier, 2);
        assert_eq!(
            packet_update.sheathe_state,
            wow_constants::SheathState::Ranged as u8
        );
        assert_eq!(
            packet_update.pvp_flags,
            wow_constants::UnitPvpFlags::PVP.bits()
        );
        assert_eq!(packet_update.pet_flags, 0x03);
        assert_eq!(
            packet_update.shapeshift_form,
            wow_constants::ShapeShiftForm::CatForm as u8
        );
    }

    #[test]
    fn bridges_item_object_only_and_item_data() {
        let mut item = Item::new(0);
        item.object_mut().set_entry(6948);

        let object_update = item_values_update_to_packet(&item.values_update()).unwrap();
        assert_eq!(object_update.changed_object_type_mask, 1 << TYPEID_OBJECT);
        assert_eq!(object_update.object_data.unwrap().entry_id, 6948);
        assert_eq!(object_update.item_data_mask, 0);

        item.clear_item_data_changes();
        item.object_mut().clear_update_mask(false);
        item.set_count(5);

        let item_update = item_values_update_to_packet(&item.values_update()).unwrap();
        assert_eq!(item_update.changed_object_type_mask, 1 << TYPEID_ITEM);
        assert!(mask_has_u64(
            item_update.item_data_mask,
            ITEM_DATA_STACK_COUNT_BIT
        ));
        assert_eq!(item_update.stack_count, 5);
    }

    #[test]
    fn bridges_bag_container_values_with_item_base() {
        let mut bag = Bag::new(0);
        bag.item_mut().object_mut().set_entry(4242);
        bag.set_bag_size(16);

        let packet_update = bag_values_update_to_packet(&bag.values_update()).unwrap();

        assert_eq!(
            packet_update.changed_object_type_mask,
            (1 << TYPEID_OBJECT) | (1 << TYPEID_CONTAINER)
        );
        assert_eq!(packet_update.object_data.unwrap().entry_id, 4242);
        assert!(mask_has_u64(
            packet_update.container_data_mask,
            CONTAINER_DATA_NUM_SLOTS_BIT
        ));
        assert_eq!(packet_update.num_slots, 16);
    }

    #[test]
    fn bridges_game_object_values_with_object_prefix() {
        let mut go = GameObject::new();
        go.world_mut().object_mut().set_entry(1001);
        go.set_display_id(22);
        let owner = wow_core::ObjectGuid::create_player(1, 42);
        go.set_created_by(owner);

        let packet_update = game_object_values_update_to_packet(&go.values_update()).unwrap();

        assert_eq!(
            packet_update.changed_object_type_mask,
            (1 << TYPEID_OBJECT) | (1 << TYPEID_GAME_OBJECT)
        );
        assert_eq!(packet_update.object_data.unwrap().entry_id, 1001);
        assert!(mask_has_u64(
            packet_update.game_object_data_mask as u64,
            GAME_OBJECT_DATA_PARENT_BIT
        ));
        assert!(mask_has_u64(
            packet_update.game_object_data_mask as u64,
            GAME_OBJECT_DATA_DISPLAY_ID_BIT
        ));
        assert!(mask_has_u64(
            packet_update.game_object_data_mask as u64,
            GAME_OBJECT_DATA_CREATED_BY_BIT
        ));
        assert_eq!(packet_update.display_id, 22);
        assert_eq!(packet_update.created_by, owner);
    }

    #[test]
    fn bridges_dynamic_object_values_with_cpp_field_order_data() {
        let mut dyn_object = DynamicObject::new(true);
        dyn_object.set_radius(7.5);

        let packet_update =
            dynamic_object_values_update_to_packet(&dyn_object.values_update()).unwrap();

        assert_eq!(
            packet_update.changed_object_type_mask,
            1 << TYPEID_DYNAMIC_OBJECT
        );
        assert!(mask_has_u64(
            packet_update.dynamic_object_data_mask as u64,
            DYNAMIC_OBJECT_DATA_PARENT_BIT
        ));
        assert!(mask_has_u64(
            packet_update.dynamic_object_data_mask as u64,
            DYNAMIC_OBJECT_DATA_RADIUS_BIT
        ));
        assert_eq!(packet_update.radius, 7.5);
    }

    #[test]
    fn bridges_corpse_values_and_items_mask() {
        let mut corpse = Corpse::new(CorpseType::Bones);
        corpse.set_display_id(123);

        let packet_update = corpse_values_update_to_packet(&corpse.values_update()).unwrap();

        assert_eq!(packet_update.changed_object_type_mask, 1 << TYPEID_CORPSE);
        assert!(mask_has_u64(
            packet_update.corpse_data_mask as u64,
            CORPSE_DATA_PARENT_BIT
        ));
        assert!(mask_has_u64(
            packet_update.corpse_data_mask as u64,
            CORPSE_DATA_DISPLAY_ID_BIT
        ));
        assert_eq!(packet_update.display_id, 123);
    }

    #[test]
    fn bridges_area_trigger_values_with_nested_full_masks() {
        let mut area_trigger = wow_entities::AreaTrigger::new();
        area_trigger.set_duration(4000);
        area_trigger.set_override_scale_constant(2.0);

        let packet_update =
            area_trigger_values_update_to_packet(&area_trigger.values_update()).unwrap();

        assert_eq!(
            packet_update.changed_object_type_mask,
            1 << TYPEID_AREA_TRIGGER
        );
        assert!(mask_has_u64(
            packet_update.area_trigger_data_mask as u64,
            AREA_TRIGGER_DATA_PARENT_BIT
        ));
        assert!(mask_has_u64(
            packet_update.area_trigger_data_mask as u64,
            AREA_TRIGGER_DATA_DURATION_BIT
        ));
        assert_eq!(packet_update.duration, 4000);
        assert_eq!(packet_update.override_scale_curve.scale_curve_mask, 0x0F);
    }

    #[test]
    fn bridges_scene_object_values() {
        let mut scene = SceneObject::new();
        scene.set_script_package_id(77);

        let packet_update = scene_object_values_update_to_packet(&scene.values_update()).unwrap();

        assert_eq!(
            packet_update.changed_object_type_mask,
            1 << TYPEID_SCENE_OBJECT
        );
        assert!(mask_has_u64(
            packet_update.scene_object_data_mask as u64,
            SCENE_OBJECT_DATA_PARENT_BIT
        ));
        assert!(mask_has_u64(
            packet_update.scene_object_data_mask as u64,
            SCENE_OBJECT_DATA_SCRIPT_PACKAGE_ID_BIT
        ));
        assert_eq!(packet_update.script_package_id, 77);
    }

    #[test]
    fn bridges_conversation_values_with_full_actor_mask() {
        let mut conversation = wow_entities::Conversation::new();
        conversation.set_last_line_end_time(1234);
        conversation.add_actor_world_object(9, 0, ObjectGuid::new(1, 55));

        let packet_update =
            conversation_values_update_to_packet(&conversation.values_update()).unwrap();

        assert_eq!(
            packet_update.changed_object_type_mask,
            1 << TYPEID_CONVERSATION
        );
        assert!(mask_has_u64(
            packet_update.conversation_data_mask as u64,
            CONVERSATION_DATA_PARENT_BIT
        ));
        assert!(mask_has_u64(
            packet_update.conversation_data_mask as u64,
            CONVERSATION_DATA_LAST_LINE_END_TIME_BIT
        ));
        assert_eq!(packet_update.last_line_end_time, 1234);
        assert_eq!(packet_update.actors.len(), 1);
        assert!(packet_update.actor_update_mask.is_none());
    }

    #[test]
    fn bridges_unit_virtual_items_from_player_values_update() {
        let mut player = Player::new(Some(7), false);
        player.clear_data_changes();
        player.unit_mut().set_virtual_item(
            2,
            Some(VisibleItemValues {
                item_id: 25,
                item_appearance_mod_id: 3,
                item_visual: 4,
            }),
        );

        let update = player.values_update(true);
        let packet_update = player_values_update_to_packet(&update).unwrap();
        let unit = packet_update.unit_data.unwrap();

        assert_eq!(packet_update.changed_object_type_mask, 1 << TYPEID_UNIT);
        assert!(mask_has(
            &unit.unit_data_mask,
            UNIT_DATA_VIRTUAL_ITEMS_PARENT_BIT
        ));
        assert!(mask_has(
            &unit.unit_data_mask,
            UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT + 2
        ));
        assert_eq!(unit.virtual_items[2].visible_item_mask, 0x0F);
        assert_eq!(unit.virtual_items[2].item_id, 25);
        assert_eq!(unit.virtual_items[2].appearance_mod_id, 3);
        assert_eq!(unit.virtual_items[2].item_visual, 4);
    }

    #[test]
    fn bridges_forced_default_value_deltas() {
        let mut player = Player::new(Some(7), false);
        player.clear_data_changes();
        player.mark_inv_slot_changed(0);
        player.mark_visible_item_slot_changed(0);
        player.unit_mut().mark_virtual_item_changed(0);

        let update = player.values_update(true);
        let packet_update = player_values_update_to_packet(&update).unwrap();
        let active = packet_update.active_player_data.unwrap();
        let unit = packet_update.unit_data.unwrap();

        assert!(mask_has(
            &active.active_player_data_mask,
            wow_entities::ACTIVE_PLAYER_DATA_INV_SLOTS_FIRST_BIT
        ));
        assert!(mask_has(
            &packet_update.player_data_mask,
            wow_entities::PLAYER_DATA_VISIBLE_ITEMS_FIRST_BIT
        ));
        assert!(mask_has(
            &unit.unit_data_mask,
            wow_entities::UNIT_DATA_VIRTUAL_ITEMS_FIRST_BIT
        ));
        assert_eq!(active.inv_slots[0], ObjectGuid::EMPTY);
        assert_eq!(packet_update.visible_items[0].item_id, 0);
        assert_eq!(unit.virtual_items[0].item_id, 0);
    }
}
