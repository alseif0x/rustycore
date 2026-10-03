//! Read-only actual-file consumer. Metadata/counts only, not spell/Player admission.
use anyhow::{Result, ensure};
use std::{env, path::Path, process};
use wow_data::forever_spells::{SpellBaseline, SpellRecords};

fn run() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    ensure!(
        matches!(args.len(), 2 | 3) && args[0] == "--ack-local-client-data",
        "Explicit acquisition acknowledgement required"
    );
    let baseline = if args.len() == 3 {
        ensure!(
            args[2] == "--ack-available-spell-info-tables",
            "Unknown spell opt-in"
        );
        SpellBaseline::AvailablePrefixes
    } else {
        SpellBaseline::Complete
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/forever-login")
        .canonicalize()?;
    let directory = Path::new(&args[1]).canonicalize()?;
    ensure!(
        directory.starts_with(root) && directory.is_dir(),
        "Outside private fixture"
    );
    let records = SpellRecords::load(&directory, baseline)?;
    print!("{{\"build\":70170,\"tables\":[");
    for (i, (name, known, unknown)) in records.counts().into_iter().enumerate() {
        if i != 0 {
            print!(",");
        }
        print!(
            "{{\"name\":\"{}\",\"known_materialized_records\":{},\"unknown_direct_baseline_records\":{}}}",
            name, known, unknown
        );
    }
    println!("],\"player_admitted\":false,\"spell_info_assembled\":false}}");
    Ok(())
}

fn main() {
    if run().is_err() {
        eprintln!("Private Forever spell tables rejected");
        process::exit(1);
    }
}
