// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;

#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::HashMap;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::RepresentedPlayerSkillLikeCpp;
use crate::session::state::SessionCore;
use wow_data::{SkillLineStore, SkillStore, SkillTiersStoreLikeCpp};
use wow_packet::packets::update::{
    ActivePlayerDataValuesUpdate, SkillInfoValuesUpdate, UpdateObject,
};

pub(super) struct SkillValuesPublicationCxLikeCpp<'a> {
    core: &'a SessionCore,
    skill_store: Option<&'a Arc<SkillStore>>,
    skill_lines: Option<&'a Arc<SkillLineStore>>,
    skill_tiers: Option<&'a Arc<SkillTiersStoreLikeCpp>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_skill_records: &'a HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_race: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_class: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    fixture_level: &'a u8,
}

impl<'a> SkillValuesPublicationCxLikeCpp<'a> {
    pub(super) fn new(
        core: &'a SessionCore,
        skill_store: Option<&'a Arc<SkillStore>>,
        skill_lines: Option<&'a Arc<SkillLineStore>>,
        skill_tiers: Option<&'a Arc<SkillTiersStoreLikeCpp>>,
        #[cfg(any(test, feature = "test-fixtures"))]
        fixture_inputs: (
            &'a HashMap<u16, RepresentedPlayerSkillLikeCpp>,
            &'a u8,
            &'a u8,
            &'a u8,
        ),
    ) -> Self {
        #[cfg(any(test, feature = "test-fixtures"))]
        let (fixture_skill_records, fixture_race, fixture_class, fixture_level) = fixture_inputs;
        Self {
            core,
            skill_store,
            skill_lines,
            skill_tiers,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_skill_records,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_race,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_class,
            #[cfg(any(test, feature = "test-fixtures"))]
            fixture_level,
        }
    }

    pub(super) fn send_like_cpp(&self) {
        let (Some(guid), Some(skill_store), Some(skill_lines), Some(skill_tiers)) = (
            self.core.player_guid(),
            self.skill_store,
            self.skill_lines,
            self.skill_tiers,
        ) else {
            return;
        };
        #[cfg(any(test, feature = "test-fixtures"))]
        let Some(player_skill_records) = self
            .core
            .resolved_player_skill_records_for_publication_like_cpp(self.fixture_skill_records)
        else {
            return;
        };
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let Some(player_skill_records) = self
            .core
            .resolved_player_skill_records_for_publication_like_cpp()
        else {
            return;
        };
        let mut records = player_skill_records.values().collect::<Vec<_>>();
        records.sort_by_key(|record| record.skill_id);
        if records.len() > 256 {
            return;
        }

        let mut skill = SkillInfoValuesUpdate::default();
        let mut set_skill_bit = |bit: usize| {
            skill.skill_info_mask[bit / 32] |= 1 << (bit % 32);
        };
        set_skill_bit(0);
        for index in 0..256 {
            for bit in [
                1 + index,
                257 + index,
                513 + index,
                769 + index,
                1025 + index,
                1281 + index,
                1537 + index,
            ] {
                set_skill_bit(bit);
            }
        }
        for (index, record) in records.into_iter().enumerate() {
            #[cfg(any(test, feature = "test-fixtures"))]
            let race = self
                .core
                .player_race_with_fixture_like_cpp(self.fixture_race);
            #[cfg(not(any(test, feature = "test-fixtures")))]
            let race = self.core.player_race_with_fixture_like_cpp();
            #[cfg(any(test, feature = "test-fixtures"))]
            let class = self
                .core
                .player_class_with_fixture_like_cpp(self.fixture_class);
            #[cfg(not(any(test, feature = "test-fixtures")))]
            let class = self.core.player_class_with_fixture_like_cpp();
            #[cfg(any(test, feature = "test-fixtures"))]
            let level = self
                .core
                .player_level_with_fixture_like_cpp(self.fixture_level);
            #[cfg(not(any(test, feature = "test-fixtures")))]
            let level = self.core.player_level_with_fixture_like_cpp();
            if let Some(entry) = skill_store.loaded_skill_info_like_cpp(
                record.skill_id,
                race,
                class,
                level,
                record.value,
                record.max,
                skill_lines,
                skill_tiers,
            ) {
                skill.skill_line_id[index] = entry.skill_id;
                skill.skill_step[index] = record.step.max(entry.step);
                skill.skill_rank[index] = entry.rank;
                skill.skill_starting_rank[index] = entry.starting_rank;
                skill.skill_max_rank[index] = entry.max_rank;
                skill.skill_temp_bonus[index] = entry.temp_bonus;
                skill.skill_perm_bonus[index] = entry.perm_bonus;
            }
        }

        let mut data = ActivePlayerDataValuesUpdate {
            skill,
            ..Default::default()
        };
        data.active_player_data_mask[0] |= 1;
        data.active_player_data_mask[1] |= 1;
        self.core
            .send_packet(&UpdateObject::full_active_player_values_update(
                guid,
                self.core.player_map_id_like_cpp(),
                data,
            ));
    }
}
