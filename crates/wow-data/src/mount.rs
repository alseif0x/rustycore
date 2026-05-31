// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Mount.db2 reader and C++ `DB2Manager::GetMount` lookup helpers.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;
use wow_database::{HotfixDatabase, HotfixStatements};

use crate::wdc4::Wdc4Reader;

pub const AREA_MOUNT_FLAG_ALLOW_GROUND_MOUNTS: u8 = 0x1;
pub const AREA_MOUNT_FLAG_ALLOW_FLYING_MOUNTS: u8 = 0x2;
pub const AREA_MOUNT_FLAG_ALLOW_SURFACE_SWIMMING_MOUNTS: u8 = 0x4;
pub const AREA_MOUNT_FLAG_ALLOW_UNDERWATER_SWIMMING_MOUNTS: u8 = 0x8;

pub const MOUNT_CAPABILITY_FLAG_GROUND: u8 = 0x1;
pub const MOUNT_CAPABILITY_FLAG_FLYING: u8 = 0x2;
pub const MOUNT_CAPABILITY_FLAG_FLOAT: u8 = 0x4;
pub const MOUNT_CAPABILITY_FLAG_UNDERWATER: u8 = 0x8;
pub const MOUNT_CAPABILITY_FLAG_IGNORE_RESTRICTIONS: u8 = 0x20;
pub const MOUNT_FLAG_SELF_MOUNT: u16 = 0x2;
pub const DISPLAYID_HIDDEN_MOUNT: i32 = 73_200;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MountEntry {
    pub id: u32,
    pub mount_type_id: u16,
    pub flags: u16,
    pub source_type_enum: i8,
    pub source_spell_id: i32,
    pub player_condition_id: u32,
    pub mount_fly_ride_height: f32,
    pub ui_model_scene_id: i32,
}

pub struct MountStore {
    by_id: HashMap<u32, MountEntry>,
    by_source_spell_id: HashMap<u32, u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountCapabilityEntry {
    pub id: u32,
    pub flags: u8,
    pub req_riding_skill: u16,
    pub req_area_id: u16,
    pub req_spell_aura_id: u32,
    pub req_spell_known_id: i32,
    pub mod_spell_aura_id: i32,
    pub req_map_id: i16,
}

pub struct MountCapabilityStore {
    by_id: HashMap<u32, MountCapabilityEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountCapabilityContextLikeCpp {
    pub riding_skill: u32,
    pub mount_flags: u8,
    pub is_submerged: bool,
    pub is_in_water: bool,
    pub map_id: i32,
    pub cosmetic_parent_map_id: i32,
    pub parent_map_id: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountTypeXCapabilityEntry {
    pub id: u32,
    pub mount_type_id: u16,
    pub mount_capability_id: u16,
    pub order_index: u8,
}

pub struct MountTypeXCapabilityStore {
    by_id: HashMap<u32, MountTypeXCapabilityEntry>,
    by_mount_type: HashMap<u16, Vec<MountTypeXCapabilityEntry>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MountXDisplayEntry {
    pub id: u32,
    pub creature_display_info_id: i32,
    pub player_condition_id: u32,
    pub mount_id: u32,
}

pub struct MountXDisplayStore {
    by_id: HashMap<u32, MountXDisplayEntry>,
    by_mount_id: HashMap<u32, Vec<MountXDisplayEntry>>,
}

impl MountStore {
    pub fn from_entries(entries: impl IntoIterator<Item = MountEntry>) -> Self {
        let mut by_id = HashMap::new();
        let mut by_source_spell_id = HashMap::new();
        for entry in entries {
            if let Ok(source_spell_id) = u32::try_from(entry.source_spell_id) {
                by_source_spell_id.insert(source_spell_id, entry.id);
            }
            by_id.insert(entry.id, entry);
        }

        Self {
            by_id,
            by_source_spell_id,
        }
    }

    fn rebuild_source_spell_index(&mut self) {
        self.by_source_spell_id.clear();
        for entry in self.by_id.values() {
            if let Ok(source_spell_id) = u32::try_from(entry.source_spell_id) {
                self.by_source_spell_id.insert(source_spell_id, entry.id);
            }
        }
    }

    /// Load Mount.db2 from `{data_dir}/dbc/{locale}/Mount.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::MountEntry`
    /// - `DB2LoadInfo.h::MountLoadInfo`
    /// - `DB2Stores.cpp` `_mountsBySpellId[mount->SourceSpellID] = mount`.
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("Mount.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = Vec::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.push(MountEntry {
                id,
                mount_type_id: reader.get_field_u16(idx, 3),
                flags: reader.get_field_u16(idx, 4),
                source_type_enum: reader.get_field_i8(idx, 5),
                source_spell_id: reader.get_field_i32(idx, 6),
                player_condition_id: reader.get_field_u32(idx, 7),
                mount_fly_ride_height: f32::from_bits(reader.get_field_u32(idx, 8)),
                ui_model_scene_id: reader.get_field_i32(idx, 9),
            });
        }

        let store = Self::from_entries(entries);
        info!("Loaded {} mounts from {}", store.len(), path.display());
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
            info!("Loaded {hotfix_rows} Mount hotfix rows");
        }
        Ok(store)
    }

    async fn load_hotfix_rows(&mut self, db: &HotfixDatabase) -> Result<usize> {
        let stmt = db.prepare(HotfixStatements::SEL_MOUNT);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        loop {
            let entry = MountEntry {
                id: result.read(3),
                mount_type_id: result.read(4),
                flags: result.read(5),
                source_type_enum: result.read(6),
                source_spell_id: result.read(7),
                player_condition_id: result.read(8),
                mount_fly_ride_height: result.read(9),
                ui_model_scene_id: result.read(10),
            };
            self.by_id.insert(entry.id, entry);
            count += 1;

            if !result.next_row() {
                break;
            }
        }

        self.rebuild_source_spell_index();
        Ok(count)
    }

    pub fn get_by_id(&self, id: u32) -> Option<&MountEntry> {
        self.by_id.get(&id)
    }

    pub fn get_by_source_spell_id_like_cpp(&self, spell_id: u32) -> Option<&MountEntry> {
        self.by_source_spell_id
            .get(&spell_id)
            .and_then(|id| self.by_id.get(id))
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

impl MountCapabilityStore {
    pub fn from_entries(entries: impl IntoIterator<Item = MountCapabilityEntry>) -> Self {
        Self {
            by_id: entries.into_iter().map(|entry| (entry.id, entry)).collect(),
        }
    }

    /// Load MountCapability.db2 from `{data_dir}/dbc/{locale}/MountCapability.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::MountCapabilityEntry`
    /// - `DB2LoadInfo.h::MountCapabilityLoadInfo`
    /// - `sMountCapabilityStore.LookupEntry`.
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("MountCapability.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = Vec::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.push(MountCapabilityEntry {
                id,
                flags: reader.get_field_u8(idx, 1),
                req_riding_skill: reader.get_field_u16(idx, 2),
                req_area_id: reader.get_field_u16(idx, 3),
                req_spell_aura_id: reader.get_field_u32(idx, 4),
                req_spell_known_id: reader.get_field_i32(idx, 5),
                mod_spell_aura_id: reader.get_field_i32(idx, 6),
                req_map_id: reader.get_field_i16(idx, 7),
            });
        }

        let store = Self::from_entries(entries);
        info!(
            "Loaded {} mount capabilities from {}",
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
            info!("Loaded {hotfix_rows} MountCapability hotfix rows");
        }
        Ok(store)
    }

    async fn load_hotfix_rows(&mut self, db: &HotfixDatabase) -> Result<usize> {
        let stmt = db.prepare(HotfixStatements::SEL_MOUNT_CAPABILITY);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        loop {
            let entry = MountCapabilityEntry {
                id: result.read(0),
                flags: result.read(1),
                req_riding_skill: result.read(2),
                req_area_id: result.read(3),
                req_spell_aura_id: result.read(4),
                req_spell_known_id: result.read(5),
                mod_spell_aura_id: result.read(6),
                req_map_id: result.read(7),
            };
            self.by_id.insert(entry.id, entry);
            count += 1;

            if !result.next_row() {
                break;
            }
        }

        Ok(count)
    }

    pub fn get(&self, id: u32) -> Option<&MountCapabilityEntry> {
        self.by_id.get(&id)
    }

    /// C++ `Unit::GetMountCapability` selection over already-computed runtime state.
    pub fn select_for_mount_type_like_cpp<AreaMatches, HasAura, HasSpell>(
        &self,
        type_store: &MountTypeXCapabilityStore,
        mount_type_id: u16,
        context: &MountCapabilityContextLikeCpp,
        area_matches: AreaMatches,
        has_aura: HasAura,
        has_spell: HasSpell,
    ) -> Option<&MountCapabilityEntry>
    where
        AreaMatches: Fn(u16) -> bool,
        HasAura: Fn(u32) -> bool,
        HasSpell: Fn(i32) -> bool,
    {
        if mount_type_id == 0 {
            return None;
        }

        let capabilities = type_store.capabilities_for_mount_type_like_cpp(mount_type_id)?;

        for mount_type_capability in capabilities {
            let Some(capability) = self.get(u32::from(mount_type_capability.mount_capability_id))
            else {
                continue;
            };

            if context.riding_skill < u32::from(capability.req_riding_skill) {
                continue;
            }

            if capability.flags & MOUNT_CAPABILITY_FLAG_IGNORE_RESTRICTIONS == 0 {
                if capability.flags & MOUNT_CAPABILITY_FLAG_GROUND != 0
                    && context.mount_flags & AREA_MOUNT_FLAG_ALLOW_GROUND_MOUNTS == 0
                {
                    continue;
                }
                if capability.flags & MOUNT_CAPABILITY_FLAG_FLYING != 0
                    && context.mount_flags & AREA_MOUNT_FLAG_ALLOW_FLYING_MOUNTS == 0
                {
                    continue;
                }
                if capability.flags & MOUNT_CAPABILITY_FLAG_FLOAT != 0
                    && context.mount_flags & AREA_MOUNT_FLAG_ALLOW_SURFACE_SWIMMING_MOUNTS == 0
                {
                    continue;
                }
                if capability.flags & MOUNT_CAPABILITY_FLAG_UNDERWATER != 0
                    && context.mount_flags & AREA_MOUNT_FLAG_ALLOW_UNDERWATER_SWIMMING_MOUNTS == 0
                {
                    continue;
                }
            }

            if !context.is_submerged {
                if !context.is_in_water {
                    if capability.flags & MOUNT_CAPABILITY_FLAG_GROUND == 0 {
                        continue;
                    }
                } else if capability.flags & MOUNT_CAPABILITY_FLAG_FLOAT == 0 {
                    continue;
                }
            } else if context.is_in_water {
                if capability.flags & MOUNT_CAPABILITY_FLAG_UNDERWATER == 0 {
                    continue;
                }
            } else if capability.flags & MOUNT_CAPABILITY_FLAG_FLOAT == 0 {
                continue;
            }

            if capability.req_map_id != -1
                && context.map_id != i32::from(capability.req_map_id)
                && context.cosmetic_parent_map_id != i32::from(capability.req_map_id)
                && context.parent_map_id != i32::from(capability.req_map_id)
            {
                continue;
            }

            if capability.req_area_id != 0 && !area_matches(capability.req_area_id) {
                continue;
            }

            if capability.req_spell_aura_id != 0 && !has_aura(capability.req_spell_aura_id) {
                continue;
            }

            if capability.req_spell_known_id != 0 && !has_spell(capability.req_spell_known_id) {
                continue;
            }

            return Some(capability);
        }

        None
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

impl MountTypeXCapabilityStore {
    pub fn from_entries(entries: impl IntoIterator<Item = MountTypeXCapabilityEntry>) -> Self {
        let mut by_id = HashMap::new();
        let mut by_mount_type = HashMap::<u16, Vec<MountTypeXCapabilityEntry>>::new();
        let mut seen_type_order = HashSet::<(u16, u8)>::new();

        for entry in entries {
            by_id.insert(entry.id, entry);
            // C++ stores pointers in `std::set` ordered by MountTypeID and OrderIndex.
            // For the same mount type and order index, the comparator treats rows as
            // equivalent, so later duplicates are not inserted.
            if seen_type_order.insert((entry.mount_type_id, entry.order_index)) {
                by_mount_type
                    .entry(entry.mount_type_id)
                    .or_default()
                    .push(entry);
            }
        }

        for entries in by_mount_type.values_mut() {
            entries.sort_by_key(|entry| entry.order_index);
        }

        Self {
            by_id,
            by_mount_type,
        }
    }

    fn rebuild_mount_type_index(&mut self) {
        let rebuilt = Self::from_entries(self.by_id.values().copied());
        self.by_mount_type = rebuilt.by_mount_type;
    }

    /// Load MountTypeXCapability.db2 from `{data_dir}/dbc/{locale}/MountTypeXCapability.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::MountTypeXCapabilityEntry`
    /// - `DB2Stores.cpp` `_mountCapabilitiesByType[MountTypeID].insert(...)`
    /// - `MountTypeXCapabilityEntryComparator`.
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("MountTypeXCapability.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = Vec::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            entries.push(MountTypeXCapabilityEntry {
                id,
                mount_type_id: reader.get_field_u16(idx, 0),
                mount_capability_id: reader.get_field_u16(idx, 1),
                order_index: reader.get_field_u8(idx, 2),
            });
        }

        let store = Self::from_entries(entries);
        info!(
            "Loaded {} mount type capability rows from {}",
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
            info!("Loaded {hotfix_rows} MountTypeXCapability hotfix rows");
        }
        Ok(store)
    }

    async fn load_hotfix_rows(&mut self, db: &HotfixDatabase) -> Result<usize> {
        let stmt = db.prepare(HotfixStatements::SEL_MOUNT_TYPE_X_CAPABILITY);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        loop {
            let entry = MountTypeXCapabilityEntry {
                id: result.read(0),
                mount_type_id: result.read(1),
                mount_capability_id: result.read(2),
                order_index: result.read(3),
            };
            self.by_id.insert(entry.id, entry);
            count += 1;

            if !result.next_row() {
                break;
            }
        }

        self.rebuild_mount_type_index();
        Ok(count)
    }

    pub fn get(&self, id: u32) -> Option<&MountTypeXCapabilityEntry> {
        self.by_id.get(&id)
    }

    pub fn capabilities_for_mount_type_like_cpp(
        &self,
        mount_type_id: u16,
    ) -> Option<&[MountTypeXCapabilityEntry]> {
        self.by_mount_type
            .get(&mount_type_id)
            .map(Vec::as_slice)
            .filter(|entries| !entries.is_empty())
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

impl MountXDisplayStore {
    pub fn from_entries(entries: impl IntoIterator<Item = MountXDisplayEntry>) -> Self {
        let mut by_id = HashMap::new();
        let mut by_mount_id = HashMap::<u32, Vec<MountXDisplayEntry>>::new();
        for entry in entries {
            by_mount_id.entry(entry.mount_id).or_default().push(entry);
            by_id.insert(entry.id, entry);
        }

        Self { by_id, by_mount_id }
    }

    fn rebuild_mount_index(&mut self) {
        self.by_mount_id.clear();
        for entry in self.by_id.values().copied() {
            self.by_mount_id
                .entry(entry.mount_id)
                .or_default()
                .push(entry);
        }
    }

    /// Load MountXDisplay.db2 from `{data_dir}/dbc/{locale}/MountXDisplay.db2`.
    ///
    /// C++ refs:
    /// - `DB2Structure.h::MountXDisplayEntry`
    /// - `DB2Stores.cpp` `_mountDisplays[MountID].push_back(...)`.
    pub fn load(data_dir: &str, locale: &str) -> Result<Self> {
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("MountXDisplay.db2");
        let reader = Wdc4Reader::open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        let mut entries = Vec::with_capacity(reader.total_count());
        for (id, idx) in reader.iter_records() {
            // C++ `MountXDisplayMeta` stores `MountID` as WDC4 relationship
            // data; the physical payload only has CreatureDisplayInfoID and
            // PlayerConditionID.
            entries.push(MountXDisplayEntry {
                id,
                creature_display_info_id: reader.get_field_i32(idx, 0),
                player_condition_id: reader.get_field_u32(idx, 1),
                mount_id: reader.get_relationship_id(idx).unwrap_or(0),
            });
        }

        let store = Self::from_entries(entries);
        info!(
            "Loaded {} mount display rows from {}",
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
            info!("Loaded {hotfix_rows} MountXDisplay hotfix rows");
        }
        Ok(store)
    }

    async fn load_hotfix_rows(&mut self, db: &HotfixDatabase) -> Result<usize> {
        let stmt = db.prepare(HotfixStatements::SEL_MOUNT_X_DISPLAY);
        let mut result = db.query(&stmt).await?;
        if result.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        loop {
            let entry = MountXDisplayEntry {
                id: result.read(0),
                creature_display_info_id: result.read(1),
                player_condition_id: result.read(2),
                mount_id: result.read(3),
            };
            self.by_id.insert(entry.id, entry);
            count += 1;

            if !result.next_row() {
                break;
            }
        }

        self.rebuild_mount_index();
        Ok(count)
    }

    pub fn get(&self, id: u32) -> Option<&MountXDisplayEntry> {
        self.by_id.get(&id)
    }

    pub fn displays_for_mount_like_cpp(&self, mount_id: u32) -> Option<&[MountXDisplayEntry]> {
        self.by_mount_id
            .get(&mount_id)
            .map(Vec::as_slice)
            .filter(|entries| !entries.is_empty())
    }

    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mount_store_indexes_by_source_spell_like_cpp() {
        let store = MountStore::from_entries([
            MountEntry {
                id: 1,
                mount_type_id: 0,
                flags: 0,
                source_type_enum: 0,
                source_spell_id: 100,
                player_condition_id: 42,
                mount_fly_ride_height: 0.0,
                ui_model_scene_id: 0,
            },
            MountEntry {
                id: 2,
                mount_type_id: 0,
                flags: 0,
                source_type_enum: 0,
                source_spell_id: -1,
                player_condition_id: 0,
                mount_fly_ride_height: 0.0,
                ui_model_scene_id: 0,
            },
        ]);

        let mount = store.get_by_source_spell_id_like_cpp(100).unwrap();
        assert_eq!(mount.id, 1);
        assert_eq!(mount.player_condition_id, 42);
        assert!(store.get_by_source_spell_id_like_cpp(999).is_none());
    }

    #[test]
    fn mount_type_capabilities_are_grouped_and_sorted_like_cpp_set() {
        let store = MountTypeXCapabilityStore::from_entries([
            MountTypeXCapabilityEntry {
                id: 1,
                mount_type_id: 7,
                mount_capability_id: 70,
                order_index: 2,
            },
            MountTypeXCapabilityEntry {
                id: 2,
                mount_type_id: 7,
                mount_capability_id: 71,
                order_index: 1,
            },
            MountTypeXCapabilityEntry {
                id: 3,
                mount_type_id: 7,
                mount_capability_id: 72,
                order_index: 1,
            },
        ]);

        let capabilities = store.capabilities_for_mount_type_like_cpp(7).unwrap();
        assert_eq!(
            capabilities
                .iter()
                .map(|entry| entry.mount_capability_id)
                .collect::<Vec<_>>(),
            vec![71, 70]
        );
        assert!(store.capabilities_for_mount_type_like_cpp(99).is_none());
    }

    #[test]
    fn mount_displays_are_grouped_by_mount_like_cpp() {
        let store = MountXDisplayStore::from_entries([
            MountXDisplayEntry {
                id: 1,
                creature_display_info_id: 1000,
                player_condition_id: 42,
                mount_id: 7,
            },
            MountXDisplayEntry {
                id: 2,
                creature_display_info_id: 1001,
                player_condition_id: 0,
                mount_id: 7,
            },
        ]);

        let displays = store.displays_for_mount_like_cpp(7).unwrap();
        assert_eq!(displays.len(), 2);
        assert_eq!(displays[0].creature_display_info_id, 1000);
        assert!(store.displays_for_mount_like_cpp(99).is_none());
    }

    #[test]
    fn load_mount_x_display_uses_cpp_parent_relationship_when_fixture_exists() {
        let data_dir = "/home/server/woltk-server-core/Data";
        let locale = "enUS";
        let path = Path::new(data_dir)
            .join("dbc")
            .join(locale)
            .join("MountXDisplay.db2");
        if !path.exists() {
            eprintln!("Skipping test: MountXDisplay.db2 not found");
            return;
        }

        let store =
            MountXDisplayStore::load(data_dir, locale).expect("failed to load MountXDisplay.db2");
        assert!(store.by_id.values().any(|display| display.mount_id != 0));
    }

    #[test]
    fn mount_capability_selection_matches_cpp_filter_order() {
        let capabilities = MountCapabilityStore::from_entries([
            MountCapabilityEntry {
                id: 10,
                flags: MOUNT_CAPABILITY_FLAG_FLYING,
                req_riding_skill: 0,
                req_area_id: 0,
                req_spell_aura_id: 0,
                req_spell_known_id: 0,
                mod_spell_aura_id: 1000,
                req_map_id: -1,
            },
            MountCapabilityEntry {
                id: 11,
                flags: MOUNT_CAPABILITY_FLAG_GROUND,
                req_riding_skill: 75,
                req_area_id: 77,
                req_spell_aura_id: 123,
                req_spell_known_id: 456,
                mod_spell_aura_id: 1001,
                req_map_id: 1,
            },
        ]);
        let type_caps = MountTypeXCapabilityStore::from_entries([
            MountTypeXCapabilityEntry {
                id: 1,
                mount_type_id: 7,
                mount_capability_id: 10,
                order_index: 0,
            },
            MountTypeXCapabilityEntry {
                id: 2,
                mount_type_id: 7,
                mount_capability_id: 11,
                order_index: 1,
            },
        ]);
        let context = MountCapabilityContextLikeCpp {
            riding_skill: 75,
            mount_flags: AREA_MOUNT_FLAG_ALLOW_GROUND_MOUNTS,
            is_submerged: false,
            is_in_water: false,
            map_id: 1,
            cosmetic_parent_map_id: -1,
            parent_map_id: -1,
        };

        let selected = capabilities
            .select_for_mount_type_like_cpp(
                &type_caps,
                7,
                &context,
                |area_id| area_id == 77,
                |aura_id| aura_id == 123,
                |spell_id| spell_id == 456,
            )
            .unwrap();
        assert_eq!(selected.id, 11);

        assert!(
            capabilities
                .select_for_mount_type_like_cpp(
                    &type_caps,
                    7,
                    &context,
                    |_| false,
                    |_| true,
                    |_| true,
                )
                .is_none()
        );
    }
}
