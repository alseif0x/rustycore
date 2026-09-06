use super::{
    Bundle, Driver, Handle, Row, Store,
    store::{Aggregate, Ecs},
};
use serde::Serialize;
use std::{hint::black_box, time::Instant};

#[derive(Debug, Clone, Serialize)]
pub struct Config {
    pub backend: String,
    pub entities: usize,
    pub ticks: usize,
    pub seed: u64,
    pub density: String,
}

impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(self.backend.as_str(), "aggregate" | "hecs") {
            return Err("backend must be aggregate|hecs".into());
        }
        if !matches!(self.density.as_str(), "sparse" | "dense") {
            return Err("density must be sparse|dense".into());
        }
        if !(8..=1_000_000).contains(&self.entities) || !(1..=1_000_000).contains(&self.ticks) {
            return Err("entities must be 8..=1000000 and ticks 1..=1000000".into());
        }
        if self
            .entities
            .checked_mul(self.ticks + 25)
            .is_none_or(|work| work > 100_000_000)
        {
            return Err("sample exceeds 100000000 entity-ticks, including warmup".into());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct Operations {
    pub updates: u64,
    pub churn: u64,
    pub transfers: u64,
}

#[derive(Debug, Serialize)]
pub struct BenchResult {
    pub schema_version: u8,
    pub mode: &'static str,
    pub lab_model: bool,
    #[serde(flatten)]
    pub config: Config,
    pub build_ns: u64,
    pub update_p50_ns: u64,
    pub update_p95_ns: u64,
    pub update_p99_ns: u64,
    pub sort_ns: u64,
    pub churn_ns: u64,
    pub transfer_ns: u64,
    pub checksum_ns: u64,
    pub checksum: String,
    pub final_entities: usize,
    pub final_optional: usize,
    pub rss_kib: Option<u64>,
    pub vmhwm_kib: Option<u64>,
    pub rss_before_population_kib: Option<u64>,
    pub rss_after_population_kib: Option<u64>,
    pub warmup_ticks: usize,
    pub operations: Operations,
    pub update_scope: &'static str,
}

/// Seeded shuffle and churn selection are independent of HashMap/archetype iteration order.
pub(super) struct Rng(pub u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
}

pub(super) struct Workload<S: Store> {
    pub driver: Driver<S>,
    pub handles: Vec<Handle>,
    rng: Rng,
}

impl<S: Store> Workload<S> {
    pub fn new(config: &Config) -> Self {
        let mut rng = Rng(config.seed);
        let mut guids: Vec<_> = (1..=config.entities as u64).collect();
        for i in (1..guids.len()).rev() {
            let j = (rng.next() % (i + 1) as u64) as usize;
            guids.swap(i, j);
        }
        let mut driver = Driver::default();
        let handles = guids
            .into_iter()
            .enumerate()
            .map(|(i, guid)| {
                let optional = config.density == "dense" || i < config.entities / 4;
                driver.install(i % 2, Bundle::new(guid, optional))
            })
            .collect();
        Self {
            driver,
            handles,
            rng,
        }
    }

    pub fn frame(&mut self, tick: usize) -> Vec<Row> {
        // Repeated controlled encounter program, NOT a claim that C++ resets every tick.
        self.driver
            .frame(
                &[
                    (self.handles[0], self.handles[2]),
                    (self.handles[1], self.handles[3]),
                ],
                tick.is_multiple_of(7),
            )
            .unwrap();
        self.driver.rows()
    }

    pub fn next_churn(&mut self) -> (usize, u64) {
        (
            4 + (self.rng.next() % (self.handles.len() - 4) as u64) as usize,
            self.rng.next() % 3,
        )
    }

    pub fn churn(&mut self, index: usize, kind: u64) {
        let h = self.handles[index];
        let map = self.driver.active(h).unwrap();
        let optional = self.driver.encounter(h).is_ok();
        match kind {
            0 => {
                // Exercise actual add/remove while preserving the boundary distribution.
                self.driver.optional(h, !optional).unwrap();
                self.driver.optional(h, optional).unwrap();
            }
            1 => {
                // Same population, new incarnation; not an ever-growing spawn-only test.
                self.driver.retire(h).unwrap();
                self.handles[index] = self.driver.install(map, Bundle::new(h.guid, optional));
            }
            2 => {
                self.driver.detach(h).unwrap();
                self.driver.attach(h, 1 - map, false).unwrap();
            }
            _ => unreachable!(),
        }
    }
}

fn word(hash: &mut u64, value: u64) {
    *hash = (*hash ^ value).wrapping_mul(0x100000001b3);
}

pub(super) fn observe<S: Store>(hash: &mut u64, rows: &[Row], workload: &Workload<S>) {
    for r in rows {
        for v in [
            r.guid,
            r.x as u64,
            r.health,
            r.revision,
            r.victim.unwrap_or(0),
        ] {
            word(hash, v);
        }
        word(hash, r.attackers.len() as u64);
        for &a in &r.attackers {
            word(hash, a);
        }
        word(hash, r.encounter.is_some().into());
        if let Some(e) = r.encounter {
            for v in [
                e.phase.into(),
                e.timer.into(),
                e.shield.into(),
                e.summon.unwrap_or(0),
                e.callbacks.into(),
                e.pulses,
            ] {
                word(hash, v);
            }
        }
        word(hash, r.policy.is_some().into());
        if let Some(p) = r.policy {
            word(hash, p.module.into());
            word(hash, p.bonus.into());
        }
        for bytes in r.payload.as_chunks::<8>().0 {
            word(hash, u64::from_le_bytes(*bytes));
        }
    }
    for event in &workload.driver.trace {
        for b in event.kind.bytes() {
            word(hash, b.into());
        }
        word(hash, event.guid);
        word(hash, event.value);
    }
    // Application identity/residence is observable too; backend-generated IDs are private.
    for &h in &workload.handles {
        word(hash, h.guid);
        word(hash, h.generation);
        word(hash, workload.driver.active(h).unwrap() as u64);
    }
}

fn ns(start: Instant) -> u64 {
    start.elapsed().as_nanos().try_into().unwrap()
}
fn quantile(sorted: &[u64], percent: usize) -> u64 {
    sorted[(sorted.len() * percent).div_ceil(100) - 1]
}
fn memory(key: &str) -> Option<u64> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find(|line| line.starts_with(key))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

fn run<S: Store>(config: Config) -> BenchResult {
    let rss_before_population_kib = memory("VmRSS:");
    let start = Instant::now();
    let mut workload = Workload::<S>::new(&config);
    let build_ns = ns(start);
    let rss_after_population_kib = memory("VmRSS:");
    for tick in 0..25 {
        black_box(workload.frame(tick));
        workload.driver.trace.clear();
        for _ in 0..(config.entities / 100).max(1) {
            let (index, kind) = workload.next_churn();
            workload.churn(index, kind);
        }
    }
    let mut update = Vec::with_capacity(config.ticks);
    let (mut sort_ns, mut churn_ns, mut transfer_ns, mut checksum_ns) = (0, 0, 0, 0);
    let mut hash = 0xcbf29ce484222325;
    let mut operations = Operations {
        updates: 0,
        churn: 0,
        transfers: 0,
    };
    for tick in 0..config.ticks {
        let start = Instant::now();
        let mut rows = black_box(workload.frame(black_box(tick + 25)));
        update.push(ns(start));
        operations.updates += config.entities as u64;
        let start = Instant::now();
        rows.sort_unstable_by_key(|r| r.guid);
        sort_ns += ns(start);
        let start = Instant::now();
        observe(&mut hash, &rows, &workload);
        black_box(hash);
        checksum_ns += ns(start);
        workload.driver.trace.clear();
        for _ in 0..(config.entities / 100).max(1) {
            let (index, kind) = workload.next_churn();
            let start = Instant::now();
            workload.churn(black_box(index), black_box(kind));
            let elapsed = ns(start);
            if kind == 2 {
                transfer_ns += elapsed;
                operations.transfers += 1;
            } else {
                churn_ns += elapsed;
                operations.churn += 1;
            }
        }
    }
    let mut final_rows = workload.driver.rows();
    let start = Instant::now();
    final_rows.sort_unstable_by_key(|r| r.guid);
    sort_ns += ns(start);
    let start = Instant::now();
    observe(&mut hash, &final_rows, &workload);
    checksum_ns += ns(start);
    update.sort_unstable();
    BenchResult {
        schema_version: 1,
        mode: "bench",
        lab_model: true,
        build_ns,
        update_p50_ns: quantile(&update, 50),
        update_p95_ns: quantile(&update, 95),
        update_p99_ns: quantile(&update, 99),
        sort_ns,
        churn_ns,
        transfer_ns,
        checksum_ns,
        checksum: format!("{hash:016x}"),
        final_entities: final_rows.len(),
        final_optional: final_rows.iter().filter(|r| r.encounter.is_some()).count(),
        rss_kib: memory("VmRSS:"),
        vmhwm_kib: memory("VmHWM:"),
        operations,
        rss_before_population_kib,
        rss_after_population_kib,
        warmup_ticks: 25,
        update_scope: "owner frame + observable row materialization; excludes sort/checksum/churn/transfer",
        config,
    }
}

pub fn benchmark(config: Config) -> Result<BenchResult, String> {
    config.validate()?;
    Ok(match config.backend.as_str() {
        "aggregate" => run::<Aggregate>(config),
        "hecs" => run::<Ecs>(config),
        _ => unreachable!(),
    })
}
