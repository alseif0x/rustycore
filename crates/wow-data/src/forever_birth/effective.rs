//! Final target birth records and ID-only indexes, never a learned Player.
//! DB2Store.cpp:127-133; DB2DatabaseLoader.cpp:27-174; DB2Stores.cpp:1539-1548
//! and ObjectMgr.cpp:3968-3989 at 02245dcd. Build-70170 hashes are acquired.
use super::{
    BirthRecords, LoadoutItemRecord, LoadoutRecord, SkillAbilityRecord, SkillLineRecord,
    SkillRaceClassRecord,
};
use crate::Db2HotfixRemovalStoreLikeCpp;
use anyhow::{Result, ensure};
use std::collections::BTreeMap;

pub const SKILL_LINE_HASH: u32 = 0xB53D_C9D6;
pub const RACE_CLASS_HASH: u32 = 0x06AD_E420;
pub const ABILITY_HASH: u32 = 0xFF44_46F6;
pub const LOADOUT_HASH: u32 = 0xE00A_47FB;
pub const LOADOUT_ITEM_HASH: u32 = 0x65C3_9BA7;

pub struct BirthCatalog {
    lines: BTreeMap<u32, SkillLineRecord>,
    race_class: BTreeMap<u32, SkillRaceClassRecord>,
    abilities: BTreeMap<u32, SkillAbilityRecord>,
    loadouts: BTreeMap<u32, LoadoutRecord>,
    items: BTreeMap<u32, LoadoutItemRecord>,
    children: BTreeMap<u32, Vec<u32>>,
    abilities_by_skill: BTreeMap<u32, Vec<u32>>,
    items_by_loadout: BTreeMap<u16, Vec<u32>>,
    unavailable_baseline_abilities: usize,
}

impl BirthRecords {
    /// Both overlay batches are consumed, then removals, then indexes.
    /// SQL keys are (ID,VerifiedBuild): duplicate overlay IDs are legal.
    /// Source Load overwrites in observed query-row order, not build-number
    /// order. No deterministic cross-database query ordering is claimed.
    pub fn finish(
        self,
        official: Self,
        custom: Self,
        removals: &Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<BirthCatalog> {
        let lines = compose(
            self.skill_lines,
            official.skill_lines,
            custom.skill_lines,
            SKILL_LINE_HASH,
            removals,
            |row| row.id,
        )?;
        let race_class = compose(
            self.race_class,
            official.race_class,
            custom.race_class,
            RACE_CLASS_HASH,
            removals,
            |row| row.id,
        )?;
        let abilities = compose(
            self.abilities,
            official.abilities,
            custom.abilities,
            ABILITY_HASH,
            removals,
            |row| row.id,
        )?;
        let loadouts = compose(
            self.loadouts,
            official.loadouts,
            custom.loadouts,
            LOADOUT_HASH,
            removals,
            |row| row.id,
        )?;
        let items = compose(
            self.loadout_items,
            official.loadout_items,
            custom.loadout_items,
            LOADOUT_ITEM_HASH,
            removals,
            |row| row.id,
        )?;
        let mut children = BTreeMap::<u32, Vec<u32>>::new();
        for row in lines.values().filter(|row| row.parent_skill != 0) {
            children.entry(row.parent_skill).or_default().push(row.id);
        }
        let mut abilities_by_skill = BTreeMap::<u32, Vec<u32>>::new();
        for row in abilities.values() {
            // Source conditional promotes int16/uint16 to int, then the
            // unordered_map key converts to uint32, including negative bits.
            let skill = if row.skillup_skill_line != 0 {
                row.skillup_skill_line as i32 as u32
            } else {
                u32::from(row.skill_line)
            };
            abilities_by_skill.entry(skill).or_default().push(row.id);
        }
        let mut items_by_loadout = BTreeMap::<u16, Vec<u32>>::new();
        for row in items.values() {
            items_by_loadout
                .entry(row.loadout)
                .or_default()
                .push(row.id);
        }
        Ok(BirthCatalog {
            lines,
            race_class,
            abilities,
            loadouts,
            items,
            children,
            abilities_by_skill,
            items_by_loadout,
            unavailable_baseline_abilities: self.unknown_ability_records,
        })
    }
}

impl BirthCatalog {
    pub fn skill_lines(&self) -> impl Iterator<Item = &SkillLineRecord> + '_ {
        self.lines.values()
    }
    pub fn race_class_record(&self, id: u32) -> Option<&SkillRaceClassRecord> {
        self.race_class.get(&id)
    }
    pub fn skill_line(&self, id: u32) -> Option<&SkillLineRecord> {
        self.lines.get(&id)
    }

    /// Canonical final relation payload, never a learned spell or fallback row.
    pub fn skill_ability(&self, id: u32) -> Option<&SkillAbilityRecord> {
        self.abilities.get(&id)
    }

    /// Complete known effective storage iteration, ascending ID. Unknown
    /// encrypted identities remain unavailable; no manufactured zero rows.
    pub fn skill_ability_records(&self) -> impl Iterator<Item = &SkillAbilityRecord> + '_ {
        self.abilities.values()
    }

    /// DB2 storage iteration is ascending ID. This is NOT the source
    /// unordered_multimap's first matching race/class candidate selection.
    pub fn race_class_records(&self) -> impl Iterator<Item = &SkillRaceClassRecord> + '_ {
        self.race_class.values()
    }

    pub fn child_lines(&self, parent: u32) -> impl Iterator<Item = &SkillLineRecord> + '_ {
        self.children
            .get(&parent)
            .into_iter()
            .flatten()
            .map(|id| &self.lines[id])
    }

    /// Source indexes by nonzero SkillupSkillLineID, else SkillLine, and
    /// preserves ascending storage-ID order inside each vector.
    pub fn abilities_for_skill(
        &self,
        skill: u32,
    ) -> impl Iterator<Item = &SkillAbilityRecord> + '_ {
        self.abilities_by_skill
            .get(&skill)
            .into_iter()
            .flatten()
            .map(|id| &self.abilities[id])
    }

    pub fn loadouts(&self) -> impl Iterator<Item = &LoadoutRecord> + '_ {
        self.loadouts.values()
    }

    /// Raw effective relations. ItemTemplate existence/count/equip admission
    /// belongs to the birth operation; it is never inferred from this index.
    pub fn items_for_loadout(&self, loadout: u16) -> impl Iterator<Item = &LoadoutItemRecord> + '_ {
        self.items_by_loadout
            .get(&loadout)
            .into_iter()
            .flatten()
            .map(|id| &self.items[id])
    }

    /// Last number is baseline acquisition uncertainty, not unresolved
    /// effective-ID count. Overlays/removals cannot certify full ID coverage.
    pub fn counts(&self) -> [usize; 6] {
        [
            self.lines.len(),
            self.race_class.len(),
            self.abilities.len(),
            self.loadouts.len(),
            self.items.len(),
            self.unavailable_baseline_abilities,
        ]
    }
}

fn compose<T>(
    baseline: Vec<T>,
    official: Vec<T>,
    custom: Vec<T>,
    hash: u32,
    removals: &Db2HotfixRemovalStoreLikeCpp,
    id: impl Fn(&T) -> u32,
) -> Result<BTreeMap<u32, T>> {
    let mut records = BTreeMap::new();
    for row in baseline {
        ensure!(
            records.insert(id(&row), row).is_none(),
            "Duplicate birth baseline ID"
        );
    }
    for row in official.into_iter().chain(custom) {
        records.insert(id(&row), row);
    }
    records.retain(|&id, _| !removals.contains_like_cpp(hash, id as i32));
    Ok(records)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
