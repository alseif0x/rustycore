//! Character, class, race, customization and power DB2 readers.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

#[derive(Debug, Clone, PartialEq)]
pub struct BarberShopStyleEntry {
    pub id: u32,
    pub display_name: String,
    pub description: String,
    pub style_type: u8,
    pub cost_modifier: f32,
    pub race: u8,
    pub sex: u8,
    pub data: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterLoadoutEntry {
    pub id: u32,
    pub race_mask: i64,
    pub chr_class_id: i8,
    pub purpose: i32,
    pub item_context: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterLoadoutItemEntry {
    pub id: u32,
    pub character_loadout_id: u32,
    pub item_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChrClassUiDisplayEntry {
    pub id: u32,
    pub chr_classes_id: u8,
    pub adv_guide_player_condition_id: u32,
    pub splash_player_condition_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChrClassesEntry {
    pub id: u32,
    pub name: String,
    pub filename: String,
    pub name_male: String,
    pub name_female: String,
    pub pet_name_token: String,
    pub create_screen_file_data_id: u32,
    pub select_screen_file_data_id: u32,
    pub icon_file_data_id: u32,
    pub low_res_screen_file_data_id: u32,
    pub flags: i32,
    pub starting_level: i32,
    pub armor_type_mask: u32,
    pub cinematic_sequence_id: u16,
    pub default_spec: u16,
    pub has_strength_attack_bonus: u8,
    pub primary_stat_priority: u8,
    pub display_power: u8,
    pub ranged_attack_power_per_agility: u8,
    pub attack_power_per_agility: u8,
    pub attack_power_per_strength: u8,
    pub spell_class_set: u8,
    pub roles_mask: u8,
    pub damage_bonus_stat: u8,
    pub has_relic_slot: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChrClassesXPowerTypesEntry {
    pub id: u32,
    pub power_type: i8,
    pub class_id: u32,
}

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

#[derive(Debug, Clone, PartialEq)]
pub struct ChrModelEntry {
    pub id: u32,
    pub face_customization_offset: [f32; 3],
    pub customize_offset: [f32; 3],
    pub sex: i8,
    pub display_id: i32,
    pub char_component_texture_layout_id: i32,
    pub flags: i32,
    pub skeleton_file_data_id: i32,
    pub model_fallback_chr_model_id: i32,
    pub texture_fallback_chr_model_id: i32,
    pub helm_vis_fallback_chr_model_id: i32,
    pub customize_scale: f32,
    pub customize_facing: f32,
    pub camera_distance_offset: f32,
    pub barber_shop_camera_offset_scale: f32,
    pub barber_shop_camera_height_offset_scale: f32,
    pub barber_shop_camera_rotation_offset: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChrRaceXChrModelEntry {
    pub id: u32,
    pub chr_races_id: u32,
    pub chr_model_id: i32,
    pub sex: i32,
    pub allowed_transmog_slots: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChrRacesEntry {
    pub id: u32,
    pub client_prefix: String,
    pub client_file_string: String,
    pub name: String,
    pub flags: i32,
    pub male_display_id: u32,
    pub female_display_id: u32,
    pub high_res_male_display_id: u32,
    pub high_res_female_display_id: u32,
    pub res_sickness_spell_id: i32,
    pub splash_sound_id: i32,
    pub create_screen_file_data_id: i32,
    pub select_screen_file_data_id: i32,
    pub low_res_screen_file_data_id: i32,
    pub altered_form_start_visual_kit_id: [u32; 3],
    pub altered_form_finish_visual_kit_id: [u32; 3],
    pub heritage_armor_achievement_id: i32,
    pub starting_level: i32,
    pub ui_display_order: i32,
    pub playable_race_bit: i32,
    pub female_skeleton_file_data_id: i32,
    pub male_skeleton_file_data_id: i32,
    pub helmet_anim_scaling_race_id: i32,
    pub transmogrify_disabled_slot_mask: i32,
    pub faction_id: i16,
    pub cinematic_sequence_id: i16,
    pub base_language: i8,
    pub creature_type: i8,
    pub alliance: i8,
    pub race_related: i8,
    pub unaltered_visual_race_id: i8,
    pub default_class_id: i8,
    pub neutral_race_id: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameGenEntry {
    pub id: u32,
    pub name: String,
    pub race_id: u8,
    pub sex: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerDisplayEntry {
    pub id: u32,
    pub global_string_base_tag: String,
    pub actual_type: u8,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PowerTypeEntry {
    pub id: u32,
    pub name_global_string_tag: String,
    pub cost_global_string_tag: String,
    pub power_type_enum: i8,
    pub min_power: i32,
    pub max_base_power: i32,
    pub center_power: i32,
    pub default_power: i32,
    pub display_modifier: i32,
    pub regen_interrupt_time_ms: i32,
    pub regen_peace: f32,
    pub regen_combat: f32,
    pub flags: i16,
}

/// C++ `Creature::UpdateLevelDependantStats` power seed after
/// `SetCreateMana`, `SetStatPctModifier`, `SetPowerType`, and the
/// `PowerTypeEntry` default/full-power branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureInitialPowerLikeCpp {
    pub max_power: i32,
    pub power: i32,
}

macro_rules! db2_store {
    ($store:ident, $entry:ty) => {
        pub struct $store {
            entries: HashMap<u32, $entry>,
        }

        impl $store {
            pub fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self {
                    entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
                }
            }

            pub fn get(&self, id: u32) -> Option<&$entry> {
                self.entries.get(&id)
            }

            pub fn len(&self) -> usize {
                self.entries.len()
            }

            pub fn is_empty(&self) -> bool {
                self.entries.is_empty()
            }
        }
    };
}

db2_store!(BarberShopStyleStore, BarberShopStyleEntry);
db2_store!(CharacterLoadoutStore, CharacterLoadoutEntry);
db2_store!(CharacterLoadoutItemStore, CharacterLoadoutItemEntry);
db2_store!(ChrClassUiDisplayStore, ChrClassUiDisplayEntry);
db2_store!(ChrClassesStore, ChrClassesEntry);
db2_store!(ChrClassesXPowerTypesStore, ChrClassesXPowerTypesEntry);
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
db2_store!(ChrModelStore, ChrModelEntry);
db2_store!(ChrRaceXChrModelStore, ChrRaceXChrModelEntry);
db2_store!(ChrRacesStore, ChrRacesEntry);
db2_store!(NameGenStore, NameGenEntry);
db2_store!(PowerDisplayStore, PowerDisplayEntry);
db2_store!(PowerTypeStore, PowerTypeEntry);

impl BarberShopStyleStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "BarberShopStyle.db2", |id, idx, r| {
            BarberShopStyleEntry {
                id,
                display_name: r.get_field_string(idx, 0),
                description: r.get_field_string(idx, 1),
                style_type: r.get_field_u8(idx, 3),
                cost_modifier: f32_field(r, idx, 4),
                race: r.get_field_u8(idx, 5),
                sex: r.get_field_u8(idx, 6),
                data: r.get_field_u8(idx, 7),
            }
        })
    }
}

impl CharacterLoadoutStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "CharacterLoadout.db2", |id, idx, r| {
            CharacterLoadoutEntry {
                id,
                race_mask: r.get_field_i64(idx, 0),
                chr_class_id: r.get_field_i8(idx, 2),
                purpose: r.get_field_i32(idx, 3),
                item_context: r.get_field_i8(idx, 4),
            }
        })
    }
}

impl CharacterLoadoutItemStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "CharacterLoadoutItem.db2",
            |id, idx, r| CharacterLoadoutItemEntry {
                id,
                character_loadout_id: r.get_relationship_id(idx).unwrap_or(0),
                item_id: r.get_field_u32(idx, 1),
            },
        )
    }
}

impl ChrClassUiDisplayStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ChrClassUIDisplay.db2", |id, idx, r| {
            ChrClassUiDisplayEntry {
                id,
                chr_classes_id: r.get_field_u8(idx, 0),
                adv_guide_player_condition_id: r.get_field_u32(idx, 1),
                splash_player_condition_id: r.get_field_u32(idx, 2),
            }
        })
    }
}

impl ChrClassesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ChrClasses.db2", |id, idx, r| {
            ChrClassesEntry {
                id,
                name: r.get_field_string(idx, 0),
                filename: r.get_field_string(idx, 1),
                name_male: r.get_field_string(idx, 2),
                name_female: r.get_field_string(idx, 3),
                pet_name_token: r.get_field_string(idx, 4),
                create_screen_file_data_id: r.get_field_u32(idx, 6),
                select_screen_file_data_id: r.get_field_u32(idx, 7),
                icon_file_data_id: r.get_field_u32(idx, 8),
                low_res_screen_file_data_id: r.get_field_u32(idx, 9),
                flags: r.get_field_i32(idx, 10),
                starting_level: r.get_field_i32(idx, 11),
                armor_type_mask: r.get_field_u32(idx, 12),
                cinematic_sequence_id: r.get_field_u16(idx, 13),
                default_spec: r.get_field_u16(idx, 14),
                has_strength_attack_bonus: r.get_field_u8(idx, 15),
                primary_stat_priority: r.get_field_u8(idx, 16),
                display_power: r.get_field_u8(idx, 17),
                ranged_attack_power_per_agility: r.get_field_u8(idx, 18),
                attack_power_per_agility: r.get_field_u8(idx, 19),
                attack_power_per_strength: r.get_field_u8(idx, 20),
                spell_class_set: r.get_field_u8(idx, 21),
                roles_mask: r.get_field_u8(idx, 22),
                damage_bonus_stat: r.get_field_u8(idx, 23),
                has_relic_slot: r.get_field_u8(idx, 24),
            }
        })
    }
}

impl ChrClassesXPowerTypesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "ChrClassesXPowerTypes.db2",
            |id, idx, r| ChrClassesXPowerTypesEntry {
                id,
                power_type: r.get_field_i8(idx, 0),
                class_id: r.get_relationship_id(idx).unwrap_or(0),
            },
        )
    }
}

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

impl ChrModelStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ChrModel.db2", |id, idx, r| {
            ChrModelEntry {
                id,
                face_customization_offset: f32_array::<3>(r, idx, 0),
                customize_offset: f32_array::<3>(r, idx, 1),
                sex: r.get_field_i8(idx, 3),
                display_id: r.get_field_i32(idx, 4),
                char_component_texture_layout_id: r.get_field_i32(idx, 5),
                flags: r.get_field_i32(idx, 6),
                skeleton_file_data_id: r.get_field_i32(idx, 7),
                model_fallback_chr_model_id: r.get_relationship_id(idx).unwrap_or(0) as i32,
                texture_fallback_chr_model_id: r.get_field_i32(idx, 9),
                helm_vis_fallback_chr_model_id: r.get_field_i32(idx, 10),
                customize_scale: f32_field(r, idx, 11),
                customize_facing: f32_field(r, idx, 12),
                camera_distance_offset: f32_field(r, idx, 13),
                barber_shop_camera_offset_scale: f32_field(r, idx, 14),
                barber_shop_camera_height_offset_scale: f32_field(r, idx, 15),
                barber_shop_camera_rotation_offset: f32_field(r, idx, 16),
            }
        })
    }
}

impl ChrRaceXChrModelStore {
    pub fn entries(&self) -> impl Iterator<Item = &ChrRaceXChrModelEntry> {
        self.entries.values()
    }

    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ChrRaceXChrModel.db2", |id, idx, r| {
            ChrRaceXChrModelEntry {
                id,
                chr_races_id: r.get_relationship_id(idx).unwrap_or(0),
                chr_model_id: r.get_field_i32(idx, 1),
                sex: r.get_field_i32(idx, 2),
                allowed_transmog_slots: r.get_field_i32(idx, 3),
            }
        })
    }
}

impl ChrRacesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "ChrRaces.db2", |id, idx, r| {
            ChrRacesEntry {
                id,
                client_prefix: r.get_field_string(idx, 0),
                client_file_string: r.get_field_string(idx, 1),
                name: r.get_field_string(idx, 2),
                flags: r.get_field_i32(idx, 15),
                male_display_id: r.get_field_u32(idx, 16),
                female_display_id: r.get_field_u32(idx, 17),
                high_res_male_display_id: r.get_field_u32(idx, 18),
                high_res_female_display_id: r.get_field_u32(idx, 19),
                res_sickness_spell_id: r.get_field_i32(idx, 20),
                splash_sound_id: r.get_field_i32(idx, 21),
                create_screen_file_data_id: r.get_field_i32(idx, 22),
                select_screen_file_data_id: r.get_field_i32(idx, 23),
                low_res_screen_file_data_id: r.get_field_i32(idx, 24),
                altered_form_start_visual_kit_id: std::array::from_fn(|i| {
                    r.get_array_element(idx, 25, i, 32)
                }),
                altered_form_finish_visual_kit_id: std::array::from_fn(|i| {
                    r.get_array_element(idx, 26, i, 32)
                }),
                heritage_armor_achievement_id: r.get_field_i32(idx, 27),
                starting_level: r.get_field_i32(idx, 28),
                ui_display_order: r.get_field_i32(idx, 29),
                playable_race_bit: r.get_field_i32(idx, 30),
                female_skeleton_file_data_id: r.get_field_i32(idx, 31),
                male_skeleton_file_data_id: r.get_field_i32(idx, 32),
                helmet_anim_scaling_race_id: r.get_field_i32(idx, 33),
                transmogrify_disabled_slot_mask: r.get_field_i32(idx, 34),
                faction_id: r.get_field_i16(idx, 39),
                cinematic_sequence_id: r.get_field_i16(idx, 40),
                base_language: r.get_field_i8(idx, 41),
                creature_type: r.get_field_i8(idx, 42),
                alliance: r.get_field_i8(idx, 43),
                race_related: r.get_field_i8(idx, 44),
                unaltered_visual_race_id: r.get_field_i8(idx, 45),
                default_class_id: r.get_field_i8(idx, 46),
                neutral_race_id: r.get_field_i8(idx, 47),
            }
        })
    }
}

impl NameGenStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "NameGen.db2", |id, idx, r| NameGenEntry {
            id,
            name: r.get_field_string(idx, 0),
            race_id: r.get_field_u8(idx, 1),
            sex: r.get_field_u8(idx, 2),
        })
    }
}

impl PowerDisplayStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "PowerDisplay.db2", |id, idx, r| {
            PowerDisplayEntry {
                id,
                global_string_base_tag: r.get_field_string(idx, 0),
                actual_type: r.get_field_u8(idx, 1),
                red: r.get_field_u8(idx, 2),
                green: r.get_field_u8(idx, 3),
                blue: r.get_field_u8(idx, 4),
            }
        })
    }
}

impl PowerTypeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "PowerType.db2", |id, idx, r| {
            PowerTypeEntry {
                id,
                name_global_string_tag: r.get_field_string(idx, 0),
                cost_global_string_tag: r.get_field_string(idx, 1),
                power_type_enum: r.get_field_i8(idx, 2),
                min_power: r.get_field_i32(idx, 3),
                max_base_power: r.get_field_i32(idx, 4),
                center_power: r.get_field_i32(idx, 5),
                default_power: r.get_field_i32(idx, 6),
                display_modifier: r.get_field_i32(idx, 7),
                regen_interrupt_time_ms: r.get_field_i32(idx, 8),
                regen_peace: f32_field(r, idx, 9),
                regen_combat: f32_field(r, idx, 10),
                flags: r.get_field_i16(idx, 11),
            }
        })
    }

    /// C++ `DB2Manager::GetPowerTypeEntry(Powers)` indexes by
    /// `PowerTypeEntry::PowerTypeEnum`, not by the DB2 row ID.
    pub fn get_by_power_type_like_cpp(&self, power_type: i8) -> Option<&PowerTypeEntry> {
        self.entries
            .values()
            .find(|entry| entry.power_type_enum == power_type)
    }

    /// Mirrors the creature-specific create/max/current power rules in:
    /// - `Creature::UpdateLevelDependantStats`
    /// - `Creature::GetCreatePowerValue`
    /// - `Unit::GetCreatePowerValue`
    /// - `Unit::UpdateMaxPower`
    pub fn creature_initial_power_like_cpp(
        &self,
        power_type: i8,
        create_mana: i32,
        mana_modifier: f32,
    ) -> CreatureInitialPowerLikeCpp {
        const UNITS_USE_DEFAULT_POWER_ON_INIT: u16 = 0x0020;
        const IS_USED_BY_NPCS: u16 = 0x0080;

        let entry = self.get_by_power_type_like_cpp(power_type);
        let create_power = match entry {
            Some(entry) if (entry.flags as u16 & IS_USED_BY_NPCS) == 0 => 0,
            _ if power_type == 0 => create_mana.max(0),
            Some(entry) => entry.max_base_power,
            None => 0,
        };
        let max_power = ((create_power as f32) * mana_modifier)
            .round()
            .clamp(0.0, i32::MAX as f32) as i32;
        let power = entry.map_or(0, |entry| {
            if entry.flags as u16 & UNITS_USE_DEFAULT_POWER_ON_INIT != 0 {
                entry.default_power.min(max_power)
            } else {
                max_power
            }
        });

        CreatureInitialPowerLikeCpp { max_power, power }
    }
}

fn load_store<T, S>(
    data_dir: &str,
    locale: &str,
    file_name: &str,
    mut read: impl FnMut(u32, usize, &Wdc4Reader) -> T,
) -> Result<S>
where
    S: FromEntries<T>,
{
    let path = Path::new(data_dir).join("dbc").join(locale).join(file_name);
    let reader =
        Wdc4Reader::open(&path).with_context(|| format!("failed to open {}", path.display()))?;

    let mut entries = Vec::with_capacity(reader.total_count());
    for (id, idx) in reader.iter_records() {
        entries.push(read(id, idx, &reader));
    }

    let store = S::from_entries(entries);
    info!("Loaded {} rows from {}", store.len(), path.display());
    Ok(store)
}

fn f32_field(reader: &Wdc4Reader, record_idx: usize, field: usize) -> f32 {
    f32::from_bits(reader.get_field_u32(record_idx, field))
}

fn f32_array<const N: usize>(reader: &Wdc4Reader, record_idx: usize, field: usize) -> [f32; N] {
    std::array::from_fn(|i| f32::from_bits(reader.get_array_element(record_idx, field, i, 32)))
}

trait FromEntries<T> {
    fn from_entries(entries: impl IntoIterator<Item = T>) -> Self;
    fn len(&self) -> usize;
}

macro_rules! impl_from_entries {
    ($store:ident, $entry:ty) => {
        impl FromEntries<$entry> for $store {
            fn from_entries(entries: impl IntoIterator<Item = $entry>) -> Self {
                Self::from_entries(entries)
            }

            fn len(&self) -> usize {
                self.len()
            }
        }
    };
}

impl_from_entries!(BarberShopStyleStore, BarberShopStyleEntry);
impl_from_entries!(CharacterLoadoutStore, CharacterLoadoutEntry);
impl_from_entries!(CharacterLoadoutItemStore, CharacterLoadoutItemEntry);
impl_from_entries!(ChrClassUiDisplayStore, ChrClassUiDisplayEntry);
impl_from_entries!(ChrClassesStore, ChrClassesEntry);
impl_from_entries!(ChrClassesXPowerTypesStore, ChrClassesXPowerTypesEntry);
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
impl_from_entries!(ChrModelStore, ChrModelEntry);
impl_from_entries!(ChrRaceXChrModelStore, ChrRaceXChrModelEntry);
impl_from_entries!(ChrRacesStore, ChrRacesEntry);
impl_from_entries!(NameGenStore, NameGenEntry);
impl_from_entries!(PowerDisplayStore, PowerDisplayEntry);
impl_from_entries!(PowerTypeStore, PowerTypeEntry);

#[cfg(test)]
mod tests {
    use super::*;

    fn power_type_entry(
        id: u32,
        power_type_enum: i8,
        max_base_power: i32,
        default_power: i32,
        flags: i16,
    ) -> PowerTypeEntry {
        PowerTypeEntry {
            id,
            name_global_string_tag: String::new(),
            cost_global_string_tag: String::new(),
            power_type_enum,
            min_power: 0,
            max_base_power,
            center_power: 0,
            default_power,
            display_modifier: 1,
            regen_interrupt_time_ms: 0,
            regen_peace: 0.0,
            regen_combat: 0.0,
            flags,
        }
    }

    #[test]
    fn character_loadout_item_uses_cpp_parent_relationship() {
        let store = CharacterLoadoutItemStore::from_entries([CharacterLoadoutItemEntry {
            id: 1,
            character_loadout_id: 9,
            item_id: 6948,
        }]);

        assert_eq!(store.get(1).unwrap().character_loadout_id, 9);
    }

    #[test]
    fn creature_initial_power_keys_by_enum_and_honors_cpp_flags() {
        let store = PowerTypeStore::from_entries([
            power_type_entry(700, 2, 100, 25, 0x0080),
            power_type_entry(701, 3, 120, 40, 0x0020 | 0x0080),
            power_type_entry(702, 6, 1_000, 0, 0),
        ]);

        assert_eq!(
            store.creature_initial_power_like_cpp(2, 999, 1.5),
            CreatureInitialPowerLikeCpp {
                max_power: 150,
                power: 150,
            },
            "C++ keys PowerType by enum and fills NPC powers without the default flag"
        );
        assert_eq!(
            store.creature_initial_power_like_cpp(3, 999, 2.0),
            CreatureInitialPowerLikeCpp {
                max_power: 240,
                power: 40,
            },
            "UnitsUseDefaultPowerOnInit preserves the DB2 DefaultPower"
        );
        assert_eq!(
            store.creature_initial_power_like_cpp(6, 999, 1.0),
            CreatureInitialPowerLikeCpp {
                max_power: 0,
                power: 0,
            },
            "Creature::GetCreatePowerValue rejects powers without IsUsedByNPCs"
        );
    }

    #[test]
    fn creature_initial_mana_uses_create_mana_and_cpp_rounding() {
        let store = PowerTypeStore::from_entries([power_type_entry(900, 0, 9_999, 0, 0x0080)]);

        assert_eq!(
            store.creature_initial_power_like_cpp(0, 101, 1.5),
            CreatureInitialPowerLikeCpp {
                max_power: 152,
                power: 152,
            },
            "Unit::GetCreatePowerValue uses BaseMana for POWER_MANA and UpdateMaxPower lroundf"
        );
    }

    #[test]
    fn load_character_progression_db2_subbatch_when_fixtures_exist() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "esES";
        let dbc_dir = Path::new(data_dir).join("dbc").join(locale);
        if !dbc_dir.exists() {
            eprintln!(
                "Skipping test: DB2 fixture directory not found at {}",
                dbc_dir.display()
            );
            return;
        }

        macro_rules! load_if_exists {
            ($file:literal, $store:ty) => {
                if dbc_dir.join($file).exists() {
                    let _store = <$store>::load(data_dir, locale)
                        .unwrap_or_else(|error| panic!("failed to load {}: {error:#}", $file));
                }
            };
        }

        load_if_exists!("BarberShopStyle.db2", BarberShopStyleStore);
        load_if_exists!("CharacterLoadout.db2", CharacterLoadoutStore);
        load_if_exists!("CharacterLoadoutItem.db2", CharacterLoadoutItemStore);
        load_if_exists!("ChrClassUIDisplay.db2", ChrClassUiDisplayStore);
        load_if_exists!("ChrClasses.db2", ChrClassesStore);
        load_if_exists!("ChrClassesXPowerTypes.db2", ChrClassesXPowerTypesStore);
        load_if_exists!("ChrCustomizationChoice.db2", ChrCustomizationChoiceStore);
        load_if_exists!(
            "ChrCustomizationDisplayInfo.db2",
            ChrCustomizationDisplayInfoStore
        );
        load_if_exists!("ChrCustomizationElement.db2", ChrCustomizationElementStore);
        load_if_exists!("ChrCustomizationOption.db2", ChrCustomizationOptionStore);
        load_if_exists!("ChrCustomizationReq.db2", ChrCustomizationReqStore);
        load_if_exists!(
            "ChrCustomizationReqChoice.db2",
            ChrCustomizationReqChoiceStore
        );
        load_if_exists!("ChrModel.db2", ChrModelStore);
        load_if_exists!("ChrRaceXChrModel.db2", ChrRaceXChrModelStore);
        load_if_exists!("ChrRaces.db2", ChrRacesStore);
        load_if_exists!("NameGen.db2", NameGenStore);
        load_if_exists!("PowerDisplay.db2", PowerDisplayStore);
        load_if_exists!("PowerType.db2", PowerTypeStore);
    }
}
