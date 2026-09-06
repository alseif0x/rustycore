use super::*;

#[test]
fn finite_contract_suite_without_performance_measurement() {
    let report = check();
    assert_eq!(report.checks.len(), 16);
    assert!(
        report.ok,
        "{}",
        serde_json::to_string_pretty(&report).unwrap()
    );
}

#[test]
fn standalone_config_bounds_are_checked_before_allocations() {
    let config = Config {
        backend: "hecs".into(),
        entities: 10_000,
        ticks: 200,
        seed: 42,
        density: "dense".into(),
    };
    assert!(config.validate().is_ok());
    assert!(
        Config {
            entities: usize::MAX,
            ..config.clone()
        }
        .validate()
        .is_err()
    );
    assert!(
        Config {
            ticks: usize::MAX,
            ..config
        }
        .validate()
        .is_err()
    );
}
