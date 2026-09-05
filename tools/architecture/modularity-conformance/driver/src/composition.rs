//! The third-module challenge may add declarative registration here, not host logic.
use conformance_contract::Result;
use conformance_encounter::Encounter;
use conformance_host::Limits;
use conformance_policy::Policy;

pub use crate::harness::{
    Descriptor, Harness, Mode, c_artifacts, compose, descriptor, empty, empty_with_limits,
    lab_root, rust_artifacts,
};

pub fn descriptors() -> Vec<Descriptor> {
    vec![
        descriptor::<Encounter>("conformance_encounter.wasm", "encounter-c.wasm"),
        descriptor::<Policy>("conformance_policy.wasm", "policy-c.wasm"),
    ]
}

pub fn build(mode: Mode) -> Result<Harness> {
    build_with_limits(mode, Limits::default())
}

pub fn build_with_limits(mode: Mode, limits: Limits) -> Result<Harness> {
    compose(mode, limits, &descriptors())
}
