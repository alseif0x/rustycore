//! Update-object block builder state definitions, part 1 of 2.
//!
//! Separated from the block.rs root under #650. Behaviour is preserved.

#[allow(unused_imports)]
use super::super::*;
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum UpdateType {
    Values = 0,
    CreateObject = 1,
    CreateObject2 = 2,
}

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
pub(super) fn write_create_block(
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

pub(in crate::packets::update) fn write_scale_curve_values_create(
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

pub(in crate::packets::update) fn write_visual_anim_values_create(
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
pub(in crate::packets::update) fn write_object_values_update_block(
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

pub(in crate::packets::update) const VALUES_TYPE_OBJECT: u32 = 1 << 0;

pub(in crate::packets::update) fn write_object_data_values_update_section(
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

pub(super) fn dynamic_mask_block(mask_blocks: &[u32], block_index: usize) -> u32 {
    mask_blocks.get(block_index).copied().unwrap_or(0)
}

pub(in crate::packets::update) fn write_dynamic_field_update_mask(
    buf: &mut WorldPacket,
    size: usize,
    update_mask: Option<&[u32]>,
) {
    write_dynamic_field_update_mask_bits(buf, size, update_mask, 32);
}

pub(in crate::packets::update) fn write_dynamic_field_update_mask_bits(
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

pub(in crate::packets::update) fn dynamic_mask_has_index(
    update_mask: Option<&[u32]>,
    index: usize,
) -> bool {
    match update_mask {
        None => true,
        Some(blocks) => {
            let block = index / 32;
            let bit = index % 32;
            dynamic_mask_block(blocks, block) & (1 << bit) != 0
        }
    }
}

pub(in crate::packets::update) fn write_changed_i32_dynamic_values(
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

pub(in crate::packets::update) fn write_chr_customization_choice_values_update(
    buf: &mut WorldPacket,
    choice: &ChrCustomizationChoiceValuesUpdate,
) {
    buf.write_uint32(choice.option_id);
    buf.write_uint32(choice.choice_id);
}

pub(in crate::packets::update) fn write_scale_curve_values_update(
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

pub(in crate::packets::update) fn write_visual_anim_values_update(
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

pub(in crate::packets::update) fn write_update_field_blocks_mask(
    buf: &mut WorldPacket,
    mask: u64,
    block_count: u32,
) {
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

pub(in crate::packets::update) fn write_update_field_blocks_mask_u32(
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

pub(in crate::packets::update) fn field_mask_has(mask: u64, bit: usize) -> bool {
    mask & (1u64 << bit) != 0
}

pub(in crate::packets::update) fn field_blocks_have(blocks: &[u32], bit: usize) -> bool {
    let block = bit / 32;
    let bit_in_block = bit % 32;
    blocks.get(block).copied().unwrap_or(0) & (1 << bit_in_block) != 0
}

pub(in crate::packets::update) fn write_passive_spell_history_values_update(
    buf: &mut WorldPacket,
    data: &PassiveSpellHistoryValuesUpdate,
) {
    buf.write_int32(data.spell_id);
    buf.write_int32(data.aura_spell_id);
}

pub(in crate::packets::update) fn write_arena_cooldown_values_update(
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

pub(in crate::packets::update) fn write_dungeon_score_summary_values_update(
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
