//! Admission and completion of the existing producers and session finalizers.
//! No actor state lives here. Dropping a ticket never acknowledges its effects.
//! C++ a5f8da2e: World.cpp:2704/2748/3420-3431 and WorldSession.cpp:162-167.
//! Main.cpp:390-393 retains KickAll -> UpdateSessions(1) -> StopNetwork;
//! this Rust handover accounts for asynchronous owners before those effects.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use crate::ActiveWorldSessionRegistryLikeCpp;

static NEXT_ISSUER: AtomicU64 = AtomicU64::new(1);

pub(super) fn next_coordination_issuer() -> u64 {
    NEXT_ISSUER.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProducerKind {
    Canonical,
    Legacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TickPhase {
    World,
    Map,
    Objects,
    PostTail,
    LegacyAdmission,
    Legacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TickDisposition {
    FullyFinished,
    AbandonedAfterAccounting,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ProducerOrigin {
    issuer: u64,
    id: u64,
    kind: ProducerKind,
}

#[derive(Debug)]
struct TickRecord {
    epoch: u64,
    phase: TickPhase,
    final_tick: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FinalizerState {
    Waiting,
    Running,
    Uncertain,
}

#[derive(Debug)]
struct FinalizerRecord {
    owns_world_pass: bool,
    state: FinalizerState,
}

#[derive(Debug, Default)]
pub(super) struct CoordinationLedger {
    shutdown_requested: bool,
    drain_authorized: bool,
    generation: u64,
    next_id: u64,
    producers: BTreeMap<u64, ProducerKind>,
    ticks: BTreeMap<u64, TickRecord>,
    finalizers: BTreeMap<u64, FinalizerRecord>,
    final_ticks_authorized: bool,
    final_ticks_used: BTreeSet<u64>,
}

impl CoordinationLedger {
    pub(super) fn close(&mut self) {
        if !self.shutdown_requested {
            self.shutdown_requested = true;
            self.generation += 1;
        }
    }

    pub(super) fn shutdown_requested(&self) -> bool {
        self.shutdown_requested
    }

    fn settled(&self) -> bool {
        self.ticks.is_empty()
            && self
                .finalizers
                .values()
                .all(|writer| writer.state == FinalizerState::Waiting)
    }

    fn legacy_settled(&self) -> bool {
        self.ticks
            .keys()
            .all(|id| self.producers.get(id) != Some(&ProducerKind::Legacy))
    }
}

#[derive(Debug)]
pub(crate) struct QuiescenceRequest {
    issuer: u64,
    generation: u64,
}

#[derive(Debug)]
/// Producer work and active writers have settled; waiting intents may remain.
pub(crate) struct QuiescenceReceipt {
    issuer: u64,
    generation: u64,
}

#[derive(Debug)]
/// Registry, ticks and all finalizer intents are empty at this closed boundary.
/// This does not acknowledge the independent quest-complete DB processor.
pub(crate) struct TerminalSettlementReceipt {
    issuer: u64,
    generation: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum QuiescenceFailure {
    WrongClosure,
    TimedOut,
    Busy,
}

#[derive(Debug)]
#[must_use = "complete the admitted work explicitly; Drop retains uncertainty"]
pub(crate) struct TickAdmission {
    registry: Arc<ActiveWorldSessionRegistryLikeCpp>,
    origin: ProducerOrigin,
    epoch: u64,
}

impl TickAdmission {
    pub(crate) fn enter_phase(&self, phase: TickPhase) {
        let mut inner = self
            .registry
            .inner
            .lock()
            .expect("session coordination lock poisoned");
        let tick = inner
            .coordination
            .ticks
            .get_mut(&self.origin.id)
            .expect("admitted tick remains owned");
        assert_eq!(tick.epoch, self.epoch);
        tick.phase = phase;
    }

    pub(crate) fn complete(self, disposition: TickDisposition) -> bool {
        let mut inner = self
            .registry
            .inner
            .lock()
            .expect("session coordination lock poisoned");
        let ledger = &mut inner.coordination;
        let Some(tick) = ledger.ticks.get(&self.origin.id) else {
            return false;
        };
        let phase_matches = match disposition {
            TickDisposition::FullyFinished => {
                matches!(tick.phase, TickPhase::PostTail | TickPhase::Legacy)
            }
            TickDisposition::AbandonedAfterAccounting => matches!(
                tick.phase,
                TickPhase::World | TickPhase::Map | TickPhase::LegacyAdmission
            ),
        };
        if tick.epoch != self.epoch || !phase_matches {
            return false;
        }
        let final_tick = tick.final_tick;
        ledger.ticks.remove(&self.origin.id);
        if final_tick && disposition == TickDisposition::FullyFinished {
            ledger.final_ticks_used.insert(self.origin.id);
        }
        drop(inner);
        self.registry.coordination_changed.notify_waiters();
        true
    }
}

#[derive(Debug)]
#[must_use = "retain the writer through finalization and destruction"]
pub(crate) struct FinalizationAdmission {
    registry: Arc<ActiveWorldSessionRegistryLikeCpp>,
    id: u64,
    completed: bool,
}

impl FinalizationAdmission {
    pub(crate) async fn wait(&self) {
        loop {
            let notified = self.registry.coordination_changed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            {
                let mut inner = self
                    .registry
                    .inner
                    .lock()
                    .expect("session coordination lock poisoned");
                let ledger = &mut inner.coordination;
                let writer = ledger
                    .finalizers
                    .get(&self.id)
                    .expect("finalizer remains owned");
                let owns_world_pass = writer.owns_world_pass;
                let may_run = writer.state == FinalizerState::Waiting
                    && !ledger
                        .finalizers
                        .values()
                        .any(|other| other.state != FinalizerState::Waiting)
                    && if owns_world_pass {
                        // The canonical producer is awaiting this writer's own
                        // World permit. Waiting for it here would wait for self.
                        ledger.legacy_settled()
                    } else {
                        ledger.ticks.is_empty()
                            && (!ledger.shutdown_requested || ledger.drain_authorized)
                    };
                if may_run {
                    ledger.finalizers.get_mut(&self.id).unwrap().state = FinalizerState::Running;
                    return;
                }
            }
            notified.await;
        }
    }

    pub(crate) fn retain(&self) {
        self.registry
            .inner
            .lock()
            .expect("session coordination lock poisoned")
            .coordination
            .finalizers
            .get_mut(&self.id)
            .expect("finalizer remains owned")
            .state = FinalizerState::Uncertain;
        self.registry.coordination_changed.notify_waiters();
    }

    pub(crate) fn complete(mut self) {
        let mut inner = self
            .registry
            .inner
            .lock()
            .expect("session coordination lock poisoned");
        assert_eq!(
            inner.coordination.finalizers.get(&self.id).unwrap().state,
            FinalizerState::Running
        );
        inner.coordination.finalizers.remove(&self.id);
        self.completed = true;
        drop(inner);
        self.registry.coordination_changed.notify_waiters();
    }
}

impl Drop for FinalizationAdmission {
    fn drop(&mut self) {
        if !self.completed {
            // Cancellation records uncertainty even for an unstarted intent.
            // It never removes an obligation or reopens producer admission.
            if let Ok(mut inner) = self.registry.inner.lock() {
                if let Some(writer) = inner.coordination.finalizers.get_mut(&self.id) {
                    writer.state = FinalizerState::Uncertain;
                }
            }
            self.registry.coordination_changed.notify_waiters();
        }
    }
}

impl ActiveWorldSessionRegistryLikeCpp {
    pub(crate) fn register_producer(&self, kind: ProducerKind) -> ProducerOrigin {
        let mut inner = self
            .inner
            .lock()
            .expect("session coordination lock poisoned");
        let ledger = &mut inner.coordination;
        ledger.next_id += 1;
        let origin = ProducerOrigin {
            issuer: self.coordination_issuer,
            id: ledger.next_id,
            kind,
        };
        ledger.producers.insert(origin.id, kind);
        origin
    }

    pub(crate) fn try_admit_tick(
        self: &Arc<Self>,
        origin: ProducerOrigin,
        epoch: u64,
        final_tick: bool,
    ) -> Option<TickAdmission> {
        let mut inner = self
            .inner
            .lock()
            .expect("session coordination lock poisoned");
        let sessions_empty = inner.sessions.is_empty();
        let ledger = &mut inner.coordination;
        if origin.issuer != self.coordination_issuer
            || ledger.producers.get(&origin.id) != Some(&origin.kind)
            || ledger.ticks.contains_key(&origin.id)
            || !ledger.finalizers.is_empty()
            || if final_tick {
                !ledger.final_ticks_authorized
                    || !sessions_empty
                    || !ledger.ticks.is_empty()
                    || ledger.final_ticks_used.contains(&origin.id)
            } else {
                ledger.shutdown_requested
            }
        {
            return None;
        }
        ledger.ticks.insert(
            origin.id,
            TickRecord {
                epoch,
                final_tick,
                phase: match origin.kind {
                    ProducerKind::Canonical => TickPhase::World,
                    ProducerKind::Legacy => TickPhase::LegacyAdmission,
                },
            },
        );
        Some(TickAdmission {
            registry: Arc::clone(self),
            origin,
            epoch,
        })
    }

    pub(crate) fn close_tick_admission(&self) -> QuiescenceRequest {
        let mut inner = self
            .inner
            .lock()
            .expect("session coordination lock poisoned");
        inner.coordination.close();
        let request = QuiescenceRequest {
            issuer: self.coordination_issuer,
            generation: inner.coordination.generation,
        };
        drop(inner);
        self.coordination_changed.notify_waiters();
        request
    }

    pub(crate) async fn wait_for_quiescence(
        &self,
        request: QuiescenceRequest,
        deadline: Duration,
    ) -> Result<QuiescenceReceipt, QuiescenceFailure> {
        self.wait_for_settlement(request, deadline, false).await
    }

    pub(crate) async fn wait_for_terminal_settlement(
        &self,
        request: QuiescenceRequest,
        deadline: Duration,
    ) -> Result<TerminalSettlementReceipt, QuiescenceFailure> {
        let receipt = self.wait_for_settlement(request, deadline, true).await?;
        Ok(TerminalSettlementReceipt {
            issuer: receipt.issuer,
            generation: receipt.generation,
        })
    }

    async fn wait_for_settlement(
        &self,
        request: QuiescenceRequest,
        deadline: Duration,
        terminal: bool,
    ) -> Result<QuiescenceReceipt, QuiescenceFailure> {
        tokio::time::timeout(deadline, async {
            loop {
                let notified = self.coordination_changed.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                {
                    let inner = self
                        .inner
                        .lock()
                        .expect("session coordination lock poisoned");
                    let ledger = &inner.coordination;
                    if request.issuer != self.coordination_issuer
                        || request.generation != ledger.generation
                        || !ledger.shutdown_requested
                    {
                        return Err(QuiescenceFailure::WrongClosure);
                    }
                    let settled = if terminal {
                        inner.sessions.is_empty()
                            && ledger.ticks.is_empty()
                            && ledger.finalizers.is_empty()
                    } else {
                        ledger.settled()
                    };
                    if settled {
                        return Ok(QuiescenceReceipt {
                            issuer: request.issuer,
                            generation: request.generation,
                        });
                    }
                }
                notified.await;
            }
        })
        .await
        .map_err(|_| QuiescenceFailure::TimedOut)?
    }

    pub(crate) fn enable_session_drain(
        &self,
        receipt: QuiescenceReceipt,
    ) -> Result<(), QuiescenceFailure> {
        let mut inner = self
            .inner
            .lock()
            .expect("session coordination lock poisoned");
        let ledger = &mut inner.coordination;
        if receipt.issuer != self.coordination_issuer
            || receipt.generation != ledger.generation
            || !ledger.shutdown_requested
        {
            return Err(QuiescenceFailure::WrongClosure);
        }
        if !ledger.settled() {
            return Err(QuiescenceFailure::Busy);
        }
        ledger.drain_authorized = true;
        drop(inner);
        self.coordination_changed.notify_waiters();
        Ok(())
    }

    pub(crate) fn session_drain_authorized(&self) -> bool {
        self.inner
            .lock()
            .expect("session coordination lock poisoned")
            .coordination
            .drain_authorized
    }

    pub(crate) fn admit_finalization(
        self: &Arc<Self>,
        world_pass: Option<&wow_world::session::mailbox::PendingWorldPhaseFinalizationLikeCpp>,
    ) -> FinalizationAdmission {
        let owns_world_pass = world_pass.is_some();
        let mut inner = self
            .inner
            .lock()
            .expect("session coordination lock poisoned");
        let ledger = &mut inner.coordination;
        ledger.next_id += 1;
        let id = ledger.next_id;
        ledger.finalizers.insert(
            id,
            FinalizerRecord {
                owns_world_pass,
                state: FinalizerState::Waiting,
            },
        );
        FinalizationAdmission {
            registry: Arc::clone(self),
            id,
            completed: false,
        }
    }

    pub(crate) fn withdraw_from_phases(&self, session_id: u64) {
        let inner = self
            .inner
            .lock()
            .expect("session coordination lock poisoned");
        if let Some(session) = inner.sessions.get(&session_id) {
            session
                .ready_for_phases_like_cpp
                .store(false, Ordering::Release);
        }
    }

    pub(crate) fn authorize_final_respawn_tick(
        &self,
        receipt: TerminalSettlementReceipt,
    ) -> Result<(), QuiescenceFailure> {
        let mut inner = self
            .inner
            .lock()
            .expect("session coordination lock poisoned");
        let sessions_empty = inner.sessions.is_empty();
        let ledger = &mut inner.coordination;
        if receipt.issuer != self.coordination_issuer || receipt.generation != ledger.generation {
            return Err(QuiescenceFailure::WrongClosure);
        }
        if !ledger.shutdown_requested
            || !ledger.drain_authorized
            || !sessions_empty
            || !ledger.ticks.is_empty()
            || !ledger.finalizers.is_empty()
        {
            return Err(QuiescenceFailure::Busy);
        }
        ledger.final_ticks_authorized = true;
        Ok(())
    }

    pub(crate) fn final_respawn_tick_authorized(&self) -> bool {
        self.inner
            .lock()
            .expect("session coordination lock poisoned")
            .coordination
            .final_ticks_authorized
    }

    pub(crate) fn close_final_tick_admission(&self) {
        let mut inner = self
            .inner
            .lock()
            .expect("session coordination lock poisoned");
        if inner.coordination.final_ticks_authorized {
            inner.coordination.final_ticks_authorized = false;
            inner.coordination.generation += 1;
        }
        drop(inner);
        self.coordination_changed.notify_waiters();
    }
}

#[cfg(test)]
mod tests;
