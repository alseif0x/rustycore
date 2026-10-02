//! Session-independent owners shared by the world shell and core.

pub use crate::player_directory as directory;

mod prelude;
pub use prelude::{
    AFLAG_SCALABLE_LIKE_CPP, PLAYER_FLAGS_AFK_LIKE_CPP, PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP,
    PLAYER_FLAGS_DND_LIKE_CPP, PLAYER_FLAGS_GHOST_LIKE_CPP, SKILL_ENCHANTING_LIKE_CPP,
    SharedCanonicalMapManager,
};

pub mod battle_pet_adapter;
#[cfg(any(test, feature = "test-fixtures"))]
pub use battle_pet_adapter::RepresentedBattlePetCageItemLikeCpp;
pub use battle_pet_adapter::{
    BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP, BATTLE_PET_SLOT_COUNT_LIKE_CPP,
    DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP, RepresentedBattlePetCalculatedStatsLikeCpp,
    RepresentedBattlePetDataLikeCpp, RepresentedBattlePetLevelCriteriaLikeCpp,
    RepresentedBattlePetQueryCompanionLikeCpp, RepresentedBattlePetSaveInfoLikeCpp,
    RepresentedBattlePetSlotLikeCpp,
};

pub mod connection_identity;
pub use connection_identity::{
    PacketCounterLikeCpp, PacketSpoofPendingBanLikeCpp, PacketSpoofPendingBanTargetLikeCpp,
    SessionState,
};

mod connection;
mod canonical_access;
mod instances;

mod creature_aggro_contracts;
pub use creature_aggro_contracts::{
    DEFAULT_VISIBILITY_BGARENAS_LIKE_CPP, LegacyCreatureAggroConfigLikeCpp,
    spell_has_no_unrepresented_runtime_hooks_from_authority_like_cpp,
};

mod creature_spell_metadata;
pub use creature_spell_metadata::creature_ai_spell_difficulty_chain_like_cpp;

pub mod persistence_capabilities;

pub mod player_binding;
#[cfg(any(test, feature = "test-fixtures"))]
pub use player_binding::PlayerTransportLoginStateLikeCpp;
pub use player_binding::PlayerIdentityBootstrapLikeCpp;

pub mod time_synchronization;
pub use time_synchronization::{game_time_ms_like_cpp, TimeSynchronizationStateLikeCpp};

pub mod mailbox;
pub mod state;
pub use state::{SessionCatalogs, SessionCore, SessionDriverPhaseLikeCpp};
pub use state::SessionWorldConfig;

pub mod map_admission;
pub use map_admission::{MMapRuntimeConfigLikeCpp, WaypointPathResolverLikeCpp};

pub mod catalog_capabilities;
pub use catalog_capabilities::ObjectMgrCatalogsLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub mod test_support;
#[cfg(any(test, feature = "test-fixtures"))]
pub use test_support::test_fixtures::PlayerBootstrapCatalogTestFixtureLikeCpp;
