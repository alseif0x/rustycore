//! One-map-at-a-time continuation for the post-session Map update phases.

use super::*;

/// Opaque progress for one admitted tick's post-session object phases.
///
/// It is deliberately non-Clone/non-Copy. Dropping it leaves the manager in
/// `Resuming`; only a complete `finalize_object_tick` returns the manager to
/// `Idle`.
pub struct MapObjectTickContinuation {
    origin: Arc<()>,
    epoch: u64,
    effective_diff_ms: u32,
    updated_maps: Vec<MapTickParticipantLikeCpp>,
    destroyed_maps: Vec<MapTickParticipantLikeCpp>,
    next_participant: usize,
    in_flight: Option<ObjectMapInFlight>,
    resumed_keys: Vec<MapKey>,
}

/// The owned continuation between one map's prepare and finish phases.
///
/// The map borrow never escapes `prepare_next_object_map`; the token also
/// carries the admission identity so finish can reject a replacement under
/// the same key.
pub struct ObjectMapUpdateToken {
    origin: Arc<()>,
    epoch: u64,
    participant_index: usize,
    participant: MapTickParticipantLikeCpp,
    effective_diff_ms: u32,
    pub(super) continuation: super::map_update::ObjectUpdateContinuation,
    accounting_started: bool,
    pub(super) actor_operation: Option<ActorStepIdentity>,
}

/// One outstanding actor operation. Requests do not own or clone this identity;
/// dropping a request cannot release the map's pending-operation slot.
pub(super) struct ActorStepIdentity {
    origin: Arc<()>,
    epoch: u64,
    participant_index: usize,
    participant: MapTickParticipantLikeCpp,
    pub(super) guid: ObjectGuid,
    pub(super) witness: crate::map::CreatureActorWitness,
}

impl ActorStepIdentity {
    pub(super) fn new(
        token: &ObjectMapUpdateToken,
        guid: ObjectGuid,
        witness: crate::map::CreatureActorWitness,
    ) -> Self {
        Self {
            origin: Arc::clone(&token.origin),
            epoch: token.epoch,
            participant_index: token.participant_index,
            participant: token.participant,
            guid,
            witness,
        }
    }

    pub(super) fn matches_token(&self, token: &ObjectMapUpdateToken) -> bool {
        Arc::ptr_eq(&self.origin, &token.origin)
            && self.epoch == token.epoch
            && self.participant_index == token.participant_index
            && self.participant == token.participant
    }
}

impl ObjectMapUpdateToken {
    /// Private provenance for the reserved loot handle; no slot ownership escapes.
    pub(in crate::manager) fn actor_loot_epoch(&self) -> u64 {
        self.epoch
    }

    #[must_use]
    pub const fn key(&self) -> MapKey {
        self.participant.key
    }

    #[must_use]
    pub const fn incarnation(&self) -> u64 {
        self.participant.incarnation
    }

    #[must_use]
    pub const fn effective_diff_ms(&self) -> u32 {
        self.effective_diff_ms
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectMapTickError {
    OriginMismatch {
        plan_epoch: u64,
    },
    WrongEpoch {
        expected_epoch: u64,
        actual_epoch: u64,
    },
    WrongState {
        expected_epoch: u64,
        state: MapTickCoordinationStateLikeCpp,
    },
    MapInFlight {
        participant: MapTickParticipantLikeCpp,
    },
    NoMapInFlight,
    RespawnOperationInFlight,
    TokenMismatch {
        key: MapKey,
    },
    ActorOperationInFlight {
        guid: ObjectGuid,
    },
    Incomplete {
        processed_participants: usize,
        total_participants: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectMapFinishOutcome {
    Completed,
    StaleParticipant {
        key: MapKey,
        admitted_incarnation: u64,
        current_incarnation: Option<u64>,
    },
}

#[derive(Clone, Copy)]
struct ObjectMapInFlight {
    participant_index: usize,
    participant: MapTickParticipantLikeCpp,
    accounting_started: bool,
}

fn state_error(expected_epoch: u64, state: MapTickCoordinationStateLikeCpp) -> ObjectMapTickError {
    match state {
        MapTickCoordinationStateLikeCpp::AwaitingSessions(actual_epoch)
        | MapTickCoordinationStateLikeCpp::Resuming(actual_epoch)
            if actual_epoch != expected_epoch =>
        {
            ObjectMapTickError::WrongEpoch {
                expected_epoch,
                actual_epoch,
            }
        }
        _ => ObjectMapTickError::WrongState {
            expected_epoch,
            state,
        },
    }
}

impl MapManager {
    /// Move an admitted tick from its session pass into staged object updates.
    pub fn begin_object_tick(
        &mut self,
        plan: MapTickPlanLikeCpp,
    ) -> Result<MapObjectTickContinuation, ObjectMapTickError> {
        self.try_begin_object_tick(plan)
            .map_err(|(error, _original_plan)| error)
    }

    /// Rejection before the first state write returns the original admitted plan.
    pub fn try_begin_object_tick(
        &mut self,
        plan: MapTickPlanLikeCpp,
    ) -> Result<MapObjectTickContinuation, (ObjectMapTickError, MapTickPlanLikeCpp)> {
        if !self.respawn_ready_for_objects(&plan) {
            return Err((ObjectMapTickError::RespawnOperationInFlight, plan));
        }
        let epoch = plan.epoch_like_cpp();
        if !self.owns_tick_plan(&plan) {
            return Err((
                ObjectMapTickError::OriginMismatch { plan_epoch: epoch },
                plan,
            ));
        }
        if self.tick_coordination_like_cpp
            != MapTickCoordinationStateLikeCpp::AwaitingSessions(epoch)
        {
            return Err((state_error(epoch, self.tick_coordination_like_cpp), plan));
        }

        let updated_maps = plan.updated_maps_like_cpp().to_vec();
        let destroyed_maps = plan.destroyed_maps_like_cpp().to_vec();
        let effective_diff_ms = plan.effective_diff_ms();
        self.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Resuming(epoch);

        Ok(MapObjectTickContinuation {
            origin: Arc::clone(&self.tick_origin),
            epoch,
            effective_diff_ms,
            resumed_keys: Vec::with_capacity(updated_maps.len()),
            updated_maps,
            destroyed_maps,
            next_participant: 0,
            in_flight: None,
        })
    }

    /// Prepare DynamicObjects for the next still-admitted map, retaining its
    /// exact selection plan until that map's Creature and object-tail phases.
    pub fn prepare_next_object_map(
        &mut self,
        tick: &mut MapObjectTickContinuation,
        object_update_selection: MapObjectUpdateSelectionLikeCpp,
    ) -> Result<Option<ObjectMapUpdateToken>, ObjectMapTickError> {
        self.require_resuming_object_tick(tick)?;
        if self.active_respawn.is_some() {
            return Err(ObjectMapTickError::RespawnOperationInFlight);
        }
        if let Some(in_flight) = tick.in_flight {
            return Err(ObjectMapTickError::MapInFlight {
                participant: in_flight.participant,
            });
        }

        while tick.next_participant < tick.updated_maps.len() {
            let participant_index = tick.next_participant;
            let participant = tick.updated_maps[participant_index];
            tick.next_participant += 1;

            if self.map_incarnation_like_cpp(participant.key) != Some(participant.incarnation)
                || !self.maps.contains_key(&participant.key)
            {
                continue;
            }

            let accounting_started = self.updater.begin_staged_object_map();
            let in_flight = ObjectMapInFlight {
                participant_index,
                participant,
                accounting_started,
            };
            tick.in_flight = Some(in_flight);

            let Some(map) = self.maps.get_mut(&participant.key) else {
                self.updater.finish_staged_object_map(accounting_started);
                tick.in_flight = None;
                continue;
            };
            let continuation =
                map.prepare_object_update(tick.effective_diff_ms, object_update_selection);

            return Ok(Some(ObjectMapUpdateToken {
                origin: Arc::clone(&tick.origin),
                epoch: tick.epoch,
                participant_index,
                participant,
                effective_diff_ms: tick.effective_diff_ms,
                continuation,
                accounting_started,
                actor_operation: None,
            }));
        }

        Ok(None)
    }

    /// Finish one prepared map. A stale incarnation resolves its updater
    /// accounting and in-flight slot without mutating the replacement.
    pub fn finish_object_map<L>(
        &mut self,
        tick: &mut MapObjectTickContinuation,
        token: ObjectMapUpdateToken,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        load_record: Option<&mut L>,
        creature_update_owner: MapCreatureUpdateOwnerLikeCpp,
    ) -> Result<ObjectMapFinishOutcome, ObjectMapTickError>
    where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        self.try_finish_object_map(tick, token, pool_update, load_record, creature_update_owner)
            .map_err(|(error, _token)| error)
    }

    /// Recoverable finish for the staged actor consumer. Every rejected
    /// gate returns the complete token without advancing Creature, tail or
    /// accounting. The public compatibility path never opens an actor operation;
    /// dropping its rejected token remains fail-stop, not settlement.
    pub fn try_finish_object_map<L>(
        &mut self,
        tick: &mut MapObjectTickContinuation,
        token: ObjectMapUpdateToken,
        pool_update: Option<(&SpawnStore, &PoolMgrLikeCpp)>,
        load_record: Option<&mut L>,
        creature_update_owner: MapCreatureUpdateOwnerLikeCpp,
    ) -> Result<ObjectMapFinishOutcome, (ObjectMapTickError, ObjectMapUpdateToken)>
    where
        L: FnMut(&mut Map, SpawnObjectType, SpawnId) -> Option<LoadedGridRespawnRecordsLikeCpp>,
    {
        let stale = match self.validate_object_map_token(tick, &token) {
            Ok(stale) => stale,
            Err(error) => return Err((error, token)),
        };
        if self.active_respawn.is_some() {
            return Err((ObjectMapTickError::RespawnOperationInFlight, token));
        }
        if let Some(operation) = &token.actor_operation {
            return Err((
                ObjectMapTickError::ActorOperationInFlight {
                    guid: operation.guid,
                },
                token,
            ));
        }
        if let Some(stale) = stale {
            self.updater
                .finish_staged_object_map(token.accounting_started);
            tick.in_flight = None;
            return Ok(stale);
        }

        let map = self
            .maps
            .get_mut(&token.participant.key)
            .expect("current token validation retains the map under this exclusive manager borrow");
        map.run_creature_phase(&token.continuation, creature_update_owner);
        map.finish_object_update(token.continuation, pool_update, load_record);

        self.updater
            .finish_staged_object_map(token.accounting_started);
        tick.in_flight = None;
        tick.resumed_keys.push(token.participant.key);
        Ok(ObjectMapFinishOutcome::Completed)
    }

    /// Preserve finish's admission/error priority while exposing its current-map
    /// classification to internal actor operations. A stale result has not yet
    /// settled updater accounting; only recoverable finish performs that write.
    pub(super) fn validate_object_map_token(
        &self,
        tick: &MapObjectTickContinuation,
        token: &ObjectMapUpdateToken,
    ) -> Result<Option<ObjectMapFinishOutcome>, ObjectMapTickError> {
        self.require_resuming_object_tick(tick)?;
        if !Arc::ptr_eq(&self.tick_origin, &token.origin) {
            return Err(ObjectMapTickError::OriginMismatch {
                plan_epoch: token.epoch,
            });
        }
        if token.epoch != tick.epoch {
            return Err(ObjectMapTickError::WrongEpoch {
                expected_epoch: tick.epoch,
                actual_epoch: token.epoch,
            });
        }
        let Some(in_flight) = tick.in_flight else {
            return Err(ObjectMapTickError::NoMapInFlight);
        };
        if in_flight.participant_index != token.participant_index
            || in_flight.participant != token.participant
            || in_flight.accounting_started != token.accounting_started
            || token.effective_diff_ms != tick.effective_diff_ms
            || token.continuation.effective_diff_ms() != token.effective_diff_ms
        {
            return Err(ObjectMapTickError::TokenMismatch {
                key: token.participant.key,
            });
        }

        let current_incarnation = self.map_incarnation_like_cpp(token.participant.key);
        if current_incarnation != Some(token.participant.incarnation)
            || !self.maps.contains_key(&token.participant.key)
        {
            return Ok(Some(ObjectMapFinishOutcome::StaleParticipant {
                key: token.participant.key,
                admitted_incarnation: token.participant.incarnation,
                current_incarnation,
            }));
        }
        Ok(None)
    }

    /// Complete the manager-wide tail only after every admitted map has
    /// finished or been discarded as stale.
    pub fn finalize_object_tick(
        &mut self,
        tick: MapObjectTickContinuation,
    ) -> Result<(), ObjectMapTickError> {
        self.try_finalize_object_tick(tick)
            .map_err(|(error, _original_tick)| error)
    }

    /// Preflight rejection returns the original continuation before any tail
    /// effect. Success consumes it once; no post-write rollback is attempted.
    pub fn try_finalize_object_tick(
        &mut self,
        tick: MapObjectTickContinuation,
    ) -> Result<(), (ObjectMapTickError, MapObjectTickContinuation)> {
        if self.active_respawn.is_some() {
            return Err((ObjectMapTickError::RespawnOperationInFlight, tick));
        }
        if let Err(error) = self.require_resuming_object_tick(&tick) {
            return Err((error, tick));
        }
        if let Some(in_flight) = tick.in_flight {
            return Err((
                ObjectMapTickError::MapInFlight {
                    participant: in_flight.participant,
                },
                tick,
            ));
        }
        if tick.next_participant < tick.updated_maps.len() {
            return Err((
                ObjectMapTickError::Incomplete {
                    processed_participants: tick.next_participant,
                    total_participants: tick.updated_maps.len(),
                },
                tick,
            ));
        }

        if self.updater.activated() {
            self.updater.wait();
        }

        // Preserve the old tail order: visibility export, admitted destruction,
        // delayed update for all current survivors, then timer/state reset.
        self.retain_selected_player_visibility_refreshes_like_cpp(tick.resumed_keys);
        for participant in &tick.destroyed_maps {
            if self.map_incarnation_like_cpp(participant.key) != Some(participant.incarnation) {
                continue;
            }
            self.maps.remove(&participant.key);
            self.map_incarnations_like_cpp.remove(&participant.key);
        }
        for map in self.maps.values_mut() {
            map.delayed_update(tick.effective_diff_ms);
        }

        self.timer.set_current(0);
        self.tick_coordination_like_cpp = MapTickCoordinationStateLikeCpp::Idle;
        Ok(())
    }

    fn require_resuming_object_tick(
        &self,
        tick: &MapObjectTickContinuation,
    ) -> Result<(), ObjectMapTickError> {
        if !Arc::ptr_eq(&self.tick_origin, &tick.origin) {
            return Err(ObjectMapTickError::OriginMismatch {
                plan_epoch: tick.epoch,
            });
        }
        match self.tick_coordination_like_cpp {
            MapTickCoordinationStateLikeCpp::Resuming(epoch) if epoch == tick.epoch => Ok(()),
            state => Err(state_error(tick.epoch, state)),
        }
    }
}

#[cfg(test)]
#[path = "actor_tick_access/tests.rs"]
mod actor_tick_tests;

#[cfg(test)]
#[path = "tick_objects/finalize_tests.rs"]
mod finalize_tests;

#[cfg(test)]
#[path = "tick_objects/begin_tests.rs"]
mod begin_tests;
