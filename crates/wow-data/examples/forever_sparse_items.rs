//! Read-only production-linked numeric consumer, never a Player/save operation.
use anyhow::{Result, ensure};
use std::{env, path::Path, process};
use wow_data::forever_birth::item_sparse::SparseItemRecords;

fn run() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    ensure!(
        args.len() == 2 && args[0] == "--ack-private-sparse-item-prefix",
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
    let records = SparseItemRecords::load_available(&directory)?;
    println!(
        "{{\"build\":70170,\"sparse_item_records\":{},\"unknown_baseline_records\":{},\"player_admitted\":false}}",
        records.records.len(),
        records.unknown_baseline_records
    );
    Ok(())
}

fn main() {
    if run().is_err() {
        eprintln!("Private Forever sparse item baseline rejected");
        process::exit(1);
    }
}
