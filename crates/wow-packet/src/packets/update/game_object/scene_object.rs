//! Scene object packets.
//!
//! Separated from game_object.rs under #689.

use super::*;

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

pub(in crate::packets::update) fn write_scene_object_create_block(
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

pub(in crate::packets::update) const VALUES_TYPE_SCENE_OBJECT: u32 = 1 << 12;

pub(in crate::packets::update) fn write_scene_object_values_update_block(
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
