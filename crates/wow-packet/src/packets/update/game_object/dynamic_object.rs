//! Dynamic object packets.
//!
//! Separated from game_object.rs under #689.

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

/// Write a single CreateObject block for a dynamic object (TypeId::DynamicObject).
///
/// DynamicObjects use Stationary (bit 5), no MovementUpdate, no Unit shared-vision payload.
pub(in crate::packets::update) fn write_dynamic_object_create_block(
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

pub(in crate::packets::update) const VALUES_TYPE_DYNAMIC_OBJECT: u32 = 1 << 9;

pub(in crate::packets::update) fn write_dynamic_object_values_update_block(
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
