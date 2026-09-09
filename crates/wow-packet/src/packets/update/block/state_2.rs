//! Update-object block builder state definitions, part 2 of 2.
//!
//! Separated from the block.rs root under #650. Behaviour is preserved.

#[allow(unused_imports)]
use super::super::*;
use super::*;

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
