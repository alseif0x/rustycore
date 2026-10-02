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

pub mod mailbox;
