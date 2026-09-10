//! Game object packets.
//!
//! Separated from game_object.rs under #689.

use super::*;

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

pub(in crate::packets::update) fn debug_gameobject_create_values_len_like_cpp(
    data: &GameObjectCreateData,
) -> usize {
    let mut values = WorldPacket::new_empty();
    data.write_values_create(&mut values);
    values.into_data().len()
}

/// Write a single CreateObject block for a gameobject (TypeId::GameObject).
///
/// GameObjects use Stationary (bit 5) + Rotation (bit 10).
///
/// C++ only sets `CreateObjectBits::GameObject` when a GO addon/template has
/// `WorldEffectID`; ordinary GameObjects must not write that extra payload.
/// No MovementUpdate block.
pub(in crate::packets::update) fn write_gameobject_create_block(
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
pub(in crate::packets::update) fn write_transport_create_block(
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

pub(in crate::packets::update) const VALUES_TYPE_GAME_OBJECT: u32 = 1 << 8;

pub(in crate::packets::update) fn write_game_object_values_update_block(
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
