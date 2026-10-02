// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Apply the C++ `LearnDefaultSkills` pass to the loaded skill rows during login.

use super::*;

impl WorldSession {
    pub(super) fn apply_default_skills_for_login_like_cpp(
        &mut self,
        race: u8,
        class: u8,
        level: u8,
        skill_records: &mut HashMap<u16, crate::session::RepresentedPlayerSkillLikeCpp>,
        skill_info_by_id: &mut BTreeMap<u16, wow_data::SkillInfoEntry>,
    ) -> Option<Vec<wow_data::SkillInfoEntry>> {
        crate::session::hub_mut(self).apply_default_skills_for_login_like_cpp(
            race,
            class,
            level,
            skill_records,
            skill_info_by_id,
        )
    }
}
