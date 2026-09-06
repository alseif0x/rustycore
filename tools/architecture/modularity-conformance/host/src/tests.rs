use crate::*;
use conformance_contract::{
    ABI_VERSION, Action, Fault, Host, Manifest, Module, Query, Result, State, capability, event,
    read_state, write_state,
};

// Deliberately non-Clone module state, shared as a Rust type by independent modules.
#[derive(Default)]
struct Counter(Box<u64>);

impl State for Counter {
    const SCHEMA: u32 = 1;
    fn encode(&self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }
    fn decode(bytes: &[u8]) -> Result<Self> {
        Ok(Self(Box::new(u64::from_le_bytes(
            bytes.try_into().map_err(|_| Fault::Invalid)?,
        ))))
    }
}

struct Sample<const ID: u64, const ORDER: i32>;

impl<const ID: u64, const ORDER: i32> Module for Sample<ID, ORDER> {
    type State = Counter;
    fn manifest() -> Manifest {
        Manifest {
            id: ID,
            name: if ID == 1 { "one" } else { "two" },
            abi: ABI_VERSION,
            schema: 1,
            capabilities: capability::ALL,
            state_limit: 8,
            order: ORDER,
            exclusive: None,
        }
    }
    fn invoke(host: &mut dyn Host, event: u32, argument: i64) -> Result<i64> {
        match event {
            event::UPDATE | event::CALLBACK => {
                let (revision, mut state) = read_state::<Counter>(host)?;
                *state.0 += 1;
                write_state(host, revision, &state)?;
                if event == event::UPDATE {
                    host.action(Action::Contribution, ID as i64)?;
                } else if argument > 0 {
                    host.action(Action::Reenter, argument - 1)?;
                }
                Ok(*state.0 as i64)
            }
            event::CUSTOM => {
                let (revision, state) = read_state::<Counter>(host)?;
                host.action(Action::Summon, 1)?;
                write_state(host, revision, &state)?;
                Ok(0)
            }
            value if value == event::CUSTOM + 1 => {
                host.action(Action::Shield, 1)?;
                host.action(Action::Fail, 0)?;
                host.action(Action::Contribution, 999)?;
                Ok(0)
            }
            value if value == event::CUSTOM + 2 => {
                let revision = host.read()?.revision;
                host.write(revision, &[0; 9])?;
                Ok(0)
            }
            value if value == event::CUSTOM + 3 => host.action(Action::Contribution, argument),
            value if value == event::CUSTOM + 4 => host.query(Query::Residence),
            value if value == event::CUSTOM + 5 => host.action(Action::Reenter, argument),
            event::RESET => {
                let (revision, mut state) = read_state::<Counter>(host)?;
                *state.0 += 10;
                write_state(host, revision, &state)?;
                Ok(*state.0 as i64)
            }
            _ => Ok(0),
        }
    }
}

fn pair() -> (HostCore, conformance_contract::Handle) {
    let mut core = HostCore::default();
    core.register::<Sample<1, 10>>().unwrap();
    core.register::<Sample<2, 0>>().unwrap();
    let handle = core.spawn(100, 0).unwrap();
    (core, handle)
}

#[test]
fn namespaces_are_module_types_not_shared_state_types_and_order_is_declared() {
    let (mut core, handle) = pair();
    core.dispatch_one(handle, 1, event::UPDATE, 0).unwrap();
    assert_eq!(
        Counter::decode(&core.state(handle, 1).unwrap().bytes)
            .unwrap()
            .0
            .as_ref(),
        &1
    );
    assert_eq!(
        Counter::decode(&core.state(handle, 2).unwrap().bytes)
            .unwrap()
            .0
            .as_ref(),
        &0
    );
    assert_eq!(
        core.dispatch(handle, event::UPDATE, 0).unwrap(),
        vec![(2, 1), (1, 2)]
    );
    assert_eq!(core.observables(handle).unwrap().contribution, 3);
}

#[test]
fn failed_attach_preserves_nonclone_allocation_and_detached_typed_state() {
    let (mut core, handle) = pair();
    core.dispatch_one(handle, 1, event::UPDATE, 0).unwrap();
    let identity = core.payload_identity(handle).unwrap();
    let before = core.snapshot(handle).unwrap();
    core.detach(handle).unwrap();
    assert_eq!(core.attach(handle, 2), Err(Fault::Invalid));
    assert_eq!(core.residence(handle), Ok(Residence::Detached));
    assert_eq!(core.snapshot(handle).unwrap(), before);
    core.dispatch_one(handle, 1, event::CALLBACK, 0).unwrap();
    core.attach(handle, 1).unwrap();
    assert_eq!(core.payload_identity(handle).unwrap(), identity);
    assert_eq!(core.residence(handle), Ok(Residence::Active(1)));
    assert_eq!(
        *Counter::decode(&core.state(handle, 1).unwrap().bytes)
            .unwrap()
            .0,
        2
    );
}

#[test]
fn reentry_rejects_outer_stale_write_but_keeps_nested_effects() {
    let (mut core, handle) = pair();
    assert_eq!(
        core.dispatch_one(handle, 1, event::CUSTOM, 0),
        Err(Fault::Revision)
    );
    assert_eq!(core.depth(), 0);
    assert_eq!(core.observables(handle).unwrap().summons, 1);
    assert_eq!(
        *Counter::decode(&core.state(handle, 1).unwrap().bytes)
            .unwrap()
            .0,
        1
    );
    assert_eq!(
        *Counter::decode(&core.state(handle, 2).unwrap().bytes)
            .unwrap()
            .0,
        1
    );
    let entered: Vec<_> = core
        .trace()
        .iter()
        .filter_map(|trace| match trace {
            Trace::Enter(frame) => Some((frame.module, frame.event, frame.argument)),
            _ => None,
        })
        .collect();
    assert_eq!(
        entered,
        vec![
            (1, event::CUSTOM, 0),
            (2, event::CALLBACK, 0),
            (1, event::CALLBACK, 0)
        ]
    );
}

#[test]
fn error_stops_followup_mutation_without_rolling_back_prior_effects() {
    let (mut core, handle) = pair();
    assert_eq!(
        core.dispatch_one(handle, 1, event::CUSTOM + 1, 0),
        Err(Fault::ActionFailed)
    );
    let observed = core.observables(handle).unwrap();
    assert!(observed.shield);
    assert_eq!(observed.contribution, 0);
    assert_eq!(core.depth(), 0);
}

#[test]
fn sparse_membership_and_detached_remove_preserve_other_modules() {
    let mut core = HostCore::default();
    core.register::<Sample<1, 10>>().unwrap();
    core.register::<Sample<2, 0>>().unwrap();
    let handle = core.spawn_with_modules(100, 0, &[]).unwrap();
    assert!(core.dispatch(handle, event::UPDATE, 0).unwrap().is_empty());
    core.add_module_state(handle, 1).unwrap();
    core.add_module_state(handle, 2).unwrap();
    core.dispatch(handle, event::UPDATE, 0).unwrap();
    core.detach(handle).unwrap();
    let identity = core.payload_identity(handle).unwrap();
    let untouched = core.state(handle, 2).unwrap();
    core.remove_module_state(handle, 1).unwrap();
    assert_eq!(core.state(handle, 1), Err(Fault::Missing));
    assert_eq!(core.state(handle, 2).unwrap(), untouched);
    assert_eq!(core.observables(handle).unwrap().contribution, 2);
    assert_eq!(core.payload_identity(handle).unwrap(), identity);
    core.add_module_state(handle, 1).unwrap();
    assert!(core.state(handle, 1).unwrap().revision > untouched.revision);
    core.attach(handle, 1).unwrap();
    assert_eq!(core.entity_modules(handle).unwrap(), vec![2, 1]);
}

#[test]
fn retire_replace_and_guid_reuse_never_revive_an_old_handle() {
    let (mut core, handle) = pair();
    assert_eq!(core.replace(handle, 2), Err(Fault::Invalid));
    assert_eq!(core.handle(handle.guid), Some(handle));
    let replacement = core.replace(handle, 1).unwrap();
    assert!(replacement.generation > handle.generation);
    assert_eq!(core.dispatch(handle, event::UPDATE, 0), Err(Fault::Stale));
    core.retire(replacement).unwrap();
    let next = core.spawn(handle.guid, 0).unwrap();
    assert!(next.generation > replacement.generation);
    assert_eq!(core.state(replacement, 1), Err(Fault::Stale));
}

#[test]
fn reset_and_replay_advance_revisions_and_reject_aba() {
    let (mut core, handle) = pair();
    let before = core.snapshot(handle).unwrap();
    core.replay(handle, &before).unwrap();
    assert_eq!(core.replay(handle, &before), Err(Fault::Revision));
    let before_reset = core.snapshot(handle).unwrap();
    core.reset(handle, 1).unwrap();
    assert!(
        core.state(handle, 1).unwrap().revision
            > before_reset
                .modules
                .iter()
                .find(|m| m.id == 1)
                .unwrap()
                .revision
    );
    assert_eq!(core.replay(handle, &before_reset), Err(Fault::Revision));
    let before_action = core.snapshot(handle).unwrap();
    core.dispatch_one(handle, 1, event::CUSTOM + 3, 7).unwrap();
    assert_eq!(core.replay(handle, &before_action), Err(Fault::Revision));
    assert_eq!(
        core.calls(),
        0,
        "replay is its own root, even when rejected"
    );
    assert!(core.trace().is_empty());
}

#[test]
fn malformed_snapshot_and_unsupported_executor_have_no_canonical_effect() {
    let (mut core, handle) = pair();
    let before = core.snapshot(handle).unwrap();
    let mut invalid = before.clone();
    invalid.modules.last_mut().unwrap().schema = 2;
    assert_eq!(core.replay(handle, &invalid), Err(Fault::Version));
    assert_eq!(core.snapshot(handle).unwrap(), before);
    assert_eq!(core.switch_executor(1, Executor::Wasm), Err(Fault::Version));
    assert_eq!(core.snapshot(handle).unwrap(), before);
    invalid = before.clone();
    invalid.modules.last_mut().unwrap().bytes = vec![0; 7];
    assert_eq!(core.replay(handle, &invalid), Err(Fault::Invalid));
    assert_eq!(core.snapshot(handle).unwrap(), before);
}

#[test]
fn limits_unwind_frames_and_reset_calls_and_trace_per_root() {
    let mut core = HostCore::new(Limits {
        depth: 2,
        ..Limits::default()
    });
    core.register::<Sample<1, 0>>().unwrap();
    let handle = core.spawn(1, 0).unwrap();
    assert_eq!(
        core.dispatch_one(handle, 1, event::CALLBACK, 9),
        Err(Fault::Limit)
    );
    assert_eq!(core.depth(), 0);
    core.dispatch_one(handle, 1, event::CUSTOM + 4, 0).unwrap();
    assert_eq!(core.trace().len(), 2);
    assert_eq!(core.calls(), 2);
    let before = core.state(handle, 1).unwrap();
    assert_eq!(
        core.dispatch_one(handle, 1, event::CUSTOM + 2, 0),
        Err(Fault::Limit)
    );
    assert_eq!(core.state(handle, 1).unwrap(), before);
    core.limits.calls = 2;
    assert_eq!(
        core.dispatch_one(handle, 1, event::UPDATE, 0),
        Err(Fault::Limit)
    );
    assert_eq!(core.depth(), 0);
    assert_eq!(core.state(handle, 1).unwrap(), before);
}

#[test]
fn duplicate_and_manifest_conflicts_precede_callbacks_or_entity_changes() {
    let (mut core, handle) = pair();
    let before = core.snapshot(handle).unwrap();
    assert_eq!(core.register::<Sample<1, 10>>(), Err(Fault::Conflict));
    assert_eq!(core.register::<Sample<2, 99>>(), Err(Fault::Conflict));
    assert_eq!(
        core.spawn_with_modules(101, 0, &[1, 1]),
        Err(Fault::Conflict)
    );
    assert_eq!(core.spawn_with_modules(101, 0, &[3]), Err(Fault::Missing));
    assert_eq!(core.snapshot(handle).unwrap(), before);
    assert!(core.trace().is_empty());
}

#[test]
fn unload_reconciles_active_and_detached_members_without_global_optional_execution() {
    let (mut core, active) = pair();
    let detached = core.spawn_with_modules(200, 1, &[1]).unwrap();
    core.detach(detached).unwrap();
    core.dispatch(active, event::UPDATE, 0).unwrap();
    core.unload_module(1).unwrap();
    assert_eq!(core.entity_modules(active).unwrap(), vec![2]);
    assert!(core.entity_modules(detached).unwrap().is_empty());
    assert_eq!(core.observables(active).unwrap().contribution, 2);
    core.unload_map(0).unwrap();
    assert_eq!(core.attach(active, 0), Err(Fault::Invalid));
    assert_eq!(core.residence(active), Ok(Residence::Detached));
    core.load_map(0).unwrap();
    core.attach(active, 0).unwrap();
    assert_eq!(core.dispatch_one(active, 2, event::CUSTOM + 4, 0), Ok(1));
    core.detach(active).unwrap();
    assert_eq!(core.dispatch_one(active, 2, event::CUSTOM + 4, 0), Ok(0));
}

fn opaque_manifest(id: u64) -> Manifest {
    Manifest {
        id,
        name: if id == 10 { "opaque-a" } else { "opaque-b" },
        abi: ABI_VERSION,
        schema: 1,
        capabilities: capability::ALL,
        state_limit: 8,
        order: 30,
        exclusive: None,
    }
}

#[test]
fn opaque_storage_envelopes_and_detached_removal_preserve_other_namespaces() {
    let mut core = HostCore::default();
    core.register::<Sample<1, 0>>().unwrap();
    core.register_opaque(opaque_manifest(10), &10u64.to_le_bytes())
        .unwrap();
    core.register_opaque(opaque_manifest(11), &11u64.to_le_bytes())
        .unwrap();
    let handle = core.spawn(5, 0).unwrap();
    let identity = core.payload_identity(handle).unwrap();
    let before = core.snapshot(handle).unwrap();
    // Native-only execution fails before invoking the earlier native module.
    assert_eq!(core.dispatch(handle, event::UPDATE, 0), Err(Fault::Version));
    assert_eq!(core.snapshot(handle).unwrap(), before);
    assert!(core.trace().is_empty());
    assert_eq!(core.replay(handle, &before), Err(Fault::Version));
    // Internal storage fixture only: the public native path cannot assert a guest codec.
    // Actual guest validation/replay is covered by WasmRuntime tests.
    let mut plan = core.prepare_replay(handle, &before).unwrap();
    while let Some(record) = plan.next_record() {
        Counter::decode(&record.bytes).unwrap();
        core.stage_replay_record(&mut plan).unwrap();
    }
    core.commit_replay(plan).unwrap();
    assert_eq!(core.state(handle, 10).unwrap().bytes, 10u64.to_le_bytes());
    assert_eq!(core.state(handle, 11).unwrap().bytes, 11u64.to_le_bytes());
    let typed = core.state(handle, 1).unwrap();
    let other = core.state(handle, 11).unwrap();
    core.detach_storage(handle).unwrap();
    let revision = core.state(handle, 10).unwrap().revision;
    core.write_state(handle, 10, revision, &20u64.to_le_bytes())
        .unwrap();
    assert_eq!(core.state(handle, 11).unwrap(), other);
    core.remove_module_state_storage(handle, 10).unwrap();
    assert_eq!(core.state(handle, 10), Err(Fault::Missing));
    assert_eq!(core.state(handle, 1).unwrap(), typed);
    assert_eq!(core.state(handle, 11).unwrap(), other);
    core.attach_storage(handle, 1).unwrap();
    assert_eq!(core.payload_identity(handle).unwrap(), identity);
    assert_eq!(core.entity_modules(handle).unwrap(), vec![1, 11]);
}

#[test]
fn registration_preflight_rejects_versions_caps_exclusive_limits_without_installing() {
    let mut core = HostCore::default();
    for (manifest, fault) in [
        (
            Manifest {
                abi: 2,
                ..opaque_manifest(10)
            },
            Fault::Version,
        ),
        (
            Manifest {
                schema: 0,
                ..opaque_manifest(10)
            },
            Fault::Version,
        ),
        (
            Manifest {
                capabilities: capability::ALL | 128,
                ..opaque_manifest(10)
            },
            Fault::Capability,
        ),
        (
            Manifest {
                state_limit: 257,
                ..opaque_manifest(10)
            },
            Fault::Limit,
        ),
    ] {
        assert_eq!(core.register_opaque(manifest, &[0; 8]), Err(fault));
        assert!(core.registered().is_empty());
        assert!(core.trace().is_empty());
    }
    let first = Manifest {
        exclusive: Some("policy-slot"),
        ..opaque_manifest(10)
    };
    core.register_opaque(first, &[0; 8]).unwrap();
    let second = Manifest {
        exclusive: Some("policy-slot"),
        ..opaque_manifest(11)
    };
    assert_eq!(core.register_opaque(second, &[0; 8]), Err(Fault::Conflict));
    assert_eq!(
        core.register_opaque(opaque_manifest(11), &[0; 9]),
        Err(Fault::Limit)
    );
    assert_eq!(core.registered(), vec![first]);
}

#[test]
fn prepared_replacement_rechecks_residence_target_and_membership_before_drop() {
    let (mut core, handle) = pair();
    let identity = core.payload_identity(handle).unwrap();
    let prepared = core.prepare_replace(handle, 1).unwrap();
    core.remove_module_state(handle, 2).unwrap();
    let before = core.snapshot(handle).unwrap();
    assert_eq!(core.commit_replace(prepared), Err(Fault::Conflict));
    assert_eq!(core.snapshot(handle).unwrap(), before);
    assert_eq!(core.payload_identity(handle).unwrap(), identity);
    let prepared = core.prepare_replace(handle, 1).unwrap();
    core.mark_map_unloaded(1).unwrap();
    assert_eq!(core.commit_replace(prepared), Err(Fault::Invalid));
    assert_eq!(core.snapshot(handle).unwrap(), before);
}

#[test]
fn reset_preserves_module_owned_value_until_its_own_lifecycle_rule_runs() {
    let (mut core, handle) = pair();
    core.dispatch_one(handle, 1, event::UPDATE, 0).unwrap();
    let before = core.state(handle, 1).unwrap();
    let other = core.state(handle, 2).unwrap();
    core.reset(handle, 1).unwrap();
    let after = core.state(handle, 1).unwrap();
    assert_eq!(
        *Counter::decode(&after.bytes).unwrap().0,
        11,
        "preserved 1 then module-added 10; not Default+10"
    );
    assert!(after.revision > before.revision);
    assert_eq!(core.state(handle, 2).unwrap(), other);
    assert_eq!(
        core.write_state(handle, 1, before.revision, &before.bytes),
        Err(Fault::Revision)
    );
    assert_eq!(core.observables(handle).unwrap().contribution, 0);
}

struct CallbackReturn<const VALUE: i64>;

impl<const VALUE: i64> Module for CallbackReturn<VALUE> {
    type State = Counter;
    fn manifest() -> Manifest {
        Manifest {
            name: "callback-return",
            capabilities: capability::SUMMON,
            ..Sample::<1, 0>::manifest()
        }
    }
    fn invoke(host: &mut dyn Host, event: u32, _: i64) -> Result<i64> {
        match event {
            event::UPDATE => {
                host.action(Action::Summon, 1)?;
                Ok(17)
            }
            event::CALLBACK => Ok(VALUE),
            _ => Ok(0),
        }
    }
}

#[test]
fn callback_return_only_divergence_changes_the_oracle_without_changing_outer_result_or_state() {
    let mut first = HostCore::default();
    let mut second = HostCore::default();
    first.register::<CallbackReturn<7>>().unwrap();
    second.register::<CallbackReturn<8>>().unwrap();
    let handle = first.spawn(1, 0).unwrap();
    assert_eq!(second.spawn(1, 0).unwrap(), handle);
    assert_eq!(first.dispatch_one(handle, 1, event::UPDATE, 0), Ok(17));
    assert_eq!(second.dispatch_one(handle, 1, event::UPDATE, 0), Ok(17));
    assert_eq!(
        first.snapshot(handle).unwrap(),
        second.snapshot(handle).unwrap()
    );
    assert_eq!(
        first.observables(handle).unwrap(),
        second.observables(handle).unwrap()
    );
    let without_results = |core: &HostCore| {
        core.trace()
            .iter()
            .filter(|trace| !matches!(trace, Trace::Leave { .. }))
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(without_results(&first), without_results(&second));
    assert_ne!(
        first.trace(),
        second.trace(),
        "only nested return changed; oracle must see it"
    );
    assert!(first.trace().iter().any(|trace| matches!(
        trace,
        Trace::Leave {
            invocation: Invocation {
                event: event::CALLBACK,
                ..
            },
            result: Ok(7)
        }
    )));
    assert!(second.trace().iter().any(|trace| matches!(
        trace,
        Trace::Leave {
            invocation: Invocation {
                event: event::CALLBACK,
                ..
            },
            result: Ok(8)
        }
    )));
}

#[test]
fn leave_reservations_preserve_effect_and_fault_evidence_at_exact_trace_limits() {
    for (trace_limit, expected_summons, expected_result, expected_len) in [
        (1, 0, Err(Fault::Limit), 0),
        (2, 0, Err(Fault::Limit), 2),
        (3, 1, Err(Fault::Limit), 3),
        (5, 1, Ok(17), 5),
    ] {
        let mut core = HostCore::new(Limits {
            trace: trace_limit,
            ..Limits::default()
        });
        core.register::<CallbackReturn<7>>().unwrap();
        let handle = core.spawn(1, 0).unwrap();
        assert_eq!(
            core.dispatch_one(handle, 1, event::UPDATE, 0),
            expected_result
        );
        assert_eq!(core.observables(handle).unwrap().summons, expected_summons);
        assert_eq!(core.depth(), 0);
        assert_eq!(core.trace().len(), expected_len);
        if trace_limit >= 2 {
            assert!(
                matches!(core.trace().last(), Some(Trace::Leave { result, .. }) if *result == expected_result)
            );
        }
    }
}

#[test]
fn negative_reentry_arguments_reject_before_effects_or_callback() {
    let (mut core, handle) = pair();
    let before = core.snapshot(handle).unwrap();
    let observed = core.observables(handle).unwrap();
    assert_eq!(
        core.dispatch_one(handle, 1, event::CUSTOM + 5, -1),
        Err(Fault::Invalid)
    );
    assert_eq!(core.snapshot(handle).unwrap(), before);
    assert_eq!(core.observables(handle).unwrap(), observed);
    assert_eq!(core.depth(), 0);
    let invocation = Invocation {
        handle,
        module: 1,
        event: event::CUSTOM + 5,
        argument: -1,
    };
    assert_eq!(
        core.trace(),
        &[
            Trace::Enter(invocation),
            Trace::Leave {
                invocation,
                result: Err(Fault::Invalid)
            },
        ]
    );
}

struct NegativeResult;

impl Module for NegativeResult {
    type State = Counter;
    fn manifest() -> Manifest {
        Manifest {
            name: "negative-result",
            ..Sample::<1, 0>::manifest()
        }
    }
    fn invoke(_: &mut dyn Host, event: u32, _: i64) -> Result<i64> {
        if event == event::UPDATE {
            Ok(-2)
        } else {
            Err(Fault::Stale)
        }
    }
}

#[test]
fn negative_success_is_invalid_but_explicit_stale_fault_remains_stale_in_leave_trace() {
    let mut core = HostCore::default();
    core.register::<NegativeResult>().unwrap();
    let handle = core.spawn(1, 0).unwrap();
    let before = core.snapshot(handle).unwrap();
    assert_eq!(
        core.dispatch_one(handle, 1, event::UPDATE, 0),
        Err(Fault::Invalid)
    );
    assert!(matches!(
        core.trace().last(),
        Some(Trace::Leave {
            result: Err(Fault::Invalid),
            ..
        })
    ));
    assert_eq!(
        core.dispatch_one(handle, 1, event::CALLBACK, 0),
        Err(Fault::Stale)
    );
    assert!(matches!(
        core.trace().last(),
        Some(Trace::Leave {
            result: Err(Fault::Stale),
            ..
        })
    ));
    assert_eq!(core.snapshot(handle).unwrap(), before);
    assert_eq!(core.depth(), 0);
}
