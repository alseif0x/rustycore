// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;
use wow_data::trait_tree::TraitNodeEntryStore;
use wow_data::{
    ExplorationBaseXpStoreLikeCpp, GlyphPropertiesStore, ImportPriceStores, ItemClassStore,
    ItemCurrencyCostStore, ItemDisenchantLootStore, ItemPriceBaseStore,
    PlayerCreateInfoCastSpellStoreLikeCpp, PlayerCreateInfoCustomSpellStoreLikeCpp,
    PlayerCreateInfoStoreLikeCpp, TalentTabStore,
};
use wow_packet::packets::misc::FeatureSystemConfigLikeCpp;

/// Process-owned C++ Player creation data and glyph catalog borrowed during login.
///
/// C++ stores the base row plus `customSpells` and per-create-mode
/// `castSpells` under `ObjectMgr` (`Globals/ObjectMgr.h:649-663`) and resolves
/// it through `sObjectMgr->GetPlayerInfo`; it is never session-owned.
pub struct PlayerBootstrapCatalogsLikeCpp {
    pub create_info: Arc<PlayerCreateInfoStoreLikeCpp>,
    /// C++ process-wide sGlyphPropertiesStore, borrowed during Player::_LoadGlyphs.
    pub glyph_properties: Arc<GlyphPropertiesStore>,
    pub talent_tabs: Arc<TalentTabStore>,
    /// C++ process-owned sTraitNodeEntryStore, borrowed while loading traits.
    pub trait_node_entries: Arc<TraitNodeEntryStore>,
    pub cast_spells: Arc<PlayerCreateInfoCastSpellStoreLikeCpp>,
    pub custom_spells: Arc<PlayerCreateInfoCustomSpellStoreLikeCpp>,
    /// C++ `World` policy consumed by `Player::LearnCustomSpells`.
    pub start_all_spells: bool,
    /// C++ `World` policy consumed by the first-login `Player` path.
    pub start_all_explored: bool,
    /// C++ `World` policy consumed by the first-login `Player` path.
    pub start_all_reputation: bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl Default for PlayerBootstrapCatalogsLikeCpp {
    fn default() -> Self {
        Self {
            create_info: Arc::new(PlayerCreateInfoStoreLikeCpp::default()),
            glyph_properties: Arc::new(GlyphPropertiesStore::from_entries([])),
            talent_tabs: Arc::new(TalentTabStore::from_entries([])),
            trait_node_entries: Arc::new(TraitNodeEntryStore::from_entries([])),
            cast_spells: Arc::new(PlayerCreateInfoCastSpellStoreLikeCpp::default()),
            custom_spells: Arc::new(PlayerCreateInfoCustomSpellStoreLikeCpp::default()),
            start_all_spells: false,
            start_all_explored: false,
            start_all_reputation: false,
        }
    }
}

/// Process-owned C++ `World` policy for party invitation admission.
///
/// C++ loads these at `World.cpp:790,913,1163`; `GroupHandler.cpp:78-108`
/// borrows them through `sWorld` while validating an invitation.
#[derive(Debug, Clone, Copy)]
pub struct GroupInvitePolicyLikeCpp {
    pub allow_gm_group: bool,
    pub allow_two_side_interaction: bool,
    pub minimum_level: u32,
}

impl Default for GroupInvitePolicyLikeCpp {
    fn default() -> Self {
        Self {
            allow_gm_group: false,
            allow_two_side_interaction: false,
            minimum_level: 1,
        }
    }
}

/// Capability-specific immutable query owner consumed by query/gameobject
/// handlers. It mirrors C++ ObjectMgr startup stores and contains no database
/// handle or mutable gameplay state.
#[derive(Debug, Clone)]
#[cfg_attr(any(test, feature = "test-fixtures"), derive(Default))]
pub struct ObjectMgrCatalogsLikeCpp {
    pub creature: Arc<wow_data::CreatureQueryCatalogLikeCpp>,
    pub gameobject: Arc<wow_data::GameObjectQueryCatalogLikeCpp>,
    pub gameobject_quest_items: Arc<wow_data::GameObjectQuestItemStoreLikeCpp>,
    pub page_text: Arc<wow_data::PageTextCatalogLikeCpp>,
}

/// Process-owned support and feature-system policy borrowed by Session edges.
///
/// C++ initializes the support switches in `World.cpp:584-595` and the feature
/// switches in `World.cpp:1597-1599`. `SupportMgr` and the feature-status send
/// helpers read that process state; it is not copied into each `WorldSession`.
#[derive(Debug, Clone, Copy)]
pub struct SupportFeaturePolicyLikeCpp {
    pub support_enabled: bool,
    pub tickets_enabled: bool,
    pub bugs_enabled: bool,
    pub complaints_enabled: bool,
    pub suggestions_enabled: bool,
    pub character_undelete_enabled: bool,
    pub bpay_store_enabled: bool,
    pub max_characters_per_realm: u32,
    pub declined_names_used: bool,
}

impl Default for SupportFeaturePolicyLikeCpp {
    fn default() -> Self {
        Self {
            support_enabled: true,
            tickets_enabled: false,
            bugs_enabled: false,
            complaints_enabled: false,
            suggestions_enabled: false,
            character_undelete_enabled: false,
            bpay_store_enabled: false,
            max_characters_per_realm: 60,
            declined_names_used: false,
        }
    }
}

impl SupportFeaturePolicyLikeCpp {
    pub fn bug_system_enabled_like_cpp(self) -> bool {
        self.support_enabled && self.bugs_enabled
    }

    pub fn complaint_system_enabled_like_cpp(self) -> bool {
        self.support_enabled && self.complaints_enabled
    }

    pub fn suggestion_system_enabled_like_cpp(self) -> bool {
        self.support_enabled && self.suggestions_enabled
    }

    pub(in crate::session) fn feature_system_config_like_cpp(self) -> FeatureSystemConfigLikeCpp {
        FeatureSystemConfigLikeCpp {
            support_tickets_enabled: self.tickets_enabled,
            support_bugs_enabled: self.bugs_enabled,
            support_complaints_enabled: self.complaints_enabled,
            support_suggestions_enabled: self.suggestions_enabled,
            char_undelete_enabled: self.character_undelete_enabled,
            bpay_store_enabled: self.bpay_store_enabled,
        }
    }
}

/// Process-owned DB2 catalogs used by C++'s static item valuation helpers.
///
/// C++ reads these globals directly in `Item::GetBuyPrice`,
/// `Item::GetSellPrice`, and `Item::GetDisenchantLoot`
/// (`Entities/Item/Item.cpp:1732-1969`); `WorldSession` owns none of them.
pub struct ItemValuationCatalogsLikeCpp {
    pub import_prices: Arc<ImportPriceStores>,
    pub price_base: Arc<ItemPriceBaseStore>,
    pub item_classes: Arc<ItemClassStore>,
    pub currency_costs: Arc<ItemCurrencyCostStore>,
    pub disenchant_loot: Arc<ItemDisenchantLootStore>,
}

/// Process-owned player-level and exploration XP catalogs plus immutable World policy.
///
/// C++ resolves these through ObjectMgr world tables and `sWorld` while
/// adapting Player progression operations. A `WorldSession` borrows this
/// capability for the operation; it does not own a copy of any catalog or
/// configuration value.
#[derive(Clone)]
#[cfg_attr(any(test, feature = "test-fixtures"), derive(Default))]
pub struct ProgressionCatalogsLikeCpp {
    pub player_xp: Arc<Vec<u32>>,
    pub exploration_base_xp: Arc<ExplorationBaseXpStoreLikeCpp>,
    pub exploration_xp_rate: f32,
    pub min_discovered_scaled_xp_ratio: u32,
    /// C++ World policy read by Player::ResetTalents, never cached on Session.
    pub no_reset_talent_cost: bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl Default for ItemValuationCatalogsLikeCpp {
    fn default() -> Self {
        Self {
            import_prices: Arc::new(ImportPriceStores {
                armor: wow_data::ImportPriceArmorStore::from_entries([]),
                quality: wow_data::ImportPriceQualityStore::from_entries([]),
                shield: wow_data::ImportPriceShieldStore::from_entries([]),
                weapon: wow_data::ImportPriceWeaponStore::from_entries([]),
            }),
            price_base: Arc::new(ItemPriceBaseStore::from_entries([])),
            item_classes: Arc::new(ItemClassStore::from_entries([])),
            currency_costs: Arc::new(ItemCurrencyCostStore::from_entries([])),
            disenchant_loot: Arc::new(ItemDisenchantLootStore::from_entries([])),
        }
    }
}
