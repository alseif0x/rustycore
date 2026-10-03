//! Explicit read-only consumer of private local target initialization assets.
//! Counts only; no Player admission, SQL, rows, keys or game-account access.
use anyhow::{Result, ensure};
use std::{env, path::Path, process};
use wow_data::forever_game_tables::InitialGameTables;
use wow_data::forever_initialization::{InitializationRecords, MapBaseline};

fn run() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    ensure!(
        matches!(args.len(), 2 | 3) && args[0] == "--ack-local-client-data",
        "explicit acquisition acknowledgement required"
    );
    let map = if args.len() == 3 {
        ensure!(args[2] == "--ack-available-initial-map", "unknown opt-in");
        MapBaseline::AvailablePrefix
    } else {
        MapBaseline::Complete
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/forever-login")
        .canonicalize()?;
    let directory = Path::new(&args[1]).canonicalize()?;
    ensure!(
        directory.starts_with(root) && directory.is_dir(),
        "outside private fixture"
    );
    let records = InitializationRecords::load(&directory, map)?;
    let gt = InitialGameTables::load(&directory)?;
    println!(
        "{{\"build\":70170,\"classes\":{},\"races\":{},\"maps\":{},\"unknown_map_records\":{},\"powers\":{},\"specializations\":{},\"class_powers\":{},\"movies\":{},\"gt_row_counts_including_unused_zero\":{:?},\"gt_numeric_bit_fingerprints\":{:?},\"player_admitted\":false}}",
        records.classes.len(),
        records.races.len(),
        records.maps.len(),
        records.unknown_map_records,
        records.powers.len(),
        records.specializations.len(),
        records.class_powers.len(),
        records.movies.len(),
        gt.counts(),
        gt.numeric_bit_fingerprints()
    );
    Ok(())
}

fn main() {
    if run().is_err() {
        // A reader's path/source context may refer to private assets. Neither
        // that context nor raw client records are printed by this consumer.
        eprintln!("Private Forever initialization catalog rejected");
        process::exit(1);
    }
}
