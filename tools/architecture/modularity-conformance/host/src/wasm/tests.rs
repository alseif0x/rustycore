//! These are pre-freeze host/ABI checks, not the independent third-module verdict.
//! Real Rust/C artifacts exercise their shared exported behavior. Generated WAT is
//! explicitly hostile input for imports/traps, not a third-party extension module.

use super::*;
use conformance_contract::{ABI_VERSION, Manifest, capability};
use std::path::PathBuf;

mod imports;
mod registration;
mod resources;
mod validation;

#[derive(Clone, Copy, Debug)]
enum Frontend {
    Rust,
    C,
}

impl Frontend {
    const ALL: [Self; 2] = [Self::Rust, Self::C];

    fn artifact(self) -> Vec<u8> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_owned();
        let path = match self {
            Self::Rust => {
                root.join("target/wasm32-unknown-unknown/release/conformance_encounter.wasm")
            }
            Self::C => {
                root.join("../../../target/modularity-conformance/artifacts/encounter-c.wasm")
            }
        };
        std::fs::read(&path).unwrap_or_else(|error| {
            panic!(
                "required real guest artifact {}: {error}; build both frontends first",
                path.display()
            )
        })
    }

    fn runtime(self) -> (WasmRuntime, Handle) {
        let mut runtime = WasmRuntime::new(HostCore::default()).unwrap();
        runtime
            .register_wasm(encounter_manifest(), &self.artifact())
            .unwrap();
        let handle = runtime.spawn(100, 0).unwrap();
        (runtime, handle)
    }
}

fn encounter_manifest() -> Manifest {
    Manifest {
        id: 1,
        name: "encounter",
        abi: ABI_VERSION,
        schema: 1,
        capabilities: capability::QUERY
            | capability::SHIELD
            | capability::SUMMON
            | capability::REENTRY_PROBE,
        state_limit: 12,
        order: 10,
        exclusive: None,
    }
}

fn hostile_manifest(id: u64, name: &'static str) -> Manifest {
    Manifest {
        id,
        name,
        abi: ABI_VERSION,
        schema: 1,
        capabilities: capability::ALL,
        state_limit: 8,
        order: 0,
        exclusive: None,
    }
}

/// WAT fixture with the same numeric ABI. `body` is deliberately hostile guest code.
fn hostile(manifest: Manifest, body: &str, extra: &str) -> Vec<u8> {
    hostile_codec(
        manifest,
        body,
        extra,
        "local.get $length i32.const 8 i32.eq if (result i32) i32.const 0 else i32.const -1 end",
    )
}

fn hostile_codec(manifest: Manifest, body: &str, extra: &str, codec: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
        (import "conformance" "read" (func $read (param i32 i32 i32) (result i32)))
        (import "conformance" "write" (func $write (param i32 i32 i64) (result i32)))
        (import "conformance" "query" (func $query (param i32) (result i64)))
        (import "conformance" "action" (func $action (param i32 i64) (result i64)))
        (import "conformance" "validation_read" (func $validation_read (param i32) (result i32)))
        (memory (export "memory") 1)
        (func (export "abi_version") (result i32) i32.const {abi})
        (func (export "module_id") (result i64) i64.const {id})
        (func (export "state_schema") (result i32) i32.const {schema})
        (func (export "capabilities") (result i64) i64.const {caps})
        (func (export "state_limit") (result i32) i32.const {limit})
        (func (export "module_order") (result i32) i32.const {order})
        (func (export "initial_state_len") (result i32) i32.const 8)
        (func (export "initial_state_byte") (param i32) (result i32) i32.const 0)
        (func (export "validate_state") (param $length i32) (result i32) {codec})
        (func (export "invoke") (param $event i32) (param $argument i64) (result i64) {body})
        {extra}
    )"#,
        abi = manifest.abi,
        id = manifest.id,
        schema = manifest.schema,
        caps = manifest.capabilities,
        limit = manifest.state_limit,
        order = manifest.order
    ))
    .expect("valid hostile WAT fixture")
}

fn hostile_runtime(manifest: Manifest, body: &str) -> (WasmRuntime, Handle) {
    let mut runtime = WasmRuntime::new(HostCore::default()).unwrap();
    runtime
        .register_wasm(manifest, &hostile(manifest, body, ""))
        .unwrap();
    let handle = runtime.spawn(100, 0).unwrap();
    (runtime, handle)
}
