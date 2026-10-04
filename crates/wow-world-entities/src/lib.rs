//! Session-independent world-entity state and represented creature publication.

mod contracts;
mod creature_publication;
mod aggro;
mod catalogs;
mod creature;
mod creature_aura_tracking;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixtures;
mod creature_interaction;
mod creature_kill;
mod creature_query;
mod creature_registry;
mod creature_visibility;
mod gameobject_contracts;
mod gameobject;
mod gameobject_access;
mod gameobject_interaction;
mod gameobject_overrides;
mod gameobject_query;
mod gameobject_state;
mod gameobject_use;
mod gameobject_use_types;
mod pending_runtime;
mod spawn;
mod spell_click;
mod state;

pub use aggro::{
    creature_threat_value_on_map_like_cpp,
    mirror_creature_threat_from_attacker_on_map_like_cpp,
};
pub use catalogs::CreatureSpawnCatalogsLikeCpp;
pub use creature::creature_message_to_set_target_allows_like_cpp;
pub use creature_registry::{
    insert_canonical_creature_map_object_on_map_like_cpp, sheath_state_from_u8_like_cpp,
    unit_stand_state_from_u8_like_cpp, CanonicalCreatureInsertOutcomeLikeCpp,
};
pub use creature_publication::represented_creature_aura_info_like_cpp;
pub use contracts::{
    CreatureCreateStatsLikeCpp, MaterializedCreatureSpawnLikeCpp,
    PendingCreatureKillRewardLikeCpp, PendingCreatureSpawn,
    RepresentedCreatureAuraLikeCpp, RepresentedSpellClickCreatureSnapshotLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use contracts::RepresentedCreatureKillEventLikeCpp;
pub use gameobject_interaction::{
    represented_gameobject_display_box_contains_like_cpp,
    represented_gameobject_interaction_distance_like_cpp,
};
pub use gameobject_contracts::{
    BattlegroundFlagDropClickTarget, RepresentedBattlegroundObjectUseRejection,
    RepresentedCapturePointStateLikeCpp, RepresentedGameObjectSpellCaster,
    RepresentedGameObjectUseEffect, RepresentedGameObjectUseState,
    RepresentedNewFlagStateRequest,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use gameobject_contracts::RepresentedGameObjectCriteriaEvent;
pub use state::WorldEntitiesState;
