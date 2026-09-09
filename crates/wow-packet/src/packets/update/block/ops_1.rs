//! Update-object block builder operations, part 1 of 2.
//!
//! The inherent `UpdateObject` impl is divided by responsibility under
//! #650; every method keeps its original body.

#[allow(unused_imports)]
use super::super::*;
use super::*;

impl UpdateObject {
    /// Human-readable block summary for live C++/Rust login comparisons.
    ///
    /// This is intentionally metadata-only: it uses the same private block writers
    /// as the packet serializer to report per-block byte sizes without dumping
    /// account/player payload bytes into normal logs.
    pub fn debug_create_summary_like_cpp(&self) -> Vec<String> {
        let mut lines = Vec::with_capacity(self.blocks.len() + 1);
        lines.push(format!(
            "update map={} num_updates={} blocks={} destroy={} out_of_range={} packet_bytes={}",
            self.map_id,
            self.num_updates,
            self.blocks.len(),
            self.destroy_guids.len(),
            self.out_of_range_guids.len(),
            self.to_bytes().len()
        ));

        for (index, block) in self.blocks.iter().enumerate() {
            match block {
                UpdateBlock::CreateObject {
                    update_type,
                    guid,
                    type_id,
                    movement,
                    create_data,
                    is_self,
                } => {
                    let mut block_buf = WorldPacket::new_empty();
                    write_create_block(
                        &mut block_buf,
                        *update_type,
                        guid,
                        *type_id,
                        movement.as_ref(),
                        create_data,
                        *is_self,
                    );
                    let block_bytes = block_buf.into_data().len();
                    let values_bytes =
                        debug_player_create_values_len_like_cpp(create_data, *is_self);
                    let movement_bytes = block_bytes.saturating_sub(
                        debug_create_header_len_like_cpp(*update_type, guid, *type_id)
                            + values_bytes,
                    );
                    let inv_slots = create_data
                        .inv_slots
                        .iter()
                        .filter(|guid| !guid.is_empty())
                        .count();
                    let visible_items = create_data
                        .visible_items
                        .iter()
                        .filter(|(item_id, _, _)| *item_id != 0)
                        .count();
                    let quest_slots = create_data
                        .quest_log
                        .iter()
                        .enumerate()
                        .filter_map(|(slot, (quest_id, state_flags, _, objective_progress))| {
                            (*quest_id != 0).then(|| {
                                let progress = objective_progress
                                    .iter()
                                    .copied()
                                    .filter(|count| *count != 0)
                                    .map(|count| count.to_string())
                                    .collect::<Vec<_>>()
                                    .join("/");
                                if progress.is_empty() {
                                    format!("{slot}:{quest_id}:0x{state_flags:X}")
                                } else {
                                    format!("{slot}:{quest_id}:0x{state_flags:X}:{progress}")
                                }
                            })
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    lines.push(format!(
                        "#{index:03} player guid={guid:?} update_type={} type_id={} self={} bytes={} movementBytes={} valuesBytes={} level={} display={} native_display={} health={}/{} inv_slots={} visible_items={} skills={} quests={} quest_slots=[{}] toys={} heirlooms={} coinage={}",
                        *update_type as u8,
                        *type_id as u8,
                        is_self,
                        block_bytes,
                        movement_bytes,
                        values_bytes,
                        create_data.level,
                        create_data.display_id,
                        create_data.native_display_id,
                        create_data.health,
                        create_data.max_health,
                        inv_slots,
                        visible_items,
                        create_data.skill_info.len(),
                        create_data.quest_log.len(),
                        quest_slots,
                        create_data.toys.len(),
                        create_data.heirlooms.len(),
                        create_data.coinage
                    ));
                }
                UpdateBlock::CreateCreature {
                    guid,
                    movement,
                    create_data,
                } => {
                    let mut block_buf = WorldPacket::new_empty();
                    write_creature_create_block(&mut block_buf, guid, movement, create_data);
                    let block_bytes = block_buf.into_data().len();
                    let values_bytes = debug_creature_create_values_len_like_cpp(create_data);
                    let movement_bytes = block_bytes.saturating_sub(
                        debug_create_header_len_like_cpp(
                            UpdateType::CreateObject,
                            guid,
                            TypeId::Unit,
                        ) + values_bytes,
                    );
                    let has_anim_kit = create_data.ai_anim_kit_id != 0
                        || create_data.movement_anim_kit_id != 0
                        || create_data.melee_anim_kit_id != 0;
                    let active_spline = movement
                        .create_object_spline
                        .as_ref()
                        .filter(|spline| create_object_spline_enabled_like_cpp(spline));
                    let spline_points = active_spline
                        .map(|spline| spline.create_object_path_points_like_cpp().len())
                        .unwrap_or(0);
                    lines.push(format!(
                        "#{index:03} creature guid={guid:?} entry={} updateType={} typeId={} display={} native_display={} level={} bytes={} movementBytes={} valuesBytes={} flags(noBirth=0 portals=0 hover={} move=1 transport=0 stationary=0 combatVictim=0 serverTime=0 vehicle={} animKit={} rotation=0 areaTrigger=0 gameObject=0 smooth=0 thisIsYou=0 scene=0 activePlayer=0 conversation=0) hasSpline={} splinePoints={} pos=({:.3},{:.3},{:.3},{:.3}) hp={}/{} npc_flags=0x{:X} unit_flags=0x{:X}/0x{:X}/0x{:X} move_flags=0x{:X}/0x{:X}/0x{:X} speeds=({:.5},{:.5}) power0={}/{} vehicle_id={} virtual_items={:?} hover={} hover_h={:.3} animkits=({},{},{})",
                        create_data.entry,
                        UpdateType::CreateObject as u8,
                        TypeId::Unit as u8,
                        create_data.display_id,
                        create_data.native_display_id,
                        create_data.level,
                        block_bytes,
                        movement_bytes,
                        values_bytes,
                        create_data.play_hover_anim as u8,
                        (create_data.vehicle_id != 0) as u8,
                        has_anim_kit as u8,
                        active_spline.is_some() as u8,
                        spline_points,
                        movement.position.x,
                        movement.position.y,
                        movement.position.z,
                        movement.position.orientation,
                        create_data.health,
                        create_data.max_health,
                        create_data.npc_flags,
                        create_data.unit_flags,
                        create_data.unit_flags2,
                        create_data.unit_flags3,
                        create_data.movement_flags,
                        movement.movement_flags2,
                        movement.movement_flags3,
                        movement.walk_speed,
                        movement.run_speed,
                        create_data.power[0],
                        create_data.max_power[0],
                        create_data.vehicle_id,
                        create_data.virtual_items,
                        create_data.play_hover_anim,
                        create_data.hover_height,
                        create_data.ai_anim_kit_id,
                        create_data.movement_anim_kit_id,
                        create_data.melee_anim_kit_id
                    ));
                }
                UpdateBlock::CreateItem {
                    update_type,
                    guid,
                    create_data,
                } => {
                    let mut block_buf = WorldPacket::new_empty();
                    write_item_create_block(&mut block_buf, *update_type, guid, create_data);
                    let block_bytes = block_buf.into_data().len();
                    let values_bytes = debug_item_create_values_len_like_cpp(create_data);
                    let movement_bytes = block_bytes.saturating_sub(
                        debug_create_header_len_like_cpp(
                            *update_type,
                            guid,
                            if create_data.container_slots > 0 {
                                TypeId::Container
                            } else {
                                TypeId::Item
                            },
                        ) + values_bytes,
                    );
                    let filled_container_slots = create_data
                        .container_item_guids
                        .iter()
                        .filter(|guid| !guid.is_empty())
                        .count();
                    lines.push(format!(
                        "#{index:03} item guid={guid:?} entry={} updateType={} type_id={} bytes={} movementBytes={} valuesBytes={} stack={} flags=0x{:X} durability={}/{} context={} contained_in={:?} container_slots={} filled_container_slots={} random=({},{})",
                        create_data.entry_id,
                        *update_type as u8,
                        if create_data.container_slots > 0 {
                            TypeId::Container as u8
                        } else {
                            TypeId::Item as u8
                        },
                        block_bytes,
                        movement_bytes,
                        values_bytes,
                        create_data.stack_count,
                        create_data.dynamic_flags,
                        create_data.durability,
                        create_data.max_durability,
                        create_data.context,
                        create_data.contained_in,
                        create_data.container_slots,
                        filled_container_slots,
                        create_data.random_properties_seed,
                        create_data.random_properties_id
                    ));
                }
                UpdateBlock::CreateGameObject {
                    update_type,
                    guid,
                    create_data,
                } => {
                    let mut block_buf = WorldPacket::new_empty();
                    write_gameobject_create_block(&mut block_buf, *update_type, guid, create_data);
                    let block_bytes = block_buf.into_data().len();
                    let values_bytes = debug_gameobject_create_values_len_like_cpp(create_data);
                    let movement_bytes = block_bytes.saturating_sub(
                        debug_create_header_len_like_cpp(*update_type, guid, TypeId::GameObject)
                            + values_bytes,
                    );
                    let has_gameobject_payload = create_data.world_effect_id != 0;
                    lines.push(format!(
                        "#{index:03} gameobject guid={guid:?} update_type={} entry={} display={} type={} bytes={} movementBytes={} valuesBytes={} flags(noBirth=0 portals=0 hover=0 move=0 transport=0 stationary=1 combatVictim=0 serverTime=0 vehicle=0 animKit=0 rotation=1 areaTrigger=0 gameObject={} smooth=0 thisIsYou=0 scene=0 activePlayer=0 conversation=0) worldEffectID={} pos=({:.3},{:.3},{:.3},{:.3})",
                        *update_type as u8,
                        create_data.entry,
                        create_data.display_id,
                        create_data.go_type,
                        block_bytes,
                        movement_bytes,
                        values_bytes,
                        has_gameobject_payload as u8,
                        create_data.world_effect_id,
                        create_data.position.x,
                        create_data.position.y,
                        create_data.position.z,
                        create_data.position.orientation
                    ));
                }
                UpdateBlock::CreateTransport {
                    guid,
                    create_data,
                    server_time_ms,
                } => {
                    let mut block_buf = WorldPacket::new_empty();
                    write_transport_create_block(
                        &mut block_buf,
                        UpdateType::CreateObject,
                        guid,
                        create_data,
                        *server_time_ms,
                    );
                    let block_bytes = block_buf.into_data().len();
                    let values_bytes = debug_gameobject_create_values_len_like_cpp(create_data);
                    let movement_bytes = block_bytes.saturating_sub(
                        debug_create_header_len_like_cpp(
                            UpdateType::CreateObject,
                            guid,
                            TypeId::GameObject,
                        ) + values_bytes,
                    );
                    lines.push(format!(
                        "#{index:03} transport guid={guid:?} entry={} display={} bytes={} movementBytes={} valuesBytes={} serverTime={}",
                        create_data.entry,
                        create_data.display_id,
                        block_bytes,
                        movement_bytes,
                        values_bytes,
                        server_time_ms
                    ));
                }
                UpdateBlock::CreateDynamicObject { guid, create_data } => {
                    lines.push(format!(
                        "#{index:03} dynamic_object guid={guid:?} spell={} visual={} radius={}",
                        create_data.spell_id, create_data.spell_visual_id, create_data.radius
                    ));
                }
                UpdateBlock::CreateAreaTrigger { guid, create_data } => {
                    lines.push(format!(
                        "#{index:03} area_trigger guid={guid:?} entry={} shape={} flags=0x{:X} bytes={} pos=({:.3},{:.3},{:.3},{:.3}) spell={} visual={} radius={:.3}",
                        create_data.entry_id,
                        create_data.shape.shape_type,
                        create_data.create_properties_flags,
                        debug_area_trigger_create_block_len_like_cpp(guid, create_data),
                        create_data.position.x,
                        create_data.position.y,
                        create_data.position.z,
                        create_data.position.orientation,
                        create_data.spell_id,
                        create_data.spell_visual_id,
                        create_data.bounds_radius_2d
                    ));
                }
                UpdateBlock::CreateCorpse { guid, create_data } => {
                    lines.push(format!(
                        "#{index:03} corpse guid={guid:?} entry={} display={} pos=({:.3},{:.3},{:.3},{:.3})",
                        create_data.entry_id,
                        create_data.display_id,
                        create_data.position.x,
                        create_data.position.y,
                        create_data.position.z,
                        create_data.position.orientation
                    ));
                }
                UpdateBlock::CreateSceneObject { guid, create_data } => {
                    lines.push(format!(
                        "#{index:03} scene_object guid={guid:?} entry={} script_package={} scene_type={} pos=({:.3},{:.3},{:.3},{:.3})",
                        create_data.entry_id,
                        create_data.script_package_id,
                        create_data.scene_type,
                        create_data.position.x,
                        create_data.position.y,
                        create_data.position.z,
                        create_data.position.orientation
                    ));
                }
                UpdateBlock::CreateConversation { guid, create_data } => {
                    lines.push(format!(
                        "#{index:03} conversation guid={guid:?} entry={} lines={} actors={} texture_kit={} pos=({:.3},{:.3},{:.3},{:.3})",
                        create_data.entry_id,
                        create_data.lines.len(),
                        create_data.actors.len(),
                        create_data.texture_kit_id,
                        create_data.position.x,
                        create_data.position.y,
                        create_data.position.z,
                        create_data.position.orientation
                    ));
                }
                UpdateBlock::ItemValuesUpdate {
                    guid,
                    stack_count,
                    dynamic_flags,
                } => {
                    lines.push(format!(
                        "#{index:03} item_values guid={guid:?} stack_count={stack_count} dynamic_flags={dynamic_flags:?}"
                    ));
                }
                UpdateBlock::PlayerValuesUpdate {
                    guid,
                    inv_slot_changes,
                    buyback_changes,
                    visible_item_changes,
                    virtual_item_changes,
                    stat_changes,
                    coinage_change,
                } => {
                    lines.push(format!(
                        "#{index:03} player_values guid={guid:?} inv_changes={} buyback_changes={} visible_changes={} virtual_changes={} stat_changes={} coinage_change={}",
                        inv_slot_changes.len(),
                        buyback_changes.len(),
                        visible_item_changes.len(),
                        virtual_item_changes.len(),
                        stat_changes.is_some(),
                        coinage_change.is_some()
                    ));
                }
                UpdateBlock::CreatureHealthUpdate {
                    guid,
                    health,
                    max_health,
                } => {
                    lines.push(format!(
                        "#{index:03} creature_health guid={guid:?} hp={health}/{max_health}"
                    ));
                }
                UpdateBlock::ObjectValuesUpdate { guid, .. } => {
                    lines.push(format!("#{index:03} object_values guid={guid:?}"));
                }
                UpdateBlock::DynamicObjectValuesUpdate { guid, .. } => {
                    lines.push(format!("#{index:03} dynamic_object_values guid={guid:?}"));
                }
                UpdateBlock::SceneObjectValuesUpdate { guid, .. } => {
                    lines.push(format!("#{index:03} scene_object_values guid={guid:?}"));
                }
                UpdateBlock::ConversationValuesUpdate { guid, .. } => {
                    lines.push(format!("#{index:03} conversation_values guid={guid:?}"));
                }
                UpdateBlock::GameObjectValuesUpdate { guid, .. } => {
                    lines.push(format!("#{index:03} gameobject_values guid={guid:?}"));
                }
                UpdateBlock::CorpseValuesUpdate { guid, .. } => {
                    lines.push(format!("#{index:03} corpse_values guid={guid:?}"));
                }
                UpdateBlock::AreaTriggerValuesUpdate { guid, .. } => {
                    lines.push(format!("#{index:03} areatrigger_values guid={guid:?}"));
                }
                UpdateBlock::FullItemValuesUpdate { guid, .. } => {
                    lines.push(format!("#{index:03} full_item_values guid={guid:?}"));
                }
                UpdateBlock::UnitValuesUpdate { guid, .. } => {
                    lines.push(format!("#{index:03} unit_values guid={guid:?}"));
                }
                UpdateBlock::FullPlayerValuesUpdate { guid, .. } => {
                    lines.push(format!("#{index:03} full_player_values guid={guid:?}"));
                }
                UpdateBlock::FullActivePlayerValuesUpdate { guid, .. } => {
                    lines.push(format!(
                        "#{index:03} full_active_player_values guid={guid:?}"
                    ));
                }
                UpdateBlock::ContainerValuesUpdate { guid, .. } => {
                    lines.push(format!("#{index:03} container_values guid={guid:?}"));
                }
                UpdateBlock::DestroyOutOfRange { guid } => {
                    lines.push(format!("#{index:03} destroy_out_of_range guid={guid:?}"));
                }
            }
        }

        lines
    }
    /// Create a creature spawn block for an object already present in the map.
    ///
    /// Speed rates from `creature_template` are multiplied by base speeds:
    /// walk = rate × 2.5, run = rate × 7.0.
    pub fn create_creature_block(
        create_data: CreatureCreateData,
        position: &Position,
    ) -> UpdateBlock {
        Self::create_creature_block_with_spline(create_data, position, None)
    }
    /// Create a creature spawn block, preserving an active C++ `Unit::movespline`
    /// when the creature is already moving as it enters the viewer's client set.
    pub fn create_creature_block_with_spline(
        create_data: CreatureCreateData,
        position: &Position,
        active_spline: Option<MoveSpline>,
    ) -> UpdateBlock {
        let walk_speed = create_data.speed_walk_rate * 2.5;
        let run_speed = create_data.speed_run_rate * 7.0;
        let movement = MovementBlock {
            position: *position,
            movement_flags: create_data.movement_flags,
            create_object_spline: active_spline,
            walk_speed,
            run_speed,
            ..Default::default()
        };
        UpdateBlock::CreateCreature {
            guid: create_data.guid,
            movement,
            create_data,
        }
    }
    /// Create a gameobject block for an object already present in the map.
    ///
    /// C++ `Object::BuildCreateUpdateBlockForPlayer` writes `CreateObject`
    /// for normal visibility and only switches to `CreateObject2` while
    /// `Map::AddToMap` has marked the object as new.
    pub fn create_gameobject_block(create_data: GameObjectCreateData) -> UpdateBlock {
        UpdateBlock::CreateGameObject {
            update_type: UpdateType::CreateObject,
            guid: create_data.guid,
            create_data,
        }
    }
    /// Create a gameobject block for the C++ `m_isNewObject` path.
    pub fn create_new_gameobject_block(create_data: GameObjectCreateData) -> UpdateBlock {
        UpdateBlock::CreateGameObject {
            update_type: UpdateType::CreateObject2,
            guid: create_data.guid,
            create_data,
        }
    }
    /// Create a map transport block for C++ `Map::SendInitTransports`.
    ///
    /// `Transport` derives from `GameObject` but sets only ServerTime,
    /// Stationary and Rotation create flags (`Transport.cpp` constructor).
    /// It does not set the generic GameObject movement extension flag.
    pub fn create_transport_block(
        create_data: GameObjectCreateData,
        server_time_ms: u32,
    ) -> UpdateBlock {
        UpdateBlock::CreateTransport {
            guid: create_data.guid,
            create_data,
            server_time_ms,
        }
    }
    /// Create a dynamic object spawn block.
    pub fn create_dynamic_object_block(create_data: DynamicObjectCreateData) -> UpdateBlock {
        UpdateBlock::CreateDynamicObject {
            guid: create_data.guid,
            create_data,
        }
    }
    pub fn create_area_trigger_block(create_data: AreaTriggerCreateData) -> UpdateBlock {
        UpdateBlock::CreateAreaTrigger {
            guid: create_data.guid,
            create_data,
        }
    }
    pub fn create_corpse_block(create_data: CorpseCreateData) -> UpdateBlock {
        UpdateBlock::CreateCorpse {
            guid: create_data.guid,
            create_data,
        }
    }
    pub fn create_scene_object_block(create_data: SceneObjectCreateData) -> UpdateBlock {
        UpdateBlock::CreateSceneObject {
            guid: create_data.guid,
            create_data,
        }
    }
    pub fn create_conversation_block(create_data: ConversationCreateData) -> UpdateBlock {
        UpdateBlock::CreateConversation {
            guid: create_data.guid,
            create_data,
        }
    }
    /// Create a batched UpdateObject with mixed world-object create blocks.
    pub fn create_world_objects(blocks: Vec<UpdateBlock>, map_id: u16) -> Self {
        Self {
            map_id,
            num_updates: blocks.len() as u32,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks,
        }
    }
    /// Create a batched UpdateObject with multiple creature blocks.
    pub fn create_creatures(blocks: Vec<UpdateBlock>, map_id: u16) -> Self {
        Self {
            map_id,
            num_updates: blocks.len() as u32,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks,
        }
    }
    /// Create a player create packet for login.
    pub fn create_player(
        guid: ObjectGuid,
        race: u8,
        class: u8,
        sex: u8,
        level: u8,
        display_id: u32,
        position: &Position,
        map_id: u16,
        zone_id: u32,
        is_self: bool,
        visible_items: [(i32, u16, u16); 19],
        inv_slots: [ObjectGuid; 141],
        combat: PlayerCombatStats,
        skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)>,
        coinage: u64,
        quest_log: Vec<(u32, u32, i64, [u16; 24])>,
    ) -> Self {
        Self::create_player_with_party_type(
            guid,
            race,
            class,
            sex,
            level,
            display_id,
            position,
            map_id,
            zone_id,
            is_self,
            visible_items,
            inv_slots,
            combat,
            skill_info,
            coinage,
            quest_log,
            [0; 2],
        )
    }
    pub fn create_player_with_party_type(
        guid: ObjectGuid,
        race: u8,
        class: u8,
        sex: u8,
        level: u8,
        display_id: u32,
        position: &Position,
        map_id: u16,
        zone_id: u32,
        is_self: bool,
        visible_items: [(i32, u16, u16); 19],
        inv_slots: [ObjectGuid; 141],
        combat: PlayerCombatStats,
        skill_info: Vec<(u16, u16, u16, u16, u16, i16, u16)>,
        coinage: u64,
        quest_log: Vec<(u32, u32, i64, [u16; 24])>,
        party_type: [u8; 2],
    ) -> Self {
        let faction = PlayerCreateData::faction_for_race(race);

        let create_data = PlayerCreateData {
            guid,
            wow_account: ObjectGuid::EMPTY,
            bnet_account: ObjectGuid::EMPTY,
            race,
            class,
            sex,
            level,
            display_id,
            native_display_id: display_id,
            health: combat.health,
            max_health: combat.max_health,
            faction_template: faction,
            current_area_id: zone_id,
            player_flags: 0,
            player_flags_ex: 0,
            stats: combat.stats,
            stat_pos_buff: combat.stat_pos_buff,
            stat_neg_buff: combat.stat_neg_buff,
            base_armor: combat.base_armor,
            base_mana: combat.base_mana,
            max_mana: combat.max_mana,
            current_power0: match class {
                1 => 1000,
                4 => 100,
                6 => 1000,
                _ => combat.max_mana.max(0).min(i64::from(i32::MAX)) as i32,
            },
            attack_power: combat.attack_power,
            attack_power_mod_pos: combat.attack_power_mod_pos,
            ranged_attack_power: combat.ranged_attack_power,
            ranged_attack_power_mod_pos: combat.ranged_attack_power_mod_pos,
            min_damage: combat.min_damage,
            max_damage: combat.max_damage,
            min_ranged_damage: combat.min_ranged_damage,
            max_ranged_damage: combat.max_ranged_damage,
            block_pct: combat.block_pct,
            dodge_pct: combat.dodge_pct,
            dodge_from_attr: combat.dodge_from_attr,
            parry_pct: combat.parry_pct,
            parry_from_attr: combat.parry_from_attr,
            crit_pct: combat.crit_pct,
            ranged_crit_pct: combat.ranged_crit_pct,
            offhand_crit_pct: combat.offhand_crit_pct,
            spell_crit_pct: combat.spell_crit_pct,
            combat_ratings: combat.combat_ratings,
            spell_power: combat.spell_power,
            visible_items,
            customizations: Vec::new(),
            inv_slots,
            farsight_object: ObjectGuid::EMPTY,
            action_buttons: [0; MAX_ACTION_BUTTONS],
            skill_info,
            coinage,
            xp: 0,
            next_level_xp: 400,
            max_level: 80,
            scaling_player_level_delta: 0,
            rest_info: [
                RestInfoValuesUpdate {
                    rest_info_mask: 0x07,
                    threshold: 0,
                    state_id: 2,
                },
                RestInfoValuesUpdate {
                    rest_info_mask: 0x07,
                    threshold: 0,
                    state_id: 2,
                },
            ],
            watched_faction_index: -1,
            party_type,
            heirlooms: Vec::new(),
            heirloom_flags: Vec::new(),
            toys: Vec::new(),
            transmog: Vec::new(),
            trait_configs: Vec::new(),
            quest_log,
        };

        let movement = MovementBlock {
            position: *position,
            ..Default::default()
        };

        let type_id = if is_self {
            TypeId::ActivePlayer
        } else {
            TypeId::Player
        };

        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::CreateObject {
                update_type: UpdateType::CreateObject,
                guid,
                type_id,
                movement: Some(movement),
                create_data,
                is_self,
            }],
        }
    }
    /// Populate PlayerData::PlayerFlags and PlayerData::PlayerFlagsEx on the
    /// self CREATE block.
    ///
    /// C++ `Player::LoadFromDB` restores these into `m_playerData` before
    /// `Map::SendInitSelf` calls `Player::BuildCreateUpdateBlockForPlayer`.
    pub fn set_player_flags_like_cpp(&mut self, player_flags: u32, player_flags_ex: u32) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject {
                create_data,
                is_self: true,
                ..
            } = block
            {
                create_data.player_flags = player_flags;
                create_data.player_flags_ex = player_flags_ex;
                return;
            }
        }
    }
    /// Populate account collection dynamic fields on the player CREATE block.
    ///
    /// C++ `CollectionMgr::LoadToys` / `LoadHeirlooms` mutates
    /// `ActivePlayerData` before the create values are written during login.
    pub fn set_player_collection_dynamic_fields_like_cpp(
        &mut self,
        toys: Vec<i32>,
        heirlooms: Vec<(i32, u32)>,
        transmog: Vec<u32>,
        trait_configs: Vec<TraitConfigCreateData>,
    ) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject {
                create_data,
                is_self: true,
                ..
            } = block
            {
                create_data.toys = toys;
                create_data.heirlooms = heirlooms.iter().map(|(item_id, _)| *item_id).collect();
                create_data.heirloom_flags =
                    heirlooms.into_iter().map(|(_, flags)| flags).collect();
                create_data.transmog = transmog;
                create_data.trait_configs = trait_configs;
                return;
            }
        }
    }
    /// Override `UnitData::Power[0]` for a player create block.
    ///
    /// C++ `Player::BuildValuesCreate` serializes the live current power and
    /// max power separately. Login loads current `characters.power1`, while
    /// non-owner visibility uses the live registry snapshot.
    pub fn set_player_current_power0_like_cpp(&mut self, current_power0: i32) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject { create_data, .. } = block {
                create_data.current_power0 = current_power0;
                return;
            }
        }
    }
    /// Override `ActivePlayerData::XP` for the self player create block.
    pub fn set_player_xp_like_cpp(&mut self, xp: i32) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject {
                create_data,
                is_self: true,
                ..
            } = block
            {
                create_data.xp = xp;
            }
        }
    }
    /// Override `ActivePlayerData::NextLevelXP` for the self player create block.
    pub fn set_player_next_level_xp_like_cpp(&mut self, next_level_xp: i32) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject {
                create_data,
                is_self: true,
                ..
            } = block
            {
                create_data.next_level_xp = next_level_xp;
            }
        }
    }
    /// Override `ActivePlayerData::MaxLevel` for the self player create block.
    pub fn set_player_max_level_like_cpp(&mut self, max_level: i32) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject {
                create_data,
                is_self: true,
                ..
            } = block
            {
                create_data.max_level = max_level;
            }
        }
    }
    /// Override `ActivePlayerData::ScalingPlayerLevelDelta` for the self player create block.
    pub fn set_player_scaling_level_delta_like_cpp(&mut self, delta: i32) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject {
                create_data,
                is_self: true,
                ..
            } = block
            {
                create_data.scaling_player_level_delta = delta;
            }
        }
    }
    /// Override `ActivePlayerData::RestInfo[index]` for the self player create block.
    pub fn set_player_rest_info_like_cpp(&mut self, index: usize, threshold: u32, state_id: u8) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject {
                create_data,
                is_self: true,
                ..
            } = block
            {
                let Some(rest_info) = create_data.rest_info.get_mut(index) else {
                    return;
                };
                *rest_info = RestInfoValuesUpdate {
                    rest_info_mask: 0x07,
                    threshold,
                    state_id,
                };
            }
        }
    }
    /// Populate C++ `Player::m_actionButtons` for the self create block.
    pub fn set_player_action_buttons_like_cpp(
        &mut self,
        action_buttons: [u32; MAX_ACTION_BUTTONS],
    ) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject {
                create_data,
                is_self: true,
                ..
            } = block
            {
                create_data.action_buttons = action_buttons;
            }
        }
    }
    /// Populate PlayerData::Customizations on a player CREATE block.
    ///
    /// C++ `Player::LoadFromDB` loads `CHAR_SEL_CHARACTER_CUSTOMIZATIONS`,
    /// calls `SetCustomizations`, then `PlayerData::WriteCreate` writes the
    /// dynamic field for both owner and non-owner viewers.
    pub fn set_player_customizations_like_cpp(
        &mut self,
        customizations: Vec<ChrCustomizationChoiceValuesUpdate>,
    ) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject { create_data, .. } = block {
                create_data.customizations = customizations;
                return;
            }
        }
    }
}
