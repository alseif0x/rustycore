// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! GameObject, area-trigger, conversation and corpse update blocks.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DynamicObjectDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub dynamic_object_data_mask: u32,
    pub caster: ObjectGuid,
    pub dynamic_object_type: u8,
    pub spell_visual_id: i32,
    pub spell_id: i32,
    pub radius: f32,
    pub cast_time_ms: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneObjectDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub scene_object_data_mask: u32,
    pub script_package_id: i32,
    pub rnd_seed_val: u32,
    pub created_by: ObjectGuid,
    pub scene_type: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversationLineValuesUpdate {
    pub conversation_line_id: i32,
    pub start_time: u32,
    pub ui_camera_id: i32,
    pub actor_index: u8,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConversationActorValuesUpdate {
    pub actor_type: u32,
    pub id: i32,
    pub creature_id: u32,
    pub creature_display_info_id: u32,
    pub actor_guid: ObjectGuid,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConversationDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub conversation_data_mask: u32,
    pub lines: Vec<ConversationLineValuesUpdate>,
    pub actors: Vec<ConversationActorValuesUpdate>,
    /// C++ `DynamicUpdateField<ConversationActor>` nested mask blocks.
    ///
    /// `None` represents `ignoreNestedChangesMask=true`, so all actors present in
    /// `actors` are marked and written. `Some(blocks)` writes exactly those
    /// nested change-mask bits and serializes only marked actor indices.
    pub actor_update_mask: Option<Vec<u32>>,
    pub last_line_end_time: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub game_object_data_mask: u32,
    pub state_world_effect_ids: Vec<u32>,
    pub enable_doodad_sets: Vec<i32>,
    pub enable_doodad_sets_update_mask: Option<Vec<u32>>,
    pub world_effects: Vec<i32>,
    pub world_effects_update_mask: Option<Vec<u32>>,
    pub display_id: i32,
    pub spell_visual_id: u32,
    pub state_spell_visual_id: u32,
    pub spawn_tracking_state_anim_id: u32,
    pub spawn_tracking_state_anim_kit_id: u32,
    pub created_by: ObjectGuid,
    pub guild_guid: ObjectGuid,
    pub flags: u32,
    pub parent_rotation: [f32; 4],
    pub faction_template: i32,
    pub level: i32,
    pub state: i8,
    pub type_id: i8,
    pub percent_health: u8,
    pub art_kit: u32,
    pub custom_param: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CorpseDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub corpse_data_mask: u32,
    pub customizations: Vec<ChrCustomizationChoiceValuesUpdate>,
    pub customizations_update_mask: Option<Vec<u32>>,
    pub dynamic_flags: u32,
    pub owner: ObjectGuid,
    pub party_guid: ObjectGuid,
    pub guild_guid: ObjectGuid,
    pub display_id: u32,
    pub race_id: u8,
    pub sex: u8,
    pub class: u8,
    pub flags: u32,
    pub faction_template: i32,
    pub items: [u32; 19],
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerDataValuesUpdate {
    pub changed_object_type_mask: u32,
    pub object_data: Option<ObjectDataValuesUpdate>,
    pub area_trigger_data_mask: u32,
    pub override_scale_curve: ScaleCurveValuesUpdate,
    pub extra_scale_curve: ScaleCurveValuesUpdate,
    pub override_move_curve_x: ScaleCurveValuesUpdate,
    pub override_move_curve_y: ScaleCurveValuesUpdate,
    pub override_move_curve_z: ScaleCurveValuesUpdate,
    pub caster: ObjectGuid,
    pub duration: u32,
    pub time_to_target: u32,
    pub time_to_target_scale: u32,
    pub time_to_target_extra_scale: u32,
    pub time_to_target_pos: u32,
    pub spell_id: i32,
    pub spell_for_visuals: i32,
    pub spell_visual_id: i32,
    pub bounds_radius_2d: f32,
    pub decal_properties_id: u32,
    pub creating_effect_guid: ObjectGuid,
    pub orbit_path_target: ObjectGuid,
    pub visual_anim: VisualAnimValuesUpdate,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AreaTriggerPosition2CreateData {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AreaTriggerPosition3CreateData {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerShapeCreateData {
    pub shape_type: u8,
    pub data: [f32; 8],
    pub polygon_vertices: Vec<AreaTriggerPosition2CreateData>,
    pub polygon_vertices_target: Vec<AreaTriggerPosition2CreateData>,
}

impl Default for AreaTriggerShapeCreateData {
    fn default() -> Self {
        Self {
            shape_type: 0,
            data: [0.0; 8],
            polygon_vertices: Vec::new(),
            polygon_vertices_target: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaTriggerOrbitCreateData {
    pub counter_clockwise: bool,
    pub can_loop: bool,
    pub time_to_target: u32,
    pub elapsed_time_for_movement: i32,
    pub start_delay: u32,
    pub radius: f32,
    pub blend_from_radius: f32,
    pub initial_angle: f32,
    pub z_offset: f32,
    pub center: AreaTriggerPosition3CreateData,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaTriggerCreateData {
    pub guid: ObjectGuid,
    pub entry_id: u32,
    pub dynamic_flags: u32,
    pub scale: f32,
    pub position: Position,
    pub time_since_created_ms: u32,
    pub roll_pitch_yaw: Position,
    pub target_roll_pitch_yaw: Position,
    pub create_properties_flags: u32,
    pub scale_curve_id: u32,
    pub morph_curve_id: u32,
    pub facing_curve_id: u32,
    pub move_curve_id: u32,
    pub shape: AreaTriggerShapeCreateData,
    pub spline_points: Vec<AreaTriggerPosition3CreateData>,
    pub orbit: Option<AreaTriggerOrbitCreateData>,
    pub override_scale_curve: ScaleCurveValuesUpdate,
    pub extra_scale_curve: ScaleCurveValuesUpdate,
    pub override_move_curve_x: ScaleCurveValuesUpdate,
    pub override_move_curve_y: ScaleCurveValuesUpdate,
    pub override_move_curve_z: ScaleCurveValuesUpdate,
    pub caster: ObjectGuid,
    pub duration: u32,
    pub time_to_target: u32,
    pub time_to_target_scale: u32,
    pub time_to_target_extra_scale: u32,
    pub time_to_target_pos: u32,
    pub spell_id: i32,
    pub spell_for_visuals: i32,
    pub spell_visual_id: i32,
    pub bounds_radius_2d: f32,
    pub decal_properties_id: u32,
    pub creating_effect_guid: ObjectGuid,
    pub orbit_path_target: ObjectGuid,
    pub visual_anim: VisualAnimValuesUpdate,
}

/// Data needed to build a gameobject create packet for the client.
#[derive(Debug, Clone)]
pub struct GameObjectCreateData {
    pub guid: ObjectGuid,
    pub entry: u32,
    pub dynamic_flags: u32,
    pub display_id: u32,
    pub go_type: u8,
    pub position: Position,
    pub rotation: [f32; 4], // rotation0..3 (quaternion)
    pub anim_progress: u8,
    pub state: i8,
    /// C++ `GameObjectData::ArtKit`, including runtime `SetGoArtKit` changes.
    pub art_kit: u32,
    pub created_by: ObjectGuid,
    pub faction_template: i32,
    pub gameobject_flags: u32,
    pub world_effect_id: u32,
    pub scale: f32,
    /// C++ `GameObjectData::Level`. For a MO_TRANSPORT (go_type 15) this is the
    /// transport's full path period = `TransportTemplate::TotalPathTime` (ms), set by
    /// `Transport::Create` -> `SetPeriod` (Transport.cpp:145; Transport.h:89
    /// `GetTransportPeriod() { return Level; }`). The 3.4.3 client divides PathProgress by
    /// this period to interpolate the transport along its path; Level=0 -> divide-by-zero ->
    /// invalid path-node index (0xFFFF) -> NULL deref in the render/anim worker (ERROR #132).
    /// For all other GameObjects this is 0 (they derive any period from AnimationData, not Level).
    pub level: u32,
    /// C++ `GameObjectData::ParentRotation` (UpdateFields). Identity quaternion
    /// `(0, 0, 0, 1)` for most GameObjects; sourced from per-spawn
    /// `gameobject_addon.parent_rotation0..3` when present (GameObject::Create,
    /// GameObject.cpp:1003-1008). Distinct from the local `rotation` packed by the
    /// movement-update Rotation flag.
    pub parent_rotation: [f32; 4],
}

impl GameObjectCreateData {
    /// Write the values block for CREATE.
    ///
    /// For GameObjects: ObjectData + GameObjectFieldData (no UnitData/PlayerData).
    pub fn write_values_create(&self, pkt: &mut WorldPacket) {
        let mut buf = WorldPacket::new_empty();

        // UpdateFieldFlag: 0x00 for non-owner
        buf.write_uint8(0x00);

        // ObjectFieldData.WriteCreate
        buf.write_int32(self.entry as i32); // EntryId
        buf.write_uint32(self.dynamic_flags); // DynamicFlags
        buf.write_float(self.scale); // Scale

        // C++ `GameObjectData::WriteCreate` (UpdateFields.cpp) order.
        buf.write_int32(self.display_id as i32); // DisplayID
        buf.write_int32(0); // SpellVisualID
        buf.write_int32(0); // StateSpellVisualID
        // C++ GameObject::Create (GameObject.cpp:1055) seeds SpawnTrackingStateAnimID with
        // DB2Manager::GetEmptyAnimStateID() = 1772 for EVERY GameObject (the Classic client
        // expects the retail AnimationData storage size; DB2Stores.cpp:1765). Shipping 0 makes
        // the client resolve a NULL anim-state record and deref it (test [NULL+0x10],0x100000)
        // in the render/anim worker (~4-5s in-world, ERROR #132) — confirmed via C++/Rust wire
        // diff on MO_TRANSPORT blocks (C++=1772, Rust was 0).
        buf.write_int32(1772); // SpawnTrackingStateAnimID = GetEmptyAnimStateID
        buf.write_int32(0); // SpawnTrackingStateAnimKitID
        buf.write_int32(0); // StateWorldEffectIDs.Count
        // No StateWorldEffectIDs entries (count=0)
        buf.write_packed_guid(&self.created_by); // CreatedBy
        write_empty_guid(&mut buf); // GuildGUID
        buf.write_uint32(self.gameobject_flags); // Flags
        // ParentRotation (Quaternion: x, y, z, w)
        // C++ uses GameObjectData::ParentRotation, not the local rotation
        // packed separately by Object::BuildMovementUpdate's Rotation flag.
        // For most GameObjects it's the identity quaternion (0, 0, 0, 1); some
        // (transports, a few addon GameObjects) carry a non-standard parent
        // rotation from gameobject_addon (GameObject::Create, GameObject.cpp:1003-1008).
        buf.write_float(self.parent_rotation[0]); // ParentRotation.X
        buf.write_float(self.parent_rotation[1]); // ParentRotation.Y
        buf.write_float(self.parent_rotation[2]); // ParentRotation.Z
        buf.write_float(self.parent_rotation[3]); // ParentRotation.W
        buf.write_int32(self.faction_template); // FactionTemplate
        buf.write_uint32(self.level); // Level (MO_TRANSPORT period = TotalPathTime; else 0)
        buf.write_int8(self.state); // State
        buf.write_int8(self.go_type as i8); // TypeID (gameobject type)
        buf.write_uint8(self.anim_progress); // PercentHealth (anim progress)
        buf.write_int32(self.art_kit as i32); // ArtKit
        buf.write_int32(0); // EnableDoodadSets.Size
        buf.write_int32(0); // CustomParam
        buf.write_int32(0); // WorldEffects.Size
        // No EnableDoodadSets/WorldEffects entries

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32);
        pkt.write_bytes(&data);
    }

    /// Pack the local rotation as a 64-bit integer for the Rotation flag.
    ///
    /// Matches Trinity C++ packed local rotation used by
    /// `GameObjectData::WriteCreate` / `WriteUpdate`: Z and Y use 21 bits,
    /// X uses 22 bits, with the sign of W applied before packing.
    /// Layout: bits[0:20]=Z(21), bits[21:41]=Y(21), bits[42:63]=X(22).
    pub fn packed_rotation(&self) -> i64 {
        const PACK_YZ: i64 = 1 << 20; // 1,048,576
        const PACK_X: i64 = PACK_YZ << 1; // 2,097,152
        const PACK_YZ_MASK: i64 = (PACK_YZ << 1) - 1; // 0x1FFFFF
        const PACK_X_MASK: i64 = (PACK_X << 1) - 1; // 0x3FFFFF

        // Normalize quaternion before packing, matching the C++ setter path.
        let (rx, ry, rz, rw) = {
            let dot = self.rotation[0] * self.rotation[0]
                + self.rotation[1] * self.rotation[1]
                + self.rotation[2] * self.rotation[2]
                + self.rotation[3] * self.rotation[3];
            let inv_len = 1.0 / dot.sqrt();
            (
                self.rotation[0] * inv_len,
                self.rotation[1] * inv_len,
                self.rotation[2] * inv_len,
                self.rotation[3] * inv_len,
            )
        };

        let w_sign: i32 = if rw >= 0.0 { 1 } else { -1 };

        let x = ((rx * PACK_X as f32) as i32 as i64) * w_sign as i64 & PACK_X_MASK;
        let y = ((ry * PACK_YZ as f32) as i32 as i64) * w_sign as i64 & PACK_YZ_MASK;
        let z = ((rz * PACK_YZ as f32) as i32 as i64) * w_sign as i64 & PACK_YZ_MASK;

        z | (y << 21) | (x << 42)
    }
}

// ── DynamicObjectCreateData ────────────────────────────────────────

/// Data needed to build a DynamicObject create packet for the client.
///
/// C++ anchors:
/// - `DynamicObject::DynamicObject(bool)` sets Stationary create flag.
/// - `DynamicObject::BuildValuesCreate` writes ObjectData then DynamicObjectData.
pub struct DynamicObjectCreateData {
    pub guid: ObjectGuid,
    pub entry_id: u32,
    pub dynamic_flags: u32,
    pub scale: f32,
    pub position: Position,
    pub caster: ObjectGuid,
    pub dynamic_object_type: u8,
    pub spell_visual_id: i32,
    pub spell_id: i32,
    pub radius: f32,
    pub cast_time_ms: u32,
}

impl DynamicObjectCreateData {
    /// Write the create-time values block: `[u32 size][u8 flags][ObjectData][DynamicObjectData]`.
    ///
    /// This is a CREATE values section, not an `UpdateType::Values` block; it intentionally
    /// does not write a packed object GUID or update masks inside the values payload.
    pub fn write_values_create(&self, pkt: &mut WorldPacket) {
        let mut buf = WorldPacket::new_empty();

        // UpdateFieldFlag: 0x00 for non-owner.
        buf.write_uint8(0x00);

        // ObjectData::WriteCreate.
        buf.write_int32(self.entry_id as i32);
        buf.write_uint32(self.dynamic_flags);
        buf.write_float(self.scale);

        // DynamicObjectData::WriteCreate.
        buf.write_packed_guid(&self.caster);
        buf.write_uint8(self.dynamic_object_type);
        buf.write_int32(self.spell_visual_id);
        buf.write_int32(self.spell_id);
        buf.write_float(self.radius);
        buf.write_uint32(self.cast_time_ms);

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32);
        pkt.write_bytes(&data);
    }
}

/// Data needed to build a Corpse CREATE block.
///
/// C++ `Corpse::BuildValuesCreate` writes `ObjectData` followed by
/// `CorpseData`; corpses use only the Stationary movement-create flag.
#[derive(Debug, Clone)]
pub struct CorpseCreateData {
    pub guid: ObjectGuid,
    pub entry_id: u32,
    pub object_dynamic_flags: u32,
    pub scale: f32,
    pub position: Position,
    pub corpse_dynamic_flags: u32,
    pub owner: ObjectGuid,
    pub party_guid: ObjectGuid,
    pub guild_guid: ObjectGuid,
    pub display_id: u32,
    pub items: [u32; 19],
    pub race_id: u8,
    pub sex: u8,
    pub class: u8,
    pub customizations: Vec<ChrCustomizationChoiceValuesUpdate>,
    pub flags: u32,
    pub faction_template: i32,
}

impl CorpseCreateData {
    fn write_values_create(&self, pkt: &mut WorldPacket) {
        let mut buf = WorldPacket::new_empty();
        buf.write_uint8(0); // UpdateFieldFlag::None
        write_object_data_create_like_cpp(
            &mut buf,
            self.entry_id,
            self.object_dynamic_flags,
            self.scale,
        );

        buf.write_uint32(self.corpse_dynamic_flags);
        buf.write_packed_guid(&self.owner);
        buf.write_packed_guid(&self.party_guid);
        buf.write_packed_guid(&self.guild_guid);
        buf.write_uint32(self.display_id);
        for item in self.items {
            buf.write_uint32(item);
        }
        buf.write_uint8(self.race_id);
        buf.write_uint8(self.sex);
        buf.write_uint8(self.class);
        buf.write_uint32(self.customizations.len() as u32);
        buf.write_uint32(self.flags);
        buf.write_int32(self.faction_template);
        for customization in &self.customizations {
            write_chr_customization_choice_values_update(&mut buf, customization);
        }

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32);
        pkt.write_bytes(&data);
    }
}

/// Data needed to build a SceneObject CREATE block.
#[derive(Debug, Clone)]
pub struct SceneObjectCreateData {
    pub guid: ObjectGuid,
    pub entry_id: u32,
    pub dynamic_flags: u32,
    pub scale: f32,
    pub position: Position,
    pub script_package_id: i32,
    pub rnd_seed_val: u32,
    pub created_by: ObjectGuid,
    pub scene_type: u32,
}

impl SceneObjectCreateData {
    fn write_values_create(&self, pkt: &mut WorldPacket) {
        let mut buf = WorldPacket::new_empty();
        buf.write_uint8(0); // UpdateFieldFlag::None
        write_object_data_create_like_cpp(&mut buf, self.entry_id, self.dynamic_flags, self.scale);
        buf.write_int32(self.script_package_id);
        buf.write_uint32(self.rnd_seed_val);
        buf.write_packed_guid(&self.created_by);
        buf.write_uint32(self.scene_type);

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32);
        pkt.write_bytes(&data);
    }
}

/// Data needed to build a Conversation CREATE block.
#[derive(Debug, Clone)]
pub struct ConversationCreateData {
    pub guid: ObjectGuid,
    pub entry_id: u32,
    pub dynamic_flags: u32,
    pub scale: f32,
    pub position: Position,
    pub texture_kit_id: u32,
    pub lines: Vec<ConversationLineValuesUpdate>,
    pub actors: Vec<ConversationActorValuesUpdate>,
    pub last_line_end_time: i32,
}

impl ConversationCreateData {
    fn write_values_create(&self, pkt: &mut WorldPacket) {
        let mut buf = WorldPacket::new_empty();
        buf.write_uint8(0); // UpdateFieldFlag::None
        write_object_data_create_like_cpp(&mut buf, self.entry_id, self.dynamic_flags, self.scale);
        buf.write_uint32(self.lines.len() as u32);
        buf.write_int32(self.last_line_end_time);
        for line in &self.lines {
            write_conversation_line_values_update(&mut buf, line);
        }
        buf.write_uint32(self.actors.len() as u32);
        for actor in &self.actors {
            write_conversation_actor_values_update(&mut buf, actor);
        }

        let data = buf.into_data();
        pkt.write_uint32(data.len() as u32);
        pkt.write_bytes(&data);
    }
}

pub(super) fn debug_gameobject_create_values_len_like_cpp(data: &GameObjectCreateData) -> usize {
    let mut values = WorldPacket::new_empty();
    data.write_values_create(&mut values);
    values.into_data().len()
}

pub(super) fn debug_area_trigger_create_block_len_like_cpp(
    guid: &ObjectGuid,
    data: &AreaTriggerCreateData,
) -> usize {
    let mut block = WorldPacket::new_empty();
    write_area_trigger_create_block(&mut block, guid, data);
    block.into_data().len()
}

/// Write a single CreateObject block for a gameobject (TypeId::GameObject).
///
/// GameObjects use Stationary (bit 5) + Rotation (bit 10).
///
/// C++ only sets `CreateObjectBits::GameObject` when a GO addon/template has
/// `WorldEffectID`; ordinary GameObjects must not write that extra payload.
/// No MovementUpdate block.
pub(super) fn write_gameobject_create_block(
    buf: &mut WorldPacket,
    update_type: UpdateType,
    guid: &ObjectGuid,
    create_data: &GameObjectCreateData,
) {
    let has_gameobject_payload = create_data.world_effect_id != 0;

    buf.write_uint8(update_type as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId = GameObject (8)
    buf.write_uint8(TypeId::GameObject as u8);

    // ── 18-bit CreateObjectBits ────────────────────────────
    buf.write_bit(false); // 0: NoBirthAnim
    buf.write_bit(false); // 1: EnablePortals
    buf.write_bit(false); // 2: PlayHoverAnim
    buf.write_bit(false); // 3: MovementUpdate (false for GOs)
    buf.write_bit(false); // 4: MovementTransport
    buf.write_bit(true); // 5: Stationary (true for GOs)
    buf.write_bit(false); // 6: CombatVictim
    buf.write_bit(false); // 7: ServerTime
    buf.write_bit(false); // 8: Vehicle
    buf.write_bit(false); // 9: AnimKit
    buf.write_bit(true); // 10: Rotation (true for GOs)
    buf.write_bit(false); // 11: AreaTrigger
    buf.write_bit(has_gameobject_payload); // 12: GameObject (WorldEffectID payload)
    buf.write_bit(false); // 13: SmoothPhasing
    buf.write_bit(false); // 14: ThisIsYou
    buf.write_bit(false); // 15: SceneObject
    buf.write_bit(false); // 16: ActivePlayer
    buf.write_bit(false); // 17: Conversation
    buf.flush_bits();

    // No MovementUpdate (bit 3 = false)

    // PauseTimes count (i32) — always 0
    buf.write_int32(0);

    // ── Stationary block (bit 5 = true) ─────────────────────
    buf.write_float(create_data.position.x);
    buf.write_float(create_data.position.y);
    buf.write_float(create_data.position.z);
    buf.write_float(create_data.position.orientation);

    // ── Rotation block (bit 10 = true) ──────────────────────
    buf.write_int64(create_data.packed_rotation());

    // ── GameObject block (bit 12 = true) ─────────────────────
    if has_gameobject_payload {
        buf.write_uint32(create_data.world_effect_id); // WorldEffectID
        buf.write_bit(false); // has extra u32
        buf.flush_bits();
    }

    // ── Values block ─────────────────────────────────────────
    create_data.write_values_create(buf);
}

/// Write a single CreateObject block for a map transport.
///
/// TrinityCore `Transport` inherits `GameObject`, but its constructor sets
/// `m_updateFlag.ServerTime`, `Stationary`, and `Rotation` only
/// (`Transport.cpp`). `Object::BuildMovementUpdate` writes the server time
/// between the stationary block and rotation.
pub(super) fn write_transport_create_block(
    buf: &mut WorldPacket,
    update_type: UpdateType,
    guid: &ObjectGuid,
    create_data: &GameObjectCreateData,
    server_time_ms: u32,
) {
    buf.write_uint8(update_type as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId = GameObject (8), matching HighGuid::Transport.
    buf.write_uint8(TypeId::GameObject as u8);

    // ── 18-bit CreateObjectBits ────────────────────────────
    buf.write_bit(false); // 0: NoBirthAnim
    buf.write_bit(false); // 1: EnablePortals
    buf.write_bit(false); // 2: PlayHoverAnim
    buf.write_bit(false); // 3: MovementUpdate
    buf.write_bit(false); // 4: MovementTransport
    buf.write_bit(true); // 5: Stationary
    buf.write_bit(false); // 6: CombatVictim
    buf.write_bit(true); // 7: ServerTime
    buf.write_bit(false); // 8: Vehicle
    buf.write_bit(false); // 9: AnimKit
    buf.write_bit(true); // 10: Rotation
    buf.write_bit(false); // 11: AreaTrigger
    buf.write_bit(false); // 12: GameObject
    buf.write_bit(false); // 13: SmoothPhasing
    buf.write_bit(false); // 14: ThisIsYou
    buf.write_bit(false); // 15: SceneObject
    buf.write_bit(false); // 16: ActivePlayer
    buf.write_bit(false); // 17: Conversation
    buf.flush_bits();

    // PauseTimes count
    buf.write_int32(0);

    // Stationary
    buf.write_float(create_data.position.x);
    buf.write_float(create_data.position.y);
    buf.write_float(create_data.position.z);
    buf.write_float(create_data.position.orientation);

    // ServerTime
    buf.write_uint32(server_time_ms);

    // Rotation
    buf.write_int64(create_data.packed_rotation());

    // Values
    create_data.write_values_create(buf);
}

/// Write a single CreateObject block for a dynamic object (TypeId::DynamicObject).
///
/// DynamicObjects use Stationary (bit 5), no MovementUpdate, no Unit shared-vision payload.
pub(super) fn write_dynamic_object_create_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    create_data: &DynamicObjectCreateData,
) {
    // UpdateType: CreateObject2 — first appearance of this object to the client
    buf.write_uint8(UpdateType::CreateObject2 as u8);

    // Object GUID
    buf.write_packed_guid(guid);

    // TypeId = DynamicObject (9)
    buf.write_uint8(TypeId::DynamicObject as u8);

    // ── 18-bit CreateObjectBits ────────────────────────────
    buf.write_bit(false); // 0: NoBirthAnim
    buf.write_bit(false); // 1: EnablePortals
    buf.write_bit(false); // 2: PlayHoverAnim
    buf.write_bit(false); // 3: MovementUpdate (false for DynamicObjects)
    buf.write_bit(false); // 4: MovementTransport
    buf.write_bit(true); // 5: Stationary (true for DynamicObjects)
    buf.write_bit(false); // 6: CombatVictim
    buf.write_bit(false); // 7: ServerTime
    buf.write_bit(false); // 8: Vehicle
    buf.write_bit(false); // 9: AnimKit
    buf.write_bit(false); // 10: Rotation
    buf.write_bit(false); // 11: AreaTrigger
    buf.write_bit(false); // 12: GameObject
    buf.write_bit(false); // 13: SmoothPhasing
    buf.write_bit(false); // 14: ThisIsYou
    buf.write_bit(false); // 15: SceneObject
    buf.write_bit(false); // 16: ActivePlayer
    buf.write_bit(false); // 17: Conversation
    buf.flush_bits();

    // No MovementUpdate (bit 3 = false)

    // PauseTimes count (i32) — always 0
    buf.write_int32(0);

    // ── Stationary block (bit 5 = true) ─────────────────────
    buf.write_float(create_data.position.x);
    buf.write_float(create_data.position.y);
    buf.write_float(create_data.position.z);
    buf.write_float(create_data.position.orientation);

    // ── Values block ─────────────────────────────────────────
    create_data.write_values_create(buf);
}

pub(super) fn write_corpse_create_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    create_data: &CorpseCreateData,
) {
    write_stationary_world_object_create_prefix_like_cpp(
        buf,
        guid,
        TypeId::Corpse,
        create_data.position,
        false,
        None,
    );
    create_data.write_values_create(buf);
}

pub(super) fn write_scene_object_create_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    create_data: &SceneObjectCreateData,
) {
    write_stationary_world_object_create_prefix_like_cpp(
        buf,
        guid,
        TypeId::SceneObject,
        create_data.position,
        true,
        None,
    );
    create_data.write_values_create(buf);
}

pub(super) fn write_conversation_create_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    create_data: &ConversationCreateData,
) {
    write_stationary_world_object_create_prefix_like_cpp(
        buf,
        guid,
        TypeId::Conversation,
        create_data.position,
        false,
        Some(create_data.texture_kit_id),
    );
    create_data.write_values_create(buf);
}

const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ABSOLUTE_ORIENTATION_LIKE_CPP: u32 = 0x00001;

const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_DYNAMIC_SHAPE_LIKE_CPP: u32 = 0x00002;

const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ATTACHED_LIKE_CPP: u32 = 0x00004;

const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_FACE_MOVEMENT_DIR_LIKE_CPP: u32 = 0x00008;

const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_FOLLOWS_TERRAIN_LIKE_CPP: u32 = 0x00010;

const AREATRIGGER_CREATE_PROPERTIES_FLAG_UNK1_LIKE_CPP: u32 = 0x00020;

const AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_TARGET_ROLL_PITCH_YAW_LIKE_CPP: u32 = 0x00040;

pub(super) fn write_area_trigger_create_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    create_data: &AreaTriggerCreateData,
) {
    buf.write_uint8(UpdateType::CreateObject as u8);
    buf.write_packed_guid(guid);
    buf.write_uint8(TypeId::AreaTrigger as u8);

    buf.write_bit(false); // NoBirthAnim
    buf.write_bit(false); // EnablePortals
    buf.write_bit(false); // PlayHoverAnim
    buf.write_bit(false); // MovementUpdate
    buf.write_bit(false); // MovementTransport
    buf.write_bit(true); // Stationary
    buf.write_bit(false); // CombatVictim
    buf.write_bit(false); // ServerTime
    buf.write_bit(false); // Vehicle
    buf.write_bit(false); // AnimKit
    buf.write_bit(false); // Rotation
    buf.write_bit(true); // AreaTrigger
    buf.write_bit(false); // GameObject
    buf.write_bit(false); // SmoothPhasing
    buf.write_bit(false); // ThisIsYou
    buf.write_bit(false); // SceneObject
    buf.write_bit(false); // ActivePlayer
    buf.write_bit(false); // Conversation
    buf.flush_bits();

    buf.write_int32(0); // PauseTimes count

    buf.write_float(create_data.position.x);
    buf.write_float(create_data.position.y);
    buf.write_float(create_data.position.z);
    buf.write_float(create_data.position.orientation);

    buf.write_uint32(create_data.time_since_created_ms);
    write_position_xyz_like_cpp(buf, create_data.roll_pitch_yaw);

    let flags = create_data.create_properties_flags;
    let has_absolute_orientation =
        flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ABSOLUTE_ORIENTATION_LIKE_CPP != 0;
    let has_dynamic_shape =
        flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_DYNAMIC_SHAPE_LIKE_CPP != 0;
    let has_attached = flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_ATTACHED_LIKE_CPP != 0;
    let has_face_movement_dir =
        flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_FACE_MOVEMENT_DIR_LIKE_CPP != 0;
    let has_follows_terrain =
        flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_FOLLOWS_TERRAIN_LIKE_CPP != 0;
    let has_unk1 = flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_UNK1_LIKE_CPP != 0;
    let has_target_roll_pitch_yaw =
        flags & AREATRIGGER_CREATE_PROPERTIES_FLAG_HAS_TARGET_ROLL_PITCH_YAW_LIKE_CPP != 0;
    let has_scale_curve_id = create_data.scale_curve_id != 0;
    let has_morph_curve_id = create_data.morph_curve_id != 0;
    let has_facing_curve_id = create_data.facing_curve_id != 0;
    let has_move_curve_id = create_data.move_curve_id != 0;
    let has_area_trigger_sphere = create_data.shape.shape_type == 0;
    let has_area_trigger_box = create_data.shape.shape_type == 1;
    let has_area_trigger_polygon = create_data.shape.shape_type == 3;
    let has_area_trigger_cylinder = create_data.shape.shape_type == 4;
    let has_disk = create_data.shape.shape_type == 5;
    let has_bounded_plane = create_data.shape.shape_type == 6;
    let has_area_trigger_spline = !create_data.spline_points.is_empty();
    let has_orbit = create_data.orbit.is_some();
    let has_movement_script = false;

    buf.write_bit(has_absolute_orientation);
    buf.write_bit(has_dynamic_shape);
    buf.write_bit(has_attached);
    buf.write_bit(has_face_movement_dir);
    buf.write_bit(has_follows_terrain);
    buf.write_bit(has_unk1);
    buf.write_bit(has_target_roll_pitch_yaw);
    buf.write_bit(has_scale_curve_id);
    buf.write_bit(has_morph_curve_id);
    buf.write_bit(has_facing_curve_id);
    buf.write_bit(has_move_curve_id);
    buf.write_bit(has_area_trigger_sphere);
    buf.write_bit(has_area_trigger_box);
    buf.write_bit(has_area_trigger_polygon);
    buf.write_bit(has_area_trigger_cylinder);
    buf.write_bit(has_disk);
    buf.write_bit(has_bounded_plane);
    buf.write_bit(has_area_trigger_spline);
    buf.write_bit(has_orbit);
    buf.write_bit(has_movement_script);
    buf.flush_bits();

    if has_area_trigger_spline {
        buf.write_uint32(create_data.time_to_target);
        buf.write_int32(0); // elapsed time for movement
        buf.write_bits(create_data.spline_points.len() as u32, 16);
        for point in &create_data.spline_points {
            buf.write_float(point.x);
            buf.write_float(point.y);
            buf.write_float(point.z);
        }
    }

    if has_target_roll_pitch_yaw {
        write_position_xyz_like_cpp(buf, create_data.target_roll_pitch_yaw);
    }
    if has_scale_curve_id {
        buf.write_uint32(create_data.scale_curve_id);
    }
    if has_morph_curve_id {
        buf.write_uint32(create_data.morph_curve_id);
    }
    if has_facing_curve_id {
        buf.write_uint32(create_data.facing_curve_id);
    }
    if has_move_curve_id {
        buf.write_uint32(create_data.move_curve_id);
    }

    let shape = &create_data.shape;
    if has_area_trigger_sphere {
        buf.write_float(shape.data[0]);
        buf.write_float(shape.data[1]);
    }
    if has_area_trigger_box {
        for index in 0..6 {
            buf.write_float(shape.data[index]);
        }
    }
    if has_area_trigger_polygon {
        buf.write_int32(shape.polygon_vertices.len() as i32);
        buf.write_int32(shape.polygon_vertices_target.len() as i32);
        buf.write_float(shape.data[0]);
        buf.write_float(shape.data[1]);
        for vertex in &shape.polygon_vertices {
            buf.write_float(vertex.x);
            buf.write_float(vertex.y);
        }
        for vertex in &shape.polygon_vertices_target {
            buf.write_float(vertex.x);
            buf.write_float(vertex.y);
        }
    }
    if has_area_trigger_cylinder {
        for index in 0..6 {
            buf.write_float(shape.data[index]);
        }
    }
    if has_disk {
        for index in 0..8 {
            buf.write_float(shape.data[index]);
        }
    }
    if has_bounded_plane {
        buf.write_float(shape.data[0]);
        buf.write_float(shape.data[1]);
        buf.write_float(shape.data[3]);
        buf.write_float(shape.data[4]);
    }

    if let Some(orbit) = create_data.orbit {
        buf.write_bit(false); // PathTarget
        buf.write_bit(true); // Center
        buf.write_bit(orbit.counter_clockwise);
        buf.write_bit(orbit.can_loop);
        buf.write_uint32(orbit.time_to_target);
        buf.write_int32(orbit.elapsed_time_for_movement);
        buf.write_uint32(orbit.start_delay);
        buf.write_float(orbit.radius);
        buf.write_float(orbit.blend_from_radius);
        buf.write_float(orbit.initial_angle);
        buf.write_float(orbit.z_offset);
        buf.write_float(orbit.center.x);
        buf.write_float(orbit.center.y);
        buf.write_float(orbit.center.z);
    }

    write_area_trigger_values_create(buf, create_data);
}

fn write_area_trigger_values_create(buf: &mut WorldPacket, data: &AreaTriggerCreateData) {
    let mut values = WorldPacket::new_empty();
    values.write_uint8(0x00); // UpdateFieldFlag

    values.write_int32(data.entry_id as i32);
    values.write_uint32(data.dynamic_flags);
    values.write_float(data.scale);

    write_scale_curve_values_create(&mut values, &data.override_scale_curve);
    values.write_packed_guid(&data.caster);
    values.write_uint32(data.duration);
    values.write_uint32(data.time_to_target);
    values.write_uint32(data.time_to_target_scale);
    values.write_uint32(data.time_to_target_extra_scale);
    values.write_uint32(data.time_to_target_pos);
    values.write_int32(data.spell_id);
    values.write_int32(data.spell_for_visuals);
    values.write_int32(data.spell_visual_id);
    values.write_float(data.bounds_radius_2d);
    values.write_uint32(data.decal_properties_id);
    values.write_packed_guid(&data.creating_effect_guid);
    values.write_packed_guid(&data.orbit_path_target);
    write_scale_curve_values_create(&mut values, &data.extra_scale_curve);
    write_scale_curve_values_create(&mut values, &data.override_move_curve_x);
    write_scale_curve_values_create(&mut values, &data.override_move_curve_y);
    write_scale_curve_values_create(&mut values, &data.override_move_curve_z);
    write_visual_anim_values_create(&mut values, &data.visual_anim);

    let data = values.into_data();
    buf.write_uint32(data.len() as u32);
    buf.write_bytes(&data);
}

pub(super) const VALUES_TYPE_GAME_OBJECT: u32 = 1 << 8;

pub(super) const VALUES_TYPE_DYNAMIC_OBJECT: u32 = 1 << 9;

pub(super) const VALUES_TYPE_CORPSE: u32 = 1 << 10;

pub(super) const VALUES_TYPE_AREA_TRIGGER: u32 = 1 << 11;

pub(super) const VALUES_TYPE_SCENE_OBJECT: u32 = 1 << 12;

pub(super) const VALUES_TYPE_CONVERSATION: u32 = 1 << 13;

pub(super) fn write_dynamic_object_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: DynamicObjectDataValuesUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(data.changed_object_type_mask);

    if data.changed_object_type_mask & VALUES_TYPE_OBJECT != 0 {
        if let Some(object_data) = data.object_data {
            write_object_data_values_update_section(&mut val_buf, object_data);
        } else {
            write_object_data_values_update_section(
                &mut val_buf,
                ObjectDataValuesUpdate {
                    changed_object_type_mask: VALUES_TYPE_OBJECT,
                    object_data_mask: 0,
                    entry_id: 0,
                    dynamic_flags: 0,
                    scale: 0.0,
                },
            );
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_DYNAMIC_OBJECT != 0 {
        let mask = data.dynamic_object_data_mask & 0x7F;
        val_buf.write_bits(mask, 7);
        val_buf.flush_bits();

        if mask & 0x01 != 0 {
            if mask & 0x02 != 0 {
                val_buf.write_packed_guid(&data.caster);
            }
            if mask & 0x04 != 0 {
                val_buf.write_uint8(data.dynamic_object_type);
            }
            if mask & 0x08 != 0 {
                val_buf.write_int32(data.spell_visual_id);
            }
            if mask & 0x10 != 0 {
                val_buf.write_int32(data.spell_id);
            }
            if mask & 0x20 != 0 {
                val_buf.write_float(data.radius);
            }
            if mask & 0x40 != 0 {
                val_buf.write_uint32(data.cast_time_ms);
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

pub(super) fn write_scene_object_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: SceneObjectDataValuesUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(data.changed_object_type_mask);

    if data.changed_object_type_mask & VALUES_TYPE_OBJECT != 0 {
        if let Some(object_data) = data.object_data {
            write_object_data_values_update_section(&mut val_buf, object_data);
        } else {
            write_object_data_values_update_section(
                &mut val_buf,
                ObjectDataValuesUpdate {
                    changed_object_type_mask: VALUES_TYPE_OBJECT,
                    object_data_mask: 0,
                    entry_id: 0,
                    dynamic_flags: 0,
                    scale: 0.0,
                },
            );
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_SCENE_OBJECT != 0 {
        let mask = data.scene_object_data_mask & 0x1F;
        val_buf.write_bits(mask, 5);
        val_buf.flush_bits();

        if mask & 0x01 != 0 {
            if mask & 0x02 != 0 {
                val_buf.write_int32(data.script_package_id);
            }
            if mask & 0x04 != 0 {
                val_buf.write_uint32(data.rnd_seed_val);
            }
            if mask & 0x08 != 0 {
                val_buf.write_packed_guid(&data.created_by);
            }
            if mask & 0x10 != 0 {
                val_buf.write_uint32(data.scene_type);
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

fn write_conversation_line_values_update(
    buf: &mut WorldPacket,
    line: &ConversationLineValuesUpdate,
) {
    buf.write_int32(line.conversation_line_id);
    buf.write_uint32(line.start_time);
    buf.write_int32(line.ui_camera_id);
    buf.write_uint8(line.actor_index);
    buf.write_uint8(line.flags);
}

fn write_conversation_actor_values_update(
    buf: &mut WorldPacket,
    actor: &ConversationActorValuesUpdate,
) {
    buf.write_bits(actor.actor_type & 1, 1);
    buf.write_int32(actor.id);

    if actor.actor_type == 1 {
        buf.write_uint32(actor.creature_id);
        buf.write_uint32(actor.creature_display_info_id);
    }

    if actor.actor_type == 0 {
        buf.write_packed_guid(&actor.actor_guid);
    }

    buf.flush_bits();
}

pub(super) fn write_conversation_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &ConversationDataValuesUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(data.changed_object_type_mask);

    if data.changed_object_type_mask & VALUES_TYPE_OBJECT != 0 {
        if let Some(object_data) = data.object_data {
            write_object_data_values_update_section(&mut val_buf, object_data);
        } else {
            write_object_data_values_update_section(
                &mut val_buf,
                ObjectDataValuesUpdate {
                    changed_object_type_mask: VALUES_TYPE_OBJECT,
                    object_data_mask: 0,
                    entry_id: 0,
                    dynamic_flags: 0,
                    scale: 0.0,
                },
            );
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_CONVERSATION != 0 {
        let mask = data.conversation_data_mask & 0x0F;
        val_buf.write_bits(mask, 4);

        if mask & 0x01 != 0 {
            if mask & 0x02 != 0 {
                val_buf.write_bits(data.lines.len() as u32, 32);
                for line in &data.lines {
                    write_conversation_line_values_update(&mut val_buf, line);
                }
            }
        }
        val_buf.flush_bits();

        if mask & 0x01 != 0 {
            if mask & 0x04 != 0 {
                write_dynamic_field_update_mask(
                    &mut val_buf,
                    data.actors.len(),
                    data.actor_update_mask.as_deref(),
                );
            }
        }
        val_buf.flush_bits();

        if mask & 0x01 != 0 {
            if mask & 0x04 != 0 {
                for (index, actor) in data.actors.iter().enumerate() {
                    if dynamic_mask_has_index(data.actor_update_mask.as_deref(), index) {
                        write_conversation_actor_values_update(&mut val_buf, actor);
                    }
                }
            }
            if mask & 0x08 != 0 {
                val_buf.write_int32(data.last_line_end_time);
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

pub(super) fn write_game_object_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &GameObjectDataValuesUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(data.changed_object_type_mask);

    if data.changed_object_type_mask & VALUES_TYPE_OBJECT != 0 {
        if let Some(object_data) = data.object_data {
            write_object_data_values_update_section(&mut val_buf, object_data);
        } else {
            write_object_data_values_update_section(
                &mut val_buf,
                ObjectDataValuesUpdate {
                    changed_object_type_mask: VALUES_TYPE_OBJECT,
                    object_data_mask: 0,
                    entry_id: 0,
                    dynamic_flags: 0,
                    scale: 0.0,
                },
            );
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_GAME_OBJECT != 0 {
        let mask = data.game_object_data_mask & 0x000F_FFFF;
        val_buf.write_bits(mask, 20);

        if mask & 0x0000_0001 != 0 && mask & 0x0000_0002 != 0 {
            val_buf.write_bits(data.state_world_effect_ids.len() as u32, 32);
            for effect_id in &data.state_world_effect_ids {
                val_buf.write_uint32(*effect_id);
            }
        }
        val_buf.flush_bits();

        if mask & 0x0000_0001 != 0 {
            if mask & 0x0000_0004 != 0 {
                write_dynamic_field_update_mask(
                    &mut val_buf,
                    data.enable_doodad_sets.len(),
                    data.enable_doodad_sets_update_mask.as_deref(),
                );
            }
            if mask & 0x0000_0008 != 0 {
                write_dynamic_field_update_mask(
                    &mut val_buf,
                    data.world_effects.len(),
                    data.world_effects_update_mask.as_deref(),
                );
            }
        }
        val_buf.flush_bits();

        if mask & 0x0000_0001 != 0 {
            if mask & 0x0000_0004 != 0 {
                write_changed_i32_dynamic_values(
                    &mut val_buf,
                    &data.enable_doodad_sets,
                    data.enable_doodad_sets_update_mask.as_deref(),
                );
            }
            if mask & 0x0000_0008 != 0 {
                write_changed_i32_dynamic_values(
                    &mut val_buf,
                    &data.world_effects,
                    data.world_effects_update_mask.as_deref(),
                );
            }
            if mask & 0x0000_0010 != 0 {
                val_buf.write_int32(data.display_id);
            }
            if mask & 0x0000_0020 != 0 {
                val_buf.write_uint32(data.spell_visual_id);
            }
            if mask & 0x0000_0040 != 0 {
                val_buf.write_uint32(data.state_spell_visual_id);
            }
            if mask & 0x0000_0080 != 0 {
                val_buf.write_uint32(data.spawn_tracking_state_anim_id);
            }
            if mask & 0x0000_0100 != 0 {
                val_buf.write_uint32(data.spawn_tracking_state_anim_kit_id);
            }
            if mask & 0x0000_0200 != 0 {
                val_buf.write_packed_guid(&data.created_by);
            }
            if mask & 0x0000_0400 != 0 {
                val_buf.write_packed_guid(&data.guild_guid);
            }
            if mask & 0x0000_0800 != 0 {
                val_buf.write_uint32(data.flags);
            }
            if mask & 0x0000_1000 != 0 {
                for component in data.parent_rotation {
                    val_buf.write_float(component);
                }
            }
            if mask & 0x0000_2000 != 0 {
                val_buf.write_int32(data.faction_template);
            }
            if mask & 0x0000_4000 != 0 {
                val_buf.write_int32(data.level);
            }
            if mask & 0x0000_8000 != 0 {
                val_buf.write_int8(data.state);
            }
            if mask & 0x0001_0000 != 0 {
                val_buf.write_int8(data.type_id);
            }
            if mask & 0x0002_0000 != 0 {
                val_buf.write_uint8(data.percent_health);
            }
            if mask & 0x0004_0000 != 0 {
                val_buf.write_uint32(data.art_kit);
            }
            if mask & 0x0008_0000 != 0 {
                val_buf.write_uint32(data.custom_param);
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

pub(super) fn write_corpse_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &CorpseDataValuesUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(data.changed_object_type_mask);

    if data.changed_object_type_mask & VALUES_TYPE_OBJECT != 0 {
        if let Some(object_data) = data.object_data {
            write_object_data_values_update_section(&mut val_buf, object_data);
        } else {
            write_object_data_values_update_section(
                &mut val_buf,
                ObjectDataValuesUpdate {
                    changed_object_type_mask: VALUES_TYPE_OBJECT,
                    object_data_mask: 0,
                    entry_id: 0,
                    dynamic_flags: 0,
                    scale: 0.0,
                },
            );
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_CORPSE != 0 {
        let mask = data.corpse_data_mask;
        val_buf.write_bits(mask, 32);

        if mask & 0x0000_0001 != 0 && mask & 0x0000_0002 != 0 {
            write_dynamic_field_update_mask(
                &mut val_buf,
                data.customizations.len(),
                data.customizations_update_mask.as_deref(),
            );
        }
        val_buf.flush_bits();

        if mask & 0x0000_0001 != 0 {
            if mask & 0x0000_0002 != 0 {
                for (index, customization) in data.customizations.iter().enumerate() {
                    if dynamic_mask_has_index(data.customizations_update_mask.as_deref(), index) {
                        write_chr_customization_choice_values_update(&mut val_buf, customization);
                    }
                }
            }
            if mask & 0x0000_0004 != 0 {
                val_buf.write_uint32(data.dynamic_flags);
            }
            if mask & 0x0000_0008 != 0 {
                val_buf.write_packed_guid(&data.owner);
            }
            if mask & 0x0000_0010 != 0 {
                val_buf.write_packed_guid(&data.party_guid);
            }
            if mask & 0x0000_0020 != 0 {
                val_buf.write_packed_guid(&data.guild_guid);
            }
            if mask & 0x0000_0040 != 0 {
                val_buf.write_uint32(data.display_id);
            }
            if mask & 0x0000_0080 != 0 {
                val_buf.write_uint8(data.race_id);
            }
            if mask & 0x0000_0100 != 0 {
                val_buf.write_uint8(data.sex);
            }
            if mask & 0x0000_0200 != 0 {
                val_buf.write_uint8(data.class);
            }
            if mask & 0x0000_0400 != 0 {
                val_buf.write_uint32(data.flags);
            }
            if mask & 0x0000_0800 != 0 {
                val_buf.write_int32(data.faction_template);
            }
        }

        if mask & 0x0000_1000 != 0 {
            for (index, item) in data.items.iter().enumerate() {
                if mask & (1 << (13 + index)) != 0 {
                    val_buf.write_uint32(*item);
                }
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}

pub(super) fn write_area_trigger_values_update_block(
    buf: &mut WorldPacket,
    guid: &ObjectGuid,
    data: &AreaTriggerDataValuesUpdate,
) {
    buf.write_uint8(UpdateType::Values as u8);
    buf.write_packed_guid(guid);

    let mut val_buf = WorldPacket::new_empty();
    val_buf.write_uint32(data.changed_object_type_mask);

    if data.changed_object_type_mask & VALUES_TYPE_OBJECT != 0 {
        if let Some(object_data) = data.object_data {
            write_object_data_values_update_section(&mut val_buf, object_data);
        } else {
            write_object_data_values_update_section(
                &mut val_buf,
                ObjectDataValuesUpdate {
                    changed_object_type_mask: VALUES_TYPE_OBJECT,
                    object_data_mask: 0,
                    entry_id: 0,
                    dynamic_flags: 0,
                    scale: 0.0,
                },
            );
        }
    }

    if data.changed_object_type_mask & VALUES_TYPE_AREA_TRIGGER != 0 {
        let mask = data.area_trigger_data_mask & 0x000F_FFFF;
        val_buf.write_bits(mask, 20);
        val_buf.flush_bits();

        if mask & 0x0000_0001 != 0 {
            if mask & 0x0000_0002 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.override_scale_curve);
            }
            if mask & 0x0000_0040 != 0 {
                val_buf.write_packed_guid(&data.caster);
            }
            if mask & 0x0000_0080 != 0 {
                val_buf.write_uint32(data.duration);
            }
            if mask & 0x0000_0100 != 0 {
                val_buf.write_uint32(data.time_to_target);
            }
            if mask & 0x0000_0200 != 0 {
                val_buf.write_uint32(data.time_to_target_scale);
            }
            if mask & 0x0000_0400 != 0 {
                val_buf.write_uint32(data.time_to_target_extra_scale);
            }
            if mask & 0x0000_0800 != 0 {
                val_buf.write_uint32(data.time_to_target_pos);
            }
            if mask & 0x0000_1000 != 0 {
                val_buf.write_int32(data.spell_id);
            }
            if mask & 0x0000_2000 != 0 {
                val_buf.write_int32(data.spell_for_visuals);
            }
            if mask & 0x0000_4000 != 0 {
                val_buf.write_int32(data.spell_visual_id);
            }
            if mask & 0x0000_8000 != 0 {
                val_buf.write_float(data.bounds_radius_2d);
            }
            if mask & 0x0001_0000 != 0 {
                val_buf.write_uint32(data.decal_properties_id);
            }
            if mask & 0x0002_0000 != 0 {
                val_buf.write_packed_guid(&data.creating_effect_guid);
            }
            if mask & 0x0004_0000 != 0 {
                val_buf.write_packed_guid(&data.orbit_path_target);
            }
            if mask & 0x0000_0004 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.extra_scale_curve);
            }
            if mask & 0x0000_0008 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.override_move_curve_x);
            }
            if mask & 0x0000_0010 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.override_move_curve_y);
            }
            if mask & 0x0000_0020 != 0 {
                write_scale_curve_values_update(&mut val_buf, &data.override_move_curve_z);
            }
            if mask & 0x0008_0000 != 0 {
                write_visual_anim_values_update(&mut val_buf, &data.visual_anim);
            }
        }
    }

    let val_data = val_buf.into_data();
    buf.write_uint32(val_data.len() as u32);
    buf.write_bytes(&val_data);
}
