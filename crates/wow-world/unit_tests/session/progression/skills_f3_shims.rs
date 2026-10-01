// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    #[allow(dead_code)]
    pub(crate) fn set_player_skill_values_like_cpp(
        &mut self,
        skill_values: HashMap<u16, u16>,
    ) -> bool {
        crate::session::hub_mut(self).set_player_skill_values_like_cpp(skill_values)
    }
    pub(crate) fn set_player_skill_records_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    ) -> bool {
        crate::session::hub_mut(self).set_player_skill_records_like_cpp(skill_records)
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_replace_player_skill_records_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        loaded: bool,
        complete: bool,
    ) -> bool {
        crate::session::hub_mut(self).fixture_replace_player_skill_records_like_cpp(
            skill_records,
            loaded,
            complete,
        )
    }
    #[cfg(test)]
    pub(in crate::session) fn replace_player_skill_runtime_exact_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        loaded: bool,
        complete: bool,
        occupied_slots: Option<u16>,
        tombstones: BTreeSet<u16>,
    ) -> bool {
        crate::session::hub_mut(self).replace_player_skill_runtime_exact_like_cpp(
            skill_records,
            loaded,
            complete,
            occupied_slots,
            tombstones,
        )
    }
    pub(crate) fn set_player_skill_occupied_slots_like_cpp(&mut self, occupied_slots: u16) -> bool {
        crate::session::hub_mut(self).set_player_skill_occupied_slots_like_cpp(occupied_slots)
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_set_player_skill_occupied_slots_like_cpp(
        &mut self,
        occupied_slots: u16,
    ) -> bool {
        crate::session::hub_mut(self)
            .fixture_set_player_skill_occupied_slots_like_cpp(occupied_slots)
    }
    #[cfg(test)]
    pub(in crate::session) fn player_skill_max_value_like_cpp(&self, skill_id: u16) -> u16 {
        crate::session::hub_ref(self).player_skill_max_value_like_cpp(skill_id)
    }
    #[cfg(test)]
    pub(crate) fn player_skill_records_like_cpp(
        &self,
    ) -> HashMap<u16, RepresentedPlayerSkillLikeCpp> {
        crate::session::hub_ref(self).player_skill_records_like_cpp()
    }
    #[cfg(test)]
    pub(crate) fn player_skill_value_like_cpp(&self, skill_id: u16) -> u16 {
        crate::session::hub_ref(self).player_skill_value_like_cpp(skill_id)
    }
    pub(crate) fn set_complete_player_skill_records_like_cpp(
        &mut self,
        skill_records: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        occupied_slots: u16,
    ) -> bool {
        crate::session::hub_mut(self)
            .set_complete_player_skill_records_like_cpp(skill_records, occupied_slots)
    }
}
