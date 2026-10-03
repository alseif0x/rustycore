//! Opt-in read-only target spell-value GT consumer. Counts/fingerprints only.
use anyhow::{Result, ensure};
use std::{env, path::Path, process};
use wow_data::forever_game_tables::SpellValueGameTables;

fn run() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    ensure!(
        args.len() == 2 && args[0] == "--ack-local-client-data",
        "explicit input acknowledgement required"
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/forever-login")
        .canonicalize()?;
    let directory = Path::new(&args[1]).canonicalize()?;
    ensure!(
        directory.starts_with(root) && directory.is_dir(),
        "outside private fixture"
    );
    let tables = SpellValueGameTables::load(directory)?;
    println!(
        "{{\"build\":70170,\"gt_row_counts_including_unused_zero\":{:?},\"gt_numeric_bit_fingerprints\":{:?},\"spell_value_ready\":false,\"player_admitted\":false}}",
        tables.counts(),
        tables.numeric_bit_fingerprints()
    );
    Ok(())
}
fn main() {
    if run().is_err() {
        eprintln!("Private Forever spell-value GameTables rejected");
        process::exit(1);
    }
}
