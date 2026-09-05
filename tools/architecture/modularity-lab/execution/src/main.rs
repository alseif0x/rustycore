mod bench;
mod checks;
#[path = "../shared/logic.rs"]
mod logic;
mod model;
mod wasm;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::PathBuf, process::ExitCode};
use wasmtime::{Result, ensure};

#[derive(Debug)]
struct Options {
    command: String,
    guest: PathBuf,
    guest_v2: PathBuf,
    backend: String,
    calls: usize,
    seed: u64,
}

fn parse(args: Vec<String>) -> Result<Options> {
    let mut args = args.into_iter();
    let command = args
        .next()
        .ok_or_else(|| wasmtime::format_err!("expected check or bench"))?;
    ensure!(
        command == "check" || command == "bench",
        "expected check or bench"
    );
    let mut flags = BTreeMap::new();
    while let Some(flag) = args.next() {
        ensure!(
            ["--guest", "--guest-v2", "--backend", "--calls", "--seed"].contains(&flag.as_str()),
            "unknown flag {flag}"
        );
        let value = args
            .next()
            .ok_or_else(|| wasmtime::format_err!("missing value for {flag}"))?;
        ensure!(
            flags.insert(flag.clone(), value).is_none(),
            "duplicate flag {flag}"
        );
    }
    let guest = PathBuf::from(
        flags
            .remove("--guest")
            .ok_or_else(|| wasmtime::format_err!("--guest PATH required"))?,
    );
    let guest_v2 = flags
        .remove("--guest-v2")
        .map(PathBuf::from)
        .unwrap_or_else(default_v2_guest);
    if command == "check" {
        ensure!(
            flags.is_empty(),
            "check accepts only --guest and --guest-v2"
        );
    }
    let backend = flags.remove("--backend").unwrap_or_else(|| "native".into());
    ensure!(
        backend == "native" || backend == "wasm",
        "backend must be native or wasm"
    );
    let calls = flags
        .remove("--calls")
        .unwrap_or_else(|| "10000".into())
        .parse::<usize>()?;
    ensure!(
        (1..=1_000_000).contains(&calls),
        "calls must be 1..=1000000"
    );
    let seed = flags
        .remove("--seed")
        .unwrap_or_else(|| "42".into())
        .parse::<u64>()?;
    Ok(Options {
        command,
        guest,
        guest_v2,
        backend,
        calls,
        seed,
    })
}

fn default_v2_guest() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("guest/target-v2/wasm32-unknown-unknown/release/execution_lab_guest.wasm")
}

fn run(options: Options) -> Result<Value> {
    let guest_bytes = std::fs::read(&options.guest)?;
    ensure!(
        guest_bytes.starts_with(b"\0asm"),
        "--guest requires compiled core Wasm"
    );
    let mut report = if options.command == "check" {
        let compiled = wasm::Compiled::load(&options.guest)?;
        let compiled_v2 = wasm::Compiled::load(&options.guest_v2)?;
        let v2_bytes = std::fs::read(&options.guest_v2)?;
        ensure!(
            guest_bytes != v2_bytes,
            "v2 must be a different compiled guest binary"
        );
        let checks = checks::all(&compiled, &compiled_v2);
        json!({"command":"check","success":checks.iter().all(|c|c.passed),"checks":checks,
            "cold_compile_ns":compiled.cold_compile_ns,"guest_v2":options.guest_v2,
            "guest_v2_sha256":format!("{:x}",Sha256::digest(&v2_bytes))})
    } else {
        let mut report = bench::run(
            &options.backend,
            &options.guest,
            options.calls,
            options.seed,
        )?;
        report["command"] = json!("bench");
        report["success"] = json!(true);
        report
    };
    report["schema_version"] = json!(1);
    report["experiment"] = json!("core-wasm-native-v1");
    report["engine"] = json!({"name":"wasmtime","version":wasm::ENGINE_VERSION,"abi":"core-wasm-v1 (NOT Component Model)",
        "fuel_per_transition":wasm::FUEL,"memory_limit_bytes":wasm::MEMORY_LIMIT,
        "payload_limit_bytes":wasm::PAYLOAD_LIMIT,"hostcall_budget":64,"output_budget":64,"callback_depth_limit":wasm::DEPTH_LIMIT});
    report["architecture"] = json!(std::env::consts::ARCH);
    report["guest_sha256"] = json!(format!("{:x}", Sha256::digest(&guest_bytes)));
    report["guest"] = json!(options.guest);
    report["limitations"] = json!([
        "Synthetic fixed host aggregate; no hecs axis or production server.",
        "Mock receipt/money state is not MariaDB/crash/unknown-COMMIT proof.",
        "V2 is a different guest binary with a bounded synthetic state migration; no in-flight hot reload or production upgrade proof.",
        "Core-Wasm reentry does not establish Component Model/WIT compatibility.",
        "Native trusted code has no fuel/memory sandbox; host contract limits are not process isolation."
    ]);
    Ok(report)
}

fn main() -> ExitCode {
    let report = parse(std::env::args().skip(1).collect()).and_then(run)
        .unwrap_or_else(|error|json!({"schema_version":1,"experiment":"core-wasm-native-v1","success":false,"error":format!("{error:#}")}));
    let success = report["success"].as_bool() == Some(true);
    println!(
        "{}",
        serde_json::to_string(&report).expect("JSON report is serializable")
    );
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

#[cfg(test)]
mod tests;
