// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! CreatureDisplayInfo.db2 and CreatureModelData.db2 helpers used by Unit model math.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_database::{HotfixDatabase, HotfixStatements};

use crate::wdc4::Wdc4Reader;

pub const DEFAULT_COLLISION_HEIGHT_LIKE_CPP: f32 = 2.03128;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureDisplayInfoEntry {
    pub id: u32,
    pub model_id: u16,
    pub creature_model_scale: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CreatureModelDataEntry {
    pub id: u32,
    pub collision_height: f32,
    pub model_scale: f32,
    pub mount_height: f32,
}

pub struct CreatureDisplayInfoStore {
    entries: HashMap<u32, CreatureDisplayInfoEntry>,
}

pub struct CreatureModelDataStore {
    entries: HashMap<u32, CreatureModelDataEntry>,
}

// C++ CreatureModelDataLoadInfo has 34 logical columns, but WDC4 stores
// GeoBox[6] as one physical DB2Meta field. These indices are the physical
// CreatureModelDataMeta fields, not the hotfix SELECT column ordinals.
const CREATURE_MODEL_DATA_COLLISION_HEIGHT_DB2_FIELD: usize = 14;
const CREATURE_MODEL_DATA_MODEL_SCALE_DB2_FIELD: usize = 19;
const CREATURE_MODEL_DATA_MOUNT_HEIGHT_DB2_FIELD: usize = 23;

impl CreatureDisplayInfoStore {
    pub fn from_entries(entries: impl IntoIterator<Item = CreatureDisplayInfoEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("CreatureDisplayInfo.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = Vec::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.push(CreatureDisplayInfoEntry {
                id,
                model_id: reader.get_field_u16(idx, 1),
                creature_model_scale: f32::from_bits(reader.get_field_u32(idx, 4)),
            });
        }

        let store = Self::from_entries(entries);
        info!(
            "Loaded {} creature display infos from {}",
            store.len(),
            path.display()
        );
        Ok(store)
    }

    pub async fn load_with_hotfixes(
        data_dir: &str,
        locale: &str,
        hotfix_db: &HotfixDatabase,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        let hotfix_rows = store.load_hotfix_rows(hotfix_db).await?;
        if hotfix_rows != 0 {
            info!("Loaded {hotfix_rows} CreatureDisplayInfo hotfix rows");
        }
        Ok(store)
    }

    async fn load_hotfix_rows(&mut self, db: &HotfixDatabase) -> Result<usize> {
        let stmt = db.prepare(HotfixStatements::SEL_CREATURE_DISPLAY_INFO);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        loop {
            let entry = CreatureDisplayInfoEntry {
                id: result.read(0),
                model_id: result.read(1),
                creature_model_scale: result.read(4),
            };
            self.entries.insert(entry.id, entry);
            count += 1;

            if !result.next_row() {
                break;
            }
        }
        Ok(count)
    }

    pub fn get(&self, id: u32) -> Option<&CreatureDisplayInfoEntry> {
        self.entries.get(&id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl CreatureModelDataStore {
    pub fn from_entries(entries: impl IntoIterator<Item = CreatureModelDataEntry>) -> Self {
        Self {
            entries: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("CreatureModelData.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = Vec::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.push(CreatureModelDataEntry {
                id,
                collision_height: f32::from_bits(
                    reader.get_field_u32(idx, CREATURE_MODEL_DATA_COLLISION_HEIGHT_DB2_FIELD),
                ),
                model_scale: f32::from_bits(
                    reader.get_field_u32(idx, CREATURE_MODEL_DATA_MODEL_SCALE_DB2_FIELD),
                ),
                mount_height: f32::from_bits(
                    reader.get_field_u32(idx, CREATURE_MODEL_DATA_MOUNT_HEIGHT_DB2_FIELD),
                ),
            });
        }

        let store = Self::from_entries(entries);
        info!(
            "Loaded {} creature model data rows from {}",
            store.len(),
            path.display()
        );
        Ok(store)
    }

    pub async fn load_with_hotfixes(
        data_dir: &str,
        locale: &str,
        hotfix_db: &HotfixDatabase,
    ) -> Result<Self> {
        let mut store = Self::load(data_dir, locale)?;
        let hotfix_rows = store.load_hotfix_rows(hotfix_db).await?;
        if hotfix_rows != 0 {
            info!("Loaded {hotfix_rows} CreatureModelData hotfix rows");
        }
        Ok(store)
    }

    async fn load_hotfix_rows(&mut self, db: &HotfixDatabase) -> Result<usize> {
        let stmt = db.prepare(HotfixStatements::SEL_CREATURE_MODEL_DATA);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        loop {
            let entry = CreatureModelDataEntry {
                id: result.read(0),
                collision_height: result.read(20),
                model_scale: result.read(25),
                mount_height: result.read(29),
            };
            self.entries.insert(entry.id, entry);
            count += 1;

            if !result.next_row() {
                break;
            }
        }
        Ok(count)
    }

    pub fn get(&self, id: u32) -> Option<&CreatureModelDataEntry> {
        self.entries.get(&id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

pub fn unit_collision_height_like_cpp(
    object_scale: f32,
    native_display_id: u32,
    mount_display_id: Option<u32>,
    display_store: &CreatureDisplayInfoStore,
    model_store: &CreatureModelDataStore,
) -> Option<f32> {
    let native_display = display_store.get(native_display_id)?;
    let native_model = model_store.get(u32::from(native_display.model_id))?;

    if let Some(mount_display_id) = mount_display_id.filter(|id| *id != 0) {
        if let Some(mount_display) = display_store.get(mount_display_id) {
            if let Some(mount_model) = model_store.get(u32::from(mount_display.model_id)) {
                let height = object_scale
                    * ((mount_model.mount_height * mount_display.creature_model_scale)
                        + (native_model.collision_height
                            * native_model.model_scale
                            * native_display.creature_model_scale
                            * 0.5));
                return Some(if height == 0.0 {
                    DEFAULT_COLLISION_HEIGHT_LIKE_CPP
                } else {
                    height
                });
            }
        }
    }

    let height = object_scale
        * native_model.collision_height
        * native_model.model_scale
        * native_display.creature_model_scale;
    Some(if height == 0.0 {
        DEFAULT_COLLISION_HEIGHT_LIKE_CPP
    } else {
        height
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_collision_height_matches_cpp_mount_formula() {
        let displays = CreatureDisplayInfoStore::from_entries([
            CreatureDisplayInfoEntry {
                id: 10,
                model_id: 100,
                creature_model_scale: 1.2,
            },
            CreatureDisplayInfoEntry {
                id: 20,
                model_id: 200,
                creature_model_scale: 1.5,
            },
        ]);
        let models = CreatureModelDataStore::from_entries([
            CreatureModelDataEntry {
                id: 100,
                collision_height: 2.0,
                model_scale: 1.1,
                mount_height: 0.0,
            },
            CreatureModelDataEntry {
                id: 200,
                collision_height: 3.0,
                model_scale: 1.0,
                mount_height: 4.0,
            },
        ]);

        let height = unit_collision_height_like_cpp(2.0, 10, Some(20), &displays, &models).unwrap();

        assert_eq!(height, 2.0 * ((4.0 * 1.5) + (2.0 * 1.1 * 1.2 * 0.5)));
    }

    #[test]
    fn unit_collision_height_matches_cpp_dismount_formula_and_default() {
        let displays = CreatureDisplayInfoStore::from_entries([CreatureDisplayInfoEntry {
            id: 10,
            model_id: 100,
            creature_model_scale: 1.2,
        }]);
        let models = CreatureModelDataStore::from_entries([CreatureModelDataEntry {
            id: 100,
            collision_height: 0.0,
            model_scale: 1.1,
            mount_height: 0.0,
        }]);

        assert_eq!(
            unit_collision_height_like_cpp(2.0, 10, None, &displays, &models),
            Some(DEFAULT_COLLISION_HEIGHT_LIKE_CPP)
        );
    }

    #[test]
    fn load_creature_model_data_uses_physical_wdc4_fields_like_cpp_meta() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "enUS";
        let path = std::path::Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("CreatureModelData.db2");
        if !path.exists() {
            eprintln!(
                "Skipping test: CreatureModelData.db2 not found at {}",
                path.display()
            );
            return;
        }

        let store = CreatureModelDataStore::load(data_dir, locale)
            .expect("CreatureModelData.db2 should load with physical WDC4 field indices");
        assert!(!store.is_empty());
        assert!(store.entries.values().any(|entry| {
            entry.collision_height > 0.0 || entry.model_scale > 0.0 || entry.mount_height > 0.0
        }));
    }
}
