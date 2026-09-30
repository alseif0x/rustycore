//! One Aggro algorithm for legacy ownership and the dormant canonical producer.
//!
//! Creature.cpp:280/2095/2518/2570/2671, CreatureAI.cpp:122/141 and
//! ThreatManager.cpp:586 (a5f8da2e). Existing representation gaps remain.

use super::{MapManager as LegacyMapManager, WorldCreature};
use crate::coords::SIZE_OF_GRID_CELL;
use std::collections::{HashMap, HashSet};
use wow_constants::{UnitFlags, UnitFlags2, UnitState};
use wow_core::{
    ObjectGuid, Position, position_is_in_dist_strict_2d_like_cpp,
    position_is_in_dist_strict_3d_like_cpp,
};
use wow_entities::{Unit, WorldObject};

mod assistance;
mod backend;
mod contracts;
mod decisions;
mod frame;
mod leash;
mod primary;
mod threat;
mod visibility;

pub(crate) use assistance::AggroLosPending;
pub(crate) use backend::AggroMap;
pub use contracts::*;
pub use decisions::{can_attack, candidate_hostile, select_ai, snapshot_hostile, trigger_alert};
pub(crate) use frame::{AggroFrame, AggroTailProgress};
pub use leash::{candidate_leash, snapshot_leash};
pub use threat::update_threat_victim;
pub use visibility::{
    candidate_accessible, candidate_has_stealth, candidate_targetable, candidate_visibility,
};

const LIQUID_MAP_IN_WATER_LIKE_CPP: u32 = 4;
const LIQUID_MAP_UNDER_WATER_LIKE_CPP: u32 = 8;

fn within_melee_range(
    attacker: Position,
    attacker_reach: f32,
    target: Position,
    target_reach: f32,
) -> bool {
    let melee_range = (attacker_reach.max(0.0) + target_reach.max(0.0) + 4.0 / 3.0).max(5.0);
    attacker.distance(&target) <= melee_range
}

fn emit(
    outcome: &mut AggroOutcome,
    map_id: u16,
    instance_id: u32,
    actor: &WorldCreature,
    kind: AggroEffectKind,
) {
    let position = match &kind {
        AggroEffectKind::MoveStop(stop) => stop.position,
        _ => actor.position(),
    };
    outcome.effects.push(AggroEffect {
        source_guid: actor.guid(),
        map_id,
        instance_id,
        position,
        visibility_range: actor.visibility_range_like_cpp(),
        kind,
    });
}

impl LegacyMapManager {
    /// Compatibility producer uses the same primary and tail algorithms.
    pub fn run_aggro_map(
        &mut self,
        map_id: u16,
        instance_id: u32,
        candidates: Vec<AggroCandidate>,
        settings: AggroSettings,
        terrain: Option<&super::LiveTerrainHeights>,
        policies: &mut AggroPolicies<'_>,
    ) -> AggroOutcome {
        let guids = self.creature_guids(map_id, instance_id);
        let mut backend = AggroMap::Legacy {
            manager: self,
            map_id,
            instance_id,
        };
        let mut frame = AggroFrame::new(map_id, instance_id, guids, candidates, settings, &backend);
        frame.run_primaries(&mut backend, policies);
        loop {
            match frame.prepare_tail(&mut backend, policies, terrain.is_some()) {
                AggroTailProgress::Complete(outcome) => return outcome,
                AggroTailProgress::Pending(pending) => {
                    let visible = pending.resolve(terrain.unwrap());
                    frame = pending.resume(visible, &mut backend, policies);
                }
            }
        }
    }
}
