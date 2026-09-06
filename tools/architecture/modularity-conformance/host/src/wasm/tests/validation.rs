//! Codec tests deliberately distinguish real Rust/C module schemas from hostile
//! WAT. No length, field or ModuleId-specific rule is added to the host.

use super::*;
use conformance_contract::{Fault, event};
use wasmtime::AsContextMut;

const WRITE_RECORD: &str = "i32.const 0 i32.const 8 i32.const 32 call $read drop
    i32.const 0 i32.const 8 i32.const 32 i64.load call $write i64.extend_i32_s";

#[test]
fn hidden_second_memory_is_rejected_before_instantiation() {
    let manifest = hostile_manifest(500, "hidden-memory");
    let bytes = hostile(manifest, "i64.const 0", "(memory 48)");
    let mut runtime = WasmRuntime::new(HostCore::default()).unwrap();
    assert!(runtime.register_wasm(manifest, &bytes).is_err());
    assert!(runtime.timings()[0].compile_ns.is_some());
    assert_eq!(runtime.timings()[0].instantiate_ns, None);
    assert!(runtime.core().registered().is_empty());
    assert!(runtime.store.data().guests.is_empty());
}

#[test]
fn initial_codec_rejection_does_not_activate_or_advance_authority() {
    let manifest = hostile_manifest(500, "reject-initial");
    for codec in ["i32.const -1", "i32.const 1", "unreachable"] {
        let mut runtime = WasmRuntime::new(HostCore::default()).unwrap();
        let handle = runtime.spawn(100, 0).unwrap();
        let before = runtime.snapshot(handle).unwrap();
        let clock = runtime.core().revision_clock;
        let expected = if codec == "unreachable" {
            Fault::Trap
        } else {
            Fault::Invalid
        };
        assert_eq!(
            runtime.register_wasm(manifest, &hostile_codec(manifest, "i64.const 0", "", codec)),
            Err(expected)
        );
        assert_eq!(runtime.snapshot(handle).unwrap(), before);
        assert_eq!(runtime.core().revision_clock, clock);
        assert!(runtime.core().registered().is_empty());
        assert!(runtime.store.data().validation.is_none());
    }
}

#[test]
fn real_rust_and_c_codecs_reject_malformed_replay_before_mutation() {
    for frontend in Frontend::ALL {
        for malformed in [vec![0; 11], vec![2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]] {
            let (mut runtime, handle) = frontend.runtime();
            let before = runtime.snapshot(handle).unwrap();
            let clock = runtime.core().revision_clock;
            let mut rejected = before.clone();
            rejected.modules[0].bytes = malformed;
            assert_eq!(
                runtime.replay(handle, &rejected),
                Err(Fault::Invalid),
                "{frontend:?}"
            );
            assert_eq!(runtime.snapshot(handle).unwrap(), before);
            assert_eq!(runtime.core().revision_clock, clock);
            assert!(runtime.store.data().validation.is_none());
            assert_eq!(
                runtime.core().calls(),
                0,
                "codec reads are not semantic calls"
            );
            runtime.replay(handle, &before).unwrap();
            runtime.reset(handle, 1).unwrap();
            runtime.remove_module_state(handle, 1).unwrap();
            assert_eq!(runtime.state(handle, 1), Err(Fault::Missing));
        }
    }
}

#[test]
fn real_rust_and_c_live_writes_validate_shape_without_consuming_revisions() {
    for frontend in Frontend::ALL {
        let (mut runtime, handle) = frontend.runtime();
        let before = runtime.snapshot(handle).unwrap();
        let clock = runtime.core().revision_clock;
        for arguments in [(11_u32, 0_u32, 0_u32), (12, 0, 2)] {
            let raw: Result<i64> = runtime.probe(handle, 1, "probe_write", arguments);
            assert_eq!(
                raw.and_then(decode_result),
                Err(Fault::Invalid),
                "{frontend:?}"
            );
            assert_eq!(runtime.snapshot(handle).unwrap(), before);
            assert_eq!(runtime.core().revision_clock, clock);
            assert_eq!(
                runtime.core().calls(),
                3,
                "entry/read/write, not codec reads"
            );
            assert!(runtime.store.data().validation.is_none());
        }
        let valid: Result<i64> = runtime.probe(handle, 1, "probe_write", (12_u32, 0_u32, 1_u32));
        assert_eq!(valid.and_then(decode_result), Ok(0));
        assert_eq!(runtime.state(handle, 1).unwrap().bytes[0], 1);
        assert_eq!(runtime.core().revision_clock, clock + 1);
    }
}

#[test]
fn semantic_imports_are_forbidden_inside_codec_even_when_error_is_ignored() {
    let attempts = [
        "i32.const -1 i32.const 257 i32.const -1 call $read drop",
        "i32.const -1 i32.const 257 i64.const -1 call $write drop",
        "i32.const 999 call $query drop",
        "i32.const 1 i64.const 1 call $action drop",
    ];
    for attempt in attempts {
        let manifest = hostile_manifest(500, "ignored-codec-fault");
        let extra = "(global $armed (mut i32) (i32.const 0))";
        let codec = format!("global.get $armed if {attempt} end i32.const 0");
        let body = format!("i32.const 1 global.set $armed {WRITE_RECORD}");
        let mut runtime = WasmRuntime::new(HostCore::default()).unwrap();
        runtime
            .register_wasm(manifest, &hostile_codec(manifest, &body, extra, &codec))
            .unwrap();
        let handle = runtime.spawn(100, 0).unwrap();
        let before = runtime.snapshot(handle).unwrap();
        let clock = runtime.core().revision_clock;
        assert_eq!(
            runtime.dispatch_one(handle, 500, event::CUSTOM, 0),
            Err(Fault::Capability),
            "{attempt}"
        );
        assert_eq!(runtime.snapshot(handle).unwrap(), before);
        assert_eq!(runtime.core().revision_clock, clock);
        assert_eq!(runtime.core().calls(), 3);
        assert!(runtime.store.data().validation.is_none());

        // Registration also fails if the validator attempts an action with no
        // invocation frame, then ignores the returned fault.
        let mut fresh = WasmRuntime::new(HostCore::default()).unwrap();
        let codec = format!("{attempt} i32.const 0");
        assert_eq!(
            fresh.register_wasm(
                manifest,
                &hostile_codec(manifest, "i64.const 0", "", &codec)
            ),
            Err(Fault::Capability)
        );
        assert!(fresh.core().registered().is_empty());
        assert_eq!(fresh.core().calls(), 0);
        assert!(fresh.store.data().validation.is_none());
    }
}

#[test]
fn validation_reads_have_scoped_separate_budget_and_sticky_failures() {
    let manifest = hostile_manifest(500, "validation-reads");
    let codec = "(local $n i32) loop $again i32.const 0 call $validation_read drop
        local.get $n i32.const 1 i32.add local.tee $n i32.const 257 i32.lt_u br_if $again end i32.const 0";
    let mut excessive = WasmRuntime::new(HostCore::default()).unwrap();
    assert_eq!(
        excessive.register_wasm(manifest, &hostile_codec(manifest, "i64.const 0", "", codec)),
        Err(Fault::Limit)
    );
    assert_eq!(excessive.core().calls(), 0);
    assert!(excessive.store.data().validation.is_none());

    let mut invalid = WasmRuntime::new(HostCore::default()).unwrap();
    assert_eq!(
        invalid.register_wasm(
            manifest,
            &hostile_codec(
                manifest,
                "i64.const 0",
                "",
                "i32.const 8 call $validation_read drop i32.const 0"
            )
        ),
        Err(Fault::Invalid)
    );
    assert!(invalid.store.data().validation.is_none());
    let (mut outside, handle) = hostile_runtime(
        manifest,
        "i32.const 0 call $validation_read i64.extend_i32_s",
    );
    assert_eq!(
        outside.dispatch_one(handle, 500, event::CUSTOM, 0),
        Err(Fault::Capability)
    );
    assert_eq!(outside.core().calls(), 2);
}

#[test]
fn codec_traps_clear_the_phase_and_preserve_previous_state() {
    let manifest = hostile_manifest(500, "codec-trap");
    let extra = "(global $armed (mut i32) (i32.const 0))";
    let codec = "global.get $armed if unreachable end i32.const 0";
    let body = format!("local.get $argument i32.wrap_i64 global.set $armed {WRITE_RECORD}");
    let mut runtime = WasmRuntime::new(HostCore::default()).unwrap();
    runtime
        .register_wasm(manifest, &hostile_codec(manifest, &body, extra, codec))
        .unwrap();
    let handle = runtime.spawn(100, 0).unwrap();
    let before = runtime.snapshot(handle).unwrap();
    let clock = runtime.core().revision_clock;
    assert_eq!(
        runtime.dispatch_one(handle, 500, event::CUSTOM, 1),
        Err(Fault::Trap)
    );
    assert_eq!(runtime.snapshot(handle).unwrap(), before);
    assert_eq!(runtime.core().revision_clock, clock);
    assert!(runtime.store.data().validation.is_none());
    assert_eq!(runtime.core().depth(), 0);
    assert_eq!(runtime.dispatch_one(handle, 500, event::CUSTOM, 0), Ok(0));
}

#[test]
fn codec_uses_remaining_root_fuel_instead_of_refilling() {
    for frontend in Frontend::ALL {
        let (mut runtime, handle) = frontend.runtime();
        let bytes = runtime.state(handle, 1).unwrap().bytes;
        let before = runtime.snapshot(handle).unwrap();
        let clock = runtime.core().revision_clock;
        let result = runtime.root(|mut context| {
            // Deliberately leave insufficient fuel in this root. A validator that
            // incorrectly replenishes FUEL would turn this exact test into success.
            context.set_fuel(1).map_err(execution_fault)?;
            super::super::validation::registered(context.as_context_mut(), 1, &bytes)
        });
        assert_eq!(result, Err(Fault::Limit), "{frontend:?}");
        assert!(runtime.store.data().validation.is_none());
        assert_eq!(runtime.snapshot(handle).unwrap(), before);
        assert_eq!(runtime.core().revision_clock, clock);
        assert_eq!(runtime.core().calls(), 0);
        // Positive control: the same real codec and same input fit a fresh root.
        assert_eq!(
            runtime.root(|context| super::super::validation::registered(context, 1, &bytes)),
            Ok(())
        );
    }
}

#[test]
fn live_write_preflight_rejects_stale_or_oversize_before_codec() {
    let manifest = hostile_manifest(500, "write-preflight");
    let extra = "(global $armed (mut i32) (i32.const 0))";
    let codec = "global.get $armed if i32.const 1 i64.const 1 call $action drop end i32.const 0";
    let body = "i32.const 1 global.set $armed
        i32.const 0 i32.const 8 i32.const 32 call $read drop
        i32.const 0
        local.get $event i32.const 1 i32.eq if (result i32) i32.const 8 else i32.const 9 end
        local.get $event i32.const 1 i32.eq if (result i64) i64.const 0 else i32.const 32 i64.load end
        call $write i64.extend_i32_s";
    let mut runtime = WasmRuntime::new(HostCore::default()).unwrap();
    runtime
        .register_wasm(manifest, &hostile_codec(manifest, body, extra, codec))
        .unwrap();
    let handle = runtime.spawn(100, 0).unwrap();
    let before = runtime.snapshot(handle).unwrap();
    let clock = runtime.core().revision_clock;
    for (event, expected) in [(1, Fault::Revision), (2, Fault::Limit)] {
        assert_eq!(runtime.dispatch_one(handle, 500, event, 0), Err(expected));
        assert_eq!(runtime.snapshot(handle).unwrap(), before);
        assert_eq!(runtime.core().revision_clock, clock);
        assert!(runtime.store.data().validation.is_none());
    }
}

#[test]
fn replay_validates_all_codec_candidates_before_any_installation() {
    let mut runtime = WasmRuntime::new(HostCore::default()).unwrap();
    for manifest in [
        hostile_manifest(500, "first"),
        hostile_manifest(501, "second"),
    ] {
        runtime
            .register_wasm(manifest, &hostile(manifest, "i64.const 0", ""))
            .unwrap();
    }
    let handle = runtime.spawn(100, 0).unwrap();
    let before = runtime.snapshot(handle).unwrap();
    let clock = runtime.core().revision_clock;
    let mut rejected = before.clone();
    rejected.modules[0].bytes[0] = 1; // Valid first candidate, must not be installed early.
    rejected.modules[1].bytes.pop(); // The second module codec rejects seven bytes.
    assert_eq!(runtime.replay(handle, &rejected), Err(Fault::Invalid));
    assert_eq!(runtime.snapshot(handle).unwrap(), before);
    assert_eq!(runtime.core().revision_clock, clock);
    assert!(runtime.store.data().validation.is_none());
}
