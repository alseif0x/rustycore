//! Loot-race sql operations.
//!
//! Moved out of loot_race.rs under #634. Behaviour is preserved.

use super::*;

pub(crate) fn loot_db_opts(url: &str, label: &str) -> Result<mysql::Opts> {
    let opts =
        mysql::Opts::from_url(url).map_err(|error| anyhow!("Bad {label} DB URL: {error}"))?;
    Ok(mysql::OptsBuilder::from_opts(opts)
        .tcp_connect_timeout(Some(Duration::from_secs(10)))
        .read_timeout(Some(Duration::from_secs(LOOT_DB_OPERATION_TIMEOUT_SECS)))
        .write_timeout(Some(Duration::from_secs(LOOT_DB_OPERATION_TIMEOUT_SECS)))
        .into())
}
pub(crate) fn validate_unique_sql_spawn(
    spawns: &[u64],
    configured_spawn: u64,
    entry: u32,
    map_id: u16,
) -> Result<()> {
    if spawns.len() != 1 {
        bail!(
            "loot-race target entry {entry} map {map_id} has {} SQL spawns ({spawns:?}); runtime GUID auto-discovery requires exactly one",
            spawns.len()
        );
    }
    if spawns[0] != configured_spawn {
        bail!(
            "loot-race target entry {entry} map {map_id} uniquely resolves SQL spawn {}, not configured spawn {configured_spawn}",
            spawns[0]
        );
    }
    Ok(())
}
