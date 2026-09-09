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

mod map_lifetime;
mod player_owner;
pub use map_lifetime::MapUnloadBlockedLikeCpp;
pub use player_owner::{
    PlayerHandle, PlayerOwnerError, PlayerResidenceLikeCpp, PlayerVisibilityRefreshIntentLikeCpp,
};

mod state_1;
mod state_2;
#[allow(unused_imports)]
pub use state_1::*;
#[allow(unused_imports)]
pub use state_2::*;

#[cfg(test)]
#[path = "manager/tests/mod.rs"]
mod tests;
