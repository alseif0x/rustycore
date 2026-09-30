//! Controlled futures implement the production inventory port, not a grant stub.
use super::support::*;
use std::collections::VecDeque;
use std::sync::Mutex;
use wow_persistence::{PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerInventoryPersistencePortLikeCpp, PlayerInventoryPersistenceRequestLikeCpp};

pub(super) struct ControlledLootInventoryPort {
    requests: Mutex<Vec<PlayerInventoryPersistenceRequestLikeCpp>>,
    outcomes: Mutex<VecDeque<PersistenceOutcomeLikeCpp>>,
    commit_gate: Option<Arc<tokio::sync::Notify>>,
}

impl ControlledLootInventoryPort {
    pub(super) fn new(outcomes: impl IntoIterator<Item = PersistenceOutcomeLikeCpp>, commit_gate: Option<Arc<tokio::sync::Notify>>) -> Arc<Self> {
        Arc::new(Self { requests: Mutex::new(Vec::new()), outcomes: Mutex::new(outcomes.into_iter().collect()), commit_gate })
    }
}

impl PlayerInventoryPersistencePortLikeCpp for ControlledLootInventoryPort {
    fn persist_inventory_mutation_like_cpp(&self, request: PlayerInventoryPersistenceRequestLikeCpp) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self.outcomes.lock().unwrap().pop_front().expect("one planned result for each real transaction");
        let gate = self.commit_gate.clone();
        Box::pin(async move {
            tokio::task::yield_now().await;
            if let Some(gate) = gate { gate.notified().await; }
            outcome
        })
    }
}

pub(super) fn install_storage_port(session: &mut WorldSession, outcome: PersistenceOutcomeLikeCpp, gate: Option<Arc<tokio::sync::Notify>>) {
    wow_world::test_fixtures::loot::prepare_money_player_residence_for_test(session);
    wow_world::test_fixtures::loot::attach_loot_inventory_port_for_test(session, ControlledLootInventoryPort::new([outcome], gate));
}

