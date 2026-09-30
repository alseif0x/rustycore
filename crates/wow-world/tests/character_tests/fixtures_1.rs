//! Shared character-handler test fixtures, part 1.
//!
//! Separated from the character_tests root under #662; every fixture is unchanged.

use super::*;

pub(super) struct HomebindPortFixtureLikeCpp {
    pub(super) requests: std::sync::Mutex<Vec<PlayerHomebindPersistenceRequestLikeCpp>>,
    pub(super) outcomes: std::sync::Mutex<std::collections::VecDeque<PersistenceOutcomeLikeCpp>>,
}

impl HomebindPortFixtureLikeCpp {
    pub(super) fn new(outcomes: impl IntoIterator<Item = PersistenceOutcomeLikeCpp>) -> Arc<Self> {
        Arc::new(Self {
            requests: std::sync::Mutex::new(Vec::new()),
            outcomes: std::sync::Mutex::new(outcomes.into_iter().collect()),
        })
    }

    pub(super) fn requests(&self) -> Vec<PlayerHomebindPersistenceRequestLikeCpp> {
        self.requests.lock().unwrap().clone()
    }
}
