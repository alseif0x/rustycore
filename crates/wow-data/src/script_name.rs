// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptIdLikeCpp(pub u32);

impl ScriptIdLikeCpp {
    pub const NONE: Self = Self(0);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptNameEntryLikeCpp {
    pub id: ScriptIdLikeCpp,
    pub is_script_database_bound: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptNameInternerLikeCpp {
    name_to_entry: BTreeMap<String, ScriptNameEntryLikeCpp>,
    index_to_name: Vec<String>,
}

impl Default for ScriptNameInternerLikeCpp {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptNameInternerLikeCpp {
    pub fn new() -> Self {
        let mut interner = Self {
            name_to_entry: BTreeMap::new(),
            index_to_name: Vec::new(),
        };
        let id = interner.insert_like_cpp("", false);
        debug_assert_eq!(id, ScriptIdLikeCpp::NONE);
        interner
    }

    pub fn reserve_like_cpp(&mut self, capacity: usize) {
        self.index_to_name.reserve(capacity);
    }

    pub fn insert_like_cpp(
        &mut self,
        script_name: impl AsRef<str>,
        is_script_database_bound: bool,
    ) -> ScriptIdLikeCpp {
        let script_name = script_name.as_ref();
        if let Some(entry) = self.name_to_entry.get(script_name) {
            return entry.id;
        }

        let id = ScriptIdLikeCpp(
            u32::try_from(self.name_to_entry.len())
                .expect("script name container exceeded u32 id range"),
        );
        self.name_to_entry.insert(
            script_name.to_string(),
            ScriptNameEntryLikeCpp {
                id,
                is_script_database_bound,
            },
        );
        self.index_to_name.push(script_name.to_string());
        id
    }

    pub fn len_like_cpp(&self) -> usize {
        self.index_to_name.len()
    }

    pub fn find_by_id_like_cpp(
        &self,
        id: ScriptIdLikeCpp,
    ) -> Option<(&str, &ScriptNameEntryLikeCpp)> {
        let name = self.index_to_name.get(usize::try_from(id.0).ok()?)?;
        self.name_to_entry
            .get_key_value(name.as_str())
            .map(|(name, entry)| (name.as_str(), entry))
    }

    pub fn find_by_name_like_cpp(&self, name: &str) -> Option<&ScriptNameEntryLikeCpp> {
        if name.is_empty() {
            return None;
        }

        self.name_to_entry.get(name)
    }

    pub fn get_script_name_like_cpp(&self, id: ScriptIdLikeCpp) -> &str {
        self.find_by_id_like_cpp(id)
            .map(|(name, _)| name)
            .unwrap_or("")
    }

    pub fn is_script_database_bound_like_cpp(&self, id: ScriptIdLikeCpp) -> bool {
        self.find_by_id_like_cpp(id)
            .map(|(_, entry)| entry.is_script_database_bound)
            .unwrap_or(false)
    }

    pub fn get_script_id_like_cpp(
        &mut self,
        name: impl AsRef<str>,
        is_database_bound: bool,
    ) -> ScriptIdLikeCpp {
        self.insert_like_cpp(name, is_database_bound)
    }

    pub fn all_db_script_names_like_cpp(&self) -> BTreeSet<String> {
        self.name_to_entry
            .iter()
            .filter_map(|(name, entry)| entry.is_script_database_bound.then(|| name.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_name_interner_reserves_empty_id_zero_like_cpp() {
        let interner = ScriptNameInternerLikeCpp::new();

        assert_eq!(interner.len_like_cpp(), 1);
        assert_eq!(interner.get_script_name_like_cpp(ScriptIdLikeCpp::NONE), "");
        assert!(interner.find_by_name_like_cpp("").is_none());
        assert!(!interner.is_script_database_bound_like_cpp(ScriptIdLikeCpp::NONE));
    }

    #[test]
    fn script_name_interner_assigns_stable_insertion_ids_like_cpp() {
        let mut interner = ScriptNameInternerLikeCpp::new();

        let first = interner.get_script_id_like_cpp("boss_one", true);
        let second = interner.get_script_id_like_cpp("boss_two", false);
        let first_again = interner.get_script_id_like_cpp("boss_one", false);

        assert_eq!(first, ScriptIdLikeCpp(1));
        assert_eq!(second, ScriptIdLikeCpp(2));
        assert_eq!(first_again, first);
        assert_eq!(interner.get_script_name_like_cpp(first), "boss_one");
        assert_eq!(interner.get_script_name_like_cpp(second), "boss_two");
        assert_eq!(interner.get_script_name_like_cpp(ScriptIdLikeCpp(99)), "");
    }

    #[test]
    fn script_name_interner_keeps_first_database_bound_flag_like_cpp() {
        let mut interner = ScriptNameInternerLikeCpp::new();

        let db_bound = interner.get_script_id_like_cpp("npc_from_db", true);
        let code_only = interner.get_script_id_like_cpp("npc_code_only", false);
        let db_bound_again = interner.get_script_id_like_cpp("npc_from_db", false);
        let code_only_again = interner.get_script_id_like_cpp("npc_code_only", true);

        assert_eq!(db_bound_again, db_bound);
        assert_eq!(code_only_again, code_only);
        assert!(interner.is_script_database_bound_like_cpp(db_bound));
        assert!(!interner.is_script_database_bound_like_cpp(code_only));
        assert_eq!(
            interner.all_db_script_names_like_cpp(),
            BTreeSet::from(["npc_from_db".to_string()])
        );
    }
}
