use std::collections::HashMap;
use storage_lab::{Config, benchmark, check};

fn config(args: &[String]) -> Result<Config, String> {
    if args.len() != 10 {
        return Err("bench requires --backend aggregate|hecs --entities N --ticks N --seed U64 --density sparse|dense".into());
    }
    let mut values = HashMap::new();
    for pair in args.as_chunks::<2>().0 {
        if !["--backend", "--entities", "--ticks", "--seed", "--density"]
            .contains(&pair[0].as_str())
            || values.insert(pair[0].as_str(), pair[1].as_str()).is_some()
        {
            return Err(format!("unknown or repeated argument: {}", pair[0]));
        }
    }
    let c = Config {
        backend: values["--backend"].into(),
        entities: values["--entities"]
            .parse()
            .map_err(|_| "invalid entities")?,
        ticks: values["--ticks"].parse().map_err(|_| "invalid ticks")?,
        seed: values["--seed"].parse().map_err(|_| "invalid seed")?,
        density: values["--density"].into(),
    };
    c.validate()?;
    Ok(c)
}

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("check") if args.len() == 1 => {
            let report = check();
            println!("{}", serde_json::to_string(&report).unwrap());
            std::process::exit(if report.ok { 0 } else { 1 });
        }
        Some("bench") => config(&args[1..]).and_then(benchmark).map(|report| serde_json::to_value(report).unwrap()),
        _ => Err("use storage-lab check or storage-lab bench --backend aggregate|hecs --entities N --ticks N --seed U64 --density sparse|dense".into()),
    };
    match result {
        Ok(value) => println!("{value}"),
        Err(error) => {
            println!(
                "{}",
                serde_json::json!({"schema_version":1,"mode":"error","ok":false,"error":error})
            );
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn args() -> Vec<String> {
        "--backend aggregate --entities 1000 --ticks 200 --seed 42 --density sparse"
            .split_whitespace()
            .map(str::to_owned)
            .collect()
    }
    #[test]
    fn valid_config_without_running_measurement() {
        assert_eq!(config(&args()).unwrap().entities, 1000);
    }
    #[test]
    fn reject_invalid_unknown_duplicate_missing_and_excessive_inputs() {
        for (i, value) in [
            (1, "both"),
            (3, "0"),
            (3, "-1"),
            (3, "999999999999999999999"),
            (5, "0"),
            (5, "1000000"),
            (7, "-1"),
            (9, "typo"),
            (0, "--seed"),
            (0, "--unknown"),
        ] {
            let mut input = args();
            input[i] = value.into();
            assert!(config(&input).is_err(), "{input:?}");
        }
        assert!(config(&args()[..8]).is_err());
    }
}
