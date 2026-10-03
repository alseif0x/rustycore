use super::*;
use crate::wdc4::creation::{CreationDb2, CreationTable};
use std::path::Path;

impl AppearanceRecords {
    /// All seven normal-CASC baselines are mandatory. Checked numeric reads
    /// reject incomplete compressed data rather than substituting zero.
    pub fn load(directory: &Path) -> Result<Self> {
        let mut records = Self::default();
        let db = CreationDb2::open(directory, CreationTable::Model)?;
        for id in db.ids() {
            records.models.push(Model {
                id,
                display: db.bits(id, 4, 0)?,
            });
        }
        let db = CreationDb2::open(directory, CreationTable::Race)?;
        for id in db.ids() {
            records.races.push(Race {
                id,
                visual_parent: db.bits(id, 38, 0)? as u8,
            });
        }
        let db = CreationDb2::open(directory, CreationTable::RaceModel)?;
        for id in db.ids() {
            records.race_models.push(RaceModel {
                id,
                race: db.bits(id, 0, 0)?,
                model: db.bits(id, 1, 0)?,
                sex: db.bits(id, 2, 0)? as i8 as i32,
            });
        }
        let db = CreationDb2::open(directory, CreationTable::Option)?;
        for id in db.ids() {
            records.options.push(OptionRecord {
                id,
                model: db.bits(id, 4, 0)?,
                requirement: db.bits(id, 10, 0)?,
            });
        }
        let db = CreationDb2::open(directory, CreationTable::Choice)?;
        for id in db.ids() {
            records.choices.push(Choice {
                id,
                option: db.bits(id, 2, 0)?,
                requirement: db.bits(id, 3, 0)?,
            });
        }
        let db = CreationDb2::open(directory, CreationTable::Requirement)?;
        for id in db.ids() {
            records.requirements.push(Requirement {
                id,
                flags: db.bits(id, 1, 0)? as i32,
                class_mask: db.bits(id, 2, 0)? as i32,
                achievement: db.bits(id, 4, 0)? as i32,
                quest: db.bits(id, 5, 0)? as i32,
                item_appearance: db.bits(id, 7, 0)? as i32,
                race_mask: [db.bits(id, 8, 0)?, db.bits(id, 8, 1)?],
            });
        }
        let db = CreationDb2::open(directory, CreationTable::RequiredChoice)?;
        for id in db.ids() {
            records.required_choices.push(RequiredChoice {
                id,
                choice: db.bits(id, 0, 0)?,
                requirement: db.bits(id, 1, 0)?,
            });
        }
        Ok(records)
    }
}
