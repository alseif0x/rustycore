//! Separate measurements for storage operations, execution and cold construction.
//! No measurement here alone is the selected-design acceptance gate.
mod dispatch;
mod storage;

pub use dispatch::run as dispatch;
pub use storage::run as storage;

use conformance_contract::Handle;
use conformance_host::{HostCore, Residence};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::time::Instant;

type Result<T> = std::result::Result<T, String>;

fn checked<T>(result: conformance_contract::Result<T>) -> Result<T> {
    result.map_err(|fault| format!("benchmark contract failed: {fault:?}"))
}

fn ns(start: Instant) -> u64 {
    u64::try_from(start.elapsed().as_nanos()).expect("bounded measurement duration")
}

fn quantiles(samples: &[u64]) -> Value {
    let mut samples = samples.to_vec();
    samples.sort_unstable();
    let percentile = |p: usize| samples[(samples.len() * p).div_ceil(100).saturating_sub(1)];
    json!({ "p50_ns": percentile(50), "p95_ns": percentile(95), "p99_ns": percentile(99) })
}

fn rss(key: &str) -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix(key))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn cpu_ticks() -> Option<u64> {
    let stat = std::fs::read_to_string("/proc/self/stat").ok()?;
    let (_, fields) = stat.rsplit_once(") ")?;
    let fields: Vec<_> = fields.split_whitespace().collect();
    fields
        .get(11)?
        .parse::<u64>()
        .ok()?
        .checked_add(fields.get(12)?.parse::<u64>().ok()?)
}

/// Full canonical data is hashed outside the timed path. Detailed functional oracles
/// separately retain complete state and ordered traces; a checksum is not their substitute.
fn population_digest(core: &HostCore, handles: &[Handle]) -> Result<String> {
    let mut digest = Sha256::new();
    let mut sorted = handles.to_vec();
    sorted.sort();
    for handle in sorted {
        let snapshot = checked(core.snapshot(handle))?;
        let observed = checked(core.observables(handle))?;
        let states: Vec<_> = snapshot
            .modules
            .iter()
            .map(|state| {
                json!({
                    "id": state.id, "abi": state.abi, "schema": state.schema,
                    "revision": state.revision, "bytes": state.bytes,
                })
            })
            .collect();
        let contributions: Vec<_> = snapshot.contributions.iter().map(|(id, c)| {
            json!({ "id": id, "shield": c.shield, "summons": c.summons, "amount": c.amount })
        }).collect();
        let residence = match observed.residence {
            Residence::Active(map) => json!({ "active": map }),
            Residence::Detached => json!("detached"),
        };
        let row = json!({ "guid": handle.guid, "generation": handle.generation,
            "core_revision": snapshot.core_revision, "residence": residence,
            "payload": observed.payload_sentinel, "states": states,
            "contributions": contributions });
        let bytes = serde_json::to_vec(&row).map_err(|error| error.to_string())?;
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    }
    Ok(format!("{:x}", digest.finalize()))
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }
}
