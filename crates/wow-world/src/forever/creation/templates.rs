//! CharacterTemplateDataStore.cpp:36-107 at 02245dcd, independent of legacy.
//! One immutable effective template owner; no SQL/session/packet authority.
use super::SourceError;
use std::collections::{HashMap, HashSet};
use wow_data::forever_initialization::InitializationCatalog;
use wow_persistence::forever::creation::templates::TemplateRows;

pub struct TemplateClass {
    class: u8,
    faction_group: u8,
}
impl TemplateClass {
    pub fn class(&self) -> u8 {
        self.class
    }
    pub fn faction_group(&self) -> u8 {
        self.faction_group
    }
}

/// No Debug: SQL labels/descriptions are wire data, not diagnostic messages.
pub struct CharacterTemplate {
    id: u32,
    classes: Vec<TemplateClass>,
    name: String,
    description: String,
    level: u8,
}
impl CharacterTemplate {
    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn classes(&self) -> &[TemplateClass] {
        &self.classes
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn description(&self) -> &str {
        &self.description
    }
    pub fn level(&self) -> u8 {
        self.level
    }
}

pub struct CharacterTemplates {
    // The C++ store is unordered too; no stable cross-language iteration order
    // is claimed. Native multi-template UI/wire acceptance is still required.
    records: HashMap<u32, CharacterTemplate>,
}
impl CharacterTemplates {
    pub fn load(
        rows: TemplateRows,
        initialization: &InitializationCatalog,
    ) -> Result<Self, SourceError> {
        let mut classes = HashMap::<u32, Vec<TemplateClass>>::new();
        for row in rows.classes {
            // DBCEnums.h FactionMasks: PLAYER=1, ALLIANCE=2, HORDE=4.
            // Source accepts extra bits and both teams; not just exact 3/5.
            if (row.faction_group & 3 != 3 && row.faction_group & 5 != 5)
                || initialization.class(u32::from(row.class)).is_none()
            {
                continue;
            }
            classes
                .entry(row.template_id)
                .or_default()
                .push(TemplateClass {
                    class: row.class,
                    faction_group: row.faction_group,
                });
        }
        let mut records = HashMap::new();
        let mut seen = HashSet::new();
        for row in rows.templates {
            // SQL primary-key violation is not a second valid template record.
            if !seen.insert(row.id) {
                return Err(SourceError::DuplicateIdentity);
            }
            let Some(classes) = classes.remove(&row.id) else {
                continue;
            };
            records.insert(
                row.id,
                CharacterTemplate {
                    id: row.id,
                    classes,
                    name: row.name,
                    description: row.description,
                    level: row.level,
                },
            );
        }
        Ok(Self { records })
    }
    pub fn get(&self, id: u32) -> Option<&CharacterTemplate> {
        self.records.get(&id)
    }
    pub fn iter(&self) -> impl Iterator<Item = &CharacterTemplate> {
        self.records.values()
    }
    pub fn count(&self) -> usize {
        self.records.len()
    }
}

#[cfg(test)]
mod tests;
