use crate::{
    logic,
    model::Native,
    wasm::{Compiled, Wasm},
};
use serde_json::{Value, json};
use std::{hint::black_box, path::Path, time::Instant};
use wasmtime::{Result, ensure};

pub const WARMUP: usize = 256;

pub fn rss(key: &str) -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find(|line| line.starts_with(key))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

fn workload(seed: &mut u64) -> (u32, i64) {
    *seed = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    match (*seed >> 32) & 15 {
        12 => (logic::START, 0),
        13 => (logic::START, 1),
        14 => (logic::RESET, 0),
        15 => (logic::REWARD, 42),
        _ => (logic::XP, (*seed % 100_000) as i64),
    }
}

struct Measurement {
    samples: Vec<u64>,
    total_ns: u128,
    checksum: u64,
    counts: [u64; 5],
}

fn measure(
    calls: usize,
    mut seed: u64,
    mut invoke: impl FnMut(u32, i64) -> Result<i64>,
) -> Result<Measurement> {
    for _ in 0..WARMUP {
        let (event, argument) = workload(&mut seed);
        black_box(invoke(event, argument)?);
    }
    let mut durations = Vec::with_capacity(calls);
    let mut checksum = 0xcbf29ce484222325_u64;
    let mut counts = [0; 5];
    let start = Instant::now();
    for _ in 0..calls {
        let (event, argument) = workload(&mut seed);
        let call_start = Instant::now();
        let result = black_box(invoke(black_box(event), black_box(argument))?);
        durations.push(call_start.elapsed().as_nanos() as u64);
        let index = match event {
            logic::XP => 0,
            logic::START if argument == 0 => 1,
            logic::START => 2,
            logic::RESET => 3,
            _ => 4,
        };
        counts[index] += 1;
        checksum = (checksum ^ result as u64).wrapping_mul(0x100000001b3);
    }
    Ok(Measurement {
        samples: durations,
        total_ns: start.elapsed().as_nanos(),
        checksum,
        counts,
    })
}

pub fn run(backend: &str, guest: &Path, calls: usize, seed: u64) -> Result<Value> {
    ensure!(
        (1..=1_000_000).contains(&calls),
        "calls must be 1..=1000000"
    );
    let pre_instance_rss_kib = rss("VmRSS:");
    let (mut measured, cold_compile_ns, instantiate_ns, rss_kib, rss_hwm_kib, final_observables) =
        match backend {
            "native" => {
                let mut native = Native::new();
                native.percent = 125;
                native.aggregate.record_trace = false;
                let mut measured = measure(calls, seed, |e, a| native.invoke(e, a))?;
                measured.checksum ^= native.aggregate.checksum ^ native.state.0;
                (
                    measured,
                    0,
                    0,
                    rss("VmRSS:"),
                    rss("VmHWM:"),
                    native.aggregate.observables(native.state.0),
                )
            }
            "wasm" => {
                let compiled = Compiled::load(guest)?;
                let mut wasm = Wasm::new(&compiled)?;
                wasm.configure(1, 125)?;
                wasm.store.data_mut().aggregate.record_trace = false;
                let mut measured = measure(calls, seed, |e, a| wasm.invoke(e, a))?;
                let state = wasm.snapshot()?;
                measured.checksum ^= wasm.store.data().aggregate.checksum ^ state;
                (
                    measured,
                    compiled.cold_compile_ns,
                    wasm.instantiate_ns,
                    rss("VmRSS:"),
                    rss("VmHWM:"),
                    wasm.store.data().aggregate.observables(state),
                )
            }
            _ => wasmtime::bail!("backend must be native or wasm"),
        };
    measured.samples.sort_unstable();
    let quantile =
        |percent: usize| measured.samples[(calls * percent).div_ceil(100).saturating_sub(1)];
    Ok(
        json!({"backend":backend,"calls":calls,"seed":seed,"warmup_calls":WARMUP,
        "total_ns":measured.total_ns,"p50_ns":quantile(50),"p95_ns":quantile(95),"p99_ns":quantile(99),
        "checksum":format!("{:016x}",measured.checksum),"cold_compile_ns":cold_compile_ns,
        "instantiate_ns":instantiate_ns,"rss_kib":rss_kib,"rss_hwm_kib":rss_hwm_kib,
        "postdrop_retained_rss_kib":rss("VmRSS:"),"final_observables":final_observables,
        "calls_by_event":{"xp":measured.counts[0],"summon_success":measured.counts[1],
            "summon_failure":measured.counts[2],"reset":measured.counts[3],"reward":measured.counts[4]},
        "pre_instance_rss_kib":pre_instance_rss_kib,
        "timing_scope":"warm invocation including per-transition budget reset; total also includes workload/timer/checksum bookkeeping",
        "workload":"seeded mix: 12/16 XP, 1/16 summon+reentry, 1/16 failed summon, 1/16 reset, 1/16 idempotent mock reward"}),
    )
}
