//! Character customization DB2 entries and stores.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChrCustomizationChoiceEntry {
    pub id: u32,
    pub name: String,
    pub chr_customization_option_id: u32,
    pub chr_customization_req_id: i32,
    pub chr_customization_vis_req_id: i32,
    pub sort_order: u16,
    pub ui_order_index: u16,
    pub flags: i32,
    pub added_in_patch: i32,
    pub sound_kit_id: i32,
    pub swatch_color: [i32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChrCustomizationDisplayInfoEntry {
    pub id: u32,
    pub shapeshift_form_id: i32,
    pub display_id: i32,
    pub barber_shop_min_camera_distance: f32,
    pub barber_shop_height_offset: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChrCustomizationElementEntry {
    pub id: u32,
    pub chr_customization_choice_id: i32,
    pub related_chr_customization_choice_id: i32,
    pub chr_customization_geoset_id: i32,
    pub chr_customization_skinned_model_id: i32,
    pub chr_customization_material_id: i32,
    pub chr_customization_bone_set_id: i32,
    pub chr_customization_cond_model_id: i32,
    pub chr_customization_display_info_id: i32,
    pub chr_cust_item_geo_modify_id: i32,
    pub chr_customization_voice_id: i32,
    pub anim_kit_id: i32,
    pub particle_color_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChrCustomizationOptionEntry {
    pub id: u32,
    pub name: String,
    pub secondary_id: u16,
    pub flags: i32,
    pub chr_model_id: u32,
    pub sort_index: i32,
    pub chr_customization_category_id: i32,
    pub option_type: i32,
    pub barber_shop_cost_modifier: f32,
    pub chr_customization_id: i32,
    pub chr_customization_req_id: i32,
    pub ui_order_index: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChrCustomizationReqEntry {
    pub id: u32,
    pub race_mask: i64,
    pub req_source: String,
    pub flags: i32,
    pub class_mask: i32,
    pub achievement_id: i32,
    pub quest_id: i32,
    pub override_archive: i32,
    pub item_modified_appearance_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChrCustomizationReqChoiceEntry {
    pub id: u32,
    pub chr_customization_choice_id: i32,
    pub chr_customization_req_id: u32,
}

db2_store!(ChrCustomizationChoiceStore, ChrCustomizationChoiceEntry);
db2_store!(
    ChrCustomizationDisplayInfoStore,
    ChrCustomizationDisplayInfoEntry
);
db2_store!(ChrCustomizationElementStore, ChrCustomizationElementEntry);
db2_store!(ChrCustomizationOptionStore, ChrCustomizationOptionEntry);
db2_store!(ChrCustomizationReqStore, ChrCustomizationReqEntry);
db2_store!(
    ChrCustomizationReqChoiceStore,
    ChrCustomizationReqChoiceEntry
);

impl ChrCustomizationChoiceStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ChrCustomizationChoice.db2",
            |id, idx, r| ChrCustomizationChoiceEntry {
                id,
                name: r.get_field_string(idx, 0),
                chr_customization_option_id: r.get_relationship_id(idx).unwrap_or(0),
                chr_customization_req_id: r.get_field_i32(idx, 3),
                chr_customization_vis_req_id: r.get_field_i32(idx, 4),
                sort_order: r.get_field_u16(idx, 5),
                ui_order_index: r.get_field_u16(idx, 6),
                flags: r.get_field_i32(idx, 7),
                added_in_patch: r.get_field_i32(idx, 8),
                sound_kit_id: r.get_field_i32(idx, 9),
                swatch_color: std::array::from_fn(|i| r.get_array_element(idx, 10, i, 32) as i32),
            },
        )
    }
}

impl ChrCustomizationDisplayInfoStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ChrCustomizationDisplayInfo.db2",
            |id, idx, r| ChrCustomizationDisplayInfoEntry {
                id,
                shapeshift_form_id: r.get_field_i32(idx, 1),
                display_id: r.get_field_i32(idx, 2),
                barber_shop_min_camera_distance: f32_field(r, idx, 3),
                barber_shop_height_offset: f32_field(r, idx, 4),
            },
        )
    }
}

impl ChrCustomizationElementStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ChrCustomizationElement.db2",
            |id, idx, r| ChrCustomizationElementEntry {
                id,
                chr_customization_choice_id: r.get_field_i32(idx, 1),
                related_chr_customization_choice_id: r.get_field_i32(idx, 2),
                chr_customization_geoset_id: r.get_field_i32(idx, 3),
                chr_customization_skinned_model_id: r.get_field_i32(idx, 4),
                chr_customization_material_id: r.get_field_i32(idx, 5),
                chr_customization_bone_set_id: r.get_field_i32(idx, 6),
                chr_customization_cond_model_id: r.get_field_i32(idx, 7),
                chr_customization_display_info_id: r.get_field_i32(idx, 8),
                chr_cust_item_geo_modify_id: r.get_field_i32(idx, 9),
                chr_customization_voice_id: r.get_field_i32(idx, 10),
                anim_kit_id: r.get_field_i32(idx, 11),
                particle_color_id: r.get_field_i32(idx, 12),
            },
        )
    }
}

impl ChrCustomizationOptionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ChrCustomizationOption.db2",
            |id, idx, r| ChrCustomizationOptionEntry {
                id,
                name: r.get_field_string(idx, 0),
                secondary_id: r.get_field_u16(idx, 2),
                flags: r.get_field_i32(idx, 3),
                chr_model_id: r.get_relationship_id(idx).unwrap_or(0),
                sort_index: r.get_field_i32(idx, 5),
                chr_customization_category_id: r.get_field_i32(idx, 6),
                option_type: r.get_field_i32(idx, 7),
                barber_shop_cost_modifier: f32_field(r, idx, 8),
                chr_customization_id: r.get_field_i32(idx, 9),
                chr_customization_req_id: r.get_field_i32(idx, 10),
                ui_order_index: r.get_field_i32(idx, 11),
            },
        )
    }
}

impl ChrCustomizationReqStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ChrCustomizationReq.db2", |id, idx, r| {
            ChrCustomizationReqEntry {
                id,
                race_mask: r.get_field_i64(idx, 0),
                req_source: r.get_field_string(idx, 1),
                flags: r.get_field_i32(idx, 3),
                class_mask: r.get_field_i32(idx, 4),
                achievement_id: r.get_field_i32(idx, 5),
                quest_id: r.get_field_i32(idx, 6),
                override_archive: r.get_field_i32(idx, 7),
                item_modified_appearance_id: r.get_field_i32(idx, 8),
            }
        })
    }
}

impl ChrCustomizationReqChoiceStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ChrCustomizationReqChoice.db2",
            |id, idx, r| ChrCustomizationReqChoiceEntry {
                id,
                chr_customization_choice_id: r.get_field_i32(idx, 0),
                chr_customization_req_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

impl_from_entries!(ChrCustomizationChoiceStore, ChrCustomizationChoiceEntry);
impl_from_entries!(
    ChrCustomizationDisplayInfoStore,
    ChrCustomizationDisplayInfoEntry
);
impl_from_entries!(ChrCustomizationElementStore, ChrCustomizationElementEntry);
impl_from_entries!(ChrCustomizationOptionStore, ChrCustomizationOptionEntry);
impl_from_entries!(ChrCustomizationReqStore, ChrCustomizationReqEntry);
impl_from_entries!(
    ChrCustomizationReqChoiceStore,
    ChrCustomizationReqChoiceEntry
);
