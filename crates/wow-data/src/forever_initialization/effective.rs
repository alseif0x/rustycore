//! DB2Storage baseline -> official -> custom -> removed; derived indexes are
//! rebuilt only from final records. 02245dcd DB2Store.cpp::LoadFromDB,
//! DB2Stores.cpp:1267-1282,1491-1502,2189-2200; SharedDefines.h:174,292,1063.
use super::{
    ClassPowerRecord, ClassRecord, InitializationRecords, MapRecord, MovieRecord, PowerRecord,
    RaceRecord, SpecializationRecord,
};
use crate::Db2HotfixRemovalStoreLikeCpp;
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

pub const MAP_HASH: u32 = 0xBD84_CD62;
pub const POWER_HASH: u32 = 0x8D89_9A57;
pub const SPECIALIZATION_HASH: u32 = 0xA00F_8E60;
pub const CLASS_POWER_HASH: u32 = 0xC031_5ACF;
pub const MOVIE_HASH: u32 = 0x032D_FA13;
pub const CLASS_HASH: u32 = 0xF588_9D8C;
pub const RACE_HASH: u32 = 0x53F1_783C;
const MAX_POWERS: usize = 26;
const MAX_POWERS_PER_CLASS: usize = 10;
const MAX_CLASSES: u8 = 16;
const MAX_SPECIALIZATIONS: u8 = 5;
const INITIAL_SPECIALIZATION_INDEX: u8 = 4;
const PET_OVERRIDE_SPEC: i32 = 0x20;

/// Sole immutable effective record owner. Indexes contain IDs, not copied
/// record mirrors; no runtime mutation, SQL pools or Session authority here.
pub struct InitializationCatalog {
    maps: BTreeMap<u32, MapRecord>,
    powers: BTreeMap<u32, PowerRecord>,
    specializations: BTreeMap<u32, SpecializationRecord>,
    class_powers: BTreeMap<u32, ClassPowerRecord>,
    movies: BTreeMap<u32, MovieRecord>,
    classes: BTreeMap<u32, ClassRecord>,
    races: BTreeMap<u32, RaceRecord>,
    power_ids: [Option<u32>; MAX_POWERS],
    specialization_ids: BTreeMap<(u8, u8), u32>,
    class_power_ids: BTreeMap<u32, Vec<u32>>,
    unknown_baseline_map_records: usize,
}

impl InitializationRecords {
    pub fn finish(
        self,
        official: Self,
        custom: Self,
        removals: &Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<InitializationCatalog> {
        let maps = effective(
            self.maps,
            official.maps,
            custom.maps,
            MAP_HASH,
            removals,
            |row| row.id,
        )?;
        let classes = effective(
            self.classes,
            official.classes,
            custom.classes,
            CLASS_HASH,
            removals,
            |row| row.id,
        )?;
        ensure!(
            classes.keys().all(|id| *id <= u32::from(u8::MAX)),
            "Target class ID exceeds byte field"
        );
        let races = effective(
            self.races,
            official.races,
            custom.races,
            RACE_HASH,
            removals,
            |row| row.id,
        )?;
        let powers = effective(
            self.powers,
            official.powers,
            custom.powers,
            POWER_HASH,
            removals,
            |row| row.id,
        )?;
        let specializations = effective(
            self.specializations,
            official.specializations,
            custom.specializations,
            SPECIALIZATION_HASH,
            removals,
            |row| row.id,
        )?;
        let class_powers = effective(
            self.class_powers,
            official.class_powers,
            custom.class_powers,
            CLASS_POWER_HASH,
            removals,
            |row| row.id,
        )?;
        let movies = effective(
            self.movies,
            official.movies,
            custom.movies,
            MOVIE_HASH,
            removals,
            |row| row.id,
        )?;
        let mut power_ids = [None; MAX_POWERS];
        for row in powers.values() {
            // Source explicitly skips unknown enums and retains the FIRST
            // ascending-ID record for a duplicate power type, not the last.
            if let Ok(index) = usize::try_from(row.power_type) {
                if let Some(slot) = power_ids.get_mut(index) {
                    slot.get_or_insert(row.id);
                }
            }
        }
        let mut specialization_ids = BTreeMap::new();
        for row in specializations.values() {
            ensure!(
                row.class < MAX_CLASSES
                    && (0..MAX_SPECIALIZATIONS as i8).contains(&row.order_index),
                "Invalid target specialization index"
            );
            let class = if row.flags & PET_OVERRIDE_SPEC != 0 {
                ensure!(row.class == 0, "Invalid target pet specialization class");
                MAX_CLASSES
            } else {
                row.class
            };
            // Source index assignment retains LAST ascending-ID specialization
            // when two rows share a (storage class, order index).
            specialization_ids.insert((class, row.order_index as u8), row.id);
        }
        // DB2Stores.cpp:1165-1178,3489-3494: comparator is (class,power),
        // NOT record ID. Its set discards duplicate pairs; first ID wins.
        let mut by_pair = BTreeMap::new();
        for row in class_powers.values() {
            ensure!(
                row.class < u32::from(MAX_CLASSES)
                    && (0..MAX_POWERS as i8).contains(&row.power_type),
                "Invalid target class power index"
            );
            by_pair.entry((row.class, row.power_type)).or_insert(row.id);
        }
        let mut class_power_ids = BTreeMap::<u32, Vec<u32>>::new();
        for ((class, _), id) in by_pair {
            let ids = class_power_ids.entry(class).or_default();
            ensure!(
                ids.len() < MAX_POWERS_PER_CLASS,
                "Target class power capacity exceeded"
            );
            ids.push(id);
        }
        Ok(InitializationCatalog {
            maps,
            powers,
            specializations,
            power_ids,
            specialization_ids,
            class_powers,
            movies,
            classes,
            races,
            class_power_ids,
            unknown_baseline_map_records: self.unknown_map_records,
        })
    }
}

impl InitializationCatalog {
    /// Target ItemTemplate uses LookupEntry, not class/index selection.
    pub fn specialization_by_id(&self, id: u32) -> Option<&SpecializationRecord> {
        self.specializations.get(&id)
    }
    pub fn class(&self, id: u32) -> Option<&ClassRecord> {
        self.classes.get(&id)
    }
    pub fn race(&self, id: u32) -> Option<&RaceRecord> {
        self.races.get(&id)
    }
    pub fn identity_counts(&self) -> [usize; 2] {
        [self.classes.len(), self.races.len()]
    }
    /// Missing/unknown/removed is not an admitted map. None does not prove
    /// that a record in an unavailable baseline section never existed.
    pub fn map(&self, id: u32) -> Option<&MapRecord> {
        self.maps.get(&id)
    }

    pub fn power(&self, power_type: i8) -> Option<&PowerRecord> {
        let index = usize::try_from(power_type).ok()?;
        self.powers
            .get(&self.power_ids.get(index).copied().flatten()?)
    }

    pub fn class_power_types(&self, class: u32) -> impl Iterator<Item = i8> + '_ {
        self.class_power_ids
            .get(&class)
            .into_iter()
            .flatten()
            .map(|id| self.class_powers[id].power_type)
    }

    /// Source's missing index sentinel is MAX_POWERS_PER_CLASS. None expresses
    /// absence without making it a valid Rust array index or inventing a slot.
    pub fn class_power_index(&self, class: u32, power_type: i8) -> Option<u8> {
        self.class_power_types(class)
            .position(|kind| kind == power_type)
            .map(|index| index as u8)
    }

    pub fn movie(&self, id: u32) -> Option<&MovieRecord> {
        self.movies.get(&id)
    }

    /// Enumeration uses the saved talent-group index, NOT creation's default.
    pub fn specialization(&self, class: u8, index: u8) -> Option<&SpecializationRecord> {
        let id = self.specialization_ids.get(&(class, index))?;
        self.specializations.get(id)
    }

    /// Target Classic fallback: initial index 4, else first populated index
    /// 0..4. Recommended flags and lowest record ID are NOT that selection.
    pub fn default_specialization(&self, class: u8) -> Option<&SpecializationRecord> {
        if class >= MAX_CLASSES {
            return None;
        }
        let id = self
            .specialization_ids
            .get(&(class, INITIAL_SPECIALIZATION_INDEX))
            .or_else(|| {
                (0..MAX_SPECIALIZATIONS)
                    .find_map(|index| self.specialization_ids.get(&(class, index)))
            })?;
        self.specializations.get(id)
    }

    /// Effective record counts and baseline unknown count, never record data.
    pub fn counts(&self) -> [usize; 6] {
        [
            self.maps.len(),
            self.powers.len(),
            self.specializations.len(),
            self.class_powers.len(),
            self.movies.len(),
            self.unknown_baseline_map_records,
        ]
    }
}

fn effective<T>(
    baseline: Vec<T>,
    official: Vec<T>,
    custom: Vec<T>,
    hash: u32,
    removals: &Db2HotfixRemovalStoreLikeCpp,
    id: impl Fn(&T) -> u32,
) -> Result<BTreeMap<u32, T>> {
    let mut records = BTreeMap::new();
    for batch in [baseline, official, custom] {
        let mut seen = BTreeSet::new();
        for row in batch {
            let id = id(&row);
            ensure!(seen.insert(id), "Duplicate initialization batch ID");
            records.insert(id, row);
        }
    }
    records.retain(|&id, _| !removals.contains_like_cpp(hash, id as i32));
    Ok(records)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
