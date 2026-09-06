use super::*;
use crate::Limits;
use conformance_contract::{Fault, MAX_CALLS, event};

#[test]
fn hostile_pointer_length_revision_and_operation_are_rejected_before_state_changes() {
    let cases = [
        (
            "i32.const -1 i32.const 8 i32.const 32 call $read i64.extend_i32_s",
            Fault::Invalid,
        ),
        (
            "i32.const 0 i32.const 257 i32.const 32 call $read i64.extend_i32_s",
            Fault::Limit,
        ),
        (
            "i32.const 0 i32.const 8 i32.const -1 call $read i64.extend_i32_s",
            Fault::Invalid,
        ),
        (
            "i32.const 64 i32.const 8 i32.const 68 call $read i64.extend_i32_s",
            Fault::Invalid,
        ),
        (
            // An empty range cannot overlap the revision output. This record
            // needs eight bytes, so capacity (not pointer validity) rejects it.
            "i32.const 1 i32.const 0 i32.const 0 call $read i64.extend_i32_s",
            Fault::Limit,
        ),
        (
            "i32.const -1 i32.const 8 i64.const 0 call $write i64.extend_i32_s",
            Fault::Invalid,
        ),
        (
            "i32.const 0 i32.const 257 i64.const 0 call $write i64.extend_i32_s",
            Fault::Limit,
        ),
        ("i32.const 999 call $query", Fault::Invalid),
        ("i32.const 999 i64.const 0 call $action", Fault::Invalid),
        (
            "i32.const 0 i32.const 8 i64.const 0 call $write i64.extend_i32_s",
            Fault::Revision,
        ),
    ];
    for (body, expected) in cases {
        let manifest = hostile_manifest(500, "hostile");
        let (mut runtime, handle) = hostile_runtime(manifest, body);
        let before = runtime.snapshot(handle).unwrap();
        assert_eq!(
            runtime.dispatch_one(handle, 500, event::CUSTOM, 0),
            Err(expected),
            "{body}"
        );
        assert_eq!(runtime.snapshot(handle).unwrap(), before);
        assert_eq!(
            runtime.core().calls(),
            2,
            "entry and invalid import both consume budget"
        );
        assert_eq!(runtime.core().depth(), 0);
    }
}

#[test]
fn malformed_imports_cannot_bypass_a_depleted_host_call_budget() {
    let manifest = hostile_manifest(500, "hostile");
    let body = "loop $again i32.const 999 call $query drop br $again end i64.const 0";
    let (mut runtime, handle) = hostile_runtime(manifest, body);
    let before = runtime.snapshot(handle).unwrap();
    assert_eq!(
        runtime.dispatch_one(handle, 500, event::CUSTOM, 0),
        Err(Fault::Limit)
    );
    assert_eq!(runtime.core().calls(), MAX_CALLS);
    assert_eq!(runtime.core().depth(), 0);
    assert_eq!(runtime.snapshot(handle).unwrap(), before);
    // The host-call cap, not a pointer-validation branch, wins after entry fills it.
    let mut runtime = WasmRuntime::new(HostCore::new(Limits {
        calls: 1,
        ..Limits::default()
    }))
    .unwrap();
    runtime
        .register_wasm(
            manifest,
            &hostile(manifest, "i32.const 999 call $query", ""),
        )
        .unwrap();
    let handle = runtime.spawn(100, 0).unwrap();
    assert_eq!(
        runtime.dispatch_one(handle, 500, event::CUSTOM, 0),
        Err(Fault::Limit)
    );
    assert_eq!(runtime.core().calls(), 1);
}

#[test]
fn missing_capability_and_detached_actions_do_not_mutate_authority() {
    let mut manifest = hostile_manifest(500, "hostile");
    manifest.capabilities = 0;
    let (mut runtime, handle) = hostile_runtime(manifest, "i32.const 1 i64.const 1 call $action");
    let before = runtime.snapshot(handle).unwrap();
    assert_eq!(
        runtime.dispatch_one(handle, 500, event::CUSTOM, 0),
        Err(Fault::Capability)
    );
    assert_eq!(runtime.snapshot(handle).unwrap(), before);
    for frontend in Frontend::ALL {
        let (mut runtime, handle) = frontend.runtime();
        runtime.detach(handle).unwrap();
        let before = runtime.snapshot(handle).unwrap();
        // Explicit effect probe does not change state before requesting Shield.
        assert_eq!(
            runtime.dispatch_one(handle, 1, event::CUSTOM + 1, 0),
            Err(Fault::NotActive)
        );
        assert_eq!(runtime.snapshot(handle).unwrap(), before);
    }
}

#[test]
fn hostile_unreachable_after_shield_is_a_trap_without_rollback() {
    let manifest = hostile_manifest(500, "hostile");
    let (mut runtime, handle) = hostile_runtime(
        manifest,
        "i32.const 1 i64.const 1 call $action drop unreachable",
    );
    assert_eq!(
        runtime.dispatch_one(handle, 500, event::CUSTOM, 0),
        Err(Fault::Trap)
    );
    assert!(runtime.observables(handle).unwrap().shield);
    assert_eq!(runtime.core().depth(), 0);
    assert_eq!(
        runtime.core().trace().len(),
        3,
        "enter, applied Shield and failed leave retained"
    );
}
