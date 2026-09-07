//! Read-only #585 fixture discovery using the production DB2 reader.
//! Usage: inspect_finalization_portal DATA_DIR LOCALE TRIGGER_ID
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    anyhow::ensure!(args.len() == 4, "expected DATA_DIR LOCALE TRIGGER_ID");
    let id: u32 = args[3].parse()?;
    let store = wow_data::AreaTriggerDb2Store::load(&args[1], &args[2])?;
    let row = store
        .get(id)
        .ok_or_else(|| anyhow::anyhow!("trigger absent"))?;
    println!("{row:#?}");
    Ok(())
}
