use std::sync::{Arc, Mutex};

use wow_persistence::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerQuestActivePersistenceRowLikeCpp,
    PlayerQuestDailyPersistenceRowLikeCpp, PlayerQuestIdPersistenceRowLikeCpp,
    PlayerQuestLoadOutcomeLikeCpp, PlayerQuestLockoutPersistenceRequestLikeCpp,
    PlayerQuestObjectivePersistenceRowLikeCpp, PlayerQuestPersistencePortLikeCpp,
    PlayerQuestSeasonalPersistenceRowLikeCpp, PlayerQuestStatusPersistenceRequestLikeCpp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PlayerQuestLoadStageFixtureLikeCpp {
    Active,
    Objectives,
    Rewarded,
    Daily,
    Weekly,
    Monthly,
    Seasonal,
}

pub(crate) struct PlayerQuestPersistencePortFixtureLikeCpp {
    pub(crate) active: Vec<PlayerQuestActivePersistenceRowLikeCpp>,
    pub(crate) objectives: Vec<PlayerQuestObjectivePersistenceRowLikeCpp>,
    pub(crate) rewarded: Vec<PlayerQuestIdPersistenceRowLikeCpp>,
    pub(crate) daily: Vec<PlayerQuestDailyPersistenceRowLikeCpp>,
    pub(crate) weekly: Vec<PlayerQuestIdPersistenceRowLikeCpp>,
    pub(crate) monthly: Vec<PlayerQuestIdPersistenceRowLikeCpp>,
    pub(crate) seasonal: Vec<PlayerQuestSeasonalPersistenceRowLikeCpp>,
    pub(crate) stages: Arc<Mutex<Vec<PlayerQuestLoadStageFixtureLikeCpp>>>,
    pub(crate) status_requests: Arc<Mutex<Vec<PlayerQuestStatusPersistenceRequestLikeCpp>>>,
    pub(crate) lockout_requests: Arc<Mutex<Vec<PlayerQuestLockoutPersistenceRequestLikeCpp>>>,
    pub(crate) outcome: PersistenceOutcomeLikeCpp,
}

impl Default for PlayerQuestPersistencePortFixtureLikeCpp {
    fn default() -> Self {
        Self {
            active: Vec::new(),
            objectives: Vec::new(),
            rewarded: Vec::new(),
            daily: Vec::new(),
            weekly: Vec::new(),
            monthly: Vec::new(),
            seasonal: Vec::new(),
            stages: Arc::new(Mutex::new(Vec::new())),
            status_requests: Arc::new(Mutex::new(Vec::new())),
            lockout_requests: Arc::new(Mutex::new(Vec::new())),
            outcome: PersistenceOutcomeLikeCpp::Applied { rows: 0 },
        }
    }
}

impl PlayerQuestPersistencePortFixtureLikeCpp {
    fn record_stage(&self, stage: PlayerQuestLoadStageFixtureLikeCpp) {
        self.stages.lock().unwrap().push(stage);
    }
}

impl PlayerQuestPersistencePortLikeCpp for PlayerQuestPersistencePortFixtureLikeCpp {
    fn load_active_statuses_like_cpp(
        &self,
        _owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestActivePersistenceRowLikeCpp>,
    > {
        self.record_stage(PlayerQuestLoadStageFixtureLikeCpp::Active);
        let rows = self.active.clone();
        Box::pin(async move { PlayerQuestLoadOutcomeLikeCpp::Loaded(rows) })
    }

    fn load_objectives_like_cpp(
        &self,
        _owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestObjectivePersistenceRowLikeCpp>,
    > {
        self.record_stage(PlayerQuestLoadStageFixtureLikeCpp::Objectives);
        let rows = self.objectives.clone();
        Box::pin(async move { PlayerQuestLoadOutcomeLikeCpp::Loaded(rows) })
    }

    fn load_rewarded_like_cpp(
        &self,
        _owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestIdPersistenceRowLikeCpp>,
    > {
        self.record_stage(PlayerQuestLoadStageFixtureLikeCpp::Rewarded);
        let rows = self.rewarded.clone();
        Box::pin(async move { PlayerQuestLoadOutcomeLikeCpp::Loaded(rows) })
    }

    fn load_daily_like_cpp(
        &self,
        _owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestDailyPersistenceRowLikeCpp>,
    > {
        self.record_stage(PlayerQuestLoadStageFixtureLikeCpp::Daily);
        let rows = self.daily.clone();
        Box::pin(async move { PlayerQuestLoadOutcomeLikeCpp::Loaded(rows) })
    }

    fn load_weekly_like_cpp(
        &self,
        _owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestIdPersistenceRowLikeCpp>,
    > {
        self.record_stage(PlayerQuestLoadStageFixtureLikeCpp::Weekly);
        let rows = self.weekly.clone();
        Box::pin(async move { PlayerQuestLoadOutcomeLikeCpp::Loaded(rows) })
    }

    fn load_monthly_like_cpp(
        &self,
        _owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestIdPersistenceRowLikeCpp>,
    > {
        self.record_stage(PlayerQuestLoadStageFixtureLikeCpp::Monthly);
        let rows = self.monthly.clone();
        Box::pin(async move { PlayerQuestLoadOutcomeLikeCpp::Loaded(rows) })
    }

    fn load_seasonal_like_cpp(
        &self,
        _owner_guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        PlayerQuestLoadOutcomeLikeCpp<PlayerQuestSeasonalPersistenceRowLikeCpp>,
    > {
        self.record_stage(PlayerQuestLoadStageFixtureLikeCpp::Seasonal);
        let rows = self.seasonal.clone();
        Box::pin(async move { PlayerQuestLoadOutcomeLikeCpp::Loaded(rows) })
    }

    fn persist_status_like_cpp(
        &self,
        request: PlayerQuestStatusPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        self.status_requests.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }

    fn persist_lockout_like_cpp(
        &self,
        request: PlayerQuestLockoutPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        self.lockout_requests.lock().unwrap().push(request);
        let outcome = self.outcome.clone();
        Box::pin(async move { outcome })
    }
}
