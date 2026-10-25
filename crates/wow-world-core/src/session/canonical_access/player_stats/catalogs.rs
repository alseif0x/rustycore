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

    pub fn aura(&self) -> Option<PlayerStatAuraCatalogs<'a>> {
        self.aura
    }

    pub fn item_enchantments(&self) -> Option<&'a SpellItemEnchantmentStore> {
        self.item_enchantments
    }

    pub fn limits(&self) -> StatsLimitsLikeCpp {
        self.limits
    }

    pub fn player_class_attack_power_coefficients(&self, class: u8) -> Option<(u8, u8, u8)> {
        self.chr_classes?.get(u32::from(class)).map(|entry| {
            (
                entry.attack_power_per_strength,
                entry.attack_power_per_agility,
                entry.ranged_attack_power_per_agility,
            )
        })
    }

    pub fn combat_rating_multiplier(&self, level: u8, rating: u32) -> f32 {
        self.combat_ratings
            .map(|table| table.rating_multiplier_like_cpp(u16::from(level), rating))
            .unwrap_or(1.0)
    }

    pub fn mana_regen_ratio(&self, level: u8, class: u8) -> f32 {
        self.regen_tables
            .map(|tables| tables.mana_regen_ratio_like_cpp(u16::from(level), class))
            .unwrap_or(0.0)
    }

    pub fn mana_regen_from_stats(&self, level: u8, class: u8, stats: [i32; 5]) -> f32 {
        (stats[3].max(0) as f32).sqrt() * stats[4] as f32 * self.mana_regen_ratio(level, class)
    }

    pub fn represented_item_fits_spell_requirements(
        &self,
        item_id: u32,
        equipped: &wow_data::SpellEquippedItemsEntry,
    ) -> bool {
        let Some(item) = self.item_store.and_then(|store| store.get(item_id)) else {
            return false;
        };

        if equipped.equipped_item_class >= 0 {
            if equipped.equipped_item_class != item.class_id as i8 {
                return false;
            }

            if equipped.equipped_item_subclass != 0 {
                let subclass = u32::from(item.subclass_id);
                if subclass >= i32::BITS
                    || (equipped.equipped_item_subclass & (1_i32 << subclass)) == 0
                {
                    return false;
                }
            }
        }

        true
    }

    pub fn represented_aura_spell_fits_weapon(
        &self,
        spell_id: i32,
        weapon_item_id: Option<u32>,
    ) -> bool {
        let Some(equipped) = self
            .spell_equipped_items
            .and_then(|store| store.entry_for_spell_id_like_cpp(spell_id))
        else {
            return true;
        };
        if equipped.equipped_item_class < 0 {
            return true;
        }
        weapon_item_id
            .is_some_and(|item_id| self.represented_item_fits_spell_requirements(item_id, equipped))
    }

    pub fn weapon_delay_seconds(&self, item_id: u32) -> f32 {
        self.item_stats
            .and_then(|store| store.weapon_template(item_id))
            .map(|weapon| f32::from(weapon.item_delay) / 1000.0)
            .unwrap_or(0.0)
    }

    pub fn shapeshift_combat_round_time(&self, form_id: u32) -> Option<f32> {
        let form = self.shapeshift_forms?.get(form_id)?;
        (form.combat_round_time > 0).then(|| f32::from(form.combat_round_time))
    }
}
