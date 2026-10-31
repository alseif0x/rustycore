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
    assert_eq!(
        report.outcome(Step::Retirement),
        Outcome::Unavailable,
        "the stale-incarnation recovery fence rejects retirement before any retirement side effect"
    );
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
    for _ in 0..6 {
        output.try_send(vec![0]).unwrap();
    }
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let mut operation = Box::pin(
        session.finalize_session_with_generator_like_cpp(Mode::CharacterSelection, &generator),
    );
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(operation.as_mut().poll(cx).is_pending()))
            .await
    );
    assert!(output.is_full());
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
    let cancel_combat = wow_constants::ServerOpcodes::CancelCombat as u16;
    let native_packets = receiver
        .try_iter()
        .filter(|bytes| bytes.len() >= 2)
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect::<Vec<_>>();
    assert_eq!(native_packets, vec![cancel_combat; 2]);
    assert_eq!(
        session
            .finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator)
            .await,
        report
    );
}

// ── `player.logout` (#1444 H1) ────────────────────────────────────────────

/// What a logout module observed: the reason, the character saves already
/// requested, the steps the ledger had run, whether the canonical map lock was
/// free and whether the player was still on the map.
type LogoutObservation = (wow_module_api::LogoutReason, usize, Vec<Step>, bool, bool);

struct LogoutProbe {
    port: Arc<LoginPort>,
    steps: Arc<Probe>,
    guid: ObjectGuid,
    calls: Mutex<Vec<LogoutObservation>>,
}

impl wow_module_api::PlayerModule for LogoutProbe {
    fn on_logout(&self, snapshot: &wow_module_api::PlayerLogoutSnapshot) {
        let saves = self
            .port
            .save_probe
            .lock()
            .unwrap()
            .as_ref()
            .map_or(0, |probe| probe.requests.lock().unwrap().len());
        let steps = self.steps.seen.lock().unwrap().clone();
        let guard = self.port.manager.try_lock();
        let map_free = guard.is_ok();
        let on_map = guard.is_ok_and(|manager| {
            manager
                .find_map(0, 0)
                .is_some_and(|map| map.map().get_typed_player(self.guid).is_some())
        });
        self.calls
            .lock()
            .unwrap()
            .push((snapshot.reason, saves, steps, map_free, on_map));
    }
}

fn install_logout_probe(
    session: &mut WorldSession,
    port: &Arc<LoginPort>,
    steps: &Arc<Probe>,
) -> Arc<LogoutProbe> {
    let probe = Arc::new(LogoutProbe {
        port: Arc::clone(port),
        steps: Arc::clone(steps),
        guid: session.player_guid().unwrap(),
        calls: Mutex::new(Vec::new()),
    });
    let mut registry = wow_module_api::ModuleRegistry::new();
    registry
        .register_module(
            wow_module_api::ModuleDescriptor::new(
                "alpha.logout",
                wow_module_api::ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Logout",
            )
            .unwrap(),
            wow_module_api::ModuleHooks::new().player(Arc::clone(&probe) as _),
        )
        .unwrap();
    session.install_module_hooks_like_cpp(Arc::new(registry));
    probe
}

/// C++ `LogoutPlayer`: `SaveToDB`, then `OnPlayerLogout`, then
/// `RemovePlayerFromMap` (`WorldSession.cpp:633-670`). Here: once, after the
/// save and every offline mark the disconnect plan runs first, while the
/// player is still on its map, with no map guard held, and never replayed.
#[tokio::test]
async fn production_logout_hook_runs_once_after_the_save_and_before_map_removal() {
    let (mut session, port, _receiver, steps) = setup(
        Step::LoginAccountOffline,
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        true,
    )
    .await;
    let probe = install_logout_probe(&mut session, &port, &steps);
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);

    let report = session
        .finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator)
        .await;
    assert_eq!(report.disposition, Disposition::Complete);
    assert_eq!(
        session
            .finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator)
            .await,
        report,
        "a completed finalization is not replayed"
    );

    let calls = probe.calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "the logout hook runs exactly once");
    let (reason, saves, seen, map_free, on_map) = &calls[0];
    assert_eq!(*reason, wow_module_api::LogoutReason::Disconnect);
    assert_eq!(
        *saves, 1,
        "the character save was requested before the hook"
    );
    for step in [
        Step::CharacterOffline,
        Step::CharacterAccountOffline,
        Step::LoginAccountOffline,
    ] {
        assert!(
            seen.contains(&step),
            "{step:?} ran before the hook: {seen:?}"
        );
    }
    assert!(*map_free, "no canonical map guard is held during the hook");
    assert!(*on_map, "the hook runs before the player leaves the map");
}

#[tokio::test]
async fn production_logout_hook_never_runs_when_an_earlier_obligation_failed() {
    let (mut session, port, _receiver, steps) = setup(
        Step::CharacterOffline,
        PersistenceOutcomeLikeCpp::Failed {
            reason: "known rollback".into(),
        },
        true,
    )
    .await;
    let probe = install_logout_probe(&mut session, &port, &steps);
    let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
    let report = session
        .finalize_session_with_generator_like_cpp(Mode::Disconnect, &generator)
        .await;
    assert_eq!(report.outcome(Step::Retirement), Outcome::NotAttempted);
    assert!(probe.calls.lock().unwrap().is_empty());
}
