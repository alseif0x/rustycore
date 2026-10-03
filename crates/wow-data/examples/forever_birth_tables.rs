//! Read-only private-asset consumer: counts, never rows or Player admission.
use anyhow::{Result, ensure};
use std::{env, path::Path, process};
use wow_data::forever_birth::{AbilityBaseline, BirthRecords};

fn run() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    ensure!(
        matches!(args.len(), 2 | 3) && args[0] == "--ack-local-client-data",
        "explicit acquisition acknowledgement required"
    );
    let baseline = if args.len() == 3 {
        ensure!(
            args[2] == "--ack-available-birth-abilities",
            "unknown opt-in"
        );
        AbilityBaseline::AvailablePrefix
    } else {
        AbilityBaseline::Complete
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/forever-login")
        .canonicalize()?;
    let directory = Path::new(&args[1]).canonicalize()?;
    ensure!(
        directory.starts_with(root) && directory.is_dir(),
        "outside private fixture"
    );
    let records = BirthRecords::load(&directory, baseline)?;
    println!(
        "{{\"build\":70170,\"skill_lines\":{},\"skill_race_class\":{},\"skill_abilities\":{},\"unknown_ability_records\":{},\"loadouts\":{},\"loadout_items\":{},\"player_admitted\":false}}",
        records.skill_lines.len(),
        records.race_class.len(),
        records.abilities.len(),
        records.unknown_ability_records,
        records.loadouts.len(),
        records.loadout_items.len()
    );
    Ok(())
}

fn main() {
    if run().is_err() {
        eprintln!("Private Forever birth catalog rejected");
        process::exit(1);
    }
}
