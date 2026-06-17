//! Entity and movement-facing DB2 readers from C++ `DB2Stores.cpp`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

use crate::wdc4::Wdc4Reader;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Db2Pos3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationDataEntry {
    pub id: u32,
    pub fallback: u16,
    pub behavior_tier: u8,
    pub behavior_id: i32,
    pub flags: [i32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimKitEntry {
    pub id: u32,
    pub one_shot_duration: u32,
    pub one_shot_stop_anim_kit_id: u16,
    pub low_def_anim_kit_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureDisplayInfoExtraEntry {
    pub id: u32,
    pub display_race_id: i8,
    pub display_sex_id: i8,
    pub display_class_id: i8,
    pub skin_id: i8,
    pub face_id: i8,
    pub hair_style_id: i8,
    pub hair_color_id: i8,
    pub facial_hair_id: i8,
    pub flags: i8,
    pub bake_material_resources_id: i32,
    pub hd_bake_material_resources_id: i32,
    pub custom_display_option: [u8; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreatureFamilyEntry {
    pub id: u32,
    pub name: String,
    pub min_scale: f32,
    pub min_scale_level: i8,
    pub max_scale: f32,
    pub max_scale_level: i8,
    pub pet_food_mask: i16,
    pub pet_talent_type: i8,
    pub category_enum_id: i32,
    pub icon_file_id: i32,
    pub skill_line: [i16; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatureTypeEntry {
    pub id: u32,
    pub name: String,
    pub flags: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestructibleModelDataEntry {
    pub id: u32,
    pub state0_impact_effect_doodad_set: i8,
    pub state0_ambient_doodad_set: u8,
    pub state1_wmo: u32,
    pub state1_destruction_doodad_set: i8,
    pub state1_impact_effect_doodad_set: i8,
    pub state1_ambient_doodad_set: u8,
    pub state2_wmo: u32,
    pub state2_destruction_doodad_set: i8,
    pub state2_impact_effect_doodad_set: i8,
    pub state2_ambient_doodad_set: u8,
    pub state3_wmo: u32,
    pub state3_init_doodad_set: u8,
    pub state3_ambient_doodad_set: u8,
    pub eject_direction: u8,
    pub do_not_highlight: u8,
    pub state0_wmo: u32,
    pub heal_effect: u8,
    pub heal_effect_speed: u16,
    pub state0_name_set: i8,
    pub state1_name_set: i8,
    pub state2_name_set: i8,
    pub state3_name_set: i8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmotesEntry {
    pub id: u32,
    pub race_mask: i64,
    pub emote_slash_command: String,
    pub anim_id: i32,
    pub emote_flags: u32,
    pub emote_spec_proc: u8,
    pub emote_spec_proc_param: u32,
    pub event_sound_id: u32,
    pub spell_visual_kit_id: u32,
    pub class_mask: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmotesTextEntry {
    pub id: u32,
    pub name: String,
    pub emote_id: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmotesTextSoundEntry {
    pub id: u32,
    pub race_id: u8,
    pub class_id: u8,
    pub sex_id: u8,
    pub sound_id: u32,
    pub emotes_text_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameObjectArtKitEntry {
    pub id: u32,
    pub attach_model_file_id: i32,
    pub texture_variation_file_id: [i32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectDisplayInfoEntry {
    pub id: u32,
    pub model_name: String,
    pub geo_box_min: Db2Pos3,
    pub geo_box_max: Db2Pos3,
    pub file_data_id: i32,
    pub object_effect_package_id: i16,
    pub override_loot_effect_scale: f32,
    pub override_name_scale: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameObjectsEntry {
    pub id: u32,
    pub name: String,
    pub pos: Db2Pos3,
    pub rot: [f32; 4],
    pub owner_id: u16,
    pub display_id: u32,
    pub scale: f32,
    pub type_id: u8,
    pub phase_use_flags: u8,
    pub phase_id: u16,
    pub phase_group_id: u16,
    pub prop_value: [i32; 8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitConditionEntry {
    pub id: u32,
    pub flags: u8,
    pub variable: [u8; 8],
    pub op: [i8; 8],
    pub value: [i32; 8],
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnitPowerBarEntry {
    pub id: u32,
    pub name: String,
    pub cost: String,
    pub out_of_error: String,
    pub tooltip: String,
    pub min_power: u32,
    pub max_power: u32,
    pub start_power: u16,
    pub center_power: u8,
    pub regeneration_peace: f32,
    pub regeneration_combat: f32,
    pub bar_type: u8,
    pub flags: u16,
    pub start_inset: f32,
    pub end_inset: f32,
    pub file_data_id: [i32; 6],
    pub color: [i32; 6],
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

db2_store!(AnimationDataStore, AnimationDataEntry);
db2_store!(AnimKitStore, AnimKitEntry);
db2_store!(CreatureDisplayInfoExtraStore, CreatureDisplayInfoExtraEntry);
db2_store!(CreatureFamilyStore, CreatureFamilyEntry);
db2_store!(CreatureTypeStore, CreatureTypeEntry);
db2_store!(DestructibleModelDataStore, DestructibleModelDataEntry);
db2_store!(EmotesStore, EmotesEntry);
db2_store!(EmotesTextStore, EmotesTextEntry);
db2_store!(EmotesTextSoundStore, EmotesTextSoundEntry);
db2_store!(GameObjectArtKitStore, GameObjectArtKitEntry);
db2_store!(GameObjectDisplayInfoStore, GameObjectDisplayInfoEntry);
db2_store!(GameObjectsStore, GameObjectsEntry);
db2_store!(UnitConditionStore, UnitConditionEntry);
db2_store!(UnitPowerBarStore, UnitPowerBarEntry);

impl AnimationDataStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "AnimationData.db2", |id, idx, r| {
            AnimationDataEntry {
                id,
                fallback: r.get_field_u16(idx, 0),
                behavior_tier: r.get_field_u8(idx, 1),
                behavior_id: r.get_field_i32(idx, 2),
                flags: [
                    r.get_array_element(idx, 3, 0, 32) as i32,
                    r.get_array_element(idx, 3, 1, 32) as i32,
                ],
            }
        })
    }
}

impl AnimKitStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "AnimKit.db2", |id, idx, r| AnimKitEntry {
            id,
            one_shot_duration: r.get_field_u32(idx, 0),
            one_shot_stop_anim_kit_id: r.get_field_u16(idx, 1),
            low_def_anim_kit_id: r.get_field_u16(idx, 2),
        })
    }
}

impl CreatureDisplayInfoExtraStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "CreatureDisplayInfoExtra.db2",
            |id, idx, r| CreatureDisplayInfoExtraEntry {
                id,
                display_race_id: r.get_field_i8(idx, 1),
                display_sex_id: r.get_field_i8(idx, 2),
                display_class_id: r.get_field_i8(idx, 3),
                skin_id: r.get_field_i8(idx, 4),
                face_id: r.get_field_i8(idx, 5),
                hair_style_id: r.get_field_i8(idx, 6),
                hair_color_id: r.get_field_i8(idx, 7),
                facial_hair_id: r.get_field_i8(idx, 8),
                flags: r.get_field_i8(idx, 9),
                bake_material_resources_id: r.get_field_i32(idx, 10),
                hd_bake_material_resources_id: r.get_field_i32(idx, 11),
                custom_display_option: [
                    r.get_array_element(idx, 12, 0, 8) as u8,
                    r.get_array_element(idx, 12, 1, 8) as u8,
                    r.get_array_element(idx, 12, 2, 8) as u8,
                ],
            },
        )
    }
}

impl CreatureFamilyStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "CreatureFamily.db2", |id, idx, r| {
            CreatureFamilyEntry {
                id,
                name: r.get_field_string(idx, 0),
                min_scale: f32_field(r, idx, 1),
                min_scale_level: r.get_field_i8(idx, 2),
                max_scale: f32_field(r, idx, 3),
                max_scale_level: r.get_field_i8(idx, 4),
                pet_food_mask: r.get_field_i16(idx, 5),
                pet_talent_type: r.get_field_i8(idx, 6),
                category_enum_id: r.get_field_i32(idx, 7),
                icon_file_id: r.get_field_i32(idx, 8),
                skill_line: [r.get_array_i16(idx, 9, 0), r.get_array_i16(idx, 9, 1)],
            }
        })
    }

    pub fn entries_like_cpp(&self) -> impl Iterator<Item = CreatureFamilyEntry> + '_ {
        self.entries.values().cloned()
    }
}

impl CreatureTypeStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "CreatureType.db2", |id, idx, r| {
            CreatureTypeEntry {
                id,
                name: r.get_field_string(idx, 0),
                flags: r.get_field_u8(idx, 1),
            }
        })
    }
}

impl DestructibleModelDataStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "DestructibleModelData.db2",
            |id, idx, r| DestructibleModelDataEntry {
                id,
                state0_impact_effect_doodad_set: r.get_field_i8(idx, 0),
                state0_ambient_doodad_set: r.get_field_u8(idx, 1),
                state1_wmo: r.get_field_u32(idx, 2),
                state1_destruction_doodad_set: r.get_field_i8(idx, 3),
                state1_impact_effect_doodad_set: r.get_field_i8(idx, 4),
                state1_ambient_doodad_set: r.get_field_u8(idx, 5),
                state2_wmo: r.get_field_u32(idx, 6),
                state2_destruction_doodad_set: r.get_field_i8(idx, 7),
                state2_impact_effect_doodad_set: r.get_field_i8(idx, 8),
                state2_ambient_doodad_set: r.get_field_u8(idx, 9),
                state3_wmo: r.get_field_u32(idx, 10),
                state3_init_doodad_set: r.get_field_u8(idx, 11),
                state3_ambient_doodad_set: r.get_field_u8(idx, 12),
                eject_direction: r.get_field_u8(idx, 13),
                do_not_highlight: r.get_field_u8(idx, 14),
                state0_wmo: r.get_field_u32(idx, 15),
                heal_effect: r.get_field_u8(idx, 16),
                heal_effect_speed: r.get_field_u16(idx, 17),
                state0_name_set: r.get_field_i8(idx, 18),
                state1_name_set: r.get_field_i8(idx, 19),
                state2_name_set: r.get_field_i8(idx, 20),
                state3_name_set: r.get_field_i8(idx, 21),
            },
        )
    }
}

impl EmotesStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "Emotes.db2", |id, idx, r| EmotesEntry {
            id,
            race_mask: r.get_field_i64(idx, 0),
            emote_slash_command: r.get_field_string(idx, 1),
            anim_id: r.get_field_i32(idx, 2),
            emote_flags: r.get_field_u32(idx, 3),
            emote_spec_proc: r.get_field_u8(idx, 4),
            emote_spec_proc_param: r.get_field_u32(idx, 5),
            event_sound_id: r.get_field_u32(idx, 6),
            spell_visual_kit_id: r.get_field_u32(idx, 7),
            class_mask: r.get_field_i32(idx, 8),
        })
    }
}

impl EmotesTextStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "EmotesText.db2", |id, idx, r| {
            EmotesTextEntry {
                id,
                name: r.get_field_string(idx, 0),
                emote_id: r.get_field_u16(idx, 1),
            }
        })
    }
}

impl EmotesTextSoundStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "EmotesTextSound.db2", |id, idx, r| {
            EmotesTextSoundEntry {
                id,
                race_id: r.get_field_u8(idx, 0),
                class_id: r.get_field_u8(idx, 1),
                sex_id: r.get_field_u8(idx, 2),
                sound_id: r.get_field_u32(idx, 3),
                emotes_text_id: r.get_relationship_id(idx).unwrap_or(0),
            }
        })
    }
}

impl GameObjectArtKitStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GameObjectArtKit.db2", |id, idx, r| {
            GameObjectArtKitEntry {
                id,
                attach_model_file_id: r.get_field_i32(idx, 0),
                texture_variation_file_id: [
                    r.get_array_element(idx, 1, 0, 32) as i32,
                    r.get_array_element(idx, 1, 1, 32) as i32,
                    r.get_array_element(idx, 1, 2, 32) as i32,
                ],
            }
        })
    }
}

impl GameObjectDisplayInfoStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(
            data_dir,
            locale,
            "GameObjectDisplayInfo.db2",
            |id, idx, r| GameObjectDisplayInfoEntry {
                id,
                model_name: r.get_field_string(idx, 0),
                geo_box_min: pos3_array_offset(r, idx, 1, 0),
                geo_box_max: pos3_array_offset(r, idx, 1, 3),
                file_data_id: r.get_field_i32(idx, 2),
                object_effect_package_id: r.get_field_i16(idx, 3),
                override_loot_effect_scale: f32_field(r, idx, 4),
                override_name_scale: f32_field(r, idx, 5),
            },
        )
    }
}

impl GameObjectsStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "GameObjects.db2", |id, idx, r| {
            GameObjectsEntry {
                id,
                name: r.get_field_string(idx, 0),
                pos: pos3_array(r, idx, 1),
                rot: [
                    f32::from_bits(r.get_array_element(idx, 2, 0, 32)),
                    f32::from_bits(r.get_array_element(idx, 2, 1, 32)),
                    f32::from_bits(r.get_array_element(idx, 2, 2, 32)),
                    f32::from_bits(r.get_array_element(idx, 2, 3, 32)),
                ],
                owner_id: r.get_field_u16(idx, 4),
                display_id: r.get_field_u32(idx, 5),
                scale: f32_field(r, idx, 6),
                type_id: r.get_field_u8(idx, 7),
                phase_use_flags: r.get_field_u8(idx, 8),
                phase_id: r.get_field_u16(idx, 9),
                phase_group_id: r.get_field_u16(idx, 10),
                prop_value: std::array::from_fn(|i| r.get_array_element(idx, 11, i, 32) as i32),
            }
        })
    }
}

impl UnitConditionStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "UnitCondition.db2", |id, idx, r| {
            UnitConditionEntry {
                id,
                flags: r.get_field_u8(idx, 0),
                variable: std::array::from_fn(|i| r.get_array_element(idx, 1, i, 8) as u8),
                op: std::array::from_fn(|i| r.get_array_element(idx, 2, i, 8) as i8),
                value: std::array::from_fn(|i| r.get_array_element(idx, 3, i, 32) as i32),
            }
        })
    }
}

impl UnitPowerBarStore {
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        load_store(data_dir, locale, "UnitPowerBar.db2", |id, idx, r| {
            UnitPowerBarEntry {
                id,
                name: r.get_field_string(idx, 0),
                cost: r.get_field_string(idx, 1),
                out_of_error: r.get_field_string(idx, 2),
                tooltip: r.get_field_string(idx, 3),
                min_power: r.get_field_u32(idx, 4),
                max_power: r.get_field_u32(idx, 5),
                start_power: r.get_field_u16(idx, 6),
                center_power: r.get_field_u8(idx, 7),
                regeneration_peace: f32_field(r, idx, 8),
                regeneration_combat: f32_field(r, idx, 9),
                bar_type: r.get_field_u8(idx, 10),
                flags: r.get_field_u16(idx, 11),
                start_inset: f32_field(r, idx, 12),
                end_inset: f32_field(r, idx, 13),
                file_data_id: std::array::from_fn(|i| r.get_array_element(idx, 14, i, 32) as i32),
                color: std::array::from_fn(|i| r.get_array_element(idx, 15, i, 32) as i32),
            }
        })
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

impl_from_entries!(AnimationDataStore, AnimationDataEntry);
impl_from_entries!(AnimKitStore, AnimKitEntry);
impl_from_entries!(CreatureDisplayInfoExtraStore, CreatureDisplayInfoExtraEntry);
impl_from_entries!(CreatureFamilyStore, CreatureFamilyEntry);
impl_from_entries!(CreatureTypeStore, CreatureTypeEntry);
impl_from_entries!(DestructibleModelDataStore, DestructibleModelDataEntry);
impl_from_entries!(EmotesStore, EmotesEntry);
impl_from_entries!(EmotesTextStore, EmotesTextEntry);
impl_from_entries!(EmotesTextSoundStore, EmotesTextSoundEntry);
impl_from_entries!(GameObjectArtKitStore, GameObjectArtKitEntry);
impl_from_entries!(GameObjectDisplayInfoStore, GameObjectDisplayInfoEntry);
impl_from_entries!(GameObjectsStore, GameObjectsEntry);
impl_from_entries!(UnitConditionStore, UnitConditionEntry);
impl_from_entries!(UnitPowerBarStore, UnitPowerBarEntry);

fn f32_field(reader: &Wdc4Reader, idx: usize, field: usize) -> f32 {
    f32::from_bits(reader.get_field_u32(idx, field))
}

fn pos3_array(reader: &Wdc4Reader, idx: usize, field: usize) -> Db2Pos3 {
    pos3_array_offset(reader, idx, field, 0)
}

fn pos3_array_offset(reader: &Wdc4Reader, idx: usize, field: usize, offset: usize) -> Db2Pos3 {
    Db2Pos3 {
        x: f32::from_bits(reader.get_array_element(idx, field, offset, 32)),
        y: f32::from_bits(reader.get_array_element(idx, field, offset + 1, 32)),
        z: f32::from_bits(reader.get_array_element(idx, field, offset + 2, 32)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emotes_text_sound_store_indexes_by_id_like_cpp_lookup_entry() {
        let store = EmotesTextSoundStore::from_entries([EmotesTextSoundEntry {
            id: 77,
            race_id: 1,
            class_id: 2,
            sex_id: 3,
            sound_id: 4,
            emotes_text_id: 5,
        }]);

        assert_eq!(store.get(77).unwrap().emotes_text_id, 5);
        assert!(store.get(78).is_none());
    }

    #[test]
    fn load_entities_movement_db2_batch_when_fixtures_exist() {
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

        load_if_exists!("AnimationData.db2", AnimationDataStore);
        load_if_exists!("AnimKit.db2", AnimKitStore);
        load_if_exists!(
            "CreatureDisplayInfoExtra.db2",
            CreatureDisplayInfoExtraStore
        );
        load_if_exists!("CreatureFamily.db2", CreatureFamilyStore);
        load_if_exists!("CreatureType.db2", CreatureTypeStore);
        load_if_exists!("DestructibleModelData.db2", DestructibleModelDataStore);
        load_if_exists!("Emotes.db2", EmotesStore);
        load_if_exists!("EmotesText.db2", EmotesTextStore);
        load_if_exists!("EmotesTextSound.db2", EmotesTextSoundStore);
        load_if_exists!("GameObjectArtKit.db2", GameObjectArtKitStore);
        load_if_exists!("GameObjectDisplayInfo.db2", GameObjectDisplayInfoStore);
        load_if_exists!("GameObjects.db2", GameObjectsStore);
        load_if_exists!("UnitCondition.db2", UnitConditionStore);
        load_if_exists!("UnitPowerBar.db2", UnitPowerBarStore);
    }
}
