use std::sync::{Arc, Mutex};

use wow_persistence::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerInventoryPersistencePortLikeCpp,
    PlayerInventoryPersistenceRequestLikeCpp,
};

pub(crate) struct PlayerInventoryPersistencePortFixtureLikeCpp {
    requests: Arc<Mutex<Vec<PlayerInventoryPersistenceRequestLikeCpp>>>,
    outcome: PersistenceOutcomeLikeCpp,
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
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }
}
