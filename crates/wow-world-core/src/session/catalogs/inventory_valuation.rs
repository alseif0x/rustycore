// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;

use wow_constants::ItemBonusType;
use wow_data::progression_rewards::{
    ContentTuningStore, CurvePointStore, CurveStore, FactionStore,
    FriendshipRepReactionStore,
};
use wow_data::{
    AreaTableStore, ChrSpecializationStore, ItemBonusDb2Store, ItemEffectStore,
    ItemLimitCategoryConditionStore, ItemLimitCategoryStore, ItemSearchNameStore, ItemStatsStore,
    ItemStore, MapStore, PlayerConditionStore, PvpItemStore,
};
use wow_entities::ItemStorageTemplate;

use crate::session::state::SessionCatalogs;

/// Borrowed immutable stores used by item-level and equipability evaluation.
/// This view selects only those catalogs; it owns no `Arc` and snapshots none.
#[derive(Clone, Copy)]
pub struct InventoryValuationCatalogViewLikeCpp<'a> {
    item_store: Option<&'a Arc<ItemStore>>,
    item_stats_store: Option<&'a Arc<ItemStatsStore>>,
    item_bonus_db2_store: Option<&'a Arc<ItemBonusDb2Store>>,
    pvp_item_store: Option<&'a Arc<PvpItemStore>>,
    content_tuning_store: Option<&'a Arc<ContentTuningStore>>,
    curve_store: Option<&'a Arc<CurveStore>>,
    curve_point_store: Option<&'a Arc<CurvePointStore>>,
    item_search_name_store: Option<&'a Arc<ItemSearchNameStore>>,
    item_effect_store: Option<&'a Arc<ItemEffectStore>>,
    faction_store: Option<&'a Arc<FactionStore>>,
    friendship_rep_reaction_store: Option<&'a Arc<FriendshipRepReactionStore>>,
    item_limit_category_store: Option<&'a Arc<ItemLimitCategoryStore>>,
    item_limit_category_condition_store: Option<&'a Arc<ItemLimitCategoryConditionStore>>,
    player_condition_store: Option<&'a Arc<PlayerConditionStore>>,
    map_store: Option<&'a Arc<MapStore>>,
    area_table_store: Option<&'a Arc<AreaTableStore>>,
    chr_specialization_store: Option<&'a Arc<ChrSpecializationStore>>,
}

impl SessionCatalogs {
    /// Select the borrowed catalogs used by represented item valuation and
    /// equipability. Callers retain the view only for the current operation.
    pub fn inventory_valuation_catalog_view_like_cpp(
        &self,
    ) -> InventoryValuationCatalogViewLikeCpp<'_> {
        InventoryValuationCatalogViewLikeCpp {
            item_store: self.items.store.as_ref(),
            item_stats_store: self.items.stats_store.as_ref(),
            item_bonus_db2_store: self.items.bonus_db2_store.as_ref(),
            pvp_item_store: self.pvp_item_store.as_ref(),
            content_tuning_store: self.content_tuning_store.as_ref(),
            curve_store: self.curve_store.as_ref(),
            curve_point_store: self.curve_point_store.as_ref(),
            item_search_name_store: self.items.search_name_store.as_ref(),
            item_effect_store: self.items.effect_store.as_ref(),
            faction_store: self.factions.store.as_ref(),
            friendship_rep_reaction_store: self.friendship_rep_reaction_store.as_ref(),
            item_limit_category_store: self.items.limit_category_store.as_ref(),
            item_limit_category_condition_store: self.items.limit_category_condition_store.as_ref(),
            player_condition_store: self.player_condition_store.as_ref(),
            map_store: self.maps.store.as_ref(),
            area_table_store: self.area_table_store.as_ref(),
            chr_specialization_store: self.chr.specialization_store.as_ref(),
        }
    }
}

impl InventoryValuationCatalogViewLikeCpp<'_> {
    pub fn item_store_like_cpp(&self) -> Option<&Arc<ItemStore>> {
        self.item_store
    }

    pub fn item_stats_store_like_cpp(&self) -> Option<&Arc<ItemStatsStore>> {
        self.item_stats_store
    }

    pub fn item_bonus_db2_store_like_cpp(&self) -> Option<&Arc<ItemBonusDb2Store>> {
        self.item_bonus_db2_store
    }

    pub fn pvp_item_store_like_cpp(&self) -> Option<&Arc<PvpItemStore>> {
        self.pvp_item_store
    }

    pub fn content_tuning_store_like_cpp(&self) -> Option<&Arc<ContentTuningStore>> {
        self.content_tuning_store
    }

    pub fn curve_store_like_cpp(&self) -> Option<&Arc<CurveStore>> {
        self.curve_store
    }

    pub fn curve_point_store_like_cpp(&self) -> Option<&Arc<CurvePointStore>> {
        self.curve_point_store
    }

    pub fn item_search_name_store_like_cpp(&self) -> Option<&Arc<ItemSearchNameStore>> {
        self.item_search_name_store
    }

    pub fn item_effect_store_like_cpp(&self) -> Option<&Arc<ItemEffectStore>> {
        self.item_effect_store
    }

    pub fn faction_store_like_cpp(&self) -> Option<&Arc<FactionStore>> {
        self.faction_store
    }

    pub fn friendship_rep_reaction_store_like_cpp(
        &self,
    ) -> Option<&Arc<FriendshipRepReactionStore>> {
        self.friendship_rep_reaction_store
    }

    pub fn item_limit_category_store_like_cpp(&self) -> Option<&Arc<ItemLimitCategoryStore>> {
        self.item_limit_category_store
    }

    pub fn item_limit_category_condition_store_like_cpp(
        &self,
    ) -> Option<&Arc<ItemLimitCategoryConditionStore>> {
        self.item_limit_category_condition_store
    }

    pub fn player_condition_store_like_cpp(&self) -> Option<&Arc<PlayerConditionStore>> {
        self.player_condition_store
    }

    pub fn map_store_like_cpp(&self) -> Option<&Arc<MapStore>> {
        self.map_store
    }

    pub fn area_table_store_like_cpp(&self) -> Option<&Arc<AreaTableStore>> {
        self.area_table_store
    }

    pub fn chr_specialization_store_like_cpp(&self) -> Option<&Arc<ChrSpecializationStore>> {
        self.chr_specialization_store
    }

    pub fn item_storage_template_like_cpp(&self, item_id: u32) -> Option<ItemStorageTemplate> {
        crate::catalogs::item::item_storage_template_like_cpp(
            self.item_store,
            self.item_stats_store,
            item_id,
        )
    }

    pub fn item_template_quality_like_cpp(&self, item_id: u32) -> Option<i8> {
        self.item_stats_store
            .and_then(|store| store.random_property_template(item_id))
            .map(|template| template.quality)
    }

    pub fn represented_item_level_bonus_like_cpp(
        &self,
        runtime_item: Option<&wow_entities::Item>,
    ) -> i64 {
        let Some(item) = runtime_item else {
            return 0;
        };
        let Some(store) = self.item_bonus_db2_store else {
            return 0;
        };

        item.data()
            .item_bonus_key
            .bonus_list_ids
            .iter()
            .filter_map(|bonus_list_id| u16::try_from(*bonus_list_id).ok())
            .flat_map(|bonus_list_id| store.entries_for_bonus_list_like_cpp(bonus_list_id))
            .filter(|bonus| {
                <ItemBonusType as num_traits::FromPrimitive>::from_u8(bonus.bonus_type)
                    == Some(ItemBonusType::ItemLevel)
            })
            .map(|bonus| i64::from(bonus.value[0]))
            .sum()
    }

    pub fn represented_pvp_item_level_bonus_like_cpp(&self, entry_id: u32) -> u8 {
        self.pvp_item_store
            .map(|store| store.item_level_bonus_like_cpp(entry_id))
            .unwrap_or(0)
    }

    pub fn represented_item_effect_spell_ids_like_cpp(&self, item_id: u32) -> Vec<(u8, i32)> {
        let mut effects: Vec<_> = self
            .item_effect_store
            .map(|store| {
                store
                    .values()
                    .filter(|effect| effect.parent_item_id == item_id)
                    .map(|effect| (effect.legacy_slot_index, effect.spell_id))
                    .collect()
            })
            .unwrap_or_default();
        effects.sort_by_key(|(slot, _)| *slot);
        effects
    }
}
