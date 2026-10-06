// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use crate::session::{MAX_SPECIALIZATIONS_LIKE_CPP, player_class_mask_for_talent_like_cpp};
use wow_data::TalentTabStore;

impl crate::session::HubMut<'_> {
    pub fn load_represented_talent_row_like_cpp(
        &mut self,
        talent_tabs: &TalentTabStore,
        talent_id: u32,
        rank: u8,
        talent_group: u8,
    ) -> bool {
        let talent_group_index = usize::from(talent_group);
        if talent_group_index >= MAX_SPECIALIZATIONS_LIKE_CPP {
            return false;
        }

        let Some(talent) = self
            .catalogs
            .talent_store()
            .and_then(|store| store.get(talent_id))
            .cloned()
        else {
            return false;
        };

        let Some(talent_tab) = talent_tabs.get(u32::from(talent.tab_id)) else {
            return false;
        };

        let Some(class_mask) =
            player_class_mask_for_talent_like_cpp(self.shared().player_class_like_cpp())
        else {
            return false;
        };

        let Ok(talent_class_mask) = u32::try_from(talent_tab.class_mask) else {
            return false;
        };
        if (class_mask & talent_class_mask) == 0 {
            return false;
        }

        let rank_index = usize::from(rank);
        let Some(spell_id) = talent.spell_rank.get(rank_index).copied() else {
            return false;
        };
        if spell_id <= 0 {
            return false;
        }

        if !self
            .shared()
            .represented_spell_valid_for_talent_like_cpp(spell_id)
        {
            return false;
        }

        self.install_loaded_talent_row_like_cpp(talent_group, talent_id, rank)
    }
}
