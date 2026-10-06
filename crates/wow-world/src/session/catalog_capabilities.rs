// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Catalog capabilities: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::EquipmentSetGuidGeneratorLikeCpp;
use super::HotfixBlobCache;
use super::LfgDungeonStoreLikeCpp;
use super::ObjectGuidGenerator;
use super::VoidStorageItemIdGeneratorLikeCpp;
use super::{AdventureMapPoiStore, Arc, AreaTriggerDb2Store, AreaTriggerScriptDispatcherLikeCpp};
use super::{AreaTriggerScriptStoreLikeCpp, AreaTriggerStore, BankBagSlotPricesStore};
use super::{BattlemasterListStore, ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp};
use super::{ChatListenRangesLikeCpp, EmotesStore, EmotesTextStore};
use super::{GraveyardStore, HighGuid};

use super::TavernAreaTriggerStoreLikeCpp;
use super::{PlayerGridLoadOutcomeLikeCpp, PlayerGridLoadResolverLikeCpp};
use super::{PlayerRegenerationRatesLikeCpp, QuestInfoStore, TactKeyStore};
pub use wow_world_core::session::ChatPolicyCatalogsLikeCpp;
use wow_world_core::session::SupportFeaturePolicyLikeCpp;

pub use wow_world_core::session::GroupInvitePolicyLikeCpp;
pub use wow_world_core::session::catalog_capabilities::ObjectMgrCatalogsLikeCpp;

pub use wow_world_core::session::ItemValuationCatalogsLikeCpp;
pub use wow_world_core::session::PlayerBootstrapCatalogsLikeCpp;

/// Process-owned C++ `World` rates borrowed by `Player`/`RestMgr` transitions.
///
/// C++ reads these from `sWorld` at `Player.cpp:17898-17899` and
/// `RestMgr.cpp:139-151`; a session does not own private copies.
#[derive(Debug, Clone, Copy)]
pub struct PlayerRestRatePolicyLikeCpp {
    pub offline_wilderness: f32,
    pub offline_tavern_or_city: f32,
    pub ingame: f32,
}

pub use wow_world_entities::CreatureSpawnCatalogsLikeCpp;

pub use wow_world_core::session::ProgressionCatalogsLikeCpp;

impl Default for PlayerRestRatePolicyLikeCpp {
    fn default() -> Self {
        Self {
            offline_wilderness: 1.0,
            offline_tavern_or_city: 1.0,
            ingame: 1.0,
        }
    }
}

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

/// Process-wide ObjectMgr identifier allocators.
///
/// C++ initializes these once from database maxima. Sessions consume values
/// but never own an allocator or return a value after a later failure.
pub struct SessionIdGeneratorsLikeCpp {
    pub player: Arc<ObjectGuidGenerator>,
    pub item: Arc<ObjectGuidGenerator>,
    pub equipment_set: Arc<EquipmentSetGuidGeneratorLikeCpp>,
    pub void_storage_item: Arc<VoidStorageItemIdGeneratorLikeCpp>,
}

#[cfg(any(test, feature = "test-fixtures"))]
impl Default for SessionIdGeneratorsLikeCpp {
    fn default() -> Self {
        Self {
            player: Arc::new(ObjectGuidGenerator::new(HighGuid::Player, 1)),
            item: Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)),
            equipment_set: Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(1)),
            void_storage_item: Arc::new(VoidStorageItemIdGeneratorLikeCpp::new(1)),
        }
    }
}

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
