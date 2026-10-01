// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn load_represented_account_heirlooms_like_cpp(
        &mut self,
        heirloom_rows: impl IntoIterator<Item = (u32, u32)>,
    ) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_represented_account_heirlooms_like_cpp(&mut hub, heirloom_rows)
    }
    pub(crate) fn load_represented_account_toys_like_cpp(
        &mut self,
        toy_rows: impl IntoIterator<Item = (u32, bool, bool)>,
    ) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_represented_account_toys_like_cpp(&mut hub, toy_rows)
    }
    pub(crate) fn mark_represented_glyphs_loaded_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.mark_represented_glyphs_loaded_like_cpp(&mut hub)
    }
    pub(crate) fn load_represented_glyph_row_like_cpp(
        &mut self,
        glyph_properties: &GlyphPropertiesStore,
        talent_group: u8,
        glyph_slot: u8,
        glyph_id: u16,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.load_represented_glyph_row_like_cpp(
            &mut hub,
            glyph_properties,
            talent_group,
            glyph_slot,
            glyph_id,
        )
    }
    #[cfg(test)]
    pub(crate) fn apply_loaded_player_flags_to_canonical_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.apply_loaded_player_flags_to_canonical_like_cpp(&mut hub)
    }
    #[cfg(test)]
    pub(crate) fn player_skill_records_loaded_like_cpp(&self) -> bool {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.player_skill_records_loaded_like_cpp(hub)
    }
    #[cfg(test)]
    pub(in crate::session) fn load_instance_time_restriction_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = (u32, u64)>,
    ) {
        crate::session::cx_lifecycle(self).load_instance_time_restriction_rows_like_cpp(rows)
    }
}
