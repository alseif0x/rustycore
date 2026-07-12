// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadPlayerInfo` base positions and player-create spell data.

use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};
use wow_core::Position;
use wow_database::{WorldDatabase, WorldStatements};

use crate::character_progression::{
    ChrClassesStore, ChrModelStore, ChrRaceXChrModelStore, ChrRacesStore,
};
use crate::{GameObjectTemplateLifecycleStoreLikeCpp, MapStore, TaxiPathNodeStore, TaxiPathStore};

pub const PLAYER_CREATE_MODE_NORMAL_LIKE_CPP: u8 = 0;
pub const PLAYER_CREATE_MODE_NPE_LIKE_CPP: u8 = 1;
pub const PLAYER_CREATE_MODE_MAX_LIKE_CPP: u8 = 2;

const RACE_HUMAN_LIKE_CPP: u8 = 1;
const MAX_RACES_LIKE_CPP: u8 = 78;
const CLASS_WARRIOR_LIKE_CPP: u8 = 1;
const MAX_CLASSES_LIKE_CPP: u8 = 15;

const RACEMASK_ALL_PLAYABLE_LIKE_CPP: u64 = 0x0003_007F_FFFF;
const CLASSMASK_ALL_PLAYABLE_LIKE_CPP: u32 = 0x1FFF;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerCreatePositionLikeCpp {
    pub map_id: u32,
    pub position: Position,
    pub transport_guid: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerCreateInfoLikeCpp {
    pub create_position: PlayerCreatePositionLikeCpp,
    pub create_position_npe: Option<PlayerCreatePositionLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerCreateInfoRowLikeCpp {
    pub race: u8,
    pub class: u8,
    pub create_position: PlayerCreatePositionLikeCpp,
    pub create_position_npe: Option<PlayerCreatePositionLikeCpp>,
    pub npe_transport_template_valid: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerCreateInfoLoadReportLikeCpp {
    pub loaded: usize,
    pub skipped_invalid_race: usize,
    pub skipped_invalid_class: usize,
    pub skipped_missing_gender_models: usize,
    pub skipped_invalid_position: usize,
    pub skipped_instanceable_map: usize,
    pub discarded_invalid_npe_map: usize,
    pub discarded_invalid_npe_transport: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerCreateInfoStoreLikeCpp {
    info_by_key: HashMap<(u8, u8), PlayerCreateInfoLikeCpp>,
    load_report: PlayerCreateInfoLoadReportLikeCpp,
}

impl PlayerCreateInfoStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = PlayerCreateInfoRowLikeCpp>,
        map_store: &MapStore,
        mut race_exists: impl FnMut(u8) -> bool,
        mut class_exists: impl FnMut(u8) -> bool,
        mut race_has_gender_models: impl FnMut(u8) -> bool,
    ) -> Self {
        let mut info_by_key = HashMap::new();
        let mut load_report = PlayerCreateInfoLoadReportLikeCpp::default();

        for row in rows {
            if !race_exists(row.race) {
                load_report.skipped_invalid_race += 1;
                continue;
            }
            if !class_exists(row.class) {
                load_report.skipped_invalid_class += 1;
                continue;
            }
            if !race_has_gender_models(row.race) {
                load_report.skipped_missing_gender_models += 1;
                continue;
            }
            if !row.create_position.position.is_valid_map_coord_like_cpp()
                || map_store.get(row.create_position.map_id).is_none()
            {
                load_report.skipped_invalid_position += 1;
                continue;
            }
            if map_store
                .get(row.create_position.map_id)
                .is_some_and(|entry| map_entry_instanceable_like_cpp(*entry))
            {
                load_report.skipped_instanceable_map += 1;
                continue;
            }

            let mut create_position_npe = row.create_position_npe;
            if create_position_npe.is_some_and(|position| map_store.get(position.map_id).is_none())
            {
                create_position_npe = None;
                load_report.discarded_invalid_npe_map += 1;
            } else if create_position_npe.is_some_and(|position| {
                position.transport_guid.is_some() && !row.npe_transport_template_valid
            }) {
                create_position_npe = None;
                load_report.discarded_invalid_npe_transport += 1;
            }

            info_by_key.insert(
                (row.race, row.class),
                PlayerCreateInfoLikeCpp {
                    create_position: row.create_position,
                    create_position_npe,
                },
            );
        }
        load_report.loaded = info_by_key.len();

        Self {
            info_by_key,
            load_report,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn load_like_cpp(
        world_db: &WorldDatabase,
        map_store: &MapStore,
        chr_races_store: &ChrRacesStore,
        chr_classes_store: &ChrClassesStore,
        chr_model_store: &ChrModelStore,
        chr_race_x_chr_model_store: &ChrRaceXChrModelStore,
        gameobject_template_store: &GameObjectTemplateLifecycleStoreLikeCpp,
        taxi_path_store: &TaxiPathStore,
        taxi_path_node_store: &TaxiPathNodeStore,
    ) -> Result<Self> {
        let stmt = world_db.prepare(WorldStatements::SEL_PLAYER_CREATEINFO);
        let mut result = world_db
            .query(&stmt)
            .await
            .context("Failed to query playercreateinfo")?;
        if result.is_empty() {
            bail!("playercreateinfo is empty");
        }

        let mut rows = Vec::new();
        loop {
            let transport_guid = result.try_read::<Option<u64>>(12).flatten();
            let transport_entry = result.try_read::<Option<u32>>(13).flatten();
            let create_position_npe = (!(7..=11).any(|column| result.is_null(column))).then(|| {
                PlayerCreatePositionLikeCpp {
                    map_id: result.try_read::<u32>(7).unwrap_or(u32::MAX),
                    position: Position::new(
                        result.try_read::<f32>(8).unwrap_or(f32::NAN),
                        result.try_read::<f32>(9).unwrap_or(f32::NAN),
                        result.try_read::<f32>(10).unwrap_or(f32::NAN),
                        result.try_read::<f32>(11).unwrap_or(f32::NAN),
                    ),
                    transport_guid,
                }
            });
            rows.push(PlayerCreateInfoRowLikeCpp {
                race: result.try_read::<u8>(0).unwrap_or(0),
                class: result.try_read::<u8>(1).unwrap_or(0),
                create_position: PlayerCreatePositionLikeCpp {
                    map_id: u32::from(result.try_read::<u16>(2).unwrap_or(u16::MAX)),
                    position: Position::new(
                        result.try_read::<f32>(3).unwrap_or(f32::NAN),
                        result.try_read::<f32>(4).unwrap_or(f32::NAN),
                        result.try_read::<f32>(5).unwrap_or(f32::NAN),
                        result.try_read::<f32>(6).unwrap_or(f32::NAN),
                    ),
                    transport_guid: None,
                },
                create_position_npe,
                npe_transport_template_valid: transport_guid.is_none()
                    || transport_entry.is_some_and(|entry| {
                        valid_transport_template_like_cpp(
                            entry,
                            gameobject_template_store,
                            taxi_path_store,
                            taxi_path_node_store,
                            map_store,
                        )
                    }),
            });
            if !result.next_row() {
                break;
            }
        }

        Ok(Self::from_rows_like_cpp(
            rows,
            map_store,
            |race| chr_races_store.get(u32::from(race)).is_some(),
            |class| chr_classes_store.get(u32::from(class)).is_some(),
            |race| {
                [0, 1].into_iter().all(|sex| {
                    chr_race_x_chr_model_store.entries().any(|race_model| {
                        race_model.chr_races_id == u32::from(race)
                            && race_model.sex == sex
                            && u32::try_from(race_model.chr_model_id)
                                .ok()
                                .is_some_and(|model_id| chr_model_store.get(model_id).is_some())
                    })
                })
            },
        ))
    }

    pub fn get(&self, race: u8, class: u8) -> Option<&PlayerCreateInfoLikeCpp> {
        self.info_by_key.get(&(race, class))
    }

    pub fn len(&self) -> usize {
        self.info_by_key.len()
    }

    pub fn is_empty(&self) -> bool {
        self.info_by_key.is_empty()
    }

    pub fn load_report_like_cpp(&self) -> &PlayerCreateInfoLoadReportLikeCpp {
        &self.load_report
    }
}

fn map_entry_instanceable_like_cpp(entry: crate::MapEntry) -> bool {
    matches!(
        entry.instance_type,
        crate::map::MAP_INSTANCE
            | crate::map::MAP_RAID
            | crate::map::MAP_BATTLEGROUND
            | crate::map::MAP_ARENA
            | crate::map::MAP_SCENARIO
    )
}

fn valid_transport_template_like_cpp(
    transport_entry: u32,
    gameobject_template_store: &GameObjectTemplateLifecycleStoreLikeCpp,
    taxi_path_store: &TaxiPathStore,
    taxi_path_node_store: &TaxiPathNodeStore,
    map_store: &MapStore,
) -> bool {
    let Some(gameobject_template) = gameobject_template_store.get(transport_entry) else {
        return false;
    };
    if gameobject_template.go_type != 15 {
        return false;
    }
    let taxi_path_id = gameobject_template.data[0];
    if taxi_path_id == 0 || taxi_path_store.get(taxi_path_id).is_none() {
        return false;
    }

    let path_maps = taxi_path_node_store
        .entries()
        .filter(|node| u32::from(node.path_id) == taxi_path_id)
        .map(|node| u32::from(node.continent_id))
        .collect::<HashSet<_>>();
    if path_maps.is_empty() {
        return false;
    }
    if path_maps.len() > 1
        && path_maps.iter().any(|map_id| {
            map_store
                .get(*map_id)
                .is_none_or(|entry| map_entry_instanceable_like_cpp(*entry))
        })
    {
        return false;
    }

    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCreateInfoCastSpellRowLikeCpp {
    pub race_mask: u64,
    pub class_mask: u32,
    pub spell_id: u32,
    pub create_mode: i8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerCreateInfoCastSpellLoadReportLikeCpp {
    pub loaded_assignments: usize,
    pub skipped_invalid_race_mask: usize,
    pub skipped_invalid_class_mask: usize,
    pub skipped_invalid_create_mode: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerCreateInfoCastSpellStoreLikeCpp {
    spells_by_key: HashMap<(u8, u8, u8), Vec<u32>>,
    load_report: PlayerCreateInfoCastSpellLoadReportLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerCreateInfoCustomSpellRowLikeCpp {
    pub race_mask: u64,
    pub class_mask: u32,
    pub spell_id: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlayerCreateInfoCustomSpellLoadReportLikeCpp {
    pub loaded_assignments: usize,
    pub skipped_invalid_race_mask: usize,
    pub skipped_invalid_class_mask: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PlayerCreateInfoCustomSpellStoreLikeCpp {
    spells_by_key: HashMap<(u8, u8), Vec<u32>>,
    load_report: PlayerCreateInfoCustomSpellLoadReportLikeCpp,
}

impl PlayerCreateInfoCastSpellStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = PlayerCreateInfoCastSpellRowLikeCpp>,
    ) -> Self {
        let mut spells_by_key = HashMap::<(u8, u8, u8), Vec<u32>>::new();
        let mut load_report = PlayerCreateInfoCastSpellLoadReportLikeCpp::default();

        for row in rows {
            if row.race_mask != 0 && row.race_mask & RACEMASK_ALL_PLAYABLE_LIKE_CPP == 0 {
                load_report.skipped_invalid_race_mask += 1;
                continue;
            }

            if row.class_mask != 0 && row.class_mask & CLASSMASK_ALL_PLAYABLE_LIKE_CPP == 0 {
                load_report.skipped_invalid_class_mask += 1;
                continue;
            }

            let Ok(create_mode) = u8::try_from(row.create_mode) else {
                load_report.skipped_invalid_create_mode += 1;
                continue;
            };
            if create_mode >= PLAYER_CREATE_MODE_MAX_LIKE_CPP {
                load_report.skipped_invalid_create_mode += 1;
                continue;
            }

            for race in RACE_HUMAN_LIKE_CPP..MAX_RACES_LIKE_CPP {
                if row.race_mask != 0 && row.race_mask & race_mask_bit_like_cpp(race) == 0 {
                    continue;
                }

                for class in CLASS_WARRIOR_LIKE_CPP..MAX_CLASSES_LIKE_CPP {
                    if row.class_mask != 0 && row.class_mask & class_mask_bit_like_cpp(class) == 0 {
                        continue;
                    }

                    spells_by_key
                        .entry((race, class, create_mode))
                        .or_default()
                        .push(row.spell_id);
                    load_report.loaded_assignments += 1;
                }
            }
        }

        Self {
            spells_by_key,
            load_report,
        }
    }

    pub async fn load_like_cpp(world_db: &WorldDatabase) -> Result<Self> {
        let stmt = world_db.prepare(WorldStatements::SEL_PLAYER_CREATEINFO_CAST_SPELL);
        let mut result = world_db
            .query(&stmt)
            .await
            .context("Failed to query playercreateinfo_cast_spell")?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(PlayerCreateInfoCastSpellRowLikeCpp {
                    race_mask: result.try_read::<u64>(0).unwrap_or(0),
                    class_mask: result.try_read::<u32>(1).unwrap_or(0),
                    spell_id: result.try_read::<u32>(2).unwrap_or(0),
                    create_mode: result.try_read::<i8>(3).unwrap_or(-1),
                });
                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(rows))
    }

    pub fn cast_spells_like_cpp(&self, race: u8, class: u8, create_mode: u8) -> &[u32] {
        self.spells_by_key
            .get(&(race, class, create_mode))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn load_report_like_cpp(&self) -> &PlayerCreateInfoCastSpellLoadReportLikeCpp {
        &self.load_report
    }
}

impl PlayerCreateInfoCustomSpellStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = PlayerCreateInfoCustomSpellRowLikeCpp>,
    ) -> Self {
        let mut spells_by_key = HashMap::<(u8, u8), Vec<u32>>::new();
        let mut load_report = PlayerCreateInfoCustomSpellLoadReportLikeCpp::default();

        for row in rows {
            if row.race_mask != 0 && row.race_mask & RACEMASK_ALL_PLAYABLE_LIKE_CPP == 0 {
                load_report.skipped_invalid_race_mask += 1;
                continue;
            }

            if row.class_mask != 0 && row.class_mask & CLASSMASK_ALL_PLAYABLE_LIKE_CPP == 0 {
                load_report.skipped_invalid_class_mask += 1;
                continue;
            }

            for race in RACE_HUMAN_LIKE_CPP..MAX_RACES_LIKE_CPP {
                if row.race_mask != 0 && row.race_mask & race_mask_bit_like_cpp(race) == 0 {
                    continue;
                }

                for class in CLASS_WARRIOR_LIKE_CPP..MAX_CLASSES_LIKE_CPP {
                    if row.class_mask != 0 && row.class_mask & class_mask_bit_like_cpp(class) == 0 {
                        continue;
                    }

                    spells_by_key
                        .entry((race, class))
                        .or_default()
                        .push(row.spell_id);
                    load_report.loaded_assignments += 1;
                }
            }
        }

        Self {
            spells_by_key,
            load_report,
        }
    }

    pub async fn load_like_cpp(world_db: &WorldDatabase) -> Result<Self> {
        let stmt = world_db.prepare(WorldStatements::SEL_PLAYER_CREATEINFO_CUSTOM_SPELL);
        let mut result = world_db
            .query(&stmt)
            .await
            .context("Failed to query playercreateinfo_spell_custom")?;
        let mut rows = Vec::new();

        if !result.is_empty() {
            loop {
                rows.push(PlayerCreateInfoCustomSpellRowLikeCpp {
                    race_mask: result.try_read::<u64>(0).unwrap_or(0),
                    class_mask: result.try_read::<u32>(1).unwrap_or(0),
                    spell_id: result.try_read::<u32>(2).unwrap_or(0),
                });
                if !result.next_row() {
                    break;
                }
            }
        }

        Ok(Self::from_rows_like_cpp(rows))
    }

    pub fn custom_spells_like_cpp(&self, race: u8, class: u8) -> &[u32] {
        self.spells_by_key
            .get(&(race, class))
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn load_report_like_cpp(&self) -> &PlayerCreateInfoCustomSpellLoadReportLikeCpp {
        &self.load_report
    }
}

fn race_mask_bit_like_cpp(race: u8) -> u64 {
    let bit = match race {
        1..=10 | 22 | 24..=30 => Some(race - 1),
        34 => Some(11),
        35 => Some(12),
        36 => Some(13),
        37 => Some(14),
        70 => Some(16),
        52 => Some(15),
        _ => None,
    };
    bit.map(|bit| 1_u64 << bit).unwrap_or(0)
}

fn class_mask_bit_like_cpp(class: u8) -> u32 {
    if class == 0 || class >= 33 {
        0
    } else {
        1_u32 << (class - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn player_create_info_store_keys_race_class_and_discards_invalid_npe_like_cpp() {
        let map_store = MapStore::from_entries([
            crate::MapEntry {
                id: 0,
                instance_type: crate::map::MAP_COMMON,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: 0,
                flags2: 0,
            },
            crate::MapEntry {
                id: 1_151,
                instance_type: crate::map::MAP_SCENARIO,
                expansion_id: 0,
                parent_map_id: -1,
                cosmetic_parent_map_id: -1,
                flags1: crate::map::MAP_FLAG_GARRISON,
                flags2: 0,
            },
        ]);
        let normal = PlayerCreatePositionLikeCpp {
            map_id: 0,
            position: Position::new(1.0, 2.0, 3.0, 4.0),
            transport_guid: None,
        };
        let npe_transport = PlayerCreatePositionLikeCpp {
            map_id: 0,
            position: Position::new(5.0, 6.0, 7.0, 8.0),
            transport_guid: Some(29),
        };
        let store = PlayerCreateInfoStoreLikeCpp::from_rows_like_cpp(
            [
                PlayerCreateInfoRowLikeCpp {
                    race: 1,
                    class: 1,
                    create_position: normal,
                    create_position_npe: Some(npe_transport),
                    npe_transport_template_valid: false,
                },
                PlayerCreateInfoRowLikeCpp {
                    race: 1,
                    class: 6,
                    create_position: PlayerCreatePositionLikeCpp {
                        position: Position::new(9.0, 10.0, 11.0, 12.0),
                        ..normal
                    },
                    create_position_npe: None,
                    npe_transport_template_valid: true,
                },
                PlayerCreateInfoRowLikeCpp {
                    race: 1,
                    class: 2,
                    create_position: PlayerCreatePositionLikeCpp {
                        map_id: 1_151,
                        ..normal
                    },
                    create_position_npe: None,
                    npe_transport_template_valid: true,
                },
            ],
            &map_store,
            |_| true,
            |_| true,
            |_| true,
        );

        assert!(store.get(1, 1).unwrap().create_position_npe.is_none());
        assert_eq!(
            store.get(1, 6).unwrap().create_position.position,
            Position::new(9.0, 10.0, 11.0, 12.0)
        );
        assert!(store.get(1, 2).is_none());
        assert_eq!(store.load_report_like_cpp().loaded, 2);
        assert_eq!(
            store.load_report_like_cpp().discarded_invalid_npe_transport,
            1
        );
        assert_eq!(store.load_report_like_cpp().skipped_instanceable_map, 1);
    }

    #[test]
    fn player_create_npe_transport_requires_loaded_cpp_transport_template() {
        let map_store = MapStore::from_entries([crate::MapEntry {
            id: 0,
            instance_type: crate::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        }]);
        let mut data = [0_u32; wow_entities::MAX_GAMEOBJECT_DATA];
        data[0] = 7;
        let gameobject_templates = GameObjectTemplateLifecycleStoreLikeCpp::from_templates([
            crate::GameObjectTemplateLifecycleRecordLikeCpp {
                entry: 100,
                go_type: 15,
                display_id: 0,
                name: "NPE transport".into(),
                size: 1.0,
                data,
                content_tuning_id: 0,
                ai_name: String::new(),
                script_name: String::new(),
                string_id: String::new(),
                addon: None,
            },
        ]);
        let taxi_paths = TaxiPathStore::from_entries([crate::TaxiPathEntry {
            id: 7,
            from_taxi_node: 1,
            to_taxi_node: 2,
            cost: 0,
        }]);
        let taxi_path_nodes = TaxiPathNodeStore::from_entries([crate::TaxiPathNodeEntry {
            id: 1,
            loc: crate::Db2Position3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            path_id: 7,
            node_index: 0,
            continent_id: 0,
            flags: 0,
            delay: 0,
            arrival_event_id: 0,
            departure_event_id: 0,
        }]);

        assert!(valid_transport_template_like_cpp(
            100,
            &gameobject_templates,
            &taxi_paths,
            &taxi_path_nodes,
            &map_store,
        ));
        assert!(!valid_transport_template_like_cpp(
            999,
            &gameobject_templates,
            &taxi_paths,
            &taxi_path_nodes,
            &map_store,
        ));
    }

    #[test]
    fn player_create_cast_spell_expands_masks_and_modes_like_cpp() {
        let store = PlayerCreateInfoCastSpellStoreLikeCpp::from_rows_like_cpp([
            PlayerCreateInfoCastSpellRowLikeCpp {
                race_mask: race_mask_bit_like_cpp(1) | race_mask_bit_like_cpp(2),
                class_mask: class_mask_bit_like_cpp(1),
                spell_id: 100,
                create_mode: PLAYER_CREATE_MODE_NORMAL_LIKE_CPP as i8,
            },
            PlayerCreateInfoCastSpellRowLikeCpp {
                race_mask: 0,
                class_mask: 0,
                spell_id: 200,
                create_mode: PLAYER_CREATE_MODE_NPE_LIKE_CPP as i8,
            },
        ]);

        assert_eq!(store.cast_spells_like_cpp(1, 1, 0), &[100]);
        assert_eq!(store.cast_spells_like_cpp(2, 1, 0), &[100]);
        assert!(store.cast_spells_like_cpp(3, 1, 0).is_empty());
        assert_eq!(store.cast_spells_like_cpp(1, 1, 1), &[200]);
        assert_eq!(store.cast_spells_like_cpp(77, 13, 1), &[200]);
    }

    #[test]
    fn player_create_cast_spell_rejects_invalid_rows_like_cpp() {
        let store = PlayerCreateInfoCastSpellStoreLikeCpp::from_rows_like_cpp([
            PlayerCreateInfoCastSpellRowLikeCpp {
                race_mask: 1_u64 << 62,
                class_mask: 0,
                spell_id: 100,
                create_mode: 0,
            },
            PlayerCreateInfoCastSpellRowLikeCpp {
                race_mask: 0,
                class_mask: 1_u32 << 31,
                spell_id: 101,
                create_mode: 0,
            },
            PlayerCreateInfoCastSpellRowLikeCpp {
                race_mask: 0,
                class_mask: 0,
                spell_id: 102,
                create_mode: 2,
            },
            PlayerCreateInfoCastSpellRowLikeCpp {
                race_mask: 0,
                class_mask: 0,
                spell_id: 103,
                create_mode: -1,
            },
        ]);

        assert_eq!(
            *store.load_report_like_cpp(),
            PlayerCreateInfoCastSpellLoadReportLikeCpp {
                skipped_invalid_race_mask: 1,
                skipped_invalid_class_mask: 1,
                skipped_invalid_create_mode: 2,
                ..PlayerCreateInfoCastSpellLoadReportLikeCpp::default()
            }
        );
        assert!(store.cast_spells_like_cpp(1, 1, 0).is_empty());
    }

    #[test]
    fn player_create_custom_spell_expands_masks_like_cpp() {
        let store = PlayerCreateInfoCustomSpellStoreLikeCpp::from_rows_like_cpp([
            PlayerCreateInfoCustomSpellRowLikeCpp {
                race_mask: race_mask_bit_like_cpp(1) | race_mask_bit_like_cpp(2),
                class_mask: class_mask_bit_like_cpp(1),
                spell_id: 100,
            },
            PlayerCreateInfoCustomSpellRowLikeCpp {
                race_mask: 0,
                class_mask: 0,
                spell_id: 200,
            },
        ]);

        assert_eq!(store.custom_spells_like_cpp(1, 1), &[100, 200]);
        assert_eq!(store.custom_spells_like_cpp(2, 1), &[100, 200]);
        assert_eq!(store.custom_spells_like_cpp(3, 1), &[200]);
        assert_eq!(store.custom_spells_like_cpp(77, 13), &[200]);
    }

    #[test]
    fn player_create_custom_spell_rejects_invalid_rows_like_cpp() {
        let store = PlayerCreateInfoCustomSpellStoreLikeCpp::from_rows_like_cpp([
            PlayerCreateInfoCustomSpellRowLikeCpp {
                race_mask: 1_u64 << 62,
                class_mask: 0,
                spell_id: 100,
            },
            PlayerCreateInfoCustomSpellRowLikeCpp {
                race_mask: 0,
                class_mask: 1_u32 << 31,
                spell_id: 101,
            },
        ]);

        assert_eq!(
            *store.load_report_like_cpp(),
            PlayerCreateInfoCustomSpellLoadReportLikeCpp {
                skipped_invalid_race_mask: 1,
                skipped_invalid_class_mask: 1,
                ..PlayerCreateInfoCustomSpellLoadReportLikeCpp::default()
            }
        );
        assert!(store.custom_spells_like_cpp(1, 1).is_empty());
    }
}
