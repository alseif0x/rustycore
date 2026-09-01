// Copyright (c) 2026 alseif0x
// RustyCore - WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 - https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `ObjectMgr::LoadNPCSpellClickSpells` data store.

use std::collections::HashMap;

pub const UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP: u64 = 0x01000000;

pub const SPELL_CLICK_USER_ANY_LIKE_CPP: u8 = 0;
pub const SPELL_CLICK_USER_FRIEND_LIKE_CPP: u8 = 1;
pub const SPELL_CLICK_USER_RAID_LIKE_CPP: u8 = 2;
pub const SPELL_CLICK_USER_PARTY_LIKE_CPP: u8 = 3;
pub const SPELL_CLICK_USER_MAX_LIKE_CPP: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellClickInfoLikeCpp {
    pub spell_id: u32,
    pub cast_flags: u8,
    pub user_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NpcSpellClickRowLikeCpp {
    pub npc_entry: u32,
    pub spell_id: u32,
    pub cast_flags: u8,
    pub user_type: u8,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NpcSpellClickLoadReportLikeCpp {
    pub loaded: usize,
    pub skipped_missing_creature_template: usize,
    pub skipped_missing_spell: usize,
    pub invalid_user_type_logged_but_loaded_like_cpp: usize,
}

#[derive(Debug, Clone, Default)]
pub struct NpcSpellClickStoreLikeCpp {
    entries: HashMap<u32, Vec<SpellClickInfoLikeCpp>>,
    load_report: NpcSpellClickLoadReportLikeCpp,
}

impl NpcSpellClickStoreLikeCpp {
    pub fn from_rows_like_cpp(
        rows: impl IntoIterator<Item = NpcSpellClickRowLikeCpp>,
        mut creature_template_exists: impl FnMut(u32) -> bool,
        mut spell_exists: impl FnMut(u32) -> bool,
    ) -> Self {
        let mut entries = HashMap::<u32, Vec<SpellClickInfoLikeCpp>>::new();
        let mut load_report = NpcSpellClickLoadReportLikeCpp::default();

        for row in rows {
            if !creature_template_exists(row.npc_entry) {
                load_report.skipped_missing_creature_template += 1;
                continue;
            }

            if !spell_exists(row.spell_id) {
                load_report.skipped_missing_spell += 1;
                continue;
            }

            if row.user_type >= SPELL_CLICK_USER_MAX_LIKE_CPP {
                load_report.invalid_user_type_logged_but_loaded_like_cpp += 1;
            }

            entries
                .entry(row.npc_entry)
                .or_default()
                .push(SpellClickInfoLikeCpp {
                    spell_id: row.spell_id,
                    cast_flags: row.cast_flags,
                    user_type: row.user_type,
                });
            load_report.loaded += 1;
        }

        Self {
            entries,
            load_report,
        }
    }

    pub fn spell_click_info_map_bounds_like_cpp(&self, npc_entry: u32) -> &[SpellClickInfoLikeCpp] {
        self.entries
            .get(&npc_entry)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn has_spell_click_info_like_cpp(&self, npc_entry: u32) -> bool {
        self.entries.contains_key(&npc_entry)
    }

    pub fn templates_with_spellclick_flag_but_no_data_like_cpp(
        &self,
        template_npc_flags: impl IntoIterator<Item = (u32, u64)>,
    ) -> Vec<u32> {
        let mut missing = template_npc_flags
            .into_iter()
            .filter_map(|(entry, npc_flags)| {
                ((npc_flags & UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP) != 0
                    && !self.has_spell_click_info_like_cpp(entry))
                .then_some(entry)
            })
            .collect::<Vec<_>>();
        missing.sort_unstable();
        missing
    }

    pub fn load_report_like_cpp(&self) -> &NpcSpellClickLoadReportLikeCpp {
        &self.load_report
    }

    pub fn len(&self) -> usize {
        self.load_report.loaded
    }

    pub fn is_empty(&self) -> bool {
        self.load_report.loaded == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npc_spellclick_rows_skip_missing_creature_and_spell_like_cpp() {
        let store = NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
            [
                NpcSpellClickRowLikeCpp {
                    npc_entry: 100,
                    spell_id: 200,
                    cast_flags: 1,
                    user_type: SPELL_CLICK_USER_ANY_LIKE_CPP,
                },
                NpcSpellClickRowLikeCpp {
                    npc_entry: 101,
                    spell_id: 200,
                    cast_flags: 2,
                    user_type: SPELL_CLICK_USER_FRIEND_LIKE_CPP,
                },
                NpcSpellClickRowLikeCpp {
                    npc_entry: 100,
                    spell_id: 201,
                    cast_flags: 3,
                    user_type: SPELL_CLICK_USER_PARTY_LIKE_CPP,
                },
            ],
            |entry| entry == 100,
            |spell| spell == 200,
        );

        assert_eq!(store.len(), 1);
        assert_eq!(
            store.spell_click_info_map_bounds_like_cpp(100),
            &[SpellClickInfoLikeCpp {
                spell_id: 200,
                cast_flags: 1,
                user_type: SPELL_CLICK_USER_ANY_LIKE_CPP,
            }]
        );
        assert_eq!(
            store.load_report_like_cpp(),
            &NpcSpellClickLoadReportLikeCpp {
                loaded: 1,
                skipped_missing_creature_template: 1,
                skipped_missing_spell: 1,
                invalid_user_type_logged_but_loaded_like_cpp: 0,
            }
        );
    }

    #[test]
    fn npc_spellclick_invalid_user_type_is_logged_but_loaded_like_cpp_code_path() {
        let store = NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
            [NpcSpellClickRowLikeCpp {
                npc_entry: 100,
                spell_id: 200,
                cast_flags: 7,
                user_type: SPELL_CLICK_USER_MAX_LIKE_CPP,
            }],
            |entry| entry == 100,
            |spell| spell == 200,
        );

        assert_eq!(store.len(), 1);
        assert_eq!(
            store.spell_click_info_map_bounds_like_cpp(100),
            &[SpellClickInfoLikeCpp {
                spell_id: 200,
                cast_flags: 7,
                user_type: SPELL_CLICK_USER_MAX_LIKE_CPP,
            }]
        );
        assert_eq!(
            store
                .load_report_like_cpp()
                .invalid_user_type_logged_but_loaded_like_cpp,
            1
        );
    }

    #[test]
    fn npc_spellclick_reports_spellclick_templates_without_data_like_cpp() {
        let store = NpcSpellClickStoreLikeCpp::from_rows_like_cpp(
            [NpcSpellClickRowLikeCpp {
                npc_entry: 100,
                spell_id: 200,
                cast_flags: 1,
                user_type: SPELL_CLICK_USER_ANY_LIKE_CPP,
            }],
            |entry| entry == 100,
            |spell| spell == 200,
        );

        assert_eq!(
            store.templates_with_spellclick_flag_but_no_data_like_cpp([
                (99, UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP),
                (100, UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP),
                (101, 0),
            ]),
            vec![99]
        );
    }
}
