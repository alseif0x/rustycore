// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::OwnedItemSetAccessLikeCpp;

impl OwnedItemSetAccessLikeCpp<'_> {
    pub fn reborrow_like_cpp(&self) -> OwnedItemSetAccessLikeCpp<'_> {
        OwnedItemSetAccessLikeCpp {
            core: self.core,
            item_set_store: self.item_set_store,
            item_set_spell_store: self.item_set_spell_store,
            spell_store: self.spell_store,
            heirloom_store: self.heirloom_store,
            item_stats_store: self.item_stats_store,
            curve_store: self.curve_store,
            curve_point_store: self.curve_point_store,
            content_tuning_store: self.content_tuning_store,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_skill_records: self.player_skill_records,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_level: self.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            primary_specialization_id: self.primary_specialization_id,
        }
    }
}
