// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadSpawnGroupTemplates`.

use std::collections::HashMap;

pub const SPAWN_GROUP_FLAG_SYSTEM_LIKE_CPP: u32 = 0x01;
pub const SPAWN_GROUP_FLAG_COMPATIBILITY_MODE_LIKE_CPP: u32 = 0x02;
pub const SPAWN_GROUP_FLAG_MANUAL_SPAWN_LIKE_CPP: u32 = 0x04;
pub const SPAWN_GROUP_FLAG_DYNAMIC_SPAWN_RATE_LIKE_CPP: u32 = 0x08;
pub const SPAWN_GROUP_FLAG_ESCORT_QUEST_NPC_LIKE_CPP: u32 = 0x10;
pub const SPAWN_GROUP_FLAG_DESPAWN_ON_CONDITION_FAILURE_LIKE_CPP: u32 = 0x20;
pub const SPAWN_GROUP_FLAGS_ALL_LIKE_CPP: u32 = SPAWN_GROUP_FLAG_SYSTEM_LIKE_CPP
    | SPAWN_GROUP_FLAG_COMPATIBILITY_MODE_LIKE_CPP
    | SPAWN_GROUP_FLAG_MANUAL_SPAWN_LIKE_CPP
    | SPAWN_GROUP_FLAG_DYNAMIC_SPAWN_RATE_LIKE_CPP
    | SPAWN_GROUP_FLAG_ESCORT_QUEST_NPC_LIKE_CPP
    | SPAWN_GROUP_FLAG_DESPAWN_ON_CONDITION_FAILURE_LIKE_CPP;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnGroupTemplate {
    pub group_id: u32,
    pub name: String,
    pub flags: u32,
}

impl SpawnGroupTemplate {
    pub const fn is_system_like_cpp(&self) -> bool {
        self.flags & SPAWN_GROUP_FLAG_SYSTEM_LIKE_CPP != 0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnGroupTemplateRow {
    pub group_id: u32,
    pub name: String,
    pub flags: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpawnGroupTemplateLoadReport {
    pub loaded: usize,
    pub invalid_flags: Vec<(u32, u32, u32)>,
    pub system_manual_spawn_flags: Vec<u32>,
    pub inserted_default_groups: Vec<u32>,
}

#[derive(Debug, Clone, Default)]
pub struct SpawnGroupTemplateStore {
    templates: HashMap<u32, SpawnGroupTemplate>,
}

impl SpawnGroupTemplateStore {
    pub fn get(&self, group_id: u32) -> Option<&SpawnGroupTemplate> {
        self.templates.get(&group_id)
    }

    pub fn len(&self) -> usize {
        self.templates.len()
    }

    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &SpawnGroupTemplate> {
        self.templates.values()
    }

    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = SpawnGroupTemplateRow>,
    ) -> (Self, SpawnGroupTemplateLoadReport) {
        let mut store = Self::default();
        let mut report = SpawnGroupTemplateLoadReport::default();

        for row in rows {
            let original_flags = row.flags;
            let mut flags = row.flags & SPAWN_GROUP_FLAGS_ALL_LIKE_CPP;
            if flags != original_flags {
                report
                    .invalid_flags
                    .push((row.group_id, original_flags, flags));
            }

            if flags & SPAWN_GROUP_FLAG_SYSTEM_LIKE_CPP != 0
                && flags & SPAWN_GROUP_FLAG_MANUAL_SPAWN_LIKE_CPP != 0
            {
                flags &= !SPAWN_GROUP_FLAG_MANUAL_SPAWN_LIKE_CPP;
                report.system_manual_spawn_flags.push(row.group_id);
            }

            store.templates.insert(
                row.group_id,
                SpawnGroupTemplate {
                    group_id: row.group_id,
                    name: row.name,
                    flags,
                },
            );
        }

        if let std::collections::hash_map::Entry::Vacant(entry) = store.templates.entry(0) {
            entry.insert(SpawnGroupTemplate {
                group_id: 0,
                name: "Default Group".to_string(),
                flags: SPAWN_GROUP_FLAG_SYSTEM_LIKE_CPP,
            });
            report.inserted_default_groups.push(0);
        }

        if let std::collections::hash_map::Entry::Vacant(entry) = store.templates.entry(1) {
            entry.insert(SpawnGroupTemplate {
                group_id: 1,
                name: "Legacy Group".to_string(),
                flags: SPAWN_GROUP_FLAG_SYSTEM_LIKE_CPP
                    | SPAWN_GROUP_FLAG_COMPATIBILITY_MODE_LIKE_CPP,
            });
            report.inserted_default_groups.push(1);
        }

        report.loaded = store.templates.len();
        (store, report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(group_id: u32, flags: u32) -> SpawnGroupTemplateRow {
        SpawnGroupTemplateRow {
            group_id,
            name: format!("group {group_id}"),
            flags,
        }
    }

    #[test]
    fn spawn_group_templates_normalize_flags_and_insert_defaults_like_cpp() {
        let (store, report) = SpawnGroupTemplateStore::from_rows_like_cpp([
            row(
                10,
                SPAWN_GROUP_FLAG_SYSTEM_LIKE_CPP | SPAWN_GROUP_FLAG_MANUAL_SPAWN_LIKE_CPP,
            ),
            row(20, 0x100 | SPAWN_GROUP_FLAG_DYNAMIC_SPAWN_RATE_LIKE_CPP),
        ]);

        assert_eq!(report.loaded, 4);
        assert_eq!(report.system_manual_spawn_flags, vec![10]);
        assert_eq!(
            report.invalid_flags,
            vec![(
                20,
                0x100 | SPAWN_GROUP_FLAG_DYNAMIC_SPAWN_RATE_LIKE_CPP,
                0x08
            )]
        );
        assert_eq!(report.inserted_default_groups, vec![0, 1]);
        assert!(store.get(0).unwrap().is_system_like_cpp());
        assert!(store.get(1).unwrap().is_system_like_cpp());
        assert!(store.get(10).unwrap().is_system_like_cpp());
        assert_eq!(
            store.get(10).unwrap().flags & SPAWN_GROUP_FLAG_MANUAL_SPAWN_LIKE_CPP,
            0
        );
        assert!(!store.get(20).unwrap().is_system_like_cpp());
    }
}
