// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! Mount.db2 reader and C++ `DB2Manager::GetMount` lookup helpers.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use anyhow::{Context, Result};
use tracing::info;

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

/// C++ `CollectionMgr::FactionSpecificMounts` loaded from `mount_definitions`.
pub struct MountDefinitionStoreLikeCpp {
    other_faction_by_spell_id: HashMap<u32, u32>,
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

/// Diagnostic-only reason for a failed C++ `Unit::GetMountCapability` pass.
///
/// Selection still returns only a capability or `None` to callers that do not
/// need the reason. The reject value is used by runtime logs to distinguish
/// data/config problems from normal C++ restrictions such as riding skill,
/// area mount flags, map/area gates or required known spells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountCapabilityRejectLikeCpp {
    EmptyMountType,
    MissingMountTypeCapabilities,
    MissingCapabilityRow,
    RidingSkill,
    AreaMountFlags,
    LiquidState,
    Map,
    Area,
    Aura,
    KnownSpell,
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
                mount_type_id: reader.get_field_u16(idx, 4),
                flags: reader.get_field_u16(idx, 5),
                source_type_enum: reader.get_field_i8(idx, 6),
                source_spell_id: reader.get_field_i32(idx, 7),
                player_condition_id: reader.get_field_u32(idx, 8),
                mount_fly_ride_height: reader.get_field_f32(idx, 9),
                ui_model_scene_id: reader.get_field_i32(idx, 10),
            });
        }

        let store = Self::from_entries(entries);
        info!("Loaded {} mounts from {}", store.len(), path.display());
        Ok(store)
    }

    /// Apply the effective Hotfix rows after the base DB2 file has loaded.
    /// C++ `DB2Storage::LoadFromDB` replaces rows by record ID before
    /// `DB2Manager` rebuilds `_mountsBySpellId`.
    pub fn apply_hotfix_entries_like_cpp(
        &mut self,
        entries: impl IntoIterator<Item = MountEntry>,
    ) -> usize {
        let mut count = 0;
        for entry in entries {
            self.by_id.insert(entry.id, entry);
            count += 1;
        }

        self.rebuild_source_spell_index();
        count
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

impl MountDefinitionStoreLikeCpp {
    pub fn from_entries(entries: impl IntoIterator<Item = (u32, u32)>) -> Self {
        Self {
            other_faction_by_spell_id: entries.into_iter().collect(),
        }
    }

    /// Build `mount_definitions` with the same DB2 validation as
    /// C++ `CollectionMgr::LoadMountDefinitions`.
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = (u32, u32)>,
        mount_store: &MountStore,
    ) -> Self {
        let mut entries = Vec::new();
        for (spell_id, other_faction_spell_id) in rows {
            if mount_store
                .get_by_source_spell_id_like_cpp(spell_id)
                .is_some()
                && (other_faction_spell_id == 0
                    || mount_store
                        .get_by_source_spell_id_like_cpp(other_faction_spell_id)
                        .is_some())
            {
                entries.push((spell_id, other_faction_spell_id));
            }
        }

        Self::from_entries(entries)
    }

    pub fn other_faction_spell_id_like_cpp(&self, spell_id: u32) -> Option<u32> {
        self.other_faction_by_spell_id.get(&spell_id).copied()
    }

    pub fn len(&self) -> usize {
        self.other_faction_by_spell_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.other_faction_by_spell_id.is_empty()
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

    /// Apply effective Hotfix rows by ID after the base DB2 load.
    pub fn apply_hotfix_entries_like_cpp(
        &mut self,
        entries: impl IntoIterator<Item = MountCapabilityEntry>,
    ) -> usize {
        let mut count = 0;
        for entry in entries {
            self.by_id.insert(entry.id, entry);
            count += 1;
        }
        count
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
        self.select_for_mount_type_with_reject_like_cpp(
            type_store,
            mount_type_id,
            context,
            area_matches,
            has_aura,
            has_spell,
        )
        .ok()
    }

    pub fn select_for_mount_type_with_reject_like_cpp<AreaMatches, HasAura, HasSpell>(
        &self,
        type_store: &MountTypeXCapabilityStore,
        mount_type_id: u16,
        context: &MountCapabilityContextLikeCpp,
        area_matches: AreaMatches,
        has_aura: HasAura,
        has_spell: HasSpell,
    ) -> Result<&MountCapabilityEntry, MountCapabilityRejectLikeCpp>
    where
        AreaMatches: Fn(u16) -> bool,
        HasAura: Fn(u32) -> bool,
        HasSpell: Fn(i32) -> bool,
    {
        if mount_type_id == 0 {
            return Err(MountCapabilityRejectLikeCpp::EmptyMountType);
        }

        let capabilities = type_store
            .capabilities_for_mount_type_like_cpp(mount_type_id)
            .ok_or(MountCapabilityRejectLikeCpp::MissingMountTypeCapabilities)?;
        let mut reject = None;

        for mount_type_capability in capabilities {
            let Some(capability) = self.get(u32::from(mount_type_capability.mount_capability_id))
            else {
                reject = Some(MountCapabilityRejectLikeCpp::MissingCapabilityRow);
                continue;
            };

            if context.riding_skill < u32::from(capability.req_riding_skill) {
                reject = Some(MountCapabilityRejectLikeCpp::RidingSkill);
                continue;
            }

            if capability.flags & MOUNT_CAPABILITY_FLAG_IGNORE_RESTRICTIONS == 0 {
                if capability.flags & MOUNT_CAPABILITY_FLAG_GROUND != 0
                    && context.mount_flags & AREA_MOUNT_FLAG_ALLOW_GROUND_MOUNTS == 0
                {
                    reject = Some(MountCapabilityRejectLikeCpp::AreaMountFlags);
                    continue;
                }
                if capability.flags & MOUNT_CAPABILITY_FLAG_FLYING != 0
                    && context.mount_flags & AREA_MOUNT_FLAG_ALLOW_FLYING_MOUNTS == 0
                {
                    reject = Some(MountCapabilityRejectLikeCpp::AreaMountFlags);
                    continue;
                }
                if capability.flags & MOUNT_CAPABILITY_FLAG_FLOAT != 0
                    && context.mount_flags & AREA_MOUNT_FLAG_ALLOW_SURFACE_SWIMMING_MOUNTS == 0
                {
                    reject = Some(MountCapabilityRejectLikeCpp::AreaMountFlags);
                    continue;
                }
                if capability.flags & MOUNT_CAPABILITY_FLAG_UNDERWATER != 0
                    && context.mount_flags & AREA_MOUNT_FLAG_ALLOW_UNDERWATER_SWIMMING_MOUNTS == 0
                {
                    reject = Some(MountCapabilityRejectLikeCpp::AreaMountFlags);
                    continue;
                }
            }

            if !context.is_submerged {
                if !context.is_in_water {
                    if capability.flags & MOUNT_CAPABILITY_FLAG_GROUND == 0 {
                        reject = Some(MountCapabilityRejectLikeCpp::LiquidState);
                        continue;
                    }
                } else if capability.flags & MOUNT_CAPABILITY_FLAG_FLOAT == 0 {
                    reject = Some(MountCapabilityRejectLikeCpp::LiquidState);
                    continue;
                }
            } else if context.is_in_water {
                if capability.flags & MOUNT_CAPABILITY_FLAG_UNDERWATER == 0 {
                    reject = Some(MountCapabilityRejectLikeCpp::LiquidState);
                    continue;
                }
            } else if capability.flags & MOUNT_CAPABILITY_FLAG_FLOAT == 0 {
                reject = Some(MountCapabilityRejectLikeCpp::LiquidState);
                continue;
            }

            if capability.req_map_id != -1
                && context.map_id != i32::from(capability.req_map_id)
                && context.cosmetic_parent_map_id != i32::from(capability.req_map_id)
                && context.parent_map_id != i32::from(capability.req_map_id)
            {
                reject = Some(MountCapabilityRejectLikeCpp::Map);
                continue;
            }

            if capability.req_area_id != 0 && !area_matches(capability.req_area_id) {
                reject = Some(MountCapabilityRejectLikeCpp::Area);
                continue;
            }

            if capability.req_spell_aura_id != 0 && !has_aura(capability.req_spell_aura_id) {
                reject = Some(MountCapabilityRejectLikeCpp::Aura);
                continue;
            }

            if capability.req_spell_known_id != 0 && !has_spell(capability.req_spell_known_id) {
                reject = Some(MountCapabilityRejectLikeCpp::KnownSpell);
                continue;
            }

            return Ok(capability);
        }

        Err(reject.unwrap_or(MountCapabilityRejectLikeCpp::MissingCapabilityRow))
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

    /// Apply effective Hotfix rows by ID and rebuild the C++ ordered index.
    pub fn apply_hotfix_entries_like_cpp(
        &mut self,
        entries: impl IntoIterator<Item = MountTypeXCapabilityEntry>,
    ) -> usize {
        let mut count = 0;
        for entry in entries {
            self.by_id.insert(entry.id, entry);
            count += 1;
        }

        self.rebuild_mount_type_index();
        count
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

    /// Apply effective Hotfix rows by ID and rebuild the relationship index.
    pub fn apply_hotfix_entries_like_cpp(
        &mut self,
        entries: impl IntoIterator<Item = MountXDisplayEntry>,
    ) -> usize {
        let mut count = 0;
        for entry in entries {
            self.by_id.insert(entry.id, entry);
            count += 1;
        }

        self.rebuild_mount_index();
        count
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
#[path = "mount/tests/mod.rs"]
mod tests;
