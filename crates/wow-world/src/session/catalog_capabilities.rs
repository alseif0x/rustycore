// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Catalog capabilities: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::GraveyardStore;
use super::HotfixBlobCache;
use super::LfgDungeonStoreLikeCpp;
use super::{AdventureMapPoiStore, Arc, AreaTriggerDb2Store, AreaTriggerScriptDispatcherLikeCpp};
use super::{AreaTriggerScriptStoreLikeCpp, AreaTriggerStore, BankBagSlotPricesStore};
use super::{BattlemasterListStore, ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp};
use super::{ChatListenRangesLikeCpp, EmotesStore, EmotesTextStore};

use super::TavernAreaTriggerStoreLikeCpp;
use super::{PlayerGridLoadOutcomeLikeCpp, PlayerGridLoadResolverLikeCpp};
use super::{PlayerRegenerationRatesLikeCpp, QuestInfoStore, TactKeyStore};
pub use wow_world_core::session::ChatPolicyCatalogsLikeCpp;
use wow_world_core::session::SupportFeaturePolicyLikeCpp;

pub use wow_world_core::session::GroupInvitePolicyLikeCpp;
pub use wow_world_core::session::catalog_capabilities::ObjectMgrCatalogsLikeCpp;

pub use wow_world_core::session::ItemValuationCatalogsLikeCpp;
pub use wow_world_core::session::PlayerBootstrapCatalogsLikeCpp;
// Moved to `wow_world_core` with the continue-login coordinator (#1363 slice 13c).
pub use wow_world_core::session::PlayerRestRatePolicyLikeCpp;

pub use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

pub use wow_world_core::session::ProgressionCatalogsLikeCpp;

/// Process-owned area-trigger lookup and extension capability.
///
/// C++ reads the DB2/ObjectMgr stores and the ScriptMgr hook while handling
/// movement/area-trigger transitions; none of those owners belong to a
/// `WorldSession`. The optional dispatcher represents the legitimate absence
/// of a linked content script, not a missing production catalog.
#[derive(Clone)]
pub struct AreaTriggerCatalogsLikeCpp {
    pub db2: Arc<AreaTriggerDb2Store>,
    pub destinations: Arc<AreaTriggerStore>,
    pub scripts: Arc<AreaTriggerScriptStoreLikeCpp>,
    pub taverns: Arc<TavernAreaTriggerStoreLikeCpp>,
    pub script_dispatcher: Option<AreaTriggerScriptDispatcherLikeCpp>,
}

// Moved to `wow_world_core` with the void-storage bodies (#1363 slice 4c).
pub use wow_world_core::session::SessionIdGeneratorsLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
impl Default for AreaTriggerCatalogsLikeCpp {
    fn default() -> Self {
        Self {
            db2: Arc::new(AreaTriggerDb2Store::from_entries([])),
            destinations: Arc::new(AreaTriggerStore::default()),
            scripts: Arc::new(AreaTriggerScriptStoreLikeCpp::default()),
            taverns: Arc::new(TavernAreaTriggerStoreLikeCpp::default()),
            script_dispatcher: None,
        }
    }
}

/// Immutable process-owned capabilities borrowed by the outer driver for one
/// session pass. Each driver phase and packet registration narrows this
/// aggregate to the exact capability it consumes; production `WorldSession`
/// never stores the aggregate.
#[derive(Clone)]
pub struct SessionHandlerCatalogsLikeCpp {
    pub object_mgr: Arc<ObjectMgrCatalogsLikeCpp>,
    pub area_triggers: Arc<AreaTriggerCatalogsLikeCpp>,
    /// Map/runtime capability borrowed only by movement and login operations.
    /// C++ `Map::EnsureGridLoadedForActiveObject` belongs to the Map, not to
    /// `WorldSession`; production installs one process-owned adapter here.
    pub player_grid_loader: PlayerGridLoadResolverLikeCpp,
    pub item_valuation: Arc<ItemValuationCatalogsLikeCpp>,
    pub player_bootstrap: Arc<PlayerBootstrapCatalogsLikeCpp>,
    pub player_rest_rates: Arc<PlayerRestRatePolicyLikeCpp>,
    pub creature_spawns: Arc<CreatureSpawnCatalogsLikeCpp>,
    pub progression: Arc<ProgressionCatalogsLikeCpp>,
    pub battle_pet_trainer_selection:
        Arc<wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp>,
    pub chat_policy: Arc<ChatPolicyCatalogsLikeCpp>,
    pub group_invite_policy: Arc<GroupInvitePolicyLikeCpp>,
    /// C++ `CONFIG_TRADE_LEVEL_REQ` / `CONFIG_ALLOW_TWO_SIDE_TRADE` (#38).
    pub trade_policy: Arc<wow_world_social::TradePolicyLikeCpp>,
    pub support_feature_policy: Arc<SupportFeaturePolicyLikeCpp>,
    /// C++ `sWorld->getRate(...)` subset consumed by `Player::Regenerate` and
    /// `Player::RegenerateHealth`.
    pub player_regeneration_rates: Arc<PlayerRegenerationRatesLikeCpp>,
    pub bank_bag_slot_prices: Arc<BankBagSlotPricesStore>,
    pub adventure_map_pois: Arc<AdventureMapPoiStore>,
    /// C++ sQuestInfoStore: borrowed by questgiver queries, never installed by dispatch.
    pub quest_info: Arc<QuestInfoStore>,
    pub battlemaster_lists: Arc<BattlemasterListStore>,
    pub emotes: Arc<EmotesStore>,
    pub emotes_text: Arc<EmotesTextStore>,
    pub graveyards: Arc<GraveyardStore>,
    pub lfg_dungeons: Arc<LfgDungeonStoreLikeCpp>,
    pub tact_keys: Arc<TactKeyStore>,
    /// C++ `sDB2Manager` hotfix delivery data, borrowed by auth and requests.
    pub hotfixes: Arc<HotfixBlobCache>,
    pub modules: Arc<wow_module_api::ModuleRegistry>,
    pub id_generators: Arc<SessionIdGeneratorsLikeCpp>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl Default for SessionHandlerCatalogsLikeCpp {
    fn default() -> Self {
        Self {
            object_mgr: Arc::new(ObjectMgrCatalogsLikeCpp::default()),
            area_triggers: Arc::new(AreaTriggerCatalogsLikeCpp::default()),
            player_grid_loader: Arc::new(|_, _, _| PlayerGridLoadOutcomeLikeCpp {
                map_unavailable: true,
                ..Default::default()
            }),
            item_valuation: Arc::new(ItemValuationCatalogsLikeCpp::default()),
            player_bootstrap: Arc::new(PlayerBootstrapCatalogsLikeCpp::default()),
            player_rest_rates: Arc::new(PlayerRestRatePolicyLikeCpp::default()),
            creature_spawns: Arc::new(CreatureSpawnCatalogsLikeCpp::default()),
            progression: Arc::new(ProgressionCatalogsLikeCpp::default()),
            battle_pet_trainer_selection: Arc::new(
                wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp::default(),
            ),
            chat_policy: Arc::new(ChatPolicyCatalogsLikeCpp::default()),
            group_invite_policy: Arc::new(GroupInvitePolicyLikeCpp::default()),
            trade_policy: Arc::new(wow_world_social::TradePolicyLikeCpp::default()),
            support_feature_policy: Arc::new(SupportFeaturePolicyLikeCpp::default()),
            player_regeneration_rates: Arc::new(PlayerRegenerationRatesLikeCpp::default()),
            bank_bag_slot_prices: Arc::new(BankBagSlotPricesStore::from_entries([])),
            adventure_map_pois: Arc::new(AdventureMapPoiStore::from_entries([])),
            quest_info: Arc::new(QuestInfoStore::from_entries([])),
            battlemaster_lists: Arc::new(BattlemasterListStore::from_entries([])),
            emotes: Arc::new(EmotesStore::from_entries([])),
            emotes_text: Arc::new(EmotesTextStore::from_entries([])),
            graveyards: Arc::new(GraveyardStore::default()),
            lfg_dungeons: Arc::new(LfgDungeonStoreLikeCpp::default()),
            tact_keys: Arc::new(TactKeyStore::from_entries([])),
            hotfixes: Arc::new(HotfixBlobCache::new()),
            modules: Arc::new(wow_module_api::ModuleRegistry::new()),
            id_generators: Arc::new(SessionIdGeneratorsLikeCpp::default()),
        }
    }
}
