use super::*;
use conformance_contract::{Fault, Host, MAX_MODULES, MEMORY_BYTES, Module, Result, State};

#[derive(Default)]
struct NativeState;

impl State for NativeState {
    const SCHEMA: u32 = 1;
    fn encode(&self) -> Vec<u8> {
        vec![0; 8]
    }
    fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() == 8 {
            Ok(Self)
        } else {
            Err(Fault::Invalid)
        }
    }
}

struct NativeSlot;

impl Module for NativeSlot {
    type State = NativeState;
    fn manifest() -> Manifest {
        hostile_manifest(500, "native-slot")
    }
    fn invoke(_: &mut dyn Host, _: u32, _: i64) -> Result<i64> {
        Ok(0)
    }
}

struct SignedReturn;

impl Module for SignedReturn {
    type State = NativeState;
    fn manifest() -> Manifest {
        hostile_manifest(501, "signed-return")
    }
    fn invoke(_: &mut dyn Host, event: u32, _: i64) -> Result<i64> {
        match event {
            1 => Ok(-2),
            2 => Err(Fault::Stale),
            _ => Ok(7),
        }
    }
}

#[test]
fn wasm_runtime_native_returns_share_portable_fault_semantics_and_leave_trace() {
    let mut native = HostCore::default();
    native.register::<SignedReturn>().unwrap();
    let handle = native.spawn(100, 0).unwrap();
    let mut mixed = WasmRuntime::new(HostCore::default()).unwrap();
    mixed.register_native::<SignedReturn>().unwrap();
    assert_eq!(mixed.spawn(100, 0), Ok(handle));
    for (event, expected) in [(1, Err(Fault::Invalid)), (2, Err(Fault::Stale)), (3, Ok(7))] {
        assert_eq!(native.dispatch_one(handle, 501, event, 0), expected);
        assert_eq!(mixed.dispatch_one(handle, 501, event, 0), expected);
        assert_eq!(mixed.core().trace(), native.trace());
        assert_eq!(mixed.snapshot(handle), native.snapshot(handle));
        assert_eq!(mixed.core().depth(), 0);
    }
}

#[test]
fn duplicate_executor_rejection_preserves_native_or_wasm_authority() {
    let manifest = NativeSlot::manifest();
    let bytes = hostile(manifest, "i64.const 0", "");
    let mut core = HostCore::default();
    core.register::<NativeSlot>().unwrap();
    let mut native = WasmRuntime::new(core).unwrap();
    let handle = native.spawn(100, 0).unwrap();
    let before = native.snapshot(handle).unwrap();
    assert_eq!(native.register_wasm(manifest, &bytes), Err(Fault::Conflict));
    assert_eq!(native.snapshot(handle).unwrap(), before);
    assert_eq!(native.core().executor(500), Ok(crate::Executor::Native));
    assert!(native.store.data().guests.is_empty());

    let mut wasm = WasmRuntime::new(HostCore::default()).unwrap();
    wasm.register_wasm(manifest, &bytes).unwrap();
    let handle = wasm.spawn(100, 0).unwrap();
    let before = wasm.snapshot(handle).unwrap();
    assert_eq!(wasm.register_native::<NativeSlot>(), Err(Fault::Conflict));
    assert_eq!(wasm.snapshot(handle).unwrap(), before);
    assert_eq!(wasm.core().executor(500), Ok(crate::Executor::Wasm));
}

#[test]
fn hostile_metadata_mismatches_never_activate_or_add_entity_state() {
    for field in [
        "abi_version",
        "module_id",
        "state_schema",
        "capabilities",
        "state_limit",
        "module_order",
        "initial_state_len",
    ] {
        let manifest = hostile_manifest(500, "hostile");
        let original = hostile(manifest, "i64.const 0", "");
        // Build a fresh textual fixture, then patch only the named export by
        // overriding the supplied manifest or the initial length function.
        let mut forged = manifest;
        match field {
            "abi_version" => forged.abi += 1,
            "module_id" => forged.id += 1,
            "state_schema" => forged.schema += 1,
            "capabilities" => forged.capabilities = 0,
            "state_limit" => forged.state_limit += 1,
            "module_order" => forged.order += 1,
            "initial_state_len" => {}
            _ => unreachable!(),
        }
        let bytes = if field == "initial_state_len" {
            // A fixture with an impossible declared default is a separate
            // artifact, not a mutation of the real Rust/C evidence.
            wat::parse_str(
                r#"(module
                (memory (export "memory") 1)
                (func (export "abi_version") (result i32) i32.const 1)
                (func (export "module_id") (result i64) i64.const 500)
                (func (export "state_schema") (result i32) i32.const 1)
                (func (export "capabilities") (result i64) i64.const 31)
                (func (export "state_limit") (result i32) i32.const 8)
                (func (export "module_order") (result i32) i32.const 0)
                (func (export "initial_state_len") (result i32) i32.const 257)
                (func (export "initial_state_byte") (param i32) (result i32) i32.const 0)
                (func (export "invoke") (param i32 i64) (result i64) i64.const 0)
            )"#,
            )
            .unwrap()
        } else {
            hostile(forged, "i64.const 0", "")
        };
        assert_ne!(bytes, original);
        let mut runtime = WasmRuntime::new(HostCore::default()).unwrap();
        let handle = runtime.spawn(100, 0).unwrap();
        let before = runtime.snapshot(handle).unwrap();
        let expected = if field == "initial_state_len" {
            Fault::Limit
        } else {
            Fault::Version
        };
        assert_eq!(
            runtime.register_wasm(manifest, &bytes),
            Err(expected),
            "{field}"
        );
        assert_eq!(runtime.snapshot(handle).unwrap(), before);
        assert!(runtime.core().registered().is_empty());
        assert!(runtime.store.data().guests.is_empty());
    }
}

#[test]
fn real_frontends_admit_matching_metadata_and_reject_duplicate_or_mismatched_identity() {
    for frontend in Frontend::ALL {
        let (mut runtime, handle) = frontend.runtime();
        let bytes = frontend.artifact();
        let before = runtime.snapshot(handle).unwrap();
        let registered = runtime.core().registered();
        assert_eq!(
            runtime.register_wasm(encounter_manifest(), &bytes),
            Err(Fault::Conflict)
        );
        let mut impostor = encounter_manifest();
        impostor.id = 2;
        impostor.name = "impostor";
        assert_eq!(runtime.register_wasm(impostor, &bytes), Err(Fault::Version));
        assert_eq!(runtime.snapshot(handle).unwrap(), before);
        assert_eq!(runtime.core().registered(), registered);
        assert_eq!(runtime.store.data().guests.len(), 1);
        let timings = runtime.timings();
        assert!(timings[0].compile_ns.is_some());
        assert!(timings[0].instantiate_ns.is_some());
        assert!(timings[0].metadata_ns.is_some());
        assert_eq!(
            timings[1].compile_ns, None,
            "duplicate rejected before compilation"
        );
        assert_eq!(timings[2].fault, Some(Fault::Version));
    }
}

#[test]
fn bad_manifest_and_oversize_binary_fail_before_guest_compilation() {
    let mut runtime = WasmRuntime::new(HostCore::default()).unwrap();
    let handle = runtime.spawn(100, 0).unwrap();
    let before = runtime.snapshot(handle).unwrap();
    let valid = hostile_manifest(500, "hostile");
    let mut invalid = valid;
    invalid.abi = 2;
    assert_eq!(runtime.register_wasm(invalid, &[]), Err(Fault::Version));
    invalid = valid;
    invalid.capabilities = 1 << 63;
    assert_eq!(runtime.register_wasm(invalid, &[]), Err(Fault::Capability));
    assert_eq!(
        runtime.register_wasm(valid, &vec![0; 4 * 1024 * 1024 + 1]),
        Err(Fault::Limit)
    );
    assert!(
        runtime
            .timings()
            .iter()
            .all(|timing| timing.compile_ns.is_none())
    );
    assert_eq!(runtime.snapshot(handle).unwrap(), before);
    assert!(runtime.core().registered().is_empty());
    assert!(runtime.store.data().guests.is_empty());
}

#[test]
fn start_functions_have_no_invocation_authority_over_existing_entities() {
    let mut runtime = WasmRuntime::new(HostCore::default()).unwrap();
    let handle = runtime.spawn(100, 0).unwrap();
    let before = runtime.snapshot(handle).unwrap();
    let manifest = hostile_manifest(500, "hostile");
    let extra = r#"(func $start i32.const 1 i64.const 1 call $action drop) (start $start)"#;
    runtime
        .register_wasm(manifest, &hostile(manifest, "i64.const 0", extra))
        .unwrap();
    assert_eq!(runtime.snapshot(handle).unwrap(), before);
    assert!(runtime.core().entity_modules(handle).unwrap().is_empty());
    assert_eq!(runtime.core().depth(), 0);
}

#[test]
fn instance_count_and_total_linear_memory_are_bounded_in_one_store() {
    let mut runtime = WasmRuntime::new(HostCore::default()).unwrap();
    let names = ["a", "b", "c", "d", "e", "f", "g", "h"];
    let extra = r#"(func (export "probe_grow") (param i32) (result i32) local.get 0 memory.grow)"#;
    for (index, name) in names.into_iter().enumerate() {
        let manifest = hostile_manifest(500 + index as u64, name);
        runtime
            .register_wasm(manifest, &hostile(manifest, "i64.const 0", extra))
            .unwrap();
    }
    let handle = runtime.spawn(100, 0).unwrap();
    for id in 500..500 + MAX_MODULES as u64 {
        assert_eq!(
            runtime.probe_grow(handle, id, (MEMORY_BYTES / 65536 - 1) as u32),
            Ok(1)
        );
        assert_eq!(runtime.probe_grow(handle, id, 1), Ok(-1));
    }
    let instances: Vec<_> = runtime
        .store
        .data()
        .guests
        .values()
        .map(|guest| guest.instance)
        .collect();
    let total: usize = instances
        .into_iter()
        .map(|instance| {
            instance
                .get_memory(&mut runtime.store, "memory")
                .unwrap()
                .data_size(&runtime.store)
        })
        .sum();
    assert_eq!(
        total,
        MAX_MODULES * MEMORY_BYTES,
        "24 MiB linear memory, not process RSS"
    );
    let manifest = hostile_manifest(999, "ninth");
    assert_eq!(
        runtime.register_wasm(manifest, &hostile(manifest, "i64.const 0", "")),
        Err(Fault::Limit)
    );
    assert_eq!(runtime.store.data().guests.len(), MAX_MODULES);
    assert_eq!(runtime.timings().last().unwrap().compile_ns, None);
}
