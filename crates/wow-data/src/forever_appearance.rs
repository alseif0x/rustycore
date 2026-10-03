//! Build-70170 appearance data, separate from the legacy character mappings.
//! 02245dcd DB2Stores.cpp:1181-1249. Only fields consumed by creation are
//! projected; this is not a full DBQuery/hotfix wire serializer.
mod load;
#[cfg(test)]
mod tests;

use crate::Db2HotfixRemovalStoreLikeCpp;
use anyhow::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
pub struct Model {
    pub id: u32,
    pub display: u32,
}
#[derive(Clone, Copy)]
pub struct Race {
    pub id: u32,
    pub visual_parent: u8,
}
#[derive(Clone, Copy)]
pub struct RaceModel {
    pub id: u32,
    pub race: u32,
    pub model: u32,
    pub sex: i32,
}
#[derive(Clone, Copy)]
pub struct OptionRecord {
    pub id: u32,
    pub model: u32,
    pub requirement: u32,
}
#[derive(Clone, Copy)]
pub struct Choice {
    pub id: u32,
    pub option: u32,
    pub requirement: u32,
}
#[derive(Clone, Copy)]
pub struct Requirement {
    pub id: u32,
    pub flags: i32,
    pub class_mask: i32,
    pub race_mask: [u32; 2],
    pub achievement: i32,
    pub quest: i32,
    pub item_appearance: i32,
}
#[derive(Clone, Copy)]
pub struct RequiredChoice {
    pub id: u32,
    pub choice: u32,
    pub requirement: u32,
}

/// Transient startup batch, consumed before the immutable indexes are built.
#[derive(Default)]
pub struct AppearanceRecords {
    pub models: Vec<Model>,
    pub races: Vec<Race>,
    pub race_models: Vec<RaceModel>,
    pub options: Vec<OptionRecord>,
    pub choices: Vec<Choice>,
    pub requirements: Vec<Requirement>,
    pub required_choices: Vec<RequiredChoice>,
}

pub struct AppearanceCatalog {
    models: BTreeMap<(u8, u8), Model>,
    options: BTreeMap<(u8, u8), Vec<OptionRecord>>,
    choices: BTreeMap<u32, Vec<Choice>>,
    requirements: BTreeMap<u32, Requirement>,
    required_choices: BTreeMap<u32, BTreeMap<u32, Vec<u32>>>,
}

impl AppearanceRecords {
    /// C++ loads official rows before custom rows. No partial effective store
    /// is published if either batch contains ambiguous duplicate IDs.
    pub fn finish(
        self,
        official: Self,
        custom: Self,
        metadata: &Db2HotfixRemovalStoreLikeCpp,
    ) -> Result<AppearanceCatalog> {
        let models = effective(
            self.models,
            official.models,
            custom.models,
            0x49349C6E,
            metadata,
            |r| r.id,
        )?;
        let races = effective(
            self.races,
            official.races,
            custom.races,
            0x53F1783C,
            metadata,
            |r| r.id,
        )?;
        let race_models = effective(
            self.race_models,
            official.race_models,
            custom.race_models,
            0xA7E150FE,
            metadata,
            |r| r.id,
        )?;
        let options = effective(
            self.options,
            official.options,
            custom.options,
            0x2FB7905B,
            metadata,
            |r| r.id,
        )?;
        let choices = effective(
            self.choices,
            official.choices,
            custom.choices,
            0x681D0F3D,
            metadata,
            |r| r.id,
        )?;
        let requirements = effective(
            self.requirements,
            official.requirements,
            custom.requirements,
            0x61431A65,
            metadata,
            |r| r.id,
        )?;
        let required_choices = effective(
            self.required_choices,
            official.required_choices,
            custom.required_choices,
            0x9B1BEE48,
            metadata,
            |r| r.id,
        )?;
        let mut by_model = BTreeMap::<u32, Vec<OptionRecord>>::new();
        for option in options.into_values() {
            by_model.entry(option.model).or_default().push(option);
        }
        let mut choices_by_option = BTreeMap::<u32, Vec<Choice>>::new();
        for &choice in choices.values() {
            choices_by_option
                .entry(choice.option)
                .or_default()
                .push(choice);
        }
        let mut dependencies = BTreeMap::<u32, BTreeMap<u32, Vec<u32>>>::new();
        for required in required_choices.into_values() {
            if let Some(choice) = choices.get(&required.choice) {
                dependencies
                    .entry(required.requirement)
                    .or_default()
                    .entry(choice.option)
                    .or_default()
                    .push(choice.id);
            }
        }
        // Source stores one derived race per visual parent; last ascending ID
        // wins. This is intentionally not a recursive/all-descendants graph.
        let mut parent_races = BTreeMap::new();
        for race in races.into_values() {
            if race.visual_parent != 0 {
                parent_races.insert(u32::from(race.visual_parent), race.id);
            }
        }
        let mut indexed_models = BTreeMap::new();
        let mut indexed_options = BTreeMap::<(u8, u8), Vec<OptionRecord>>::new();
        for relation in race_models.into_values() {
            if let Some(&model) = models.get(&relation.model) {
                // Same narrowing as C++'s uint8 keys; unsupported large IDs
                // must not silently introduce a different target identity.
                let key = (u8::try_from(relation.race)?, u8::try_from(relation.sex)?);
                indexed_models.insert(key, model);
                if let Some(options) = by_model.get(&model.id) {
                    indexed_options
                        .entry(key)
                        .or_default()
                        .extend_from_slice(options);
                    if let Some(&race) = parent_races.get(&relation.race) {
                        indexed_options
                            .entry((u8::try_from(race)?, key.1))
                            .or_default()
                            .extend_from_slice(options);
                    }
                }
            }
        }
        Ok(AppearanceCatalog {
            models: indexed_models,
            options: indexed_options,
            choices: choices_by_option,
            requirements,
            required_choices: dependencies,
        })
    }
}

impl AppearanceCatalog {
    pub fn model(&self, race: u8, sex: u8) -> Option<&Model> {
        self.models.get(&(race, sex))
    }
    pub fn options(&self, race: u8, sex: u8) -> Option<&[OptionRecord]> {
        self.options.get(&(race, sex)).map(Vec::as_slice)
    }
    pub fn choices(&self, option: u32) -> Option<&[Choice]> {
        self.choices.get(&option).map(Vec::as_slice)
    }
    pub fn requirement(&self, id: u32) -> Option<&Requirement> {
        self.requirements.get(&id)
    }
    pub fn required_choices(&self, id: u32) -> Option<&BTreeMap<u32, Vec<u32>>> {
        self.required_choices.get(&id)
    }
    pub fn indexed_race_gender_count(&self) -> usize {
        self.options.len()
    }
}

fn effective<T>(
    baseline: Vec<T>,
    official: Vec<T>,
    custom: Vec<T>,
    hash: u32,
    metadata: &Db2HotfixRemovalStoreLikeCpp,
    id: impl Fn(&T) -> u32,
) -> Result<BTreeMap<u32, T>> {
    let mut records = BTreeMap::new();
    for batch in [baseline, official, custom] {
        let mut seen = BTreeSet::new();
        for record in batch {
            let id = id(&record);
            ensure!(seen.insert(id), "Duplicate appearance batch ID");
            records.insert(id, record);
        }
    }
    records.retain(|&id, _| !metadata.contains_like_cpp(hash, id as i32));
    Ok(records)
}
