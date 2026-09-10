//! Conversation packets.
//!
//! Separated from game_object.rs under #689.

use super::*;

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

pub(in crate::packets::update) fn write_conversation_create_block(
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

pub(in crate::packets::update) const VALUES_TYPE_CONVERSATION: u32 = 1 << 13;

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

pub(in crate::packets::update) fn write_conversation_values_update_block(
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
