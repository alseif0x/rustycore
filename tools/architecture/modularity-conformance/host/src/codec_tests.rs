//! Rejected state inputs must not change canonical data or future revision allocation.

use crate::*;
use conformance_contract::{
    ABI_VERSION, Fault, Host, Manifest, Module, Result, State, capability, event,
};

#[derive(Default)]
struct Byte(u8);

impl State for Byte {
    const SCHEMA: u32 = 1;
    fn encode(&self) -> Vec<u8> {
        vec![self.0]
    }
    fn decode(bytes: &[u8]) -> Result<Self> {
        match bytes {
            [value] => Ok(Self(*value)),
            _ => Err(Fault::Invalid),
        }
    }
}

struct Writer<const ID: u64, const CAPS: u64>;

impl<const ID: u64, const CAPS: u64> Module for Writer<ID, CAPS> {
    type State = Byte;
    fn manifest() -> Manifest {
        Manifest {
            id: ID,
            name: if ID == 1 { "writer" } else { "second" },
            abi: ABI_VERSION,
            schema: 1,
            capabilities: CAPS,
            state_limit: 2, // Envelope admits lengths the codec rejects.
            order: ID as i32,
            exclusive: None,
        }
    }
    fn invoke(host: &mut dyn Host, event: u32, argument: i64) -> Result<i64> {
        let revision = host.read()?.revision;
        if event == event::CUSTOM {
            host.write(revision, &[])?;
        } else {
            host.write(revision, &[argument as u8])?;
        }
        Ok(0)
    }
}

fn pair() -> (HostCore, conformance_contract::Handle) {
    let mut core = HostCore::default();
    core.register::<Writer<1, 0>>().unwrap();
    core.register::<Writer<2, 0>>().unwrap();
    let handle = core.spawn(1, 0).unwrap();
    (core, handle)
}

#[test]
fn rejected_write_codec_does_not_consume_a_future_revision() {
    let (mut rejected, handle) = pair();
    let (mut control, control_handle) = pair();
    let before = rejected.snapshot(handle).unwrap();
    let clock = rejected.revision_clock;
    assert_eq!(
        rejected.dispatch_one(handle, 1, event::CUSTOM, 0),
        Err(Fault::Invalid)
    );
    assert_eq!(rejected.snapshot(handle).unwrap(), before);
    assert_eq!(rejected.revision_clock, clock);
    assert!(
        !rejected
            .trace()
            .iter()
            .any(|entry| matches!(entry, Trace::Write { .. }))
    );
    assert_eq!(rejected.depth(), 0);
    rejected.dispatch_one(handle, 1, event::UPDATE, 9).unwrap();
    control
        .dispatch_one(control_handle, 1, event::UPDATE, 9)
        .unwrap();
    assert_eq!(
        rejected.snapshot(handle).unwrap(),
        control.snapshot(control_handle).unwrap()
    );
    assert_eq!(rejected.trace(), control.trace());
    assert_eq!(rejected.calls(), control.calls());
}

#[test]
fn replay_late_codec_failure_drops_staging_without_advancing_any_revision() {
    let (mut core, handle) = pair();
    let (mut control, control_handle) = pair();
    let before = core.snapshot(handle).unwrap();
    let clock = core.revision_clock;
    let mut malformed = before.clone();
    malformed.modules[0].bytes = vec![8];
    malformed.modules[1].bytes.clear();
    assert_eq!(core.replay(handle, &malformed), Err(Fault::Invalid));
    assert_eq!(core.snapshot(handle).unwrap(), before);
    assert_eq!(core.revision_clock, clock);
    assert!(core.trace().is_empty());
    core.replay(handle, &before).unwrap();
    control.replay(control_handle, &before).unwrap();
    assert_eq!(
        core.snapshot(handle).unwrap(),
        control.snapshot(control_handle).unwrap()
    );
}

#[test]
fn replay_rejects_unauthorized_contributions_before_codecs_or_changes() {
    let (mut core, handle) = pair();
    let before = core.snapshot(handle).unwrap();
    let clock = core.revision_clock;
    for contribution in [
        Contribution {
            shield: true,
            ..Contribution::default()
        },
        Contribution {
            summons: 1,
            ..Contribution::default()
        },
        Contribution {
            amount: 1,
            ..Contribution::default()
        },
    ] {
        let mut forged = before.clone();
        forged.contributions.push((1, contribution));
        forged.modules[0].bytes.clear(); // Capability denial precedes even a malformed codec.
        assert_eq!(core.replay(handle, &forged), Err(Fault::Capability));
        assert_eq!(core.snapshot(handle).unwrap(), before);
        assert_eq!(core.revision_clock, clock);
        assert!(core.trace().is_empty());
        assert_eq!(core.calls(), 0);
    }
    // Zeroed cleanup is not an authority grant.
    let mut empty = before;
    empty.contributions.push((1, Contribution::default()));
    core.replay(handle, &empty).unwrap();
}

#[test]
fn replay_accepts_authorized_contributions_and_rechecks_owned_plan() {
    let mut core = HostCore::default();
    core.register::<Writer<1, { capability::ALL }>>().unwrap();
    let handle = core.spawn(1, 0).unwrap();
    let mut snapshot = core.snapshot(handle).unwrap();
    snapshot.contributions.push((
        1,
        Contribution {
            shield: true,
            summons: 2,
            amount: 3,
        },
    ));
    core.replay(handle, &snapshot).unwrap();
    let observed = core.observables(handle).unwrap();
    assert!(observed.shield);
    assert_eq!((observed.summons, observed.contribution), (2, 3));

    let mut plan = core
        .prepare_replay(handle, &core.snapshot(handle).unwrap())
        .unwrap();
    core.stage_replay_record(&mut plan).unwrap();
    core.dispatch_one(handle, 1, event::UPDATE, 7).unwrap();
    let before = core.snapshot(handle).unwrap();
    let clock = core.revision_clock;
    assert_eq!(core.commit_replay(plan), Err(Fault::Revision));
    assert_eq!(core.snapshot(handle).unwrap(), before);
    assert_eq!(core.revision_clock, clock);
}

#[test]
fn write_ticket_revalidates_after_an_intervening_write_without_second_charge() {
    let (mut core, handle) = pair();
    core.begin_root().unwrap();
    let invocation = Invocation {
        handle,
        module: 1,
        event: event::UPDATE,
        argument: 0,
    };
    core.enter_call(invocation).unwrap();
    let revision = core.state(handle, 1).unwrap().revision;
    let ticket = core.preflight_write_scoped(revision, &[8]).unwrap();
    assert_eq!(core.calls(), 2); // Enter + one write attempt.
    core.write_state(handle, 1, revision, &[9]).unwrap();
    let before = core.snapshot(handle).unwrap();
    let clock = core.revision_clock;
    assert_eq!(core.commit_write_scoped(ticket), Err(Fault::Revision));
    assert_eq!(core.calls(), 2);
    assert_eq!(core.snapshot(handle).unwrap(), before);
    assert_eq!(core.revision_clock, clock);
    core.leave_call(invocation, Err(Fault::Revision));
    core.end_root();
}

/// Deliberately broken codecs demonstrate that successful decode alone is insufficient.
struct Transform<const DEFAULT: u8, const EXPAND: bool>(u8);

impl<const DEFAULT: u8, const EXPAND: bool> Default for Transform<DEFAULT, EXPAND> {
    fn default() -> Self {
        Self(DEFAULT)
    }
}

impl<const DEFAULT: u8, const EXPAND: bool> State for Transform<DEFAULT, EXPAND> {
    const SCHEMA: u32 = 1;

    fn encode(&self) -> Vec<u8> {
        if EXPAND && self.0 == 16 {
            vec![16; 3]
        } else {
            vec![self.0]
        }
    }

    fn decode(bytes: &[u8]) -> Result<Self> {
        let [value] = bytes else {
            return Err(Fault::Invalid);
        };
        Ok(Self(if EXPAND {
            if *value == 255 { 16 } else { *value }
        } else {
            *value & 15
        }))
    }
}

struct TransformModule<const DEFAULT: u8, const EXPAND: bool>;

impl<const DEFAULT: u8, const EXPAND: bool> Module for TransformModule<DEFAULT, EXPAND> {
    type State = Transform<DEFAULT, EXPAND>;

    fn manifest() -> Manifest {
        Manifest {
            name: "transform-codec",
            ..Writer::<1, 0>::manifest()
        }
    }

    fn invoke(host: &mut dyn Host, _: u32, argument: i64) -> Result<i64> {
        let revision = host.read()?.revision;
        host.write(revision, &[argument as u8])?;
        Ok(0)
    }
}

#[test]
fn noncanonical_or_expanding_initial_codec_is_rejected_before_registration() {
    let mut core = HostCore::default();
    let handle = core.spawn(1, 0).unwrap();
    let before = core.snapshot(handle).unwrap();
    let clock = core.revision_clock;
    assert_eq!(
        core.register::<TransformModule<17, false>>(),
        Err(Fault::Invalid)
    );
    assert_eq!(
        core.register::<TransformModule<255, true>>(),
        Err(Fault::Limit)
    );
    assert!(core.registered().is_empty());
    assert_eq!(core.snapshot(handle).unwrap(), before);
    assert_eq!(core.revision_clock, clock);
}

fn transformed_state_rejection<const EXPAND: bool>(input: u8, fault: Fault) {
    fn prepared<const EXPAND: bool>() -> (HostCore, conformance_contract::Handle) {
        let mut core = HostCore::default();
        core.register::<TransformModule<7, EXPAND>>().unwrap();
        let handle = core.spawn(1, 0).unwrap();
        (core, handle)
    }
    let (mut core, handle) = prepared::<EXPAND>();
    let (mut control, control_handle) = prepared::<EXPAND>();
    let before = core.snapshot(handle).unwrap();
    let clock = core.revision_clock;
    assert_eq!(before.modules[0].bytes, [7]);
    assert_eq!(
        core.dispatch_one(handle, 1, event::UPDATE, i64::from(input)),
        Err(fault)
    );
    assert_eq!(core.snapshot(handle).unwrap(), before);
    assert_eq!(core.revision_clock, clock);
    assert!(
        !core
            .trace()
            .iter()
            .any(|entry| matches!(entry, Trace::Write { .. }))
    );
    let mut malformed = before.clone();
    malformed.modules[0].bytes = vec![input];
    assert_eq!(core.replay(handle, &malformed), Err(fault));
    assert_eq!(core.snapshot(handle).unwrap(), before);
    assert_eq!(core.revision_clock, clock);
    assert!(core.trace().is_empty());
    core.dispatch_one(handle, 1, event::UPDATE, 9).unwrap();
    control
        .dispatch_one(control_handle, 1, event::UPDATE, 9)
        .unwrap();
    assert_eq!(
        core.snapshot(handle).unwrap(),
        control.snapshot(control_handle).unwrap()
    );
    assert_eq!(core.trace(), control.trace());
    let valid = core.snapshot(handle).unwrap();
    core.replay(handle, &valid).unwrap();
    control.replay(control_handle, &valid).unwrap();
    assert_eq!(
        core.snapshot(handle).unwrap(),
        control.snapshot(control_handle).unwrap()
    );
}

#[test]
fn normalizing_codec_write_and_replay_reject_without_revision_consumption() {
    transformed_state_rejection::<false>(17, Fault::Invalid);
}

#[test]
fn expanding_codec_write_and_replay_reject_without_revision_consumption() {
    transformed_state_rejection::<true>(255, Fault::Limit);
}
