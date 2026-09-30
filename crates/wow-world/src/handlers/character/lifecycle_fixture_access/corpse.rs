//! Feature-gated forwards to the real Character operation; no replacement logic.

use super::*;

use crate::handlers::character::corpse_loading as original;

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedMapCorpseRowForTest {
    pub position: Position,
    pub map_id: u16,
    pub display_id: u32,
    pub items: [u32; wow_entities::CORPSE_ITEMS],
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub flags: u32,
    pub dynamic_flags: u32,
    pub ghost_time: i64,
    pub corpse_type: CorpseType,
    pub instance_id: u32,
    pub owner_db_guid: u64,
}

impl From<LoadedMapCorpseRowForTest> for original::LoadedMapCorpseRowLikeCpp {
    fn from(value: LoadedMapCorpseRowForTest) -> Self {
        Self {
            position: value.position,
            map_id: value.map_id,
            display_id: value.display_id,
            items: value.items,
            race: value.race,
            class: value.class,
            sex: value.sex,
            flags: value.flags,
            dynamic_flags: value.dynamic_flags,
            ghost_time: value.ghost_time,
            corpse_type: value.corpse_type,
            instance_id: value.instance_id,
            owner_db_guid: value.owner_db_guid,
        }
    }
}

impl From<original::LoadedMapCorpseRowLikeCpp> for LoadedMapCorpseRowForTest {
    fn from(value: original::LoadedMapCorpseRowLikeCpp) -> Self {
        Self {
            position: value.position,
            map_id: value.map_id,
            display_id: value.display_id,
            items: value.items,
            race: value.race,
            class: value.class,
            sex: value.sex,
            flags: value.flags,
            dynamic_flags: value.dynamic_flags,
            ghost_time: value.ghost_time,
            corpse_type: value.corpse_type,
            instance_id: value.instance_id,
            owner_db_guid: value.owner_db_guid,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapCorpseLoadOutcomeForTest {
    pub already_loaded: bool,
    pub rows_seen: u32,
    pub corpses_added: u32,
    pub invalid_type_rows: u32,
    pub invalid_race_rows: u32,
    pub invalid_position_rows: u32,
    pub add_to_map_errors: u32,
}

impl From<MapCorpseLoadOutcomeForTest> for original::MapCorpseLoadOutcomeLikeCpp {
    fn from(value: MapCorpseLoadOutcomeForTest) -> Self {
        Self {
            already_loaded: value.already_loaded,
            rows_seen: value.rows_seen,
            corpses_added: value.corpses_added,
            invalid_type_rows: value.invalid_type_rows,
            invalid_race_rows: value.invalid_race_rows,
            invalid_position_rows: value.invalid_position_rows,
            add_to_map_errors: value.add_to_map_errors,
        }
    }
}

impl From<original::MapCorpseLoadOutcomeLikeCpp> for MapCorpseLoadOutcomeForTest {
    fn from(value: original::MapCorpseLoadOutcomeLikeCpp) -> Self {
        Self {
            already_loaded: value.already_loaded,
            rows_seen: value.rows_seen,
            corpses_added: value.corpses_added,
            invalid_type_rows: value.invalid_type_rows,
            invalid_race_rows: value.invalid_race_rows,
            invalid_position_rows: value.invalid_position_rows,
            add_to_map_errors: value.add_to_map_errors,
        }
    }
}

pub fn materialize_loaded_map_corpses_for_test(
    map: &mut wow_map::Map,
    realm_id: u16,
    rows: Vec<LoadedMapCorpseRowForTest>,
    phases: &HashMap<u64, BTreeSet<u32>>,
    customizations: &HashMap<u64, Vec<CorpseCustomizationChoice>>,
    faction_templates_by_race: &HashMap<u8, i32>,
) -> MapCorpseLoadOutcomeForTest {
    crate::handlers::character::corpse_loading::materialize_loaded_map_corpses_like_cpp(map, realm_id, rows.into_iter().map(Into::into).collect(), phases, customizations, faction_templates_by_race).into()
}
