//! The same bounded scenarios run through native, Rust Wasm, C Wasm and mixed adapters.
//! Literal assertions derive from the named fixture contract, not agreement alone.

use conformance_contract::{Action, Fault, Handle, Snapshot, State, event};
use conformance_encounter::{EncounterState, FAIL_AFTER_SHIELD, STALE_OUTER_WRITE};
use conformance_host::{Executor, Limits, Residence, Trace};
use conformance_policy::PolicyState;
use serde_json::json;

use super::{CaseResult, Check, checked, fault, ok, oracle, require};
use crate::composition::{self, Harness, Mode};

const ENCOUNTER: u64 = conformance_encounter::MODULE_ID;
const POLICY: u64 = conformance_policy::MODULE_ID;

fn entity(mode: Mode) -> CaseResult<(Harness, Handle)> {
    let mut host = ok(composition::build(mode))?;
    let identity = ok(host.spawn(700, 0))?;
    Ok((host, identity))
}

fn observe(host: &Harness, identity: Handle) -> CaseResult {
    observe_trace(host, identity, host.trace())
}

fn observe_trace(host: &Harness, identity: Handle, trace: &[Trace]) -> CaseResult {
    let saved = ok(host.snapshot(identity))?;
    let states = saved
        .modules
        .into_iter()
        .map(|module| {
            (
                module.id,
                Snapshot {
                    revision: module.revision,
                    bytes: module.bytes,
                },
            )
        })
        .collect();
    let mut value = oracle::render(
        &host.registered(),
        ok(host.observables(identity))?,
        states,
        trace,
    );
    value["core_revision"] = json!(saved.core_revision);
    value["snapshot_format"] = json!(saved.format);
    value["depth"] = json!(host.depth());
    value["host_calls"] = json!(host.calls());
    Ok(value)
}

fn encounter(host: &Harness, identity: Handle) -> CaseResult<EncounterState> {
    ok(EncounterState::decode(
        &ok(host.state(identity, ENCOUNTER))?.bytes,
    ))
}

fn policy(host: &Harness, identity: Handle) -> CaseResult<PolicyState> {
    ok(PolicyState::decode(
        &ok(host.state(identity, POLICY))?.bytes,
    ))
}

fn action_position(host: &Harness, action: Action) -> Option<usize> {
    host.trace().iter().position(|entry| {
        matches!(entry,
        Trace::Action { module: ENCOUNTER, action: actual, .. } if *actual == action)
    })
}

fn registration_order(mode: Mode) -> CaseResult {
    let (mut host, identity) = entity(mode)?;
    let manifests = host.registered();
    let mut expected = manifests.clone();
    expected.sort_by_key(|manifest| (manifest.order, manifest.id));
    require(
        manifests == expected,
        "registration order is not (order, id)",
    )?;
    let values = ok(host.dispatch(identity, event::POLICY, 31))?;
    require(
        values.iter().map(|(id, _)| *id).collect::<Vec<_>>()
            == expected
                .iter()
                .map(|manifest| manifest.id)
                .collect::<Vec<_>>(),
        "dispatch did not preserve declared module order",
    )?;
    let entered: Vec<_> = host
        .trace()
        .iter()
        .filter_map(|entry| match entry {
            Trace::Enter(invocation) => Some(invocation.module),
            _ => None,
        })
        .collect();
    require(
        entered == values.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        "observed callback order differs from returned dispatch order",
    )?;
    Ok(json!({ "values": values, "oracle": observe(&host, identity)? }))
}

fn zero_optional(mode: Mode) -> CaseResult {
    let mut host = ok(composition::empty(mode))?;
    require(
        host.registered().is_empty(),
        "zero-module factory registered optional modules",
    )?;
    let identity = ok(host.spawn(700, 0))?;
    let before = ok(host.observables(identity))?;
    require(
        ok(host.dispatch(identity, event::UPDATE, 1))?.is_empty(),
        "zero-module dispatch invoked behavior",
    )?;
    require(
        ok(host.observables(identity))? == before,
        "zero modules altered the base record",
    )?;
    require(
        host.trace().is_empty(),
        "zero modules emitted callback/action trace",
    )?;
    let allocation = ok(host.payload_identity(identity))?;
    ok(host.detach(identity))?;
    ok(host.attach(identity, 1))?;
    require(
        ok(host.payload_identity(identity))? == allocation,
        "zero-module transfer cloned/replaced the base payload",
    )?;
    require(
        ok(host.observables(identity))?.payload_sentinel == before.payload_sentinel,
        "zero-module transfer lost base state",
    )?;
    observe(&host, identity)
}

fn policy_isolation(mode: Mode) -> CaseResult {
    let (mut host, first) = entity(mode)?;
    let second = ok(host.spawn(701, 1))?;
    let untouched = ok(host.snapshot(second))?;
    let encounter_before = ok(host.state(first, ENCOUNTER))?;
    let amount = ok(host.dispatch_one(first, POLICY, event::POLICY, 123))?;
    require(
        amount == 123,
        "custom policy did not preserve its declared 100 percent rule",
    )?;
    let state = policy(&host, first)?;
    require(
        state.calls == 1 && state.percent == 100,
        "policy state not updated exactly once",
    )?;
    require(
        ok(host.state(first, ENCOUNTER))? == encounter_before,
        "policy overwrote another module's state",
    )?;
    require(
        ok(host.snapshot(second))? == untouched,
        "policy leaked across entity/map scope",
    )?;
    let contribution = ok(host.observables(first))?
        .by_module
        .into_iter()
        .find(|(id, _)| *id == POLICY);
    require(
        contribution
            .is_some_and(|(_, value)| value.amount == 100 && !value.shield && value.summons == 0),
        "policy contribution has wrong amount or leaked another capability",
    )?;
    Ok(
        json!({ "value": amount, "first": observe(&host, first)?, "second": observe(&host, second)? }),
    )
}

fn summon_reentry(mode: Mode) -> CaseResult {
    let (host, identity) = entity(mode)?;
    summon_reentry_on(host, identity)
}

fn summon_reentry_on(mut host: Harness, identity: Handle) -> CaseResult {
    let value = ok(host.dispatch_one(identity, ENCOUNTER, event::UPDATE, 1))?;
    let state = encounter(&host, identity)?;
    require(
        value == 1 && state.phase == 1 && state.callbacks == 1,
        "successful summon did not observe its synchronous callback before return",
    )?;
    let shield = action_position(&host, Action::Shield).ok_or("missing shield action")?;
    let summon = action_position(&host, Action::Summon).ok_or("missing summon action")?;
    let first_write = host
        .trace()
        .iter()
        .position(|entry| {
            matches!(
                entry,
                Trace::Write {
                    module: ENCOUNTER,
                    ..
                }
            )
        })
        .ok_or("missing phase mutation")?;
    let callback = host.trace().iter().position(|entry| matches!(entry,
        Trace::Enter(invocation) if invocation.module == ENCOUNTER && invocation.event == event::CALLBACK))
        .ok_or("missing actual nested callback")?;
    require(
        first_write < shield && shield < summon && summon < callback,
        "phase/shield/summon/callback causal order differs from the contract",
    )?;
    require(
        host.depth() == 0,
        "successful reentry leaked its invocation frame",
    )?;
    let initial = observe(&host, identity)?;
    let repeated = ok(host.dispatch_one(identity, ENCOUNTER, event::UPDATE, 1))?;
    require(
        repeated == 1 && encounter(&host, identity)?.callbacks == 1,
        "same encounter phase summoned/callbacked twice",
    )?;
    Ok(
        json!({ "first_result": value, "first": initial, "repeat_result": repeated,
        "repeat": observe(&host, identity)? }),
    )
}

fn nullable_summon(mode: Mode) -> CaseResult {
    let (mut host, identity) = entity(mode)?;
    require(
        ok(host.dispatch_one(identity, ENCOUNTER, event::UPDATE, 0))? == 0,
        "nullable failure was not a normal zero result",
    )?;
    let state = encounter(&host, identity)?;
    let observed = ok(host.observables(identity))?;
    require(
        state.phase == 1 && state.callbacks == 0 && observed.shield && observed.summons == 0,
        "nullable summon undid phase/shield or ran a callback",
    )?;
    require(
        !host.trace().iter().any(|entry| {
            matches!(entry,
        Trace::Enter(invocation) if invocation.event == event::CALLBACK)
        }),
        "null summon entered a callback",
    )?;
    observe(&host, identity)
}

fn callback_result_oracle(mode: Mode) -> CaseResult {
    let (mut host, identity) = entity(mode)?;
    ok(host.dispatch_one(identity, ENCOUNTER, event::UPDATE, 1))?;
    let before = ok(host.snapshot(identity))?;
    let correct = observe(&host, identity)?;
    let mut changed = host.trace().to_vec();
    let returned = changed
        .iter_mut()
        .find_map(|entry| match entry {
            Trace::Leave { invocation, result }
                if invocation.module == POLICY && invocation.event == event::CALLBACK =>
            {
                Some(result)
            }
            _ => None,
        })
        .ok_or("missing real Policy callback return")?;
    require(
        *returned == Ok(0),
        "unexpected Policy callback baseline return",
    )?;
    *returned = Ok(1);
    let mutant = observe_trace(&host, identity, &changed)?;
    require(
        correct != mutant,
        "oracle ignored a wrong callback result with identical state",
    )?;
    require(
        ok(host.snapshot(identity))? == before,
        "oracle counterfactual changed canonical state",
    )?;
    Ok(
        json!({ "actual": correct, "counterfactual_wrong_return": mutant,
        "coverage": "real callback trace; only copied return is mutated to test oracle sensitivity" }),
    )
}

fn stale_outer(mode: Mode) -> CaseResult {
    let (host, identity) = entity(mode)?;
    stale_outer_on(host, identity)
}

fn stale_outer_on(mut host: Harness, identity: Handle) -> CaseResult {
    let before = ok(host.state(identity, ENCOUNTER))?;
    fault(
        host.dispatch_one(identity, ENCOUNTER, STALE_OUTER_WRITE, 0),
        Fault::Revision,
    )?;
    let after = ok(host.state(identity, ENCOUNTER))?;
    let state = encounter(&host, identity)?;
    require(
        state.phase == 0 && state.callbacks == 1 && after.revision > before.revision,
        "stale outer write clobbered the callback's current state/revision",
    )?;
    require(
        ok(host.observables(identity))?.summons == 1,
        "CAS rejection undid the successful prior summon",
    )?;
    require(host.depth() == 0, "CAS failure leaked a frame")?;
    let writes = host
        .trace()
        .iter()
        .filter(|entry| {
            matches!(
                entry,
                Trace::Write {
                    module: ENCOUNTER,
                    ..
                }
            )
        })
        .count();
    require(
        writes == 1,
        "obsolete outer write was published as a second successful mutation",
    )?;
    observe(&host, identity)
}

fn reverse_reentry(mode: Mode) -> CaseResult {
    // Executor selection follows composition position, whereas runtime order follows metadata.
    // Reversing descriptors makes Mixed summon originate in C Wasm and call native Policy.
    let descriptors: Vec<_> = composition::descriptors().into_iter().rev().collect();
    let mut normal = ok(composition::compose(mode, Limits::default(), &descriptors))?;
    let normal_identity = ok(normal.spawn(700, 0))?;
    let mut conflict = ok(composition::compose(mode, Limits::default(), &descriptors))?;
    let conflict_identity = ok(conflict.spawn(700, 0))?;
    let positive = summon_reentry_on(normal, normal_identity)?;
    let stale = stale_outer_on(conflict, conflict_identity)?;
    Ok(json!({ "successful": positive, "stale_outer": stale }))
}

fn fallible_action(mode: Mode) -> CaseResult {
    let (mut host, identity) = entity(mode)?;
    let before = ok(host.state(identity, ENCOUNTER))?;
    fault(
        host.dispatch_one(identity, ENCOUNTER, FAIL_AFTER_SHIELD, 0),
        Fault::ActionFailed,
    )?;
    require(
        ok(host.observables(identity))?.shield,
        "fallible action rolled back the earlier shield",
    )?;
    require(
        ok(host.state(identity, ENCOUNTER))? == before,
        "execution error continued into an unexpected state write",
    )?;
    require(host.depth() == 0, "fallible action leaked a frame")?;
    require(
        host.trace()
            .iter()
            .filter(|entry| matches!(entry, Trace::Action { .. }))
            .count()
            == 1,
        "failed action emitted a success or later action",
    )?;
    observe(&host, identity)
}

fn transfer(mode: Mode) -> CaseResult {
    let (mut host, identity) = entity(mode)?;
    ok(host.dispatch_one(identity, ENCOUNTER, event::UPDATE, 1))?;
    ok(host.dispatch_one(identity, POLICY, event::POLICY, 17))?;
    let allocation = ok(host.payload_identity(identity))?;
    let first_encounter = ok(host.state(identity, ENCOUNTER))?;
    let first_policy = ok(host.state(identity, POLICY))?;
    ok(host.detach(identity))?;
    require(
        ok(host.residence(identity))? == Residence::Detached,
        "detach did not publish detached residence",
    )?;
    require(
        ok(host.payload_identity(identity))? == allocation,
        "detach replaced the non-Clone payload",
    )?;
    require(
        ok(host.state(identity, ENCOUNTER))? == first_encounter
            && ok(host.state(identity, POLICY))? == first_policy,
        "detach copied/reset a state family",
    )?;
    let detached = observe(&host, identity)?;
    ok(host.attach(identity, 1))?;
    require(
        ok(host.residence(identity))? == Residence::Active(1),
        "attach used the wrong map",
    )?;
    require(
        ok(host.payload_identity(identity))? == allocation,
        "attach replaced the non-Clone payload",
    )?;
    require(
        ok(host.state(identity, ENCOUNTER))? == first_encounter
            && ok(host.state(identity, POLICY))? == first_policy,
        "attach lost module state/revisions",
    )?;
    Ok(json!({ "detached": detached, "attached": observe(&host, identity)? }))
}

fn failed_attach(mode: Mode) -> CaseResult {
    let (mut host, identity) = entity(mode)?;
    ok(host.detach(identity))?;
    let before = ok(host.snapshot(identity))?;
    let allocation = ok(host.payload_identity(identity))?;
    fault(host.attach(identity, 2), Fault::Invalid)?;
    require(
        ok(host.residence(identity))? == Residence::Detached,
        "failed attach lost detached residence",
    )?;
    require(
        ok(host.snapshot(identity))? == before
            && ok(host.payload_identity(identity))? == allocation,
        "failed attach changed state, revisions or allocation",
    )?;
    fault(
        host.dispatch_one(identity, ENCOUNTER, FAIL_AFTER_SHIELD, 0),
        Fault::NotActive,
    )?;
    require(
        ok(host.snapshot(identity))? == before,
        "detached spatial action changed persistent module state",
    )?;
    let detached = observe(&host, identity)?;
    ok(host.attach(identity, 1))?;
    Ok(json!({ "failed": detached, "recovered": observe(&host, identity)? }))
}

fn stale_handles(mode: Mode) -> CaseResult {
    let (mut host, old) = entity(mode)?;
    let new = ok(host.replace(old, 1))?;
    require(
        new.guid == old.guid && new.generation > old.generation,
        "replacement did not retire incarnation",
    )?;
    let before = ok(host.snapshot(new))?;
    fault(
        host.dispatch_one(old, ENCOUNTER, event::UPDATE, 1),
        Fault::Stale,
    )?;
    let forged = Handle {
        guid: new.guid + 999,
        generation: new.generation,
    };
    fault(
        host.dispatch_one(forged, ENCOUNTER, event::UPDATE, 1),
        Fault::Stale,
    )?;
    require(
        ok(host.snapshot(new))? == before,
        "stale/forged work mutated replacement",
    )?;
    require(
        host.trace().is_empty(),
        "stale/forged admission ran a callback",
    )?;
    let replacement = observe(&host, new)?;
    ok(host.retire(new))?;
    fault(host.state(new, ENCOUNTER), Fault::Stale)?;
    let newest = ok(host.spawn(new.guid, 0))?;
    require(
        newest.generation > new.generation,
        "retire/spawn reused an old incarnation",
    )?;
    Ok(json!({ "replacement": replacement, "respawned": observe(&host, newest)? }))
}

fn reset(mode: Mode) -> CaseResult {
    let (mut host, identity) = entity(mode)?;
    ok(host.dispatch_one(identity, ENCOUNTER, event::UPDATE, 1))?;
    ok(host.dispatch_one(identity, POLICY, event::POLICY, 2))?;
    let other = ok(host.state(identity, POLICY))?;
    let revision = ok(host.state(identity, ENCOUNTER))?.revision;
    ok(host.reset(identity, ENCOUNTER))?;
    let state = encounter(&host, identity)?;
    require(
        state.phase == 0 && state.callbacks == 0,
        "reset did not reset encounter state",
    )?;
    require(
        ok(host.state(identity, ENCOUNTER))?.revision > revision,
        "reset reused a stale revision",
    )?;
    require(
        ok(host.state(identity, POLICY))? == other,
        "encounter reset rewrote policy state",
    )?;
    let observed = ok(host.observables(identity))?;
    require(
        !observed.shield
            && observed
                .by_module
                .iter()
                .any(|(id, value)| *id == POLICY && value.amount == 100),
        "reset removed another module's contribution or retained its shield",
    )?;
    observe(&host, identity)
}

fn unload(mode: Mode) -> CaseResult {
    let (mut host, identity) = entity(mode)?;
    ok(host.dispatch_one(identity, ENCOUNTER, event::UPDATE, 1))?;
    ok(host.dispatch_one(identity, POLICY, event::POLICY, 2))?;
    let other = ok(host.state(identity, ENCOUNTER))?;
    ok(host.unload_module(POLICY))?;
    fault(host.state(identity, POLICY), Fault::Missing)?;
    require(
        ok(host.state(identity, ENCOUNTER))? == other,
        "unload rewrote another module's state",
    )?;
    let observed = ok(host.observables(identity))?;
    require(
        observed.shield && !observed.by_module.iter().any(|(id, _)| *id == POLICY),
        "unload removed the encounter shield or retained removed contributions",
    )?;
    fault(
        host.dispatch_one(identity, POLICY, event::POLICY, 99),
        Fault::Missing,
    )?;
    observe(&host, identity)
}

fn replay(mode: Mode) -> CaseResult {
    let (mut host, identity) = entity(mode)?;
    ok(host.dispatch_one(identity, POLICY, event::POLICY, 10))?;
    let saved = ok(host.snapshot(identity))?;
    let mut invalid = saved.clone();
    invalid.format += 1;
    fault(host.replay(identity, &invalid), Fault::Version)?;
    let mut invalid = saved.clone();
    invalid
        .modules
        .first_mut()
        .ok_or("snapshot missing module")?
        .schema += 1;
    fault(host.replay(identity, &invalid), Fault::Version)?;
    require(
        ok(host.snapshot(identity))? == saved,
        "version rejection partially replaced state",
    )?;
    ok(host.replay(identity, &saved))?;
    let after = ok(host.snapshot(identity))?;
    require(
        after.core_revision > saved.core_revision,
        "replay reused core revision",
    )?;
    require(
        after.contributions == saved.contributions,
        "replay changed declared contributions",
    )?;
    for old in &saved.modules {
        let new = after
            .modules
            .iter()
            .find(|module| module.id == old.id)
            .ok_or("replay lost module")?;
        require(
            new.bytes == old.bytes && new.revision > old.revision,
            "replay lost bytes or reused module revision",
        )?;
    }
    fault(host.replay(identity, &saved), Fault::Revision)?;
    require(
        ok(host.snapshot(identity))? == after,
        "reused snapshot overwrote newer replay state",
    )?;
    Ok(
        json!({ "oracle": observe(&host, identity)?, "evidence": "same-incarnation in-memory replay; not database durability" }),
    )
}

fn stale_snapshot(mode: Mode) -> CaseResult {
    let (mut host, identity) = entity(mode)?;
    let saved = ok(host.snapshot(identity))?;
    ok(host.dispatch_one(identity, POLICY, event::POLICY, 10))?;
    let current = ok(host.snapshot(identity))?;
    fault(host.replay(identity, &saved), Fault::Revision)?;
    require(
        ok(host.snapshot(identity))? == current,
        "late checkpoint erased newer module/core state",
    )?;
    let module_mutation = observe(&host, identity)?;

    let (mut core_only, other) = entity(mode)?;
    let saved = ok(core_only.snapshot(other))?;
    fault(
        core_only.dispatch_one(other, ENCOUNTER, FAIL_AFTER_SHIELD, 0),
        Fault::ActionFailed,
    )?;
    let changed = ok(core_only.snapshot(other))?;
    require(
        changed.modules == saved.modules && changed.core_revision > saved.core_revision,
        "core-only rejection fixture did not isolate a newer effect",
    )?;
    fault(core_only.replay(other, &saved), Fault::Revision)?;
    require(
        ok(core_only.snapshot(other))? == changed,
        "replay ignored the core revision and undid an applied effect",
    )?;
    Ok(json!({ "module_mutation": module_mutation, "core_only": observe(&core_only, other)? }))
}

fn executor_switch(mode: Mode) -> CaseResult {
    let (mut host, identity) = entity(mode)?;
    let current = ok(host.executor(ENCOUNTER))?;
    let unsupported = match current {
        Executor::Native => Executor::Wasm,
        Executor::Wasm => Executor::Native,
    };
    let before = ok(host.snapshot(identity))?;
    fault(host.switch_executor(ENCOUNTER, unsupported), Fault::Version)?;
    require(
        ok(host.snapshot(identity))? == before && ok(host.executor(ENCOUNTER))? == current,
        "unsupported executor switch changed identity, state or provenance",
    )?;
    // Canonical equality deliberately excludes the mode-specific requested executor.
    observe(&host, identity)
}

fn limited(mode: Mode, limits: Limits, event: u32, argument: i64) -> CaseResult<(Harness, Handle)> {
    let mut host = ok(composition::build_with_limits(mode, limits))?;
    let identity = ok(host.spawn(700, 0))?;
    fault(
        host.dispatch_one(identity, ENCOUNTER, event, argument),
        Fault::Limit,
    )?;
    require(
        host.depth() == 0,
        "resource error leaked an invocation frame",
    )?;
    Ok((host, identity))
}

fn host_calls(mode: Mode) -> CaseResult {
    let (host, identity) = limited(
        mode,
        Limits {
            calls: 6,
            ..Limits::default()
        },
        event::UPDATE,
        1,
    )?;
    let state = encounter(&host, identity)?;
    let observed = ok(host.observables(identity))?;
    require(
        host.calls() == 6
            && state.phase == 1
            && state.callbacks == 0
            && observed.shield
            && observed.summons == 1,
        "host-call budget was refilled on reentry or prior effects were rolled back",
    )?;
    require(
        matches!(host.trace(), [.., Trace::Enter(entered),
            Trace::Leave { invocation: callback, result: Err(Fault::Limit) },
            Trace::Leave { invocation: root, result: Err(Fault::Limit) }]
            if entered == callback && callback.module == ENCOUNTER
                && callback.event == event::CALLBACK && root.event == event::UPDATE),
        "call-budget rejection did not occur inside the nested callback",
    )?;
    // Positive control has identical inputs; only the configured resource ceiling differs.
    let (mut control, control_identity) = entity(mode)?;
    ok(control.dispatch_one(control_identity, ENCOUNTER, event::UPDATE, 1))?;
    require(
        encounter(&control, control_identity)?.callbacks == 1,
        "positive host-call control did not complete callback",
    )?;
    Ok(
        json!({ "limited": observe(&host, identity)?, "control": observe(&control, control_identity)? }),
    )
}

fn depth(mode: Mode) -> CaseResult {
    let (host, identity) = limited(
        mode,
        Limits {
            depth: 2,
            ..Limits::default()
        },
        event::CALLBACK,
        2,
    )?;
    require(
        encounter(&host, identity)?.callbacks == 2,
        "depth limit did not preserve exactly two completed callback writes",
    )?;
    let entries = host.trace().iter().filter(|entry| matches!(entry,
        Trace::Enter(invocation) if invocation.module == ENCOUNTER && invocation.event == event::CALLBACK)).count();
    require(
        entries == 2,
        "depth cap admitted too many/few callback frames",
    )?;
    let (mut control, control_identity) = entity(mode)?;
    ok(control.dispatch_one(control_identity, ENCOUNTER, event::CALLBACK, 2))?;
    require(
        encounter(&control, control_identity)?.callbacks == 3,
        "depth positive control failed to finish",
    )?;
    Ok(
        json!({ "limited": observe(&host, identity)?, "control": observe(&control, control_identity)? }),
    )
}

fn output_limit(mode: Mode) -> CaseResult {
    let (host, identity) = limited(
        mode,
        Limits {
            trace: 5,
            ..Limits::default()
        },
        event::UPDATE,
        1,
    )?;
    let state = encounter(&host, identity)?;
    let observed = ok(host.observables(identity))?;
    require(
        host.trace().len() == 5
            && state.phase == 1
            && state.callbacks == 0
            && observed.shield
            && observed.summons == 1,
        "trace rejection dropped prior effects or continued into a callback mutation",
    )?;
    require(
        matches!(host.trace(), [.., Trace::Action { action: Action::Summon, .. },
            Trace::Leave { invocation, result: Err(Fault::Limit) }]
            if invocation.event == event::UPDATE),
        "output cap did not reject precisely before callback entry",
    )?;
    let (mut control, control_identity) = entity(mode)?;
    ok(control.dispatch_one(control_identity, ENCOUNTER, event::UPDATE, 1))?;
    require(
        control.trace().len() > 5,
        "positive trace control did not cross the reduced cap",
    )?;
    Ok(
        json!({ "limited": observe(&host, identity)?, "control": observe(&control, control_identity)? }),
    )
}

pub(super) fn run(mode: Mode) -> Vec<Check> {
    vec![
        checked("registration_order", || registration_order(mode)),
        checked("zero_optional_neutrality", || zero_optional(mode)),
        checked("policy_and_state_isolation", || policy_isolation(mode)),
        checked("summon_reentry_order", || summon_reentry(mode)),
        checked("mixed_reverse_reentry", || reverse_reentry(mode)),
        checked("callback_result_oracle", || callback_result_oracle(mode)),
        checked("nullable_summon_partial_effects", || nullable_summon(mode)),
        checked("stale_outer_write", || stale_outer(mode)),
        checked("fallible_action_partial_effects", || fallible_action(mode)),
        checked("active_detached_transfer", || transfer(mode)),
        checked("failed_attach_retains_state", || failed_attach(mode)),
        checked("stale_and_forged_handles", || stale_handles(mode)),
        checked("reset_scoped_contribution", || reset(mode)),
        checked("unload_preserves_other_module", || unload(mode)),
        checked("versioned_snapshot_replay", || replay(mode)),
        checked("stale_snapshot_rejected", || stale_snapshot(mode)),
        checked("unsupported_executor_switch", || executor_switch(mode)),
        checked("cumulative_host_calls", || host_calls(mode)),
        checked("bounded_reentry_depth", || depth(mode)),
        checked("output_limit_partial_effects", || output_limit(mode)),
    ]
}
