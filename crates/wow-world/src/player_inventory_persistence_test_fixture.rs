use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use wow_persistence::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerInventoryPersistencePortLikeCpp,
    PlayerInventoryPersistenceRequestLikeCpp,
};

pub(crate) struct PlayerInventoryPersistencePortFixtureLikeCpp {
    requests: Arc<Mutex<Vec<PlayerInventoryPersistenceRequestLikeCpp>>>,
    outcome: PersistenceOutcomeLikeCpp,
    outcomes: Mutex<VecDeque<PersistenceOutcomeLikeCpp>>,
}

impl PlayerInventoryPersistencePortFixtureLikeCpp {
    pub(crate) fn new_like_cpp(
        outcome: PersistenceOutcomeLikeCpp,
    ) -> (
        Arc<Self>,
        Arc<Mutex<Vec<PlayerInventoryPersistenceRequestLikeCpp>>>,
    ) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        (
            Arc::new(Self {
                requests: Arc::clone(&requests),
                outcome,
                outcomes: Mutex::new(VecDeque::new()),
            }),
            requests,
        )
    }

    pub(crate) fn with_outcomes_like_cpp(
        outcomes: impl IntoIterator<Item = PersistenceOutcomeLikeCpp>,
    ) -> (
        Arc<Self>,
        Arc<Mutex<Vec<PlayerInventoryPersistenceRequestLikeCpp>>>,
    ) {
        let requests = Arc::new(Mutex::new(Vec::new()));
        (
            Arc::new(Self {
                requests: Arc::clone(&requests),
                outcome: PersistenceOutcomeLikeCpp::Applied { rows: 0 },
                outcomes: Mutex::new(outcomes.into_iter().collect()),
            }),
            requests,
        )
    }

    pub(crate) fn failed() -> Arc<Self> {
        Self::new_like_cpp(PersistenceOutcomeLikeCpp::Failed {
            reason: "inventory fixture rollback".into(),
        })
        .0
    }
}

impl PlayerInventoryPersistencePortLikeCpp for PlayerInventoryPersistencePortFixtureLikeCpp {
    fn persist_inventory_mutation_like_cpp(
        &self,
        request: PlayerInventoryPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        self.requests.lock().unwrap().push(request);
        let outcome = self
            .outcomes
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| self.outcome.clone());
        Box::pin(async move { outcome })
    }
}
