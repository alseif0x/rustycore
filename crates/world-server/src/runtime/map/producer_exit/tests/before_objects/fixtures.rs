//! Real Map admission/rejection and poison boundaries, not producer fault hooks.
use super::*;
use crate::runtime::map_tick::{
    CanonicalMapSessionPassMapLikeCpp, CanonicalMapSessionPassPlanLikeCpp,
};
use crate::session_supervision::TickAdmission;
use wow_world::session::mailbox::SessionPhasePermitLikeCpp;

pub(super) struct PlanIdentity {
    pub(super) updated: *const wow_map::MapTickParticipantLikeCpp,
    pub(super) destroyed: *const wow_map::MapTickParticipantLikeCpp,
    pub(super) participants: *const CanonicalMapSessionPassMapLikeCpp,
    pub(super) epoch: u64,
    pub(super) diff: u32,
}

pub(super) fn admitted(diff: u32) -> (wow_map::MapManager, CanonicalMapSessionPassPlanLikeCpp) {
    let mut manager = wow_map::MapManager::new(wow_map::MIN_GRID_DELAY_MS, 200);
    manager.create_world_map(1, 0);
    manager.create_world_map(2, 0);
    manager.create_map_entry(
        33,
        7,
        1,
        wow_map::ManagedMapKind::Dungeon {
            has_reset_schedule: false,
        },
    );
    manager.find_map_mut(33, 7).unwrap().set_can_unload(true);
    let plan = manager.begin_tick_like_cpp(diff).into_started().unwrap();
    let participants = plan
        .updated_maps_like_cpp()
        .iter()
        .map(|participant| CanonicalMapSessionPassMapLikeCpp {
            key: participant.key,
            incarnation: participant.incarnation,
            participants: manager.map_session_pass_participants_like_cpp(participant.key),
        })
        .collect();
    (
        manager,
        CanonicalMapSessionPassPlanLikeCpp { plan, participants },
    )
}

pub(super) fn identity(plan: &CanonicalMapSessionPassPlanLikeCpp) -> PlanIdentity {
    PlanIdentity {
        updated: plan.plan.updated_maps_like_cpp().as_ptr(),
        destroyed: plan.plan.destroyed_maps_like_cpp().as_ptr(),
        participants: plan.participants.as_ptr(),
        epoch: plan.plan.epoch_like_cpp(),
        diff: plan.plan.effective_diff_ms(),
    }
}

pub(super) fn rejected_return(
    foreign: &Mutex<wow_map::MapManager>,
    plan: CanonicalMapSessionPassPlanLikeCpp,
    admission: TickAdmission,
    unresolved_permits: Vec<Arc<SessionPhasePermitLikeCpp>>,
    shutdown: bool,
) -> CanonicalMapProducerExit {
    let mut manager = foreign.lock().unwrap();
    let CanonicalMapSessionPassPlanLikeCpp { plan, participants } = plan;
    let (status, plan) = manager.try_abandon_tick(plan).unwrap_err();
    drop(manager);
    CanonicalMapProducerExit::RetainedBeforeObjects {
        cause: if shutdown {
            BeforeObjectsExitCause::ShutdownAbandonRejected(status)
        } else {
            BeforeObjectsExitCause::UnresolvedMapPassRejected(status)
        },
        plan: CanonicalMapSessionPassPlanLikeCpp { plan, participants },
        admission,
        unresolved_permits,
    }
}

pub(super) fn poisoned_return(
    manager: &Mutex<wow_map::MapManager>,
    plan: CanonicalMapSessionPassPlanLikeCpp,
    admission: TickAdmission,
    unresolved_permits: Vec<Arc<SessionPhasePermitLikeCpp>>,
    shutdown: bool,
) -> CanonicalMapProducerExit {
    // A real poisoned fixture mutex; no injectable producer fault or recovery.
    let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = manager.lock().unwrap();
        panic!("controlled fixture mutex poison");
    }));
    assert!(poisoned.is_err());
    match manager.lock() {
        Err(error) => drop(error),
        Ok(_) => panic!("the actual poison must be observed before any abandon call"),
    }
    CanonicalMapProducerExit::RetainedBeforeObjects {
        cause: if shutdown {
            BeforeObjectsExitCause::ShutdownAbandonManagerPoisoned
        } else {
            BeforeObjectsExitCause::UnresolvedMapPassManagerPoisoned
        },
        plan,
        admission,
        unresolved_permits,
    }
}

pub(super) fn assert_original(exit: &CanonicalMapProducerExit, expected: &PlanIdentity) {
    let CanonicalMapProducerExit::RetainedBeforeObjects {
        plan, admission, ..
    } = exit
    else {
        panic!("before-Objects owners must survive the terminal return");
    };
    assert_eq!(plan.plan.updated_maps_like_cpp().as_ptr(), expected.updated);
    assert_eq!(
        plan.plan.destroyed_maps_like_cpp().as_ptr(),
        expected.destroyed
    );
    assert_eq!(plan.participants.as_ptr(), expected.participants);
    assert_eq!(plan.plan.epoch_like_cpp(), expected.epoch);
    assert_eq!(plan.plan.effective_diff_ms(), expected.diff);
    assert_eq!(plan.plan.updated_maps_like_cpp().len(), 2);
    assert_eq!(plan.plan.destroyed_maps_like_cpp().len(), 1);
    assert_eq!(plan.participants.len(), 2);
    for (row, original) in plan
        .participants
        .iter()
        .zip(plan.plan.updated_maps_like_cpp())
    {
        assert_eq!(row.key, original.key);
        assert_eq!(row.incarnation, original.incarnation);
        assert!(row.participants.is_empty());
    }
    assert_eq!(plan.participants[0].key, wow_map::MapKey::new(1, 0));
    assert_eq!(plan.participants[1].key, wow_map::MapKey::new(2, 0));
    assert_eq!(
        plan.plan.destroyed_maps_like_cpp()[0].key,
        wow_map::MapKey::new(33, 7)
    );
    // Uses the exact ticket's existing ledger record and epoch; no accounting.
    admission.enter_phase(TickPhase::Map);
}
