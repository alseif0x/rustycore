// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Shared update-block framing, masks and value writers.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UpdateType {
    Values = 0,
    CreateObject = 1,
    CreateObject2 = 2,
}

// ── MovementBlock ───────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data_mask: u32,
    pub entry_id: i32,
    pub dynamic_flags: u32,
    pub scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChrCustomizationChoiceValuesUpdate {
    pub option_id: u32,
    pub choice_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScaleCurveValuesUpdate {
    pub scale_curve_mask: u32,
    pub override_active: bool,
    pub start_time_offset: u32,
    pub parameter_curve: u32,
    pub points: [(f32, f32); 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VisualAnimValuesUpdate {
    pub visual_anim_mask: u32,
    pub field_c: bool,
    pub animation_data_id: u32,
    pub anim_kit_id: u32,
    pub anim_progress: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassiveSpellHistoryValuesUpdate {
    pub spell_id: i32,
    pub aura_spell_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ArenaCooldownValuesUpdate {
    pub arena_cooldown_mask: u32,
    pub spell_id: i32,
    pub item_id: i32,
    pub charges: i32,
    pub flags: u32,
    pub start_time: u32,
    pub end_time: u32,
    pub next_charge_time: u32,
    pub max_charges: u8,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DungeonScoreMapSummaryValuesUpdate {
    pub challenge_mode_id: i32,
    pub map_score: f32,
    pub best_run_level: i32,
    pub best_run_duration_ms: i32,
    pub finished_success: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DungeonScoreSummaryValuesUpdate {
    pub overall_score_current_season: f32,
    pub ladder_score_current_season: f32,
    pub runs: Vec<DungeonScoreMapSummaryValuesUpdate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResearchValuesUpdate {
    pub research_project_id: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct SpellPctModByLabelValuesUpdate {
    pub mod_index: i32,
    pub modifier_value: f32,
    pub label_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpellFlatModByLabelValuesUpdate {
    pub mod_index: i32,
    pub modifier_value: i32,
    pub label_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CategoryCooldownModValuesUpdate {
    pub spell_category_id: i32,
    pub mod_cooldown: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WeeklySpellUseValuesUpdate {
    pub spell_category_id: i32,
    pub uses: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CompletedProjectValuesUpdate {
    pub completed_project_mask: u8,
    pub project_id: u32,
    pub first_completed: i64,
    pub completion_count: u32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ResearchHistoryValuesUpdate {
    pub research_history_mask: u8,
    pub completed_projects: Vec<CompletedProjectValuesUpdate>,
    pub completed_projects_update_mask: Option<Vec<u32>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StableInfoValuesUpdate {
    pub stable_info_mask: u8,
    pub pets: Vec<StablePetInfoValuesUpdate>,
    pub pets_update_mask: Option<Vec<u32>>,
    pub stable_master: ObjectGuid,
}

/// A single update block within an UpdateObject packet.
pub enum UpdateBlock {
    CreateObject {
        update_type: UpdateType,
        guid: ObjectGuid,
        type_id: TypeId,
        movement: Option<MovementBlock>,
        create_data: PlayerCreateData,
        is_self: bool,
    },
    CreateCreature {
        guid: ObjectGuid,
        movement: MovementBlock,
        create_data: CreatureCreateData,
    },
    CreateGameObject {
        update_type: UpdateType,
        guid: ObjectGuid,
        create_data: GameObjectCreateData,
    },
    CreateTransport {
        guid: ObjectGuid,
        create_data: GameObjectCreateData,
        server_time_ms: u32,
    },
    CreateDynamicObject {
        guid: ObjectGuid,
        create_data: DynamicObjectCreateData,
    },
    CreateAreaTrigger {
        guid: ObjectGuid,
        create_data: AreaTriggerCreateData,
    },
    CreateCorpse {
        guid: ObjectGuid,
        create_data: CorpseCreateData,
    },
    CreateSceneObject {
        guid: ObjectGuid,
        create_data: SceneObjectCreateData,
    },
    CreateConversation {
        guid: ObjectGuid,
        create_data: ConversationCreateData,
    },
    CreateItem {
        update_type: UpdateType,
        guid: ObjectGuid,
        create_data: ItemCreateData,
    },
    /// VALUES update for an item store. `dynamic_flags` is present when the
    /// same C++ `_StoreItem` call both grows a stack and binds it.
    ItemValuesUpdate {
        guid: ObjectGuid,
        stack_count: u32,
        dynamic_flags: Option<u32>,
    },
    /// VALUES update for a player: only changed InvSlots, VisibleItems, VirtualItems.
    PlayerValuesUpdate {
        guid: ObjectGuid,
        /// Changed InvSlots: (slot_index 0-140, new ObjectGuid or EMPTY).
        inv_slot_changes: Vec<(u8, ObjectGuid)>,
        /// Changed BuybackPrice/BuybackTimestamp rows: (buyback slot 94-105, price, timestamp).
        buyback_changes: Vec<(u8, u32, i64)>,
        /// Changed VisibleItems in PlayerData: (slot 0-18, item_id, appearance_mod, visual).
        visible_item_changes: Vec<(u8, i32, u16, u16)>,
        /// Changed VirtualItems in UnitData: (index 0-2 for MH/OH/Ranged, item_id, app, visual).
        virtual_item_changes: Vec<(u8, i32, u16, u16)>,
        /// Optional stat changes to include in UnitData section.
        stat_changes: Option<PlayerStatChanges>,
        /// Optional coinage update (ActivePlayerData.Coinage field, block 0 bit 28).
        coinage_change: Option<u64>,
    },
    /// VALUES update for a creature: only health and max health.
    CreatureHealthUpdate {
        guid: ObjectGuid,
        health: i64,
        max_health: i64,
    },
    /// Generic ObjectData VALUES update.
    ObjectValuesUpdate {
        guid: ObjectGuid,
        data: ObjectDataValuesUpdate,
    },
    /// VALUES update for DynamicObjectData.
    DynamicObjectValuesUpdate {
        guid: ObjectGuid,
        data: DynamicObjectDataValuesUpdate,
    },
    /// VALUES update for SceneObjectData.
    SceneObjectValuesUpdate {
        guid: ObjectGuid,
        data: SceneObjectDataValuesUpdate,
    },
    /// VALUES update for ConversationData.
    ConversationValuesUpdate {
        guid: ObjectGuid,
        data: ConversationDataValuesUpdate,
    },
    /// VALUES update for GameObjectData.
    GameObjectValuesUpdate {
        guid: ObjectGuid,
        data: GameObjectDataValuesUpdate,
    },
    /// VALUES update for CorpseData.
    CorpseValuesUpdate {
        guid: ObjectGuid,
        data: CorpseDataValuesUpdate,
    },
    /// VALUES update for AreaTriggerData.
    AreaTriggerValuesUpdate {
        guid: ObjectGuid,
        data: AreaTriggerDataValuesUpdate,
    },
    /// VALUES update for ItemData.
    FullItemValuesUpdate {
        guid: ObjectGuid,
        data: ItemDataValuesDeltaUpdate,
    },
    /// VALUES update for UnitData.
    UnitValuesUpdate {
        guid: ObjectGuid,
        data: UnitDataValuesDeltaUpdate,
    },
    /// VALUES update for PlayerData, optionally including UnitData.
    FullPlayerValuesUpdate {
        guid: ObjectGuid,
        data: PlayerDataValuesDeltaUpdate,
    },
    /// VALUES update for ActivePlayerData.
    FullActivePlayerValuesUpdate {
        guid: ObjectGuid,
        data: ActivePlayerDataValuesUpdate,
    },
    /// VALUES update for ContainerData, optionally including ItemData.
    ContainerValuesUpdate {
        guid: ObjectGuid,
        data: ContainerDataValuesUpdate,
    },
    /// Out-of-range destroy (removes object from client view without full destroy).
    DestroyOutOfRange { guid: ObjectGuid },
}

// ── UpdateObject (SMSG_UPDATE_OBJECT) ───────────────────────────────

/// The main update packet used to create, update, or destroy objects.
///
/// Wire format (matches C++ UpdateData::BuildPacket + UpdateObject write):
/// ```text
/// [u32] NumObjUpdates
/// [u16] MapID
/// [byte[]] Data — built from:
///   [bit] HasDestroyOrOutOfRange
///     if true: [u16 destroyCount][i32 totalCount][PackedGuid... destroy][PackedGuid... oor]
///   [i32] dataBlockSize
///   [bytes] concatenated update blocks
/// ```
pub struct UpdateObject {
    pub map_id: u16,
    pub num_updates: u32,
    pub destroy_guids: Vec<ObjectGuid>,
    pub out_of_range_guids: Vec<ObjectGuid>,
    pub blocks: Vec<UpdateBlock>,
}

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

    /// Populate C++ `Unit::m_movementInfo.transport` on a player CREATE.
    ///
    /// `Map::SendInitSelf` creates the player's current transport before the
    /// player block, and the player `MovementUpdate` references it through the
    /// nested `HasTransport` branch.
    pub fn set_player_movement_transport_like_cpp(&mut self, transport: TransportInfo) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject {
                movement: Some(movement),
                ..
            } = block
            {
                movement.transport = Some(Box::new(transport));
                return;
            }
        }
    }

    /// Populate PlayerData::WowAccount and PlayerData::BnetAccount on the
    /// self CREATE block.
    ///
    /// C++ `Player::LoadFromDB` sets these from `WorldSession` before
    /// `PlayerData::WriteCreate`.
    pub fn set_player_account_guids_like_cpp(
        &mut self,
        wow_account: ObjectGuid,
        bnet_account: ObjectGuid,
    ) {
        for block in &mut self.blocks {
            if let UpdateBlock::CreateObject {
                create_data,
                is_self: true,
                ..
            } = block
            {
                create_data.wow_account = wow_account;
                create_data.bnet_account = bnet_account;
                return;
            }
        }
    }

    /// Create a player VALUES update for changed inventory fields.
    ///
    /// Used when items are swapped/equipped/unequipped to update the client's
    /// InvSlots (ActivePlayerData) and VisibleItems (PlayerData) without
    /// recreating the entire player object.
    pub fn player_values_update(
        guid: ObjectGuid,
        map_id: u16,
        inv_slot_changes: Vec<(u8, ObjectGuid)>,
        visible_item_changes: Vec<(u8, i32, u16, u16)>,
        virtual_item_changes: Vec<(u8, i32, u16, u16)>,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes,
                buyback_changes: Vec::new(),
                visible_item_changes,
                virtual_item_changes,
                stat_changes: None,
                coinage_change: None,
            }],
        }
    }

    /// Create a player VALUES update for changed inventory and buyback fields.
    pub fn player_values_buyback_update(
        guid: ObjectGuid,
        map_id: u16,
        inv_slot_changes: Vec<(u8, ObjectGuid)>,
        buyback_changes: Vec<(u8, u32, i64)>,
        coinage: Option<u64>,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes,
                buyback_changes,
                visible_item_changes: Vec::new(),
                virtual_item_changes: Vec::new(),
                stat_changes: None,
                coinage_change: coinage,
            }],
        }
    }

    /// Create a VALUES update for player coinage + optional inv slot change.
    ///
    /// Used after buy/sell to update the client's displayed gold and inventory.
    pub fn player_money_update(
        guid: ObjectGuid,
        map_id: u16,
        coinage: u64,
        inv_slot_change: Option<(u8, ObjectGuid)>,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes: inv_slot_change.map(|c| vec![c]).unwrap_or_default(),
                buyback_changes: Vec::new(),
                visible_item_changes: Vec::new(),
                virtual_item_changes: Vec::new(),
                stat_changes: None,
                coinage_change: Some(coinage),
            }],
        }
    }

    /// Create a VALUES update for player stats only (after equip/desequip).
    pub fn player_stat_update(guid: ObjectGuid, map_id: u16, changes: PlayerStatChanges) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::PlayerValuesUpdate {
                guid,
                inv_slot_changes: Vec::new(),
                buyback_changes: Vec::new(),
                visible_item_changes: Vec::new(),
                virtual_item_changes: Vec::new(),
                stat_changes: Some(changes),
                coinage_change: None,
            }],
        }
    }

    /// Create a VALUES update for the base `UF::ObjectData` section.
    ///
    /// The mask follows TrinityCore `UF::ObjectData`: bit 0 is the parent bit,
    /// bits 1/2/3 are EntryID/DynamicFlags/Scale.
    pub fn object_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ObjectDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ObjectValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::DynamicObjectData`.
    pub fn dynamic_object_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: DynamicObjectDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::DynamicObjectValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::SceneObjectData`.
    pub fn scene_object_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: SceneObjectDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::SceneObjectValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::ConversationData`.
    pub fn conversation_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ConversationDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ConversationValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::GameObjectData`.
    pub fn game_object_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: GameObjectDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::GameObjectValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::CorpseData`.
    pub fn corpse_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: CorpseDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::CorpseValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::AreaTriggerData`.
    pub fn area_trigger_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: AreaTriggerDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::AreaTriggerValuesUpdate { guid, data }],
        }
    }

    /// Create a full VALUES update for `UF::ItemData`.
    pub fn full_item_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ItemDataValuesDeltaUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::FullItemValuesUpdate { guid, data }],
        }
    }

    /// Create a full VALUES update for `UF::UnitData`.
    pub fn unit_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: UnitDataValuesDeltaUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::UnitValuesUpdate { guid, data }],
        }
    }

    /// Create a full VALUES update for `UF::PlayerData`.
    pub fn full_player_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: PlayerDataValuesDeltaUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::FullPlayerValuesUpdate { guid, data }],
        }
    }

    /// Create a full VALUES update for `UF::ActivePlayerData`.
    pub fn full_active_player_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ActivePlayerDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::FullActivePlayerValuesUpdate { guid, data }],
        }
    }

    /// Create a VALUES update for `UF::ContainerData`, with optional `ItemData`.
    pub fn container_values_update(
        guid: ObjectGuid,
        map_id: u16,
        data: ContainerDataValuesUpdate,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ContainerValuesUpdate { guid, data }],
        }
    }

    /// Create an UpdateObject with item CREATE blocks.
    ///
    /// Each item gets its own block. Sent BEFORE the player CREATE packet
    /// so the client has item objects when it processes InvSlots.
    pub fn create_items(items: Vec<ItemCreateData>, map_id: u16) -> Self {
        Self::create_items_with_update_type(items, map_id, UpdateType::CreateObject2)
    }

    /// Create inventory item blocks from C++ `Player::_StoreItem`.
    ///
    /// `_StoreItem` calls `Item::AddToWorld` directly and then
    /// `SendUpdateToPlayer`; unlike `Map::AddToMap`, that path never raises
    /// `Object::m_isNewObject`, so `BuildCreateUpdateBlockForPlayer` writes
    /// `CreateObject` rather than `CreateObject2`.
    pub fn create_stored_items(items: Vec<ItemCreateData>, map_id: u16) -> Self {
        Self::create_items_with_update_type(items, map_id, UpdateType::CreateObject)
    }

    fn create_items_with_update_type(
        items: Vec<ItemCreateData>,
        map_id: u16,
        update_type: UpdateType,
    ) -> Self {
        let num = items.len() as u32;
        let blocks = items
            .into_iter()
            .map(|data| {
                let guid = data.item_guid;
                UpdateBlock::CreateItem {
                    update_type,
                    guid,
                    create_data: data,
                }
            })
            .collect();

        Self {
            map_id,
            num_updates: num,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks,
        }
    }

    /// Create an item VALUES update for changed stack count.
    pub fn item_stack_count_update(guid: ObjectGuid, map_id: u16, stack_count: u32) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ItemValuesUpdate {
                guid,
                stack_count,
                dynamic_flags: None,
            }],
        }
    }

    /// Create the single ItemData VALUES update emitted by C++ `_StoreItem`
    /// when an existing stack changes both count and binding flags.
    pub fn item_stack_count_and_flags_update(
        guid: ObjectGuid,
        map_id: u16,
        stack_count: u32,
        dynamic_flags: u32,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::ItemValuesUpdate {
                guid,
                stack_count,
                dynamic_flags: Some(dynamic_flags),
            }],
        }
    }
}

impl ServerPacket for UpdateObject {
    const OPCODE: ServerOpcodes = ServerOpcodes::UpdateObject;

    fn write(&self, pkt: &mut WorldPacket) {
        // Top level: NumObjUpdates + MapID
        pkt.write_uint32(self.num_updates);
        pkt.write_uint16(self.map_id);

        // Build the Data buffer (matches C++ UpdateData::BuildPacket)
        let mut data_buf = WorldPacket::new_empty();
        let destroy_guids: BTreeSet<ObjectGuid> = self.destroy_guids.iter().copied().collect();
        let out_of_range_guids: BTreeSet<ObjectGuid> =
            self.out_of_range_guids.iter().copied().collect();

        // HasDestroyOrOutOfRange bit
        let has_destroy_or_oor = !destroy_guids.is_empty() || !out_of_range_guids.is_empty();
        data_buf.write_bit(has_destroy_or_oor);

        if has_destroy_or_oor {
            data_buf.write_uint16(destroy_guids.len() as u16);
            data_buf.write_uint32((destroy_guids.len() + out_of_range_guids.len()) as u32);
            for g in &destroy_guids {
                data_buf.write_packed_guid(g);
            }
            for g in &out_of_range_guids {
                data_buf.write_packed_guid(g);
            }
        }

        // Build all update blocks into a separate buffer
        let mut blocks_buf = WorldPacket::new_empty();
        for block in &self.blocks {
            match block {
                UpdateBlock::CreateObject {
                    update_type,
                    guid,
                    type_id,
                    movement,
                    create_data,
                    is_self,
                } => {
                    write_create_block(
                        &mut blocks_buf,
                        *update_type,
                        guid,
                        *type_id,
                        movement.as_ref(),
                        create_data,
                        *is_self,
                    );
                }
                UpdateBlock::CreateCreature {
                    guid,
                    movement,
                    create_data,
                } => {
                    write_creature_create_block(&mut blocks_buf, guid, movement, create_data);
                }
                UpdateBlock::CreateGameObject {
                    update_type,
                    guid,
                    create_data,
                } => {
                    write_gameobject_create_block(&mut blocks_buf, *update_type, guid, create_data);
                }
                UpdateBlock::CreateTransport {
                    guid,
                    create_data,
                    server_time_ms,
                } => {
                    write_transport_create_block(
                        &mut blocks_buf,
                        UpdateType::CreateObject,
                        guid,
                        create_data,
                        *server_time_ms,
                    );
                }
                UpdateBlock::CreateDynamicObject { guid, create_data } => {
                    write_dynamic_object_create_block(&mut blocks_buf, guid, create_data);
                }
                UpdateBlock::CreateAreaTrigger { guid, create_data } => {
                    write_area_trigger_create_block(&mut blocks_buf, guid, create_data);
                }
                UpdateBlock::CreateCorpse { guid, create_data } => {
                    write_corpse_create_block(&mut blocks_buf, guid, create_data);
                }
                UpdateBlock::CreateSceneObject { guid, create_data } => {
                    write_scene_object_create_block(&mut blocks_buf, guid, create_data);
                }
                UpdateBlock::CreateConversation { guid, create_data } => {
                    write_conversation_create_block(&mut blocks_buf, guid, create_data);
                }
                UpdateBlock::CreateItem {
                    update_type,
                    guid,
                    create_data,
                } => {
                    write_item_create_block(&mut blocks_buf, *update_type, guid, create_data);
                }
                UpdateBlock::ItemValuesUpdate {
                    guid,
                    stack_count,
                    dynamic_flags,
                } => {
                    write_item_values_update_block(
                        &mut blocks_buf,
                        guid,
                        *stack_count,
                        *dynamic_flags,
                    );
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
                    write_player_values_update_block(
                        &mut blocks_buf,
                        guid,
                        inv_slot_changes,
                        buyback_changes,
                        visible_item_changes,
                        virtual_item_changes,
                        stat_changes.as_ref(),
                        *coinage_change,
                    );
                }
                UpdateBlock::CreatureHealthUpdate {
                    guid,
                    health,
                    max_health,
                } => {
                    write_creature_health_update_block(&mut blocks_buf, guid, *health, *max_health);
                }
                UpdateBlock::ObjectValuesUpdate { guid, data } => {
                    write_object_values_update_block(&mut blocks_buf, guid, *data);
                }
                UpdateBlock::DynamicObjectValuesUpdate { guid, data } => {
                    write_dynamic_object_values_update_block(&mut blocks_buf, guid, *data);
                }
                UpdateBlock::SceneObjectValuesUpdate { guid, data } => {
                    write_scene_object_values_update_block(&mut blocks_buf, guid, *data);
                }
                UpdateBlock::ConversationValuesUpdate { guid, data } => {
                    write_conversation_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::GameObjectValuesUpdate { guid, data } => {
                    write_game_object_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::CorpseValuesUpdate { guid, data } => {
                    write_corpse_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::AreaTriggerValuesUpdate { guid, data } => {
                    write_area_trigger_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::FullItemValuesUpdate { guid, data } => {
                    write_full_item_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::UnitValuesUpdate { guid, data } => {
                    write_full_unit_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::FullPlayerValuesUpdate { guid, data } => {
                    write_full_player_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::FullActivePlayerValuesUpdate { guid, data } => {
                    write_full_active_player_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::ContainerValuesUpdate { guid, data } => {
                    write_container_values_update_block(&mut blocks_buf, guid, data);
                }
                UpdateBlock::DestroyOutOfRange { .. } => {
                    // Handled via destroy_guids / out_of_range_guids, not as a block.
                }
            }
        }

        let blocks_data = blocks_buf.into_data();
        data_buf.write_uint32(blocks_data.len() as u32); // Data block size
        data_buf.write_bytes(&blocks_data);

        // Write the assembled Data buffer into the packet
        let assembled = data_buf.into_data();
        pkt.write_bytes(&assembled);
    }
}

/// Write a single CreateObject block.
fn write_create_block(
    buf: &mut WorldPacket,
    update_type: UpdateType,
    guid: &ObjectGuid,
    type_id: TypeId,
    movement: Option<&MovementBlock>,
    create_data: &PlayerCreateData,
    is_self: bool,
) {
    let write_active_player_movement = is_self;

    // UpdateType byte
    buf.write_uint8(update_type as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId byte
    buf.write_uint8(type_id as u8);

    // ── 18-bit CreateObjectBits ────────────────────────────────
    let has_movement = movement.is_some();
    buf.write_bit(false); // 0: NoBirthAnim
    buf.write_bit(false); // 1: EnablePortals
    buf.write_bit(false); // 2: PlayHoverAnim
    buf.write_bit(has_movement); // 3: MovementUpdate
    buf.write_bit(false); // 4: MovementTransport
    buf.write_bit(false); // 5: Stationary
    buf.write_bit(false); // 6: CombatVictim
    buf.write_bit(false); // 7: ServerTime
    buf.write_bit(false); // 8: Vehicle
    buf.write_bit(false); // 9: AnimKit
    buf.write_bit(false); // 10: Rotation
    buf.write_bit(false); // 11: AreaTrigger
    buf.write_bit(false); // 12: GameObject
    buf.write_bit(false); // 13: SmoothPhasing
    buf.write_bit(is_self); // 14: ThisIsYou
    buf.write_bit(false); // 15: SceneObject
    buf.write_bit(write_active_player_movement); // 16: ActivePlayer
    buf.write_bit(false); // 17: Conversation
    buf.flush_bits();

    // ── MovementUpdate block ───────────────────────────────────
    if let Some(mv) = movement.filter(|_| has_movement) {
        write_movement_update(buf, guid, mv);
    }

    // PauseTimes count (i32) — always 0, written after movement regardless of flags
    buf.write_int32(0);

    // No Stationary, CombatVictim, ServerTime, Vehicle, AnimKit, Rotation,
    // AreaTrigger, GameObject, SmoothPhasing, SceneObject blocks
    // (all flags are false)

    // MovementTransport block — not present (bit 4 = false)

    // ── ActivePlayer block (bit 16) ─────────────────────────────
    // C++ Object::BuildMovementUpdate writes this when flags.ActivePlayer is true.
    // Contains: 3 bits (HasSceneInstanceIDs, HasRuneState, HasActionButtons)
    //           + optional scene IDs, rune data, and 180 action buttons.
    if write_active_player_movement {
        write_active_player_movement_block(buf, &create_data.action_buttons);
    }

    // No Conversation block (bit 17 = false)

    // ── Values block ───────────────────────────────────────────
    create_data.write_values_create(buf, is_self);
}

pub(super) fn write_scale_curve_values_create(
    buf: &mut WorldPacket,
    data: &ScaleCurveValuesUpdate,
) {
    buf.write_uint32(data.start_time_offset);
    for point in data.points {
        buf.write_float(point.0);
        buf.write_float(point.1);
    }
    buf.write_uint32(data.parameter_curve);
    buf.write_bit(data.override_active);
    buf.flush_bits();
}

pub(super) fn write_visual_anim_values_create(
    buf: &mut WorldPacket,
    data: &VisualAnimValuesUpdate,
) {
    buf.write_uint32(data.animation_data_id);
    buf.write_uint32(data.anim_kit_id);
    buf.write_uint32(data.anim_progress);
    buf.write_bit(data.field_c);
    buf.flush_bits();
}

/// Write a VALUES update block containing only the base `UF::ObjectData` delta.
///
/// C++ refs:
/// - `Object::PrepareValuesUpdateBuffer`
/// - `Unit/GameObject/...::BuildValuesUpdate`
/// - `UF::ObjectData::WriteUpdate`
pub(super) fn write_object_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: ObjectDataValuesUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(data.changed_object_type_mask);

    if data.changed_object_type_mask & 1 != 0 {
        let mask = data.object_data_mask & 0x0F;
        val_buf.write_bits(mask, 4);
        val_buf.flush_bits();

        if mask & 0x01 != 0 {
            if mask & 0x02 != 0 {
                val_buf.write_int32(data.entry_id);
            }
            if mask & 0x04 != 0 {
                val_buf.write_uint32(data.dynamic_flags);
            }
            if mask & 0x08 != 0 {
                val_buf.write_float(data.scale);
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

pub(super) const VALUES_TYPE_OBJECT: u32 = 1 << 0;

pub(super) fn write_object_data_values_update_section(
    buf: &mut WorldPacket,
    data: ObjectDataValuesUpdate,
) {
    let mask = data.object_data_mask & 0x0F;
    buf.write_bits(mask, 4);
    buf.flush_bits();

    if mask & 0x01 != 0 {
        if mask & 0x02 != 0 {
            buf.write_int32(data.entry_id);
        }
        if mask & 0x04 != 0 {
            buf.write_uint32(data.dynamic_flags);
        }
        if mask & 0x08 != 0 {
            buf.write_float(data.scale);
        }
    }
}

fn dynamic_mask_block(mask_blocks: &[u32], block_index: usize) -> u32 {
    mask_blocks.get(block_index).copied().unwrap_or(0)
}

pub(super) fn write_dynamic_field_update_mask(
    buf: &mut WorldPacket,
    size: usize,
    update_mask: Option<&[u32]>,
) {
    write_dynamic_field_update_mask_bits(buf, size, update_mask, 32);
}

pub(super) fn write_dynamic_field_update_mask_bits(
    buf: &mut WorldPacket,
    size: usize,
    update_mask: Option<&[u32]>,
    bits_for_size: u32,
) {
    buf.write_bits(size as u32, bits_for_size);

    if size > 32 {
        for block in 0..(size / 32) {
            let mask = update_mask
                .map(|blocks| dynamic_mask_block(blocks, block))
                .unwrap_or(0xFFFF_FFFF);
            buf.write_uint32(mask);
        }
    } else if size == 32 {
        let mask = update_mask
            .map(|blocks| dynamic_mask_block(blocks, 0))
            .unwrap_or(0xFFFF_FFFF);
        buf.write_bits(mask, 32);
        return;
    }

    if size % 32 != 0 {
        let block = size / 32;
        let bits = (size % 32) as u32;
        let mask = update_mask
            .map(|blocks| dynamic_mask_block(blocks, block))
            .unwrap_or(0xFFFF_FFFF);
        buf.write_bits(mask, bits);
    }
}

pub(super) fn dynamic_mask_has_index(update_mask: Option<&[u32]>, index: usize) -> bool {
    match update_mask {
        None => true,
        Some(blocks) => {
            let block = index / 32;
            let bit = index % 32;
            dynamic_mask_block(blocks, block) & (1 << bit) != 0
        }
    }
}

pub(super) fn write_changed_i32_dynamic_values(
    buf: &mut WorldPacket,
    values: &[i32],
    update_mask: Option<&[u32]>,
) {
    for (index, value) in values.iter().enumerate() {
        if dynamic_mask_has_index(update_mask, index) {
            buf.write_int32(*value);
        }
    }
}

pub(super) fn write_chr_customization_choice_values_update(
    buf: &mut WorldPacket,
    choice: &ChrCustomizationChoiceValuesUpdate,
) {
    buf.write_uint32(choice.option_id);
    buf.write_uint32(choice.choice_id);
}

pub(super) fn write_scale_curve_values_update(
    buf: &mut WorldPacket,
    data: &ScaleCurveValuesUpdate,
) {
    let mask = data.scale_curve_mask & 0x7F;
    buf.write_bits(mask, 7);

    if mask & 0x01 != 0 && mask & 0x02 != 0 {
        buf.write_bit(data.override_active);
    }
    buf.flush_bits();

    if mask & 0x01 != 0 {
        if mask & 0x04 != 0 {
            buf.write_uint32(data.start_time_offset);
        }
        if mask & 0x08 != 0 {
            buf.write_uint32(data.parameter_curve);
        }
    }

    if mask & 0x10 != 0 {
        for (index, point) in data.points.iter().enumerate() {
            if mask & (1 << (5 + index)) != 0 {
                buf.write_float(point.0);
                buf.write_float(point.1);
            }
        }
    }
    buf.flush_bits();
}

pub(super) fn write_visual_anim_values_update(
    buf: &mut WorldPacket,
    data: &VisualAnimValuesUpdate,
) {
    let mask = data.visual_anim_mask & 0x1F;
    buf.write_bits(mask, 5);

    if mask & 0x01 != 0 && mask & 0x02 != 0 {
        buf.write_bit(data.field_c);
    }
    buf.flush_bits();

    if mask & 0x01 != 0 {
        if mask & 0x04 != 0 {
            buf.write_uint32(data.animation_data_id);
        }
        if mask & 0x08 != 0 {
            buf.write_uint32(data.anim_kit_id);
        }
        if mask & 0x10 != 0 {
            buf.write_uint32(data.anim_progress);
        }
    }
    buf.flush_bits();
}

pub(super) fn write_update_field_blocks_mask(buf: &mut WorldPacket, mask: u64, block_count: u32) {
    let mut blocks_mask = 0u32;
    for block in 0..block_count {
        if ((mask >> (block * 32)) & 0xFFFF_FFFF) != 0 {
            blocks_mask |= 1 << block;
        }
    }

    buf.write_bits(blocks_mask, block_count);
    for block in 0..block_count {
        let block_bits = ((mask >> (block * 32)) & 0xFFFF_FFFF) as u32;
        if block_bits != 0 {
            buf.write_bits(block_bits, 32);
        }
    }
}

pub(super) fn write_update_field_blocks_mask_u32(
    buf: &mut WorldPacket,
    blocks: &[u32],
    block_count_bits: u32,
) {
    let mut blocks_mask = 0u32;
    for (block, value) in blocks.iter().enumerate() {
        if *value != 0 {
            blocks_mask |= 1 << block;
        }
    }

    buf.write_bits(blocks_mask, block_count_bits);
    for value in blocks {
        if *value != 0 {
            buf.write_bits(*value, 32);
        }
    }
}

pub(super) fn field_mask_has(mask: u64, bit: usize) -> bool {
    mask & (1u64 << bit) != 0
}

pub(super) fn field_blocks_have(blocks: &[u32], bit: usize) -> bool {
    let block = bit / 32;
    let bit_in_block = bit % 32;
    blocks.get(block).copied().unwrap_or(0) & (1 << bit_in_block) != 0
}

pub(super) fn write_passive_spell_history_values_update(
    buf: &mut WorldPacket,
    data: &PassiveSpellHistoryValuesUpdate,
) {
    buf.write_int32(data.spell_id);
    buf.write_int32(data.aura_spell_id);
}

pub(super) fn write_arena_cooldown_values_update(
    buf: &mut WorldPacket,
    data: &ArenaCooldownValuesUpdate,
) {
    let mask = data.arena_cooldown_mask & 0x01FF;
    buf.write_bits(mask, 9);
    buf.flush_bits();

    if mask & 0x001 != 0 {
        if mask & 0x002 != 0 {
            buf.write_int32(data.spell_id);
        }
        if mask & 0x004 != 0 {
            buf.write_int32(data.item_id);
        }
        if mask & 0x008 != 0 {
            buf.write_int32(data.charges);
        }
        if mask & 0x010 != 0 {
            buf.write_uint32(data.flags);
        }
        if mask & 0x020 != 0 {
            buf.write_uint32(data.start_time);
        }
        if mask & 0x040 != 0 {
            buf.write_uint32(data.end_time);
        }
        if mask & 0x080 != 0 {
            buf.write_uint32(data.next_charge_time);
        }
        if mask & 0x100 != 0 {
            buf.write_uint8(data.max_charges);
        }
    }
}

pub(super) fn write_dungeon_score_summary_values_update(
    buf: &mut WorldPacket,
    data: &DungeonScoreSummaryValuesUpdate,
) {
    buf.write_float(data.overall_score_current_season);
    buf.write_float(data.ladder_score_current_season);
    buf.write_uint32(data.runs.len() as u32);
    for run in &data.runs {
        buf.write_int32(run.challenge_mode_id);
        buf.write_float(run.map_score);
        buf.write_int32(run.best_run_level);
        buf.write_int32(run.best_run_duration_ms);
        buf.write_bit(run.finished_success);
        buf.flush_bits();
    }
}

pub fn write_research_values_update(buf: &mut WorldPacket, data: ResearchValuesUpdate) {
    buf.write_int16(data.research_project_id);
}

pub fn write_spell_pct_mod_by_label_values_update(
    buf: &mut WorldPacket,
    data: SpellPctModByLabelValuesUpdate,
) {
    buf.write_int32(data.mod_index);
    buf.write_float(data.modifier_value);
    buf.write_int32(data.label_id);
}

pub fn write_spell_flat_mod_by_label_values_update(
    buf: &mut WorldPacket,
    data: SpellFlatModByLabelValuesUpdate,
) {
    buf.write_int32(data.mod_index);
    buf.write_int32(data.modifier_value);
    buf.write_int32(data.label_id);
}

pub fn write_category_cooldown_mod_values_update(
    buf: &mut WorldPacket,
    data: CategoryCooldownModValuesUpdate,
) {
    buf.write_int32(data.spell_category_id);
    buf.write_int32(data.mod_cooldown);
}

pub fn write_weekly_spell_use_values_update(
    buf: &mut WorldPacket,
    data: WeeklySpellUseValuesUpdate,
) {
    buf.write_int32(data.spell_category_id);
    buf.write_uint8(data.uses);
}

pub fn write_completed_project_values_update(
    buf: &mut WorldPacket,
    data: CompletedProjectValuesUpdate,
) {
    let mask = data.completed_project_mask & 0x0F;
    buf.write_bits(mask as u32, 4);

    buf.flush_bits();
    if mask & 0x01 != 0 {
        if mask & 0x02 != 0 {
            buf.write_uint32(data.project_id);
        }
        if mask & 0x04 != 0 {
            buf.write_int64(data.first_completed);
        }
        if mask & 0x08 != 0 {
            buf.write_uint32(data.completion_count);
        }
    }
}

pub fn write_research_history_values_update(
    buf: &mut WorldPacket,
    data: &ResearchHistoryValuesUpdate,
) {
    let mask = data.research_history_mask & 0x03;
    buf.write_bits(mask as u32, 2);

    if mask & 0x01 != 0 && mask & 0x02 != 0 {
        write_dynamic_field_update_mask(
            buf,
            data.completed_projects.len(),
            data.completed_projects_update_mask.as_deref(),
        );
    }
    buf.flush_bits();

    if mask & 0x01 != 0 && mask & 0x02 != 0 {
        for (index, project) in data.completed_projects.iter().enumerate() {
            if dynamic_mask_has_index(data.completed_projects_update_mask.as_deref(), index) {
                write_completed_project_values_update(buf, *project);
            }
        }
    }
}

pub fn write_stable_info_values_update(buf: &mut WorldPacket, data: &StableInfoValuesUpdate) {
    let mask = data.stable_info_mask & 0x07;
    buf.write_bits(mask as u32, 3);

    if mask & 0x01 != 0 && mask & 0x02 != 0 {
        write_dynamic_field_update_mask(buf, data.pets.len(), data.pets_update_mask.as_deref());
    }
    buf.flush_bits();

    if mask & 0x01 != 0 {
        if mask & 0x02 != 0 {
            for (index, pet) in data.pets.iter().enumerate() {
                if dynamic_mask_has_index(data.pets_update_mask.as_deref(), index) {
                    write_stable_pet_info_values_update(buf, pet);
                }
            }
        }
        if mask & 0x04 != 0 {
            buf.write_packed_guid(&data.stable_master);
        }
    }
}

impl UpdateObject {
    /// Build a single-creature health VALUES update packet.
    pub fn creature_health_update(
        guid: ObjectGuid,
        health: i64,
        max_health: i64,
        map_id: u16,
    ) -> Self {
        Self {
            map_id,
            num_updates: 1,
            destroy_guids: Vec::new(),
            out_of_range_guids: Vec::new(),
            blocks: vec![UpdateBlock::CreatureHealthUpdate {
                guid,
                health,
                max_health,
            }],
        }
    }

    /// Build an UpdateObject that hard-destroys objects (they no longer exist).
    pub fn destroy_objects(guids: Vec<ObjectGuid>, map_id: u16) -> Self {
        Self {
            map_id,
            num_updates: 0, // no create/update blocks
            destroy_guids: guids,
            out_of_range_guids: Vec::new(),
            blocks: Vec::new(),
        }
    }

    /// Build an UpdateObject that removes objects from the client's view
    /// because they moved out of range (they still exist in the world).
    /// C++ refs: `Object::BuildOutOfRangeUpdateBlock` →
    /// `UpdateData::AddOutOfRangeGUID`.
    pub fn out_of_range_objects(guids: Vec<ObjectGuid>, map_id: u16) -> Self {
        Self {
            map_id,
            num_updates: 0, // no create/update blocks
            destroy_guids: Vec::new(),
            out_of_range_guids: guids,
            blocks: Vec::new(),
        }
    }
}
