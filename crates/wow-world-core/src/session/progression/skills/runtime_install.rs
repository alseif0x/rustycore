// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::{BTreeSet, HashMap};

use crate::session::{
    PlayerSkillTestFixtureLikeCpp, RepresentedPlayerSkillLikeCpp, SessionCore,
    SKILL_ENCHANTING_LIKE_CPP, canonical_player_skill_record_like_cpp,
    represented_skill_values_from_records_like_cpp,
};

impl SessionCore {
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn replace_player_skill_runtime_exact_like_cpp(
        &self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        loaded: bool,
        complete: bool,
        occupied_slots: Option<u16>,
        tombstones: BTreeSet<u16>,
        fixture_inputs: (&mut PlayerSkillTestFixtureLikeCpp, &mut u16),
    ) -> bool {
        let canonical_records = skill_records
            .values()
            .copied()
            .map(canonical_player_skill_record_like_cpp)
            .collect();
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.replace_skill_records_like_cpp(
                    canonical_records,
                    loaded,
                    complete,
                    occupied_slots,
                    tombstones.clone(),
                );
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.player_handle_like_cpp.is_none() {
            let (skill_fixture, represented_enchanting_skill) = fixture_inputs;
            skill_fixture.player_skill_values_like_cpp =
                represented_skill_values_from_records_like_cpp(&skill_records);
            *represented_enchanting_skill = skill_records
                .get(&SKILL_ENCHANTING_LIKE_CPP)
                .map(|skill| skill.value)
                .unwrap_or(0);
            skill_fixture.player_skill_records_like_cpp = skill_records;
            skill_fixture.player_skill_non_durable_tombstones_like_cpp = tombstones;
            skill_fixture.player_skill_records_loaded_like_cpp = loaded;
            skill_fixture.player_skill_records_complete_like_cpp = loaded && complete;
            skill_fixture.player_skill_occupied_slots_like_cpp = occupied_slots;
            return true;
        }
        canonical
    }
}
