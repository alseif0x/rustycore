// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::HashMap;

use crate::session::state::{HubRef, SessionCore};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::session::RepresentedPlayerSkillLikeCpp;
use crate::session::represented_skill_values_from_records_like_cpp;
use wow_data::{
    ContentTuningStore, CurvePointStore, CurveStore, HeirloomStore, ItemSetEntry,
    ItemSetSpellEntry, ItemStatsStore, SpellStore,
};
use wow_entities::Player;

/// Read-only inputs for the ItemSet add/remove transition.
///
/// The capability retains only selected catalog references and exposes no
/// Player, fixture aggregate, or mutable runtime state.
pub struct OwnedItemSetAccessLikeCpp<'a> {
    core: &'a SessionCore,
    item_set_store: Option<&'a wow_data::ItemSetStore>,
    item_set_spell_store: Option<&'a wow_data::ItemSetSpellStore>,
    spell_store: Option<&'a SpellStore>,
    heirloom_store: Option<&'a HeirloomStore>,
    item_stats_store: Option<&'a ItemStatsStore>,
    curve_store: Option<&'a CurveStore>,
    curve_point_store: Option<&'a CurvePointStore>,
    content_tuning_store: Option<&'a ContentTuningStore>,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_skill_records: &'a HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    player_level: &'a u8,
    #[cfg(any(test, feature = "test-fixtures"))]
    primary_specialization_id: &'a u32,
}

impl SessionCore {
    /// Build ItemSet access from the selected catalogs and fixture inputs.
    pub fn owned_item_set_access_like_cpp<'a>(
        &'a self,
        item_set_store: Option<&'a wow_data::ItemSetStore>,
        item_set_spell_store: Option<&'a wow_data::ItemSetSpellStore>,
        spell_store: Option<&'a SpellStore>,
        heirloom_store: Option<&'a HeirloomStore>,
        item_stats_store: Option<&'a ItemStatsStore>,
        curve_store: Option<&'a CurveStore>,
        curve_point_store: Option<&'a CurvePointStore>,
        content_tuning_store: Option<&'a ContentTuningStore>,
        #[cfg(any(test, feature = "test-fixtures"))]
        player_skill_records: &'a HashMap<u16, RepresentedPlayerSkillLikeCpp>,
        #[cfg(any(test, feature = "test-fixtures"))]
        player_level: &'a u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        primary_specialization_id: &'a u32,
    ) -> OwnedItemSetAccessLikeCpp<'a> {
        OwnedItemSetAccessLikeCpp {
            core: self,
            item_set_store,
            item_set_spell_store,
            spell_store,
            heirloom_store,
            item_stats_store,
            curve_store,
            curve_point_store,
            content_tuning_store,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_skill_records,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            primary_specialization_id,
        }
    }
}

impl HubRef<'_> {
    /// Select only the catalog and fixture inputs consumed by ItemSet transitions.
    pub fn owned_item_set_access_like_cpp(&self) -> OwnedItemSetAccessLikeCpp<'_> {
        self.core.owned_item_set_access_like_cpp(
            self.catalogs.items.set_store.as_deref(),
            self.catalogs.spell_catalogs.item_set_spell_store.as_deref(),
            self.catalogs.spell_catalogs.spell_store.as_deref(),
            self.catalogs.heirloom_store.as_deref(),
            self.catalogs.items.stats_store.as_deref(),
            self.catalogs.curve_store.as_deref(),
            self.catalogs.curve_point_store.as_deref(),
            self.catalogs.content_tuning_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .progression
                .player_skill_test_fixture_like_cpp
                .player_skill_records_like_cpp,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.identity.player_level,
            #[cfg(any(test, feature = "test-fixtures"))]
            &self
                .fixtures
                .progression
                .represented_primary_specialization_id_like_cpp,
        )
    }
}

impl OwnedItemSetAccessLikeCpp<'_> {
    pub fn item_set_for_item_id_like_cpp(&self, item_id: u32) -> Option<&ItemSetEntry> {
        self.item_set_store?
            .item_set_for_item_id_like_cpp(item_id)
    }

    pub fn item_set_spells_like_cpp(&self, item_set_id: u32) -> Vec<&ItemSetSpellEntry> {
        self.item_set_spell_store
            .map(|store| store.item_set_spells_like_cpp(item_set_id))
            .unwrap_or_default()
    }

    pub fn represented_item_set_spell_exists_like_cpp(&self, spell_id: u32) -> bool {
        let Ok(spell_id) = i32::try_from(spell_id) else {
            return false;
        };
        self.spell_store
            .is_none_or(|store| store.get(spell_id).is_some())
    }

    pub fn resolved_player_skill_value_like_cpp(&self, skill_id: u16) -> Option<u16> {
        let records = self.core.resolved_player_skill_records_for_publication_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            self.player_skill_records,
        )?;
        Some(
            represented_skill_values_from_records_like_cpp(&records)
                .get(&skill_id)
                .copied()
                .unwrap_or(0),
        )
    }

    pub fn primary_specialization_id_like_cpp(
        &self,
        consumer_test: bool,
    ) -> Option<u32> {
        let canonical = self
            .core
            .with_owned_player_like_cpp(Player::primary_specialization_id_like_cpp);
        #[cfg(any(test, feature = "test-fixtures"))]
        if consumer_test && canonical.is_none() && self.core.player_handle_like_cpp.is_none() {
            return Some(*self.primary_specialization_id);
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        let _ = consumer_test;
        canonical
    }

    pub fn player_level_like_cpp(&self) -> u8 {
        #[cfg(any(test, feature = "test-fixtures"))]
        {
            self.core.player_level_with_fixture_like_cpp(self.player_level)
        }
        #[cfg(not(any(test, feature = "test-fixtures")))]
        {
            self.core.player_level_with_fixture_like_cpp()
        }
    }

    pub fn heirloom_store_like_cpp(&self) -> Option<&HeirloomStore> {
        self.heirloom_store
    }

    pub fn item_stats_store_like_cpp(&self) -> Option<&ItemStatsStore> {
        self.item_stats_store
    }

    pub fn curve_store_like_cpp(&self) -> Option<&CurveStore> {
        self.curve_store
    }

    pub fn curve_point_store_like_cpp(&self) -> Option<&CurvePointStore> {
        self.curve_point_store
    }

    pub fn content_tuning_store_like_cpp(&self) -> Option<&ContentTuningStore> {
        self.content_tuning_store
    }
}
