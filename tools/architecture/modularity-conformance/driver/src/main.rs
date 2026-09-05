//! Single-mode executable results. The campaign runner owns matrix/freeze acceptance.
use conformance_driver::{bench, checks, composition::Mode};
use serde_json::json;

fn run() -> Result<bool, String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [command] if command == "list" => {
            println!("{}", json!({
                "schema_version": 2,
                "common_cases": checks::COMMON_CASES,
                "native_only_cases": checks::NATIVE_ONLY_CASES,
                "modes": Mode::ALL.map(Mode::as_str),
                "wasm_enabled": cfg!(feature = "wasm"),
            }));
            Ok(true)
        }
        [command, mode] if command == "checks" => {
            let mode: Mode = mode.parse()?;
            let results = checks::run(mode);
            let passed = !results.is_empty() && results.iter().all(|case| case.passed);
            println!("{}", json!({
                "schema_version": 2,
                "kind": "mode-contract-checks",
                "mode": mode.as_str(),
                "passed": passed,
                "checks": results,
                "scope": "single-mode fixtures only; not complete conformance or production acceptance",
            }));
            Ok(passed)
        }
        [command, mode, workload, calls, seed] if command == "dispatch" => {
            let result = bench::dispatch(mode.parse()?, workload,
                calls.parse().map_err(|_| "invalid calls")?, seed.parse().map_err(|_| "invalid seed")?)?;
            println!("{result}");
            Ok(true)
        }
        [command, mode, population, density, ticks, seed] if command == "storage" => {
            let result = bench::storage(mode.parse()?, population.parse().map_err(|_| "invalid population")?,
                density, ticks.parse().map_err(|_| "invalid ticks")?, seed.parse().map_err(|_| "invalid seed")?)?;
            println!("{result}");
            Ok(true)
        }
        _ => Err("usage: conformance-driver list | checks MODE | dispatch MODE WORKLOAD CALLS SEED | storage MODE POPULATION DENSITY TICKS SEED".into()),
    }
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => std::process::exit(1),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(2);
        }
    }
}
