//! Session-independent world-entity state and represented creature publication.

mod aggro;
mod catalogs;
mod contracts;
mod creature;
pub mod creature_aggro_contracts;
mod creature_aura_tracking;
mod creature_interaction;
mod creature_kill;
pub mod creature_movement_adapter;
mod creature_publication;
mod creature_query;
mod creature_registry;
pub mod creature_spell_admission;
pub mod creature_spell_metadata;
pub mod creature_spell_planning;
pub mod creature_spell_publication;
mod creature_visibility;
#[cfg(any(test, feature = "test-fixtures"))]
mod fixtures;
mod gameobject;
mod gameobject_access;
mod gameobject_contracts;
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
    creature_threat_value_on_map_like_cpp, mirror_creature_threat_from_attacker_on_map_like_cpp,
};
pub use catalogs::CreatureSpawnCatalogsLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use contracts::RepresentedCreatureKillEventLikeCpp;
pub use contracts::{
    CreatureCreateStatsLikeCpp, MaterializedCreatureSpawnLikeCpp, PendingCreatureKillRewardLikeCpp,
    PendingCreatureSpawn, RepresentedCreatureAuraLikeCpp,
    RepresentedSpellClickCreatureSnapshotLikeCpp,
};
pub use creature::creature_message_to_set_target_allows_like_cpp;
pub use creature_publication::represented_creature_aura_info_like_cpp;
pub use creature_registry::{
    CanonicalCreatureInsertOutcomeLikeCpp, insert_canonical_creature_map_object_on_map_like_cpp,
    sheath_state_from_u8_like_cpp, unit_stand_state_from_u8_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use gameobject_contracts::RepresentedGameObjectCriteriaEvent;
pub use gameobject_contracts::{
    BattlegroundFlagDropClickTarget, RepresentedBattlegroundObjectUseRejection,
    RepresentedCapturePointStateLikeCpp, RepresentedGameObjectSpellCaster,
    RepresentedGameObjectUseEffect, RepresentedGameObjectUseState, RepresentedNewFlagStateRequest,
};
pub use gameobject_interaction::{
    represented_gameobject_display_box_contains_like_cpp,
    represented_gameobject_interaction_distance_like_cpp,
};
pub use state::WorldEntitiesState;
