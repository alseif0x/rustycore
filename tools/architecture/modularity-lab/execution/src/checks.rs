use crate::{
    logic,
    model::{Entry, Native},
    wasm::{Compiled, FUEL, Wasm},
};
use serde::Serialize;
use wasmtime::{Result, ensure};

#[derive(Serialize)]
pub struct Check {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

fn check(name: &'static str, test: impl FnOnce() -> Result<()>) -> Check {
    match test() {
        Ok(()) => Check {
            name,
            passed: true,
            detail: "passed (isolated synthetic host, not live server/DB)".into(),
        },
        Err(error) => Check {
            name,
            passed: false,
            detail: format!("{error:#}"),
        },
    }
}

fn rejection<T>(result: Result<T>, expected: &str) -> Result<wasmtime::Error> {
    result
        .err()
        .ok_or_else(|| wasmtime::format_err!("expected rejection: {expected}"))
}

pub fn memory_limit(wasm: &mut Wasm) -> Result<()> {
    let memory = wasm
        .instance
        .get_memory(&mut wasm.store, "memory")
        .ok_or_else(|| wasmtime::format_err!("missing guest memory"))?;
    let grow = wasm
        .instance
        .get_typed_func::<u32, u32>(&mut wasm.store, "probe_grow")?;
    let initial = memory.data_size(&wasm.store);
    ensure!(
        grow.call(&mut wasm.store, 1)? == (initial / 65536) as u32,
        "positive memory growth failed"
    );
    let before = memory.data_size(&wasm.store);
    ensure!(
        before == initial + 65536,
        "positive memory growth size mismatch"
    );
    let requested_pages = (before / 65536) as u64 + 64;
    ensure!(
        requested_pages < 65536
            && memory
                .ty(&wasm.store)
                .maximum()
                .is_none_or(|max| max >= requested_pages),
        "probe must be valid without the host memory cap"
    );
    ensure!(
        grow.call(&mut wasm.store, 64)? == u32::MAX,
        "memory cap allowed growth"
    );
    ensure!(
        memory.data_size(&wasm.store) == before,
        "rejected growth changed memory size"
    );
    Ok(())
}

pub fn nested_fuel_limit(wasm: &mut Wasm) -> Result<()> {
    let burn = wasm
        .instance
        .get_typed_func::<u32, u64>(&mut wasm.store, "probe_burn")?;
    wasm.store.set_fuel(FUEL)?;
    burn.call(&mut wasm.store, 256)?;
    let one_burn = FUEL - wasm.store.get_fuel()?;
    ensure!(
        one_burn > 100 && one_burn < FUEL / 4,
        "invalid finite fuel calibration"
    );
    // Enough for either finite half, insufficient for both. No time/CPU claim is made.
    wasm.store.set_fuel(one_burn + one_burn / 2)?;
    let nested = wasm
        .instance
        .get_typed_func::<u32, u64>(&mut wasm.store, "probe_nested_burn")?;
    let error = rejection(nested.call(&mut wasm.store, 256), "nested fuel must trap")?;
    ensure!(
        format!("{error:#}").contains("fuel"),
        "nested call failed for something other than fuel: {error:#}"
    );
    ensure!(
        wasm.store.data().aggregate.depth == 0,
        "fuel unwind leaked callback depth"
    );
    ensure!(
        wasm.store.data().aggregate.hostcall_attempts == 1,
        "test reached multiple callbacks/depth cap"
    );
    ensure!(
        wasm.store.data().aggregate.trace == vec![Entry(logic::BURN_PROBE, logic::HANDLE, 256)],
        "callback completed before fuel exhaustion (unexpected callback_finished event)"
    );
    // Only after the transition has failed, replenish enough fuel to inspect the marker.
    wasm.store.set_fuel(1000)?;
    let stage = wasm
        .instance
        .get_typed_func::<(), u32>(&mut wasm.store, "probe_stage")?
        .call(&mut wasm.store, ())?;
    ensure!(
        stage == 2,
        "fuel was not exhausted inside the reentrant guest callback"
    );
    Ok(())
}

pub fn all(compiled: &Compiled, compiled_v2: &Compiled) -> Vec<Check> {
    let mut checks = vec![
        check("actual_rust_guest_native_full_trace_equality", || {
            let mut native = Native::new();
            let mut wasm = Wasm::new(compiled)?;
            native.percent = 125;
            wasm.configure(1, 125)?;
            for (event, argument) in [
                (logic::XP, 123),
                (logic::XP, -1),
                (logic::XP, 1_000_001),
                (logic::START, 0),
                (logic::RESET, 0),
                (logic::START, 1),
                (logic::REWARD, 42),
                (logic::REWARD, 42),
                (logic::RESET, 0),
            ] {
                ensure!(
                    native.invoke(event, argument)? == wasm.invoke(event, argument)?,
                    "return mismatch"
                );
                ensure!(native.state.0 == wasm.snapshot()?, "state mismatch");
                let actual = &wasm.store.data().aggregate;
                ensure!(
                    native.aggregate.trace == actual.trace,
                    "operation trace mismatch"
                );
                ensure!(
                    native.aggregate.durable == actual.durable,
                    "receipt/money mismatch"
                );
                ensure!(
                    native.aggregate.shield == actual.shield
                        && native.aggregate.summons == actual.summons,
                    "canonical host aggregate mismatch"
                );
            }
            Ok(())
        }),
        check(
            "phase_shield_before_action_same_guest_reentry_read_after_action",
            || {
                let mut wasm = Wasm::new(compiled)?;
                ensure!(
                    wasm.invoke(logic::START, 0)? == 1,
                    "action not immediately visible"
                );
                let h = logic::HANDLE;
                ensure!(
                    wasm.store.data().aggregate.trace
                        == vec![
                            Entry(logic::SHIELD, h, 1),
                            Entry(logic::SUMMON, h, 0),
                            Entry(logic::OBSERVE, h, 257),
                            Entry(100, h, 110),
                            Entry(logic::READ_SUMMONS, h, 0),
                            Entry(logic::OBSERVE, h, 65793)
                        ],
                    "wrong observable callback order"
                );
                ensure!(
                    logic::State(wasm.snapshot()?).callbacks() == 1,
                    "outer call overwrote nested state"
                );
                ensure!(
                    wasm.store.data().aggregate.shield,
                    "canonical shield not applied before callback"
                );
                Ok(())
            },
        ),
        check("fallible_summon_preserves_prior_effect_then_reset", || {
            let mut wasm = Wasm::new(compiled)?;
            ensure!(wasm.invoke(logic::START, 1)? == -3, "summon should fail");
            ensure!(
                wasm.snapshot()? == 257 && wasm.store.data().aggregate.shield,
                "prior effects rolled back"
            );
            ensure!(
                wasm.store.data().aggregate.summons == 0,
                "failed summon created entity"
            );
            wasm.invoke(logic::RESET, 0)?;
            ensure!(
                wasm.snapshot()? == 0 && !wasm.store.data().aggregate.shield,
                "reset incomplete"
            );
            Ok(())
        }),
        check("fuel_interrupts_actual_guest_infinite_loop", || {
            let mut wasm = Wasm::new(compiled)?;
            wasm.store.set_fuel(1000)?;
            let spin = wasm
                .instance
                .get_typed_func::<(), ()>(&mut wasm.store, "probe_spin")?;
            let error = rejection(spin.call(&mut wasm.store, ()), "spin must trap")?;
            ensure!(
                format!("{error:#}").contains("fuel"),
                "wrong trap: {error:#}"
            );
            Ok(())
        }),
        check("memory_growth_limit", || {
            let mut wasm = Wasm::new(compiled)?;
            memory_limit(&mut wasm)
        }),
        check("nested_fuel_exhaustion_before_depth_cap", || {
            let mut wasm = Wasm::new(compiled)?;
            nested_fuel_limit(&mut wasm)
        }),
        check(
            "native_wasm_callback_failure_stops_before_followup_mutation",
            || {
                let mut native = Native::new();
                let mut wasm = Wasm::new(compiled)?;
                native.aggregate.remaining_outputs = 2;
                wasm.store.data_mut().aggregate.remaining_outputs = 2;
                let invoke = wasm
                    .instance
                    .get_typed_func::<(u32, i64), i64>(&mut wasm.store, "invoke")?;
                let native_error = rejection(
                    native.invoke_current_budget(logic::START, 0),
                    "native callback must fail",
                )?;
                let wasm_error = rejection(
                    invoke.call(&mut wasm.store, (logic::START, 0)),
                    "Wasm callback must fail",
                )?;
                ensure!(
                    format!("{native_error:#}").contains("output budget")
                        && format!("{wasm_error:#}").contains("output budget"),
                    "unexpected callback failure cause"
                );
                ensure!(
                    native.state.0 == 257 && wasm.snapshot()? == 257,
                    "mutation continued after failed callback"
                );
                let actual = &wasm.store.data().aggregate;
                ensure!(
                    actual.trace
                        == vec![
                            Entry(logic::SHIELD, logic::HANDLE, 1),
                            Entry(logic::SUMMON, logic::HANDLE, 0)
                        ]
                        && native.aggregate.trace == actual.trace,
                    "followup actions ran after failed callback"
                );
                ensure!(
                    actual.observables(257) == native.aggregate.observables(257),
                    "partial outcomes differ"
                );
                ensure!(
                    actual.shield
                        && actual.summons == 1
                        && actual.hostcall_attempts == 3
                        && actual.depth == 0
                        && native.aggregate.depth == 0,
                    "partial outcome or callback unwind wrong"
                );
                Ok(())
            },
        ),
    ];

    for (name, pointer, length) in [
        ("oversize_payload_rejected_before_allocation", 0, 1025),
        ("out_of_bounds_payload_rejected", u32::MAX, 16),
    ] {
        checks.push(check(name, || {
            let mut wasm = Wasm::new(compiled)?;
            let probe = wasm
                .instance
                .get_typed_func::<(u32, u32), i64>(&mut wasm.store, "probe_payload")?;
            ensure!(
                probe.call(&mut wasm.store, (pointer, length)).is_err(),
                "invalid payload accepted"
            );
            Ok(())
        }));
    }
    for (name, op, handle) in [
        ("forged_handle_rejected", logic::SHIELD, 0),
        (
            "stale_handle_rejected",
            logic::SHIELD,
            logic::HANDLE - (1_u64 << 32),
        ),
        ("unauthorized_action_rejected", 999, logic::HANDLE),
    ] {
        checks.push(check(name, || {
            let mut wasm = Wasm::new(compiled)?;
            let probe = wasm
                .instance
                .get_typed_func::<(u32, u64, i64), i64>(&mut wasm.store, "probe_action")?;
            ensure!(
                probe.call(&mut wasm.store, (op, handle, 1)).is_err(),
                "unauthorized effect accepted"
            );
            ensure!(
                !wasm.store.data().aggregate.shield,
                "rejected request mutated canonical state"
            );
            Ok(())
        }));
    }
    checks.extend([
        check("cumulative_hostcall_budget", || {
            let mut wasm = Wasm::new(compiled)?;
            wasm.store.data_mut().aggregate.remaining_calls = 8;
            let spam = wasm
                .instance
                .get_typed_func::<u32, ()>(&mut wasm.store, "probe_spam")?;
            let error = rejection(spam.call(&mut wasm.store, 100), "spam must trap")?;
            ensure!(
                format!("{error:#}").contains("host-call budget"),
                "wrong trap"
            );
            ensure!(
                wasm.store.data().aggregate.trace.len() == 8,
                "unbounded output"
            );
            Ok(())
        }),
        check("output_cap_does_not_rollback_prior_effect", || {
            let mut wasm = Wasm::new(compiled)?;
            wasm.store.data_mut().aggregate.remaining_outputs = 1;
            let invoke = wasm
                .instance
                .get_typed_func::<(u32, i64), i64>(&mut wasm.store, "invoke")?;
            let error = rejection(
                invoke.call(&mut wasm.store, (logic::START, 0)),
                "output cap must trap",
            )?;
            ensure!(format!("{error:#}").contains("output budget"), "wrong trap");
            ensure!(
                wasm.store.data().aggregate.shield && wasm.store.data().aggregate.summons == 0,
                "partial effects not retained exactly"
            );
            Ok(())
        }),
        check("callback_depth_cap", || {
            let mut wasm = Wasm::new(compiled)?;
            let recurse = wasm
                .instance
                .get_typed_func::<u32, i64>(&mut wasm.store, "probe_recurse")?;
            let error = rejection(recurse.call(&mut wasm.store, 100), "depth must trap")?;
            ensure!(
                format!("{error:#}").contains("callback depth"),
                "wrong trap"
            );
            ensure!(
                wasm.store.data().aggregate.depth == 0,
                "unwinding leaked depth"
            );
            Ok(())
        }),
        check("trap_after_reward_does_not_undo_or_replay_effect", || {
            let mut wasm = Wasm::new(compiled)?;
            let mut native = Native::new();
            ensure!(
                wasm.invoke(logic::TRAP_AFTER_REWARD, 42).is_err(),
                "guest did not trap"
            );
            ensure!(
                native.invoke(logic::TRAP_AFTER_REWARD, 42).is_err(),
                "native did not fail"
            );
            ensure!(
                wasm.snapshot()? == 0,
                "receipt acknowledgement unexpectedly installed"
            );
            ensure!(
                wasm.store.data().aggregate.durable.money == 100,
                "host effect rolled back"
            );
            ensure!(wasm.invoke(logic::REWARD, 42)? == 0, "reward replayed");
            native.invoke(logic::REWARD, 42)?;
            ensure!(
                native.aggregate.durable == wasm.store.data().aggregate.durable,
                "mock outcome mismatch"
            );
            ensure!(
                native.aggregate.trace == wasm.store.data().aggregate.trace,
                "partial trace mismatch"
            );
            Ok(())
        }),
        check(
            "actual_v2_binary_migrates_state_and_retains_mock_receipt_idempotency",
            || {
                let mut old = Wasm::new(compiled)?;
                old.invoke(logic::REWARD, 42)?;
                let snapshot = old.snapshot()?;
                ensure!(
                    old.invoke(logic::TRAP_AFTER_REWARD, 43).is_err(),
                    "missing trap"
                );
                let durable = old.store.data().aggregate.durable.clone();
                drop(old);
                let mut replacement = Wasm::new(compiled_v2)?;
                let revision = replacement
                    .instance
                    .get_typed_func::<(), u32>(&mut replacement.store, "module_revision")?
                    .call(&mut replacement.store, ())?;
                ensure!(revision == 2, "actual v2 guest binary is required");
                replacement.store.data_mut().aggregate.durable = durable;
                replacement.configure(2, 150)?;
                ensure!(
                    replacement.restore(99, snapshot)? == -1,
                    "unsupported state schema accepted"
                );
                ensure!(
                    replacement.restore(1, snapshot)? == 0,
                    "state restore rejected"
                );
                ensure!(
                    replacement.snapshot()? == snapshot | (1 << 32),
                    "schema1 migration failed to preserve original state and initialize reset_epoch"
                );
                ensure!(
                    replacement.invoke(logic::XP, 100)? == 150,
                    "new config did not activate"
                );
                ensure!(
                    replacement.invoke(logic::REWARD, 43)? == 0,
                    "lost acknowledgement replayed reward"
                );
                ensure!(
                    replacement.store.data().aggregate.durable.money == 200,
                    "mock state lost"
                );
                replacement.invoke(logic::START, 0)?;
                ensure!(
                    replacement.snapshot()? >> 32 == 1,
                    "START/reentry dropped migrated reset_epoch"
                );
                replacement.invoke(logic::RESET, 0)?;
                let migrated = replacement.snapshot()?;
                ensure!(
                    migrated == 2 << 32,
                    "v2 reset did not advance new state field"
                );
                let mut second_restart = Wasm::new(compiled_v2)?;
                ensure!(
                    second_restart.restore(2, migrated)? == 0
                        && second_restart.snapshot()? == migrated,
                    "schema2 state failed roundtrip"
                );
                let mut v1 = Wasm::new(compiled)?;
                ensure!(
                    v1.restore(2, migrated)? == -1,
                    "unsupported downgrade accepted"
                );
                let edge = (5_u64 << 32) | (65535 << 16) | 257;
                ensure!(
                    second_restart.restore(2, edge)? == 0,
                    "valid edge state rejected"
                );
                second_restart.invoke(logic::CALLBACK, 0)?;
                ensure!(
                    second_restart.snapshot()? == (5_u64 << 32) | 257,
                    "callback counter carried into reset_epoch"
                );
                let exhausted = u64::from(u32::MAX) << 32;
                second_restart.restore(2, exhausted)?;
                ensure!(
                    second_restart.invoke(logic::RESET, 0)? == -2
                        && second_restart.snapshot()? == exhausted,
                    "reset_epoch overflow was not rejected without mutation"
                );
                // Actual different binary and state migration; still NOT a DB/crash/hot-reload test.
                Ok(())
            },
        ),
        check("invalid_configuration_is_rejected", || {
            let mut wasm = Wasm::new(compiled)?;
            ensure!(
                wasm.configure(99, 100).is_err(),
                "unknown config revision accepted"
            );
            ensure!(
                wasm.configure(2, 1001).is_err(),
                "unbounded multiplier accepted"
            );
            Ok(())
        }),
    ]);
    checks
}
