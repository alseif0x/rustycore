//! Real production executor with controlled persistence. No DB durability claim.
use super::*;
use std::future::Future;
use std::sync::Mutex;
use wow_world::{
    FinalizationDisposition as Disposition, FinalizationMode as Mode,
    FinalizationOutcome as Outcome, FinalizationStep as Step,
};

pub(super) struct Probe {
    pub target: Step,
    pub result: PersistenceOutcomeLikeCpp,
    pub released: AtomicBool,
    pub seen: Mutex<Vec<Step>>,
}

impl Probe {
    fn execute(
        self: Arc<Self>,
        step: Step,
    ) -> PersistenceFutureLikeCpp<'static, PersistenceOutcomeLikeCpp> {
        Box::pin(async move {
            self.seen.lock().unwrap().push(step);
            if step == self.target {
                std::future::poll_fn(|_| {
                    if self.released.load(Ordering::SeqCst) {
                        std::task::Poll::Ready(())
                    } else {
                        std::task::Poll::Pending
                    }
                })
                .await;
                self.result.clone()
            } else {
                PersistenceOutcomeLikeCpp::Applied { rows: 0 }
            }
        })
    }

    pub(super) fn offline(
        self: Arc<Self>,
        mark: PlayerOfflineMarkLikeCpp,
    ) -> PersistenceFutureLikeCpp<'static, PersistenceOutcomeLikeCpp> {
        let step = match mark {
            PlayerOfflineMarkLikeCpp::Character { .. } => Step::CharacterOffline,
            PlayerOfflineMarkLikeCpp::CharacterAccount { .. } => Step::CharacterAccountOffline,
            PlayerOfflineMarkLikeCpp::LoginAccount { .. } => Step::LoginAccountOffline,
        };
        self.execute(step)
    }

    pub(super) fn collection(
        self: Arc<Self>,
        save: AccountCollectionSaveLikeCpp,
    ) -> PersistenceFutureLikeCpp<'static, PersistenceOutcomeLikeCpp> {
        let step = match save {
            AccountCollectionSaveLikeCpp::Mounts(_) => Step::Mounts,
            AccountCollectionSaveLikeCpp::Toys(_) => Step::Toys,
            AccountCollectionSaveLikeCpp::Heirlooms(_) => Step::Heirlooms,
            AccountCollectionSaveLikeCpp::ItemAppearances { .. } => Step::Appearances,
            AccountCollectionSaveLikeCpp::TransmogIllusions { .. } => Step::Illusions,
        };
        self.execute(step)
    }
}

async fn setup(
    target: Step,
    result: PersistenceOutcomeLikeCpp,
    released: bool,
) -> (
    WorldSession,
    Arc<LoginPort>,
    flume::Receiver<Vec<u8>>,
    Arc<Probe>,
) {
    let (session, port, _, receiver) = hydrate(true, true, true).await;
    *port.save_probe.lock().unwrap() = Some(Arc::new(save::SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(true),
        outcome: PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    }));
    let probe = Arc::new(Probe {
        target,
        result,
        released: AtomicBool::new(released),
        seen: Mutex::new(vec![]),
    });
    *port.finalization_probe.lock().unwrap() = Some(probe.clone());
    (session, port, receiver, probe)
}

#[tokio::test]
async fn production_finalization_retires_only_after_all_disconnect_obligations() {
    let (mut session, port, _receiver, probe) = setup(
        Step::LoginAccountOffline,
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        false,
    )
    .await;
    let guid = session.player_guid().unwrap();
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let mut operation =
        Box::pin(session.finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator));
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(operation.as_mut().poll(cx).is_pending()))
            .await
    );
    assert!(
        port.manager
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(guid)
            .is_some()
    );
    probe.released.store(true, Ordering::SeqCst);
    let report = operation.await;
    assert_eq!(report.disposition, Disposition::Complete);
    for step in [
        Step::CharacterSave,
        Step::CharacterOffline,
        Step::CharacterAccountOffline,
        Step::LoginAccountOffline,
        Step::Retirement,
        Step::Release,
    ] {
        assert_eq!(report.outcome(step), Outcome::Applied);
    }
    assert!(
        port.manager
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(guid)
            .is_none()
    );
    let count = probe.seen.lock().unwrap().len();
    assert_eq!(
        session
            .finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator)
            .await,
        report
    );
    assert_eq!(
        probe.seen.lock().unwrap().len(),
        count,
        "completed disconnect is not replayed"
    );
}

#[tokio::test]
async fn production_each_offline_failure_is_not_character_save_success() {
    for target in [
        Step::CharacterOffline,
        Step::CharacterAccountOffline,
        Step::LoginAccountOffline,
    ] {
        for (result, expected) in [
            (
                PersistenceOutcomeLikeCpp::Failed {
                    reason: "known rollback".into(),
                },
                Outcome::DefinitelyRolledBack,
            ),
            (
                PersistenceOutcomeLikeCpp::Unknown {
                    reason: "lost reply".into(),
                },
                Outcome::Unknown,
            ),
        ] {
            let (mut session, port, _receiver, probe) = setup(target, result, true).await;
            let guid = session.player_guid().unwrap();
            let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
            let report = session
                .finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator)
                .await;
            assert_eq!(report.outcome(Step::CharacterSave), Outcome::Applied);
            assert_eq!(report.outcome(target), expected);
            assert_eq!(report.disposition, Disposition::RetainAndEscalate);
            assert_eq!(report.outcome(Step::Retirement), Outcome::NotAttempted);
            assert_eq!(report.outcome(Step::Release), Outcome::NotAttempted);
            assert!(
                port.manager
                    .lock()
                    .unwrap()
                    .find_map(0, 0)
                    .unwrap()
                    .map()
                    .get_typed_player(guid)
                    .is_some()
            );
            let count = probe.seen.lock().unwrap().len();
            assert_eq!(
                session
                    .finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator)
                    .await,
                report
            );
            assert_eq!(probe.seen.lock().unwrap().len(), count);
        }
    }
}

#[tokio::test]
async fn production_cancelled_offline_write_retains_inflight_step_and_never_replays() {
    let (mut session, port, _receiver, probe) = setup(
        Step::CharacterAccountOffline,
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        false,
    )
    .await;
    let guid = session.player_guid().unwrap();
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let mut operation =
        Box::pin(session.finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator));
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(operation.as_mut().poll(cx).is_pending()))
            .await
    );
    drop(operation);
    let report = session.interrupt_finalization_like_cpp().unwrap();
    assert_eq!(report.outcome(Step::CharacterSave), Outcome::Applied);
    assert_eq!(
        report.outcome(Step::CharacterAccountOffline),
        Outcome::InFlight
    );
    assert_eq!(report.disposition, Disposition::RetainAndEscalate);
    probe.released.store(true, Ordering::SeqCst);
    let count = probe.seen.lock().unwrap().len();
    assert_eq!(
        session
            .finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator)
            .await,
        report
    );
    assert_eq!(probe.seen.lock().unwrap().len(), count);
    assert!(
        port.manager
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .get_typed_player(guid)
            .is_some()
    );
}

#[tokio::test]
async fn production_unpolled_finalization_has_no_admission_or_effects() {
    let (mut session, _port, _receiver, probe) = setup(
        Step::CharacterOffline,
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        true,
    )
    .await;
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    drop(session.finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator));
    assert!(session.finalization_report_like_cpp().is_none());
    assert!(probe.seen.lock().unwrap().is_empty());
    assert_eq!(
        session
            .finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator)
            .await
            .disposition,
        Disposition::Complete
    );
}

#[tokio::test]
async fn production_retirement_failure_reaches_caller_and_preserves_replacement() {
    let (mut session, port, _receiver, probe) = setup(
        Step::LoginAccountOffline,
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        false,
    )
    .await;
    let guid = session.player_guid().unwrap();
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let mut operation =
        Box::pin(session.finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator));
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(operation.as_mut().poll(cx).is_pending()))
            .await
    );
    let replacement = {
        let mut player = Box::new(wow_entities::Player::new(Some(1), false));
        player.unit_mut().world_mut().object_mut().create(guid);
        port.manager
            .lock()
            .unwrap()
            .install_detached_player_like_cpp(player)
            .unwrap()
    };
    probe.released.store(true, Ordering::SeqCst);
    let report = operation.await;
    assert_eq!(report.outcome(Step::Retirement), Outcome::RetirementFailed);
    assert_eq!(report.outcome(Step::Release), Outcome::NotAttempted);
    assert_eq!(report.disposition, Disposition::RetainAndEscalate);
    assert_ne!(report.player, Some(replacement));
    assert_eq!(session.cleanup_shared_runtime_state(), Outcome::Unavailable);
    assert!(
        port.manager
            .lock()
            .unwrap()
            .with_player_like_cpp(replacement, |_| ())
            .is_some()
    );
}

#[tokio::test]
async fn production_each_collection_failure_stops_before_offline_and_retirement() {
    for target in [
        Step::Mounts,
        Step::Toys,
        Step::Heirlooms,
        Step::Appearances,
        Step::Illusions,
    ] {
        for (result, expected) in [
            (
                PersistenceOutcomeLikeCpp::Failed {
                    reason: "known rollback".into(),
                },
                Outcome::DefinitelyRolledBack,
            ),
            (
                PersistenceOutcomeLikeCpp::Unknown {
                    reason: "lost collection commit reply".into(),
                },
                Outcome::Unknown,
            ),
        ] {
            let (mut session, port, _receiver, probe) = setup(target, result, true).await;
            let guid = session.player_guid().unwrap();
            {
                let mut owner = port.manager.lock().unwrap();
                let collections = &mut owner
                    .find_map_mut(0, 0)
                    .unwrap()
                    .map_mut()
                    .get_typed_player_mut(guid)
                    .unwrap()
                    .gameplay_state_mut()
                    .collections;
                collections.add_mount_like_cpp(458, 1);
                collections.add_toy_like_cpp(100, 1);
                collections.add_heirloom_like_cpp(
                    101,
                    wow_entities::PlayerAccountHeirloomDataLikeCpp {
                        flags: 1,
                        bonus_id: 0,
                    },
                );
                collections.add_item_appearance_like_cpp(102);
                collections
                    .replace_transmog_illusions_like_cpp(std::collections::HashSet::from([103]));
            }
            let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
            let report = session
                .finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator)
                .await;
            assert_eq!(report.outcome(Step::CharacterSave), Outcome::Applied);
            assert_eq!(report.outcome(target), expected);
            assert_eq!(report.disposition, Disposition::RetainAndEscalate);
            assert_eq!(
                report.outcome(Step::CharacterOffline),
                Outcome::NotAttempted
            );
            assert_eq!(report.outcome(Step::Retirement), Outcome::NotAttempted);
            assert_eq!(report.outcome(Step::Release), Outcome::NotAttempted);
            let calls = probe.seen.lock().unwrap().clone();
            assert_eq!(calls.last(), Some(&target));
            assert_eq!(
                session
                    .finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator)
                    .await,
                report
            );
            assert_eq!(*probe.seen.lock().unwrap(), calls);
        }
    }
}

#[tokio::test]
async fn production_saturated_logout_publication_yields_and_cancellation_keeps_release_closed() {
    let (mut session, port, output, receiver) = hydrate(true, true, true).await;
    *port.save_probe.lock().unwrap() = Some(Arc::new(save::SaveProbe {
        requests: Mutex::new(vec![]),
        released: AtomicBool::new(true),
        outcome: PersistenceOutcomeLikeCpp::Applied { rows: 1 },
    }));
    while receiver.try_recv().is_ok() {}
    for _ in 0..8 {
        output.try_send(vec![0]).unwrap();
    }
    assert!(output.is_full());
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let mut operation = Box::pin(
        session.finalize_session_with_generator_like_cpp(Mode::CharacterSelection, &generator),
    );
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(operation.as_mut().poll(cx).is_pending()))
            .await
    );
    drop(operation);
    let report = session.interrupt_finalization_like_cpp().unwrap();
    assert_eq!(report.outcome(Step::Retirement), Outcome::Applied);
    assert_eq!(report.outcome(Step::LogoutPublication), Outcome::InFlight);
    assert_eq!(
        report.outcome(Step::CharacterAccountOffline),
        Outcome::NotAttempted
    );
    assert_eq!(report.outcome(Step::Release), Outcome::NotAttempted);
    assert_eq!(report.disposition, Disposition::RetainAndEscalate);
    assert_eq!(
        session
            .finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator)
            .await,
        report
    );
}
