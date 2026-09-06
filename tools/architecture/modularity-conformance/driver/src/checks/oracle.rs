//! Mode-independent observables: preserve order, revisions and every state byte.
//! Executor provenance and native addresses belong in separate diagnostics, not equality.

use conformance_contract::{Handle, Manifest, Snapshot};
use conformance_host::{HostCore, Observables, Residence, Trace};
use serde_json::{Value, json};

use super::{CaseResult, ok};

fn handle(value: Handle) -> Value {
    json!({ "guid": value.guid, "generation": value.generation })
}

fn trace(value: &Trace) -> Value {
    match value {
        Trace::Enter(invocation) => json!({
            "kind": "enter", "handle": handle(invocation.handle),
            "module": invocation.module, "event": invocation.event,
            "argument": invocation.argument,
        }),
        Trace::Leave { invocation, result } => {
            let result = match result {
                Ok(value) => json!({ "ok": value }),
                Err(fault) => json!({ "fault": format!("{fault:?}") }),
            };
            json!({
                "kind": "leave", "handle": handle(invocation.handle),
                "module": invocation.module, "event": invocation.event,
                "argument": invocation.argument, "result": result,
            })
        }
        Trace::Write {
            handle: identity,
            module,
            revision,
            bytes,
        } => json!({
            "kind": "write", "handle": handle(*identity), "module": module,
            "revision": revision, "bytes": bytes,
        }),
        Trace::Action {
            handle: identity,
            module,
            action,
            argument,
            value,
        } => json!({
            "kind": "action", "handle": handle(*identity), "module": module,
            "action": *action as u32, "argument": argument, "value": value,
        }),
    }
}

pub(super) fn render(
    manifests: &[Manifest],
    observed: Observables,
    states: Vec<(u64, Snapshot)>,
    events: &[Trace],
) -> Value {
    let states: Vec<_> = states.into_iter().map(|(id, state)| {
        let schema = manifests.iter().find(|manifest| manifest.id == id).map(|m| m.schema);
        json!({ "module": id, "schema": schema, "revision": state.revision, "bytes": state.bytes })
    }).collect();
    let contributions: Vec<_> = observed
        .by_module
        .iter()
        .map(|(id, contribution)| {
            json!({ "module": id, "shield": contribution.shield,
            "amount": contribution.amount, "summons": contribution.summons })
        })
        .collect();
    let residence = match observed.residence {
        Residence::Active(map) => json!({ "kind": "active", "map": map }),
        Residence::Detached => json!({ "kind": "detached" }),
    };
    json!({
        "handle": handle(observed.handle), "residence": residence,
        "payload_sentinel": observed.payload_sentinel, "shield": observed.shield,
        "summons": observed.summons, "contribution": observed.contribution,
        "by_module": contributions, "states": states,
        "trace": events.iter().map(trace).collect::<Vec<_>>(),
    })
}

pub(super) fn native(core: &HostCore, identity: Handle) -> CaseResult {
    let manifests = core.registered();
    let states = manifests
        .iter()
        .map(|m| ok(core.state(identity, m.id)).map(|state| (m.id, state)))
        .collect::<CaseResult<Vec<_>>>()?;
    let mut value = render(
        &manifests,
        ok(core.observables(identity))?,
        states,
        core.trace(),
    );
    value["core_revision"] = json!(ok(core.snapshot(identity))?.core_revision);
    Ok(value)
}
