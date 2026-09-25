// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Apply the C++ `LearnDefaultSkills` pass to the loaded skill rows during login.

use super::*;

impl WorldSession {
    /// Returns the newly learned default skill entries, or `None` after
    /// kicking when the canonical Player skill owner is unavailable.
    pub(super) fn apply_default_skills_for_login_like_cpp(
        &mut self,
        race: u8,
        class: u8,
        level: u8,
        skill_records: &mut HashMap<u16, crate::session::RepresentedPlayerSkillLikeCpp>,
        skill_info_by_id: &mut BTreeMap<u16, wow_data::SkillInfoEntry>,
    ) -> Option<Vec<wow_data::SkillInfoEntry>> {
        // C++ calls `LearnDefaultSkills` after `_LoadSkills`, `_LoadSpells`
        // and quest-status loading. Only Availability == 1 rows at or below
        // the player's level are candidates; `LearnDefaultSkill` computes the
        // range-specific value/max and `SetSkill` immediately runs
        // `LearnSkillRewardedSpells` with that real value.
        let mut default_skill_entries = Vec::new();
        if let (Some(skill_store), Some(skill_line_store), Some(skill_tiers_store)) = (
            self.skill_store().cloned(),
            self.skill_line_store().cloned(),
            self.skill_tiers_store().cloned(),
        ) {
            for entry in skill_store.default_starting_skill_info_like_cpp(
                race,
                class,
                level,
                skill_line_store.as_ref(),
                skill_tiers_store.as_ref(),
            ) {
                if skill_records
                    .get(&entry.skill_id)
                    .is_some_and(|skill| skill.value > 0)
                {
                    continue;
                }
                if skill_info_by_id.len() >= 256 {
                    break;
                }

                let profession_slot = skill_records
                    .get(&entry.skill_id)
                    .map(|skill| skill.profession_slot)
                    .unwrap_or(-1);
                let state = skill_records
                    .get(&entry.skill_id)
                    .map(|skill| {
                        if skill.state
                            == crate::session::RepresentedPlayerSkillStateLikeCpp::Deleted
                        {
                            crate::session::RepresentedPlayerSkillStateLikeCpp::Changed
                        } else {
                            crate::session::RepresentedPlayerSkillStateLikeCpp::New
                        }
                    })
                    .unwrap_or(crate::session::RepresentedPlayerSkillStateLikeCpp::New);
                skill_records.insert(
                    entry.skill_id,
                    crate::session::RepresentedPlayerSkillLikeCpp {
                        skill_id: entry.skill_id,
                        step: entry.step,
                        value: entry.rank,
                        max: entry.max_rank,
                        profession_slot,
                        state,
                    },
                );
                skill_info_by_id.insert(entry.skill_id, entry);
                default_skill_entries.push(entry);
            }
            if !self.replace_player_skill_records_like_cpp(skill_records.clone(), true, false) {
                self.kick("canonical Player skill owner unavailable while applying default skills");
                return None;
            }
        }
        Some(default_skill_entries)
    }
}
