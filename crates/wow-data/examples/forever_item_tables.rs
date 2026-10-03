//! Read-only all-four-table consumer. No DB overlay or Player admission.
use anyhow::{Result, ensure};
use std::{env, path::Path, process};
use wow_data::forever_birth::item_records::ItemRecords;

fn run() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    ensure!(
        args.len() == 2 && args[0] == "--ack-private-item-prefixes",
        "explicit private prefix acknowledgement required"
    );
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/forever-login")
        .canonicalize()?;
    let directory = Path::new(&args[1]).canonicalize()?;
    ensure!(
        directory.starts_with(root) && directory.is_dir(),
        "outside private fixture"
    );
    let rows = ItemRecords::load_available(&directory)?;
    println!(
        "{{\"build\":70170,\"records\":[{},{},{},{}],\"unknown_baseline_records\":{:?},\"player_admitted\":false}}",
        rows.items.len(),
        rows.sparse.len(),
        rows.effects.len(),
        rows.relations.len(),
        rows.unknown_baseline_records
    );
    Ok(())
}
fn main() {
    if run().is_err() {
        eprintln!("Private Forever item baseline batch rejected");
        process::exit(1);
    }
}
