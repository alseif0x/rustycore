// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Catalog capabilities: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::VoidStorageItemIdGeneratorLikeCpp;
use super::{AdventureMapPoiStore, Arc, AreaTriggerDb2Store, AreaTriggerScriptDispatcherLikeCpp};
use super::{AreaTriggerScriptStoreLikeCpp, AreaTriggerStore, BankBagSlotPricesStore};
use super::{BattlemasterListStore, ChatFloodConfigLikeCpp, ChatLevelRequirementsLikeCpp};
use super::{ChatListenRangesLikeCpp, CreatureAddonStoreLikeCpp, CreatureBaseStatsStoreLikeCpp};
use super::{CreatureClassificationHealthRatesLikeCpp, CreatureDifficultyStoreLikeCpp};
use super::{CreatureEquipmentStoreLikeCpp, EmotesStore, EmotesTextStore};
use super::EquipmentSetGuidGeneratorLikeCpp;
use super::{GraveyardStore, HighGuid};
use super::HotfixBlobCache;
use super::LfgDungeonStoreLikeCpp;
use super::ObjectGuidGenerator;

use super::{PlayerGridLoadOutcomeLikeCpp, PlayerGridLoadResolverLikeCpp};
use super::{PlayerRegenerationRatesLikeCpp, PowerTypeStore, QuestInfoStore, TactKeyStore};
use super::TavernAreaTriggerStoreLikeCpp;
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

/// Process-owned C++ `ObjectMgr` creature materialization catalogs and
/// `World` creature-health policy.
///
/// C++ resolves these through `sObjectMgr`/`sWorld` while `Creature::InitEntry`,
/// `UpdateLevelDependantStats`, `LoadEquipment`, and `GetCreatureAddon` build a
/// creature (`Creature.cpp:491-615,1550-1615,1931-1965,2722-2755`). A
/// `WorldSession` borrows them only while adapting visibility; it owns none of
/// the stores or rates.
pub struct CreatureSpawnCatalogsLikeCpp {
    pub difficulty: Arc<CreatureDifficultyStoreLikeCpp>,
    pub base_stats: Arc<CreatureBaseStatsStoreLikeCpp>,
    pub health_rates: CreatureClassificationHealthRatesLikeCpp,
    pub addons: Arc<CreatureAddonStoreLikeCpp>,
    pub equipment: Arc<CreatureEquipmentStoreLikeCpp>,
    pub power_types: Arc<PowerTypeStore>,
}

pub use wow_world_core::session::ProgressionCatalogsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
impl Default for CreatureSpawnCatalogsLikeCpp {
    fn default() -> Self {
        Self {
            difficulty: Arc::new(CreatureDifficultyStoreLikeCpp::default()),
            base_stats: Arc::new(CreatureBaseStatsStoreLikeCpp::default()),
            health_rates: CreatureClassificationHealthRatesLikeCpp::default(),
            addons: Arc::new(CreatureAddonStoreLikeCpp::default()),
            equipment: Arc::new(CreatureEquipmentStoreLikeCpp::default()),
            power_types: Arc::new(PowerTypeStore::from_entries([])),
        }
    }
}

impl Default for PlayerRestRatePolicyLikeCpp {
    fn default() -> Self {
        Self {
            offline_wilderness: 1.0,
            offline_tavern_or_city: 1.0,
            ingame: 1.0,
        }
    }
}

/// Process-owned C++ `World` chat policy borrowed by chat handlers.
///
/// C++ loads these values once into `World::{m_bool,m_int,m_float}_configs`
/// (`World.cpp:769,785-789,1241-1245,1294-1296,1323-1325`) and handlers read
/// them through `sWorld`; a `WorldSession` never owns a private policy copy.
#[derive(Debug, Clone, Copy)]
pub struct ChatPolicyCatalogsLikeCpp {
    pub addon_channel: bool,
    pub fake_message_preventing: bool,
    pub strict_link_checking_kick: bool,
    pub level_requirements: ChatLevelRequirementsLikeCpp,
    pub listen_ranges: ChatListenRangesLikeCpp,
    pub flood: ChatFloodConfigLikeCpp,
    pub party_raid_warnings: bool,
}

impl Default for ChatPolicyCatalogsLikeCpp {
    fn default() -> Self {
        Self {
            addon_channel: true,
            fake_message_preventing: false,
            strict_link_checking_kick: false,
            level_requirements: ChatLevelRequirementsLikeCpp::default(),
            listen_ranges: ChatListenRangesLikeCpp::default(),
            flood: ChatFloodConfigLikeCpp::default(),
            party_raid_warnings: false,
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
