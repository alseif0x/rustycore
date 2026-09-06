use super::*;

fn compiled_guest() -> wasm::Compiled {
    let guest = std::env::var_os("EXECUTION_LAB_GUEST")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("guest/target/wasm32-unknown-unknown/release/execution_lab_guest.wasm")
        });
    wasm::Compiled::load(&guest).expect("build the real guest first")
}

#[test]
fn safety_checks_reject_missing_memory_cap_and_nested_fuel_refill_mutants() {
    let compiled = compiled_guest();
    let mut no_cap = wasm::Wasm::without_memory_cap(&compiled).unwrap();
    let error =
        checks::memory_limit(&mut no_cap).expect_err("memory-cap mutant must fail the check");
    assert!(format!("{error:#}").contains("memory cap allowed growth"));
    let mut refilling = wasm::Wasm::new(&compiled).unwrap();
    refilling.store.data_mut().refill_reentry_fuel = true;
    let error = checks::nested_fuel_limit(&mut refilling)
        .expect_err("fuel-refill mutant must fail the check");
    assert!(format!("{error:#}").contains("nested fuel must trap"));
}

#[test]
fn real_guest_contract_suite() {
    let guest = std::env::var_os("EXECUTION_LAB_GUEST")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("guest/target/wasm32-unknown-unknown/release/execution_lab_guest.wasm")
        });
    let compiled = wasm::Compiled::load(&guest)
        .expect("build the real Rust guest first; never skip this test");
    let v2 = std::env::var_os("EXECUTION_LAB_GUEST_V2")
        .map(PathBuf::from)
        .unwrap_or_else(default_v2_guest);
    let compiled_v2 =
        wasm::Compiled::load(&v2).expect("build the actual feature-v2 Rust guest first");
    for check in checks::all(&compiled, &compiled_v2) {
        assert!(check.passed, "{}: {}", check.name, check.detail);
    }
}

#[test]
fn cli_rejects_zero_negative_duplicate_and_unknown_inputs() {
    for args in [
        vec!["bench", "--guest", "x", "--calls", "0"],
        vec!["bench", "--guest", "x", "--calls", "-1"],
        vec!["bench", "--guest", "x", "--seed", "-1"],
        vec!["bench", "--guest", "x", "--seed", "1", "--seed", "2"],
        vec!["check", "--guest", "x", "--surprise", "1"],
    ] {
        assert!(parse(args.into_iter().map(str::to_owned).collect()).is_err());
    }
}
