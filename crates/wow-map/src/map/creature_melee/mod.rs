//! Synchronous creature melee under the caller's manager guard.
//! The Legacy entry also retains the canonical -> legacy writer lock order.
//! Catalogs and packet publication remain application responsibilities. This
//! motor preserves the represented Rust stages and their existing C++ gaps.

mod absorption;
mod catalogs;
mod commit;
mod creature_victim;
mod engine;
mod geometry;
mod models;
mod player_victim;
mod secondary_targets;
mod share;
mod source;
mod split;
mod threat;

pub use catalogs::CreatureMeleeCatalogsLikeCpp;
pub use commit::is_creature_melee_los_clear_like_cpp;
pub use engine::{CreatureMeleeReadiness, creature_melee_readiness};
pub use models::{
    CreatureDamageThreatOutcomeLikeCpp, CreatureMeleePlayerHit, CreatureMeleeSwingOutcome,
    CreatureMeleeVictimSyncIdentityLikeCpp, CreatureMeleeVictimSyncStateLikeCpp,
    CreatureVictimCompatibilitySyncLikeCpp, MeleeAbsorbConsumption, MeleeEffect, MeleePresentation,
    MeleeThreatSpellFacts, PendingCreatureSwingLikeCpp, ShareAuraIdentityLikeCpp,
    ShareAuraSnapshotLikeCpp,
};
use models::{
    CreatureDamageThreatPlanLikeCpp, CreatureMeleeApplyResultLikeCpp, MeleeSwingStateLikeCpp,
};

use crate::map_manager::WorldCreature;
use crate::{ManagedMapInnerLikeCpp, MapManager};
use geometry::*;
use wow_combat::*;
use wow_core::{ObjectGuid, Position};
use wow_entities::{Creature, Player, UnitValuesUpdate};

#[cfg(test)]
mod tests;
