//! Native registration/codec adversaries. These are not mislabeled Wasm sandbox tests.
//! Generic module wrappers share one State type to test namespace isolation by Module.

use conformance_contract::{
    ABI_VERSION, Action, Fault, Host, Manifest, Module, Result, State, capability, event,
};
use conformance_host::{HostCore, Limits};
use serde_json::json;

use super::{CaseResult, Check, checked, fault, ok, oracle, require};

const WRITE: u32 = event::CUSTOM + 40;
const MALFORMED: u32 = event::CUSTOM + 41;
const OVERSIZED: u32 = event::CUSTOM + 42;
const FORBIDDEN: u32 = event::CUSTOM + 43;

#[derive(Default)]
struct Counter(u32);

impl State for Counter {
    const SCHEMA: u32 = 1;

    fn encode(&self) -> Vec<u8> {
        self.0.to_le_bytes().to_vec()
    }

    fn decode(bytes: &[u8]) -> Result<Self> {
        let bytes: [u8; 4] = bytes.try_into().map_err(|_| Fault::Invalid)?;
        Ok(Self(u32::from_le_bytes(bytes)))
    }
}

struct Probe<const ID: u64, const VARIANT: u8>;

impl<const ID: u64, const VARIANT: u8> Module for Probe<ID, VARIANT> {
    type State = Counter;

    fn manifest() -> Manifest {
        let mut descriptor = Manifest {
            id: ID,
            name: match ID {
                11 => "probe_a",
                12 => "probe_b",
                _ => "other_probe",
            },
            abi: ABI_VERSION,
            schema: Counter::SCHEMA,
            capabilities: capability::QUERY,
            state_limit: 4,
            order: 0,
            exclusive: None,
        };
        match VARIANT {
            1 => descriptor.abi += 1,
            2 => descriptor.schema += 1,
            3 => descriptor.capabilities |= 1 << 63,
            4 => descriptor.name = "",
            5 => descriptor.state_limit = conformance_contract::MAX_STATE_BYTES + 1,
            6 => descriptor.state_limit = 3,
            8 => descriptor.name = "probe_a",
            9 => descriptor.exclusive = Some("exclusive_probe"),
            10 => descriptor.name = "same_id_other_type",
            _ => {}
        }
        descriptor
    }

    fn invoke(host: &mut dyn Host, event: u32, argument: i64) -> Result<i64> {
        match event {
            WRITE => {
                let state = host.read()?;
                let number = u32::try_from(argument).map_err(|_| Fault::Invalid)?;
                host.write(state.revision, &number.to_le_bytes())?;
                Ok(i64::from(number))
            }
            MALFORMED | OVERSIZED => {
                let state = host.read()?;
                let bytes: &[u8] = if event == MALFORMED {
                    &[1, 2, 3]
                } else {
                    &[1, 2, 3, 4, 5]
                };
                host.write(state.revision, bytes)?;
                Ok(0)
            }
            FORBIDDEN => host.action(Action::Shield, 1),
            _ => Ok(0),
        }
    }
}

fn unchanged_rejection(
    rejected: impl FnOnce(&mut HostCore) -> Result<()>,
    expected: Fault,
) -> CaseResult {
    let mut core = HostCore::default();
    let identity = ok(core.spawn(500, 0))?;
    let before = ok(core.observables(identity))?;
    fault(rejected(&mut core), expected)?;
    require(
        core.registered().is_empty(),
        "rejected module was registered",
    )?;
    require(
        ok(core.observables(identity))? == before,
        "rejected registration mutated the base owner",
    )?;
    require(
        core.trace().is_empty(),
        "rejected registration dispatched callbacks",
    )?;
    oracle::native(&core, identity)
}

fn identity() -> CaseResult {
    let zero = unchanged_rejection(|core| core.register::<Probe<0, 0>>(), Fault::Invalid)?;
    let blank = unchanged_rejection(|core| core.register::<Probe<11, 4>>(), Fault::Invalid)?;
    Ok(json!({ "zero": zero, "blank": blank, "coverage": "native-host-registration" }))
}

fn versions() -> CaseResult {
    let abi = unchanged_rejection(|core| core.register::<Probe<11, 1>>(), Fault::Version)?;
    let schema = unchanged_rejection(|core| core.register::<Probe<11, 2>>(), Fault::Version)?;
    Ok(json!({ "abi": abi, "schema": schema, "coverage": "native-host-registration" }))
}

fn capabilities() -> CaseResult {
    let unknown = unchanged_rejection(|core| core.register::<Probe<11, 3>>(), Fault::Capability)?;
    let mut core = HostCore::default();
    ok(core.register::<Probe<11, 0>>())?;
    let identity = ok(core.spawn(500, 0))?;
    let before = ok(core.observables(identity))?;
    fault(
        core.dispatch_one(identity, 11, FORBIDDEN, 0),
        Fault::Capability,
    )?;
    require(
        ok(core.observables(identity))? == before,
        "unauthorized action applied an effect",
    )?;
    require(
        core.depth() == 0,
        "unauthorized-action error leaked a frame",
    )?;
    Ok(
        json!({ "unknown": unknown, "action": oracle::native(&core, identity)?,
        "coverage": "native-host-registration-and-action" }),
    )
}

fn conflicts() -> CaseResult {
    let mut core = HostCore::default();
    ok(core.register::<Probe<11, 0>>())?;
    let identity = ok(core.spawn(500, 0))?;
    let state = ok(core.state(identity, 11))?;
    fault(core.register::<Probe<11, 0>>(), Fault::Conflict)?;
    fault(core.register::<Probe<11, 10>>(), Fault::Conflict)?;
    fault(core.register::<Probe<12, 8>>(), Fault::Conflict)?;
    require(
        core.registered().len() == 1,
        "conflicting registration changed membership",
    )?;
    require(
        ok(core.state(identity, 11))? == state,
        "conflicting registration rewrote existing state",
    )?;

    let mut exclusive = HostCore::default();
    ok(exclusive.register::<Probe<11, 9>>())?;
    let other = ok(exclusive.spawn(500, 0))?;
    fault(exclusive.register::<Probe<12, 9>>(), Fault::Conflict)?;
    require(
        exclusive.registered().len() == 1,
        "exclusive conflict partially registered",
    )?;
    Ok(
        json!({ "identity_type_name": oracle::native(&core, identity)?,
        "exclusive": oracle::native(&exclusive, other)?, "coverage": "native-host-registration" }),
    )
}

fn module_limit() -> CaseResult {
    let mut core = HostCore::new(Limits {
        modules: 1,
        ..Limits::default()
    });
    ok(core.register::<Probe<11, 0>>())?;
    let identity = ok(core.spawn(500, 0))?;
    let before = ok(core.state(identity, 11))?;
    fault(core.register::<Probe<12, 0>>(), Fault::Limit)?;
    require(
        core.registered().len() == 1,
        "module limit admitted an extra module",
    )?;
    require(
        ok(core.state(identity, 11))? == before,
        "module-limit failure changed existing state",
    )?;
    oracle::native(&core, identity)
}

fn state_limit() -> CaseResult {
    let declaration = unchanged_rejection(|core| core.register::<Probe<11, 5>>(), Fault::Limit)?;
    let default_value = unchanged_rejection(|core| core.register::<Probe<11, 6>>(), Fault::Limit)?;
    let mut core = HostCore::new(Limits {
        state_bytes: 3,
        ..Limits::default()
    });
    fault(core.register::<Probe<11, 0>>(), Fault::Limit)?;
    require(core.registered().is_empty(), "host state cap was ignored")?;
    Ok(json!({ "declaration": declaration, "default": default_value, "host_cap": "rejected" }))
}

fn rejected_write(event: u32, expected: Fault) -> CaseResult {
    let mut core = HostCore::default();
    ok(core.register::<Probe<11, 0>>())?;
    let identity = ok(core.spawn(500, 0))?;
    ok(core.dispatch_one(identity, 11, WRITE, 123))?;
    let state = ok(core.state(identity, 11))?;
    let observed = ok(core.observables(identity))?;
    ok(core.clear_trace())?;
    fault(core.dispatch_one(identity, 11, event, 0), expected)?;
    require(
        ok(core.state(identity, 11))? == state,
        "rejected write changed bytes or revision",
    )?;
    require(
        ok(core.observables(identity))? == observed,
        "rejected write changed core effects",
    )?;
    require(core.depth() == 0, "write rejection leaked a frame")?;
    require(
        !core
            .trace()
            .iter()
            .any(|event| matches!(event, conformance_host::Trace::Write { .. })),
        "rejected write published a successful state mutation",
    )?;
    oracle::native(&core, identity)
}

fn shared_type() -> CaseResult {
    let mut core = HostCore::default();
    ok(core.register::<Probe<11, 0>>())?;
    ok(core.register::<Probe<12, 0>>())?;
    let identity = ok(core.spawn(500, 0))?;
    ok(core.dispatch_one(identity, 11, WRITE, 41))?;
    let first = ok(core.state(identity, 11))?;
    require(
        first.bytes == 41_u32.to_le_bytes(),
        "first namespace write missing",
    )?;
    require(
        ok(core.state(identity, 12))?.bytes == 0_u32.to_le_bytes(),
        "same State type aliased module state",
    )?;
    ok(core.dispatch_one(identity, 12, WRITE, 73))?;
    require(
        ok(core.state(identity, 11))? == first,
        "second namespace clobbered first",
    )?;
    require(
        ok(core.state(identity, 12))?.bytes == 73_u32.to_le_bytes(),
        "second namespace write missing",
    )?;
    oracle::native(&core, identity)
}

pub(super) fn run() -> Vec<Check> {
    vec![
        checked("invalid_registration_identity", identity),
        checked("invalid_registration_versions", versions),
        checked("invalid_registration_capabilities", capabilities),
        checked("registration_conflicts", conflicts),
        checked("module_count_limit", module_limit),
        checked("registration_state_limit", state_limit),
        checked("malformed_state_write", || {
            rejected_write(MALFORMED, Fault::Invalid)
        }),
        checked("oversized_state_write", || {
            rejected_write(OVERSIZED, Fault::Limit)
        }),
        checked("shared_state_type_isolation", shared_type),
    ]
}
