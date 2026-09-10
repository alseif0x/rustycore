//! Corpse packets.
//!
//! Separated from game_object.rs under #689.

use super::*;

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

pub(in crate::packets::update) fn write_corpse_create_block(
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

pub(in crate::packets::update) const VALUES_TYPE_CORPSE: u32 = 1 << 10;

pub(in crate::packets::update) fn write_corpse_values_update_block(
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
