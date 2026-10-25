// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Borrowed catalog inputs selected for the shared character-stats planner.

use wow_data::character_progression::ChrClassesStore;
use wow_data::{
    CombatRatingsGameTableLikeCpp, ItemStatsStore, ItemStore, PlayerStatsStore,
    RegenGameTablesLikeCpp, SpellEquippedItemsStore, SpellItemEnchantmentStore,
    SpellShapeshiftFormStore, StatsLimitsLikeCpp,
};

use crate::session::{PlayerStatAuraCatalogs, SessionCatalogs};

/// Named immutable catalog references used by the existing stats planner.
///
/// Optional references preserve each catalog's current absence behavior; the
/// planner decides which absent stores are required for a particular query.
pub struct PlayerStatCatalogs<'a> {
    player_stats: Option<&'a PlayerStatsStore>,
    chr_classes: Option<&'a ChrClassesStore>,
    combat_ratings: Option<&'a CombatRatingsGameTableLikeCpp>,
    regen_tables: Option<&'a RegenGameTablesLikeCpp>,
    aura: Option<PlayerStatAuraCatalogs<'a>>,
    spell_equipped_items: Option<&'a SpellEquippedItemsStore>,
    item_store: Option<&'a ItemStore>,
    item_stats: Option<&'a ItemStatsStore>,
    item_enchantments: Option<&'a SpellItemEnchantmentStore>,
    shapeshift_forms: Option<&'a SpellShapeshiftFormStore>,
    limits: StatsLimitsLikeCpp,
}

impl<'a> PlayerStatCatalogs<'a> {
    /// Select catalog references and copied limits without doing store queries.
    pub fn from_session_catalogs(
        catalogs: &'a SessionCatalogs,
        limits: StatsLimitsLikeCpp,
    ) -> Self {
        Self::from_refs(
            catalogs.player_stats.as_deref(),
            catalogs.chr.classes_store.as_deref(),
            catalogs.combat_ratings_game_table.as_deref(),
            catalogs.regen_game_tables.as_deref(),
            catalogs.player_stat_aura_catalogs(),
            catalogs
                .spell_catalogs
                .spell_equipped_items_store
                .as_deref(),
            catalogs.items.store.as_deref(),
            catalogs.items.stats_store.as_deref(),
            catalogs
                .spell_catalogs
                .spell_item_enchantment_store
                .as_deref(),
            catalogs
                .spell_catalogs
                .spell_shapeshift_form_store
                .as_deref(),
            limits,
        )
    }

    /// Assemble the same selected inputs from references supplied by a
    /// non-Session owner, such as the native Map planner.
    #[allow(clippy::too_many_arguments)]
    pub fn from_refs(
        player_stats: Option<&'a PlayerStatsStore>,
        chr_classes: Option<&'a ChrClassesStore>,
        combat_ratings: Option<&'a CombatRatingsGameTableLikeCpp>,
        regen_tables: Option<&'a RegenGameTablesLikeCpp>,
        aura: Option<PlayerStatAuraCatalogs<'a>>,
        spell_equipped_items: Option<&'a SpellEquippedItemsStore>,
        item_store: Option<&'a ItemStore>,
        item_stats: Option<&'a ItemStatsStore>,
        item_enchantments: Option<&'a SpellItemEnchantmentStore>,
        shapeshift_forms: Option<&'a SpellShapeshiftFormStore>,
        limits: StatsLimitsLikeCpp,
    ) -> Self {
        Self {
            player_stats,
            chr_classes,
            combat_ratings,
            regen_tables,
            aura,
            spell_equipped_items,
            item_store,
            item_stats,
            item_enchantments,
            shapeshift_forms,
            limits,
        }
    }

    pub fn player_stats(&self) -> Option<&'a PlayerStatsStore> {
        self.player_stats
    }

    pub fn chr_classes(&self) -> Option<&'a ChrClassesStore> {
        self.chr_classes
    }

    pub fn combat_ratings(&self) -> Option<&'a CombatRatingsGameTableLikeCpp> {
        self.combat_ratings
    }

    pub fn regen_tables(&self) -> Option<&'a RegenGameTablesLikeCpp> {
        self.regen_tables
    }

    pub fn aura(&self) -> Option<PlayerStatAuraCatalogs<'a>> {
        self.aura
    }

    pub fn spell_equipped_items(&self) -> Option<&'a SpellEquippedItemsStore> {
        self.spell_equipped_items
    }

    pub fn item_store(&self) -> Option<&'a ItemStore> {
        self.item_store
    }

    pub fn item_stats(&self) -> Option<&'a ItemStatsStore> {
        self.item_stats
    }

    pub fn item_enchantments(&self) -> Option<&'a SpellItemEnchantmentStore> {
        self.item_enchantments
    }

    pub fn shapeshift_forms(&self) -> Option<&'a SpellShapeshiftFormStore> {
        self.shapeshift_forms
    }

    pub fn limits(&self) -> StatsLimitsLikeCpp {
        self.limits
    }
}
