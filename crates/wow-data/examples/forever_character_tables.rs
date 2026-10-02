//! Operator-only, read-only consumer of the actual target DB2 ID catalog.
use std::{env, path::Path};

use anyhow::{Result, ensure};
use wow_data::forever_character_ids::ForeverCharacterIds;

fn main() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    ensure!(
        args.len() == 2 && args[0] == "--ack-local-client-data",
        "Usage: --ack-local-client-data <private acquired DB2 directory>"
    );
    let directory = Path::new(&args[1]).canonicalize()?;
    ensure!(
        directory
            .to_string_lossy()
            .contains("/target/forever-login/"),
        "Only the ignored isolated data directory is admitted"
    );
    let tables = ForeverCharacterIds::load(&directory)?;
    // IDs are public game metadata, not assets, character records or credentials.
    println!(
        "Target character tables decoded: classes={}, races={}; playable_combinations_tested=false",
        tables.classes().len(),
        tables.races().len()
    );
    println!(
        "class_ids={:?}; race_95_present={}; race_96_present={}",
        tables.classes(),
        tables.races().contains(&95),
        tables.races().contains(&96)
    );
    Ok(())
}
