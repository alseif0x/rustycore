//! Session-owned account state stays behind its typed persistence port.

use super::*;

use std::sync::Mutex;
use wow_persistence::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, SessionAccountDataLoadOutcomeLikeCpp,
    SessionAccountDataRowLikeCpp, SessionAccountDataSaveLikeCpp, SessionAccountDataScopeLikeCpp,
    SessionAccountStatePortLikeCpp, SessionTutorialsLoadOutcomeLikeCpp,
};

struct RecordingSessionAccountStatePortLikeCpp {
    account_load: SessionAccountDataLoadOutcomeLikeCpp,
    tutorials_load: SessionTutorialsLoadOutcomeLikeCpp,
    save_outcome: PersistenceOutcomeLikeCpp,
    scopes: Mutex<Vec<SessionAccountDataScopeLikeCpp>>,
    saves: Mutex<Vec<SessionAccountDataSaveLikeCpp>>,
}

impl RecordingSessionAccountStatePortLikeCpp {
    fn new(
        account_load: SessionAccountDataLoadOutcomeLikeCpp,
        tutorials_load: SessionTutorialsLoadOutcomeLikeCpp,
        save_outcome: PersistenceOutcomeLikeCpp,
    ) -> Arc<Self> {
        Arc::new(Self {
            account_load,
            tutorials_load,
            save_outcome,
            scopes: Mutex::new(Vec::new()),
            saves: Mutex::new(Vec::new()),
        })
    }
}

impl SessionAccountStatePortLikeCpp for RecordingSessionAccountStatePortLikeCpp {
    fn load_account_data_like_cpp<'a>(
        &'a self,
        scope: SessionAccountDataScopeLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, SessionAccountDataLoadOutcomeLikeCpp> {
        self.scopes.lock().unwrap().push(scope);
        let outcome = self.account_load.clone();
        Box::pin(async move { outcome })
    }

    fn load_tutorials_like_cpp<'a>(
        &'a self,
        _account_id: u32,
    ) -> PersistenceFutureLikeCpp<'a, SessionTutorialsLoadOutcomeLikeCpp> {
        let outcome = self.tutorials_load.clone();
        Box::pin(async move { outcome })
    }

    fn save_account_data_like_cpp<'a>(
        &'a self,
        save: SessionAccountDataSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, PersistenceOutcomeLikeCpp> {
        self.saves.lock().unwrap().push(save);
        let outcome = self.save_outcome.clone();
        Box::pin(async move { outcome })
    }
}

#[tokio::test]
async fn account_data_load_keeps_scope_and_cpp_mask_validation_in_session() {
    let (mut session, _, _) = make_session();
    session.account_data_like_cpp[0].time = 99;
    session.account_data_like_cpp[1].time = 88;
    let port = RecordingSessionAccountStatePortLikeCpp::new(
        SessionAccountDataLoadOutcomeLikeCpp::Loaded(vec![
            SessionAccountDataRowLikeCpp {
                data_type: 0,
                time: 17,
                data: "global".to_owned(),
            },
            SessionAccountDataRowLikeCpp {
                data_type: 1,
                time: 29,
                data: "wrong table".to_owned(),
            },
            SessionAccountDataRowLikeCpp {
                data_type: u8::MAX,
                time: 31,
                data: "invalid".to_owned(),
            },
        ]),
        SessionTutorialsLoadOutcomeLikeCpp::Loaded(None),
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_session_account_state_port_like_cpp(port.clone());

    session.load_global_account_data_like_cpp().await;

    assert_eq!(session.account_data_like_cpp[0].time, 17);
    assert_eq!(session.account_data_like_cpp[0].data, "global");
    assert_eq!(session.account_data_like_cpp[1].time, 88);
    assert_eq!(
        *port.scopes.lock().unwrap(),
        [SessionAccountDataScopeLikeCpp::Global { account_id: 1 }]
    );

    let character_guid = ObjectGuid::create_player(1, 73);
    let character_port = RecordingSessionAccountStatePortLikeCpp::new(
        SessionAccountDataLoadOutcomeLikeCpp::Loaded(vec![SessionAccountDataRowLikeCpp {
            data_type: 1,
            time: 37,
            data: "character".to_owned(),
        }]),
        SessionTutorialsLoadOutcomeLikeCpp::Loaded(None),
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_session_account_state_port_like_cpp(character_port.clone());
    session
        .load_player_account_data_like_cpp(character_guid)
        .await;
    assert_eq!(session.account_data_like_cpp[1].time, 37);
    assert_eq!(
        *character_port.scopes.lock().unwrap(),
        [SessionAccountDataScopeLikeCpp::Character { guid_low: 73 }]
    );

    let empty_port = RecordingSessionAccountStatePortLikeCpp::new(
        SessionAccountDataLoadOutcomeLikeCpp::Loaded(Vec::new()),
        SessionTutorialsLoadOutcomeLikeCpp::Loaded(None),
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.account_data_like_cpp[0].time = 101;
    session.set_session_account_state_port_like_cpp(empty_port);
    session.load_global_account_data_like_cpp().await;
    assert_eq!(
        session.account_data_like_cpp[0],
        AccountDataLikeCpp::default()
    );
}

#[tokio::test]
async fn tutorial_load_publishes_only_a_successful_typed_outcome_like_cpp() {
    let (mut session, _, _) = make_session();
    let port = RecordingSessionAccountStatePortLikeCpp::new(
        SessionAccountDataLoadOutcomeLikeCpp::Loaded(Vec::new()),
        SessionTutorialsLoadOutcomeLikeCpp::Loaded(Some([1, 2, 3, 4, 5, 6, 7, 8])),
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_session_account_state_port_like_cpp(port);

    session.load_tutorials_data_like_cpp().await;

    assert_eq!(session.tutorials_like_cpp, [1, 2, 3, 4, 5, 6, 7, 8]);
    assert!(session.tutorials_loaded_from_db_like_cpp);
    assert!(session.tutorials_loaded_coherently_like_cpp);
    assert!(!session.tutorials_changed_like_cpp);

    let (mut failed_session, _, _) = make_session();
    let failed_port = RecordingSessionAccountStatePortLikeCpp::new(
        SessionAccountDataLoadOutcomeLikeCpp::Loaded(Vec::new()),
        SessionTutorialsLoadOutcomeLikeCpp::Failed {
            reason: "read failed".to_owned(),
        },
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    failed_session.set_session_account_state_port_like_cpp(failed_port);
    failed_session.load_tutorials_data_like_cpp().await;
    assert!(!failed_session.tutorials_loaded_coherently_like_cpp);
}

#[tokio::test]
async fn account_data_save_updates_memory_only_after_applied_but_keeps_missing_port_fallback() {
    let (mut session, _, _) = make_session();
    let failed = RecordingSessionAccountStatePortLikeCpp::new(
        SessionAccountDataLoadOutcomeLikeCpp::Loaded(Vec::new()),
        SessionTutorialsLoadOutcomeLikeCpp::Loaded(None),
        PersistenceOutcomeLikeCpp::Failed {
            reason: "definite failure".to_owned(),
        },
    );
    session.set_session_account_state_port_like_cpp(failed.clone());
    assert!(
        !session
            .set_account_data_persisted_like_cpp(0, 41, "failed".to_owned())
            .await
    );
    assert_eq!(
        session.account_data_like_cpp[0],
        AccountDataLikeCpp::default()
    );
    assert_eq!(failed.saves.lock().unwrap().len(), 1);

    let applied = RecordingSessionAccountStatePortLikeCpp::new(
        SessionAccountDataLoadOutcomeLikeCpp::Loaded(Vec::new()),
        SessionTutorialsLoadOutcomeLikeCpp::Loaded(None),
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    );
    session.set_session_account_state_port_like_cpp(applied.clone());
    assert!(
        session
            .set_account_data_persisted_like_cpp(0, 42, "applied".to_owned())
            .await
    );
    assert_eq!(session.account_data_like_cpp[0].time, 42);
    assert_eq!(applied.saves.lock().unwrap().len(), 1);

    session.session_account_state_port_like_cpp = None;
    assert!(
        session
            .set_account_data_persisted_like_cpp(0, 43, "fallback".to_owned())
            .await
    );
    assert_eq!(session.account_data_like_cpp[0].time, 43);
    assert_eq!(session.account_data_like_cpp[0].data, "fallback");
}
