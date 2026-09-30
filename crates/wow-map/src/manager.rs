//! MapManager skeleton.
//!
//! C++ references:
//! - `game/Maps/MapManager.h`
//! - `game/Maps/MapManager.cpp`

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use crate::DEFAULT_VISIBILITY_NOTIFY_PERIOD;
use crate::MapKey;
use crate::map::{
    AreaTriggersUpdateSummaryLikeCpp, ConversationsUpdateSummaryLikeCpp,
    CreatureUpdateSummaryLikeCpp, DynamicMapTreeUpdateSummaryLikeCpp,
    DynamicObjectsUpdateSummaryLikeCpp, FarSpellCallbackDrainSummaryLikeCpp,
    GameObjectsUpdateSummaryLikeCpp, GridStatesUpdateSummaryLikeCpp,
    LoadedGridRespawnRecordsLikeCpp, Map, MapCommandLikeCpp, MapCommandOutcomeLikeCpp, MapRuntime,
    MapUpdateMetricsSummaryLikeCpp, MoveListDrainSummaryLikeCpp, NoopGridLifecycle,
    NoopTerrainGridLoader, PersonalPhaseTrackerUpdateSummaryLikeCpp,
    ProcessRelocationNotifiesOutcome, SceneObjectUpdateContextLikeCpp,
    SceneObjectsUpdateSummaryLikeCpp, ScriptScheduleProcessSummaryLikeCpp,
    SendObjectUpdatesSummaryLikeCpp, TransportsUpdateSummaryLikeCpp, WeatherUpdateSummaryLikeCpp,
};
use crate::pool::PoolMgrLikeCpp;
use crate::spawn::{Difficulty, SpawnId, SpawnObjectType, SpawnStore};
use wow_core::{GameTime, ObjectGuid};
use wow_entities::CreatureRuntimeUpdateContext;

mod admin;
mod actor_admission;
mod actor_transport;
mod actor_respawn;
pub use actor_respawn::{ActorRespawnError, ActorRespawnProgress, ActorRespawnRequest,
    ActorRespawnReply, ActorRespawnRejected};
pub use actor_admission::FreshCreatureActorMapAdmissionError;
mod actor_movement;
mod actor_aggro;
mod actor_spell;
pub use actor_spell::{ActorSpellContinuation, ActorSpellError, ActorSpellLosQuery,
    ActorSpellLosRequest, ActorSpellPrepareFailure, ActorSpellProgress, ActorSpellResumeFailure,
    ActorSpellPublicationContinuation, ActorSpellPublicationFailure};
pub use actor_aggro::{
    ActorAggroContinuation, ActorAggroError, ActorAggroLosQuery, ActorAggroLosRequest,
    ActorAggroPrepareFailure, ActorAggroProgress, ActorAggroResumeFailure,
};
pub use actor_movement::{
    ActorGridHeightContinuation, ActorGridHeightQuery, ActorGridHeightRequest,
    ActorMovementCompletion, ActorMovementError, ActorMovementPending, ActorMovementProgress,
    ActorMovementResumeFailure, ActorMovementTraceFacts, ActorPathContinuation, ActorPathRequest,
    ActorStaticHeightContinuation, ActorStaticHeightQuery, ActorStaticHeightRequest,
};
mod actor_tick_access;
mod creature_loot;
pub use creature_loot::{CreatureLootAccess, CreatureLootAccessError,
    CreatureLootActorHandle, CreatureLootActorObservation};
mod actor_melee;
pub(crate) use actor_melee::MeleeKillCollector;
pub use actor_melee::{SelectedMeleeExecution, PendingMeleeKills, PreparedMeleeKill, PreparedMeleeLoot,
    MeleeKillPhaseError, MeleeKillCaptureError, MeleeLootError};
mod actor_kill;
pub use actor_tick_access::ActorTickAccessError;
mod map_lifetime;
mod map_update;
mod player_owner;
mod updater;
pub use map_lifetime::MapUnloadBlockedLikeCpp;
pub use player_owner::{
    PlayerHandle, PlayerOwnerError, PlayerResidenceLikeCpp, PlayerVisibilityRefreshIntentLikeCpp,
};

mod state_1;
mod state_2;
mod tick_objects;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;
pub use tick_objects::{
    MapObjectTickContinuation, ObjectMapFinishOutcome,
    ObjectMapTickError, ObjectMapUpdateToken,
};
pub use updater::MapUpdater;

#[cfg(test)]
#[path = "manager/tests/mod.rs"]
mod tests;
