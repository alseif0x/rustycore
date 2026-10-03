// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

pub fn represented_equipment_set_from_packet_like_cpp(
    set: wow_packet::packets::misc::EquipmentSetDataLikeCpp,
    guid: u64,
    state: wow_entities::PlayerEquipmentSetUpdateStateLikeCpp,
) -> Option<wow_entities::PlayerEquipmentSetLikeCpp> {
    let set_type =
        wow_entities::PlayerEquipmentSetTypeLikeCpp::handler_branch_from_i32_like_cpp(set.set_type)?;
    Some(wow_entities::PlayerEquipmentSetLikeCpp {
        raw_set_type: set.set_type,
        set_type,
        guid,
        set_id: set.set_id,
        ignore_mask: set.ignore_mask,
        pieces: set.pieces,
        appearances: set.appearances,
        enchants: set.enchants,
        secondary_shoulder_appearance_id: set.secondary_shoulder_appearance_id,
        secondary_shoulder_slot: set.secondary_shoulder_slot,
        secondary_weapon_appearance_id: set.secondary_weapon_appearance_id,
        secondary_weapon_slot: set.secondary_weapon_slot,
        assigned_spec_index: set.assigned_spec_index,
        set_name: set.set_name,
        set_icon: set.set_icon,
        state,
    })
}
