// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: explicit Forever terrain reader and shared tile lifetime.
//! One checked owner of supplied complete MAPS and rooted VMAP_4.E geometry.
//!
//! The build is an explicit declaration, not provenance authentication. Callers
//! supply complete primary/cosmetic topology and already phase-resolved map IDs.
//! MAPS bytes come from their separately authenticated acquisition layer; Absent
//! is an explicit supplied fact, not an I/O fallback. Complete MAPS are verified
//! before the existing RustyCore GridMap reader consumes their height data.
//!
//! Preparation/verification can block on local files and must run outside map,
//! entity and session locks. Only this geometry owner's write lock spans I/O.
//! Queries take a read lock, never load files, and use the caller's query budget.
//! A conservative inclusive XY grid rectangle must be prepared for a segment;
//! missing coverage is an error, never clear LOS. This may require more tiles
//! than the exact diagonal intersects. Cleanup has no timer or background task.
//!
//! Leases implement the TerrainInfo consumer layer: only the first preparation
//! loads the underlying StaticStore, release decrements ownership, and explicit
//! cleanup unloads unreferenced cached tiles once. They do not change the store's
//! separate native repeated-load/unload-all contract. No legacy bool/f32 trait
//! is implemented: configured errors must remain visible to the eventual caller.
//! No phase recursion, area/liquid, dynamic GO, navigation or server activation.
use crate::grid_map::GridMap;
use forever_vmap::store::{self, StaticStore, fs::RootedSource};
use std::{
    fmt,
    path::Path,
    sync::{Arc, RwLock},
};

mod load;
mod query;
pub use query::{NO_HEIGHT, ObjectHit};
#[cfg(test)]
mod tests;

pub use forever_vmap::query::{Budget as QueryBudget, Limits as QueryLimits};
pub use forever_vmap::store::fs::FsLimits as SourceLimits;
pub use forever_vmap::store::{Limits as StoreLimits, MapParent, Options};

pub const MAX_TILES: usize = 65_536;
pub const MAX_LEASES: usize = 1_000_000;
pub const MAX_TERRAIN_BYTES: usize = 512 * 1024 * 1024;
/// Conservative transient allowance for the existing fixed-size GridMap arrays.
/// Actual retained capacities are measured after parsing. Not allocator/RSS data.
const GRID_PARSE_ALLOWANCE: usize = 512 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Profile {
    build: u32,
}
impl Profile {
    pub fn declared_build(build: u32) -> Result<Self, Error> {
        if matches!(build, 70170 | 70205) {
            Ok(Self { build })
        } else {
            Err(Error::Profile)
        }
    }
    pub const fn build(self) -> u32 {
        self.build
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_tiles: usize,
    pub max_leases: usize,
    pub max_terrain_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_tiles: MAX_TILES,
            max_leases: MAX_LEASES,
            max_terrain_bytes: MAX_TERRAIN_BYTES,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        if self.max_tiles > MAX_TILES
            || self.max_leases > MAX_LEASES
            || self.max_terrain_bytes > MAX_TERRAIN_BYTES
        {
            return Err(Error::Limits);
        }
        Ok(())
    }
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TileKey {
    map: u32,
    x: u8,
    y: u8,
}
impl TileKey {
    pub fn new(map: u32, x: u32, y: u32) -> Result<Self, Error> {
        if x >= 64 || y >= 64 {
            return Err(Error::Coordinates);
        }
        Ok(Self {
            map,
            x: x as u8,
            y: y as u8,
        })
    }
    pub const fn map_id(self) -> u32 {
        self.map
    }
    pub const fn x(self) -> u8 {
        self.x
    }
    pub const fn y(self) -> u8 {
        self.y
    }
}
/// Explicit complete source or a known absent ground tile; never partial bytes.
#[derive(Clone, Copy)]
pub enum Ground<'a> {
    Complete(&'a [u8]),
    Absent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Profile,
    Limits,
    Allocation,
    Coordinates,
    NonFinite,
    UnknownMap,
    Unprepared,
    InputChanged,
    LockPoisoned,
    GroundFormat,
    Source(store::SourceError),
    Collision(store::Error),
}
impl Error {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Profile => "forever_terrain_profile",
            Self::Limits => "forever_terrain_limit",
            Self::Allocation => "forever_terrain_allocation",
            Self::Coordinates => "forever_terrain_coordinates",
            Self::NonFinite => "forever_terrain_nonfinite",
            Self::UnknownMap => "forever_terrain_unknown_map",
            Self::Unprepared => "forever_terrain_unprepared",
            Self::InputChanged => "forever_terrain_input_changed",
            Self::LockPoisoned => "forever_terrain_lock_poisoned",
            Self::GroundFormat => "forever_terrain_ground_format",
            Self::Source(_) => "forever_terrain_source",
            Self::Collision(_) => "forever_terrain_collision",
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}
impl std::error::Error for Error {}
impl From<store::Error> for Error {
    fn from(e: store::Error) -> Self {
        Self::Collision(e)
    }
}

struct Tile {
    key: TileKey,
    consumers: usize,
    bytes: Option<Vec<u8>>,
    ground: Option<GridMap>,
    collision_loaded: bool,
}
impl Tile {
    fn heap_bytes(&self) -> usize {
        self.bytes.as_ref().map_or(0, Vec::capacity)
            + self.ground.as_ref().map_or(0, GridMap::heap_bytes)
    }
    fn same_input(&self, ground: Ground<'_>) -> bool {
        match (&self.bytes, ground) {
            (None, Ground::Absent) => true,
            (Some(old), Ground::Complete(new)) => old.as_slice() == new,
            _ => false,
        }
    }
}
struct State {
    store: StaticStore<RootedSource>,
    tiles: Vec<Tile>,
    leases: usize,
    input_changed: bool,
    admitted_peak_bytes: usize,
}
impl State {
    fn healthy(&self, map: u32) -> Result<(), Error> {
        if self.input_changed || self.store.is_poisoned() {
            return Err(Error::InputChanged);
        }
        if !self.store.is_declared(map) {
            return Err(Error::UnknownMap);
        }
        Ok(())
    }
    fn position(&self, key: TileKey) -> Result<usize, usize> {
        self.tiles.binary_search_by_key(&key, |tile| tile.key)
    }
    fn resident_bytes(&self) -> usize {
        self.tiles.capacity() * size_of::<Tile>()
            + self.tiles.iter().map(Tile::heap_bytes).sum::<usize>()
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub tiles: usize,
    pub leases: usize,
    pub models: usize,
    pub active_instances: usize,
    pub terrain_bytes: usize,
    pub admitted_peak_bytes: usize,
    pub collision_bytes: usize,
    pub source_reads: u64,
    pub input_changed: bool,
}
/// Single canonical geometry cache, shared across map instances and consumers.
pub struct ForeverTerrain {
    profile: Profile,
    limits: Limits,
    state: RwLock<State>,
}
impl fmt::Debug for ForeverTerrain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ForeverTerrain")
            .field("profile", &self.profile)
            .finish_non_exhaustive()
    }
}
impl ForeverTerrain {
    /// `root` is the VMAP package directory: models live directly beneath it,
    /// while each map's tree/tile/index files live under its padded map directory
    /// (for example `0001/0001_32_32.vmtile`). No flat-layout fallback is tried.
    pub fn open(
        root: &Path,
        profile: Profile,
        parents: &[MapParent],
        options: Options,
        limits: Limits,
        store_limits: StoreLimits,
        source_limits: SourceLimits,
    ) -> Result<Arc<Self>, Error> {
        limits.validate()?;
        let source = RootedSource::new(root, source_limits).map_err(Error::Source)?;
        let store = StaticStore::new(source, parents, options, store_limits)?;
        Ok(Arc::new(Self {
            profile,
            limits,
            state: RwLock::new(State {
                store,
                tiles: Vec::new(),
                leases: 0,
                input_changed: false,
                admitted_peak_bytes: 0,
            }),
        }))
    }
    pub const fn profile(&self) -> Profile {
        self.profile
    }
    pub fn stats(&self) -> Result<Stats, Error> {
        let state = self.state.read().map_err(|_| Error::LockPoisoned)?;
        let s = state.store.stats();
        Ok(Stats {
            tiles: state.tiles.len(),
            leases: state.leases,
            models: s.models,
            active_instances: s.active_instances,
            terrain_bytes: state.resident_bytes(),
            admitted_peak_bytes: state.admitted_peak_bytes,
            collision_bytes: s.resident_bytes,
            source_reads: s.source_reads,
            input_changed: state.input_changed || state.store.is_poisoned(),
        })
    }
    /// Explicit metadata checkpoint for the rooted VMAP source. Retained MAPS
    /// bytes are immutable; repeated preparations compare their complete bytes.
    pub fn verify_source(&self) -> Result<(), Error> {
        let mut state = self.state.write().map_err(|_| Error::LockPoisoned)?;
        if state.input_changed || state.store.is_poisoned() {
            return Err(Error::InputChanged);
        }
        state.store.verify_source().map_err(Error::Collision)
    }
    /// No I/O. Only zero-consumer entries are removed; active leases survive.
    pub fn cleanup_unreferenced(&self) -> Result<usize, Error> {
        let mut state = self.state.write().map_err(|_| Error::LockPoisoned)?;
        if state.input_changed || state.store.is_poisoned() {
            return Err(Error::InputChanged);
        }
        let State { store, tiles, .. } = &mut *state;
        let mut removed = 0;
        let mut failure = None;
        // Compact once, preserving sorted order. Repeated Vec::remove would
        // move the remaining cache quadratically for alternating live entries.
        // On an error, retain that entry and the unprocessed suffix; earlier
        // successful unloads have already removed their matching ground data.
        tiles.retain(|tile| {
            if tile.consumers != 0 || failure.is_some() {
                return true;
            }
            if tile.collision_loaded {
                let key = tile.key;
                if let Err(error) = store.unload_tile(key.map, u16::from(key.x), key.y) {
                    failure = Some(Error::Collision(error));
                    return true;
                }
            }
            removed += 1;
            false
        });
        failure.map_or(Ok(removed), Err)
    }
}
/// A unique consumer reference. Dropping releases ownership but performs no I/O
/// and does not evict geometry until explicit cleanup. It is deliberately not Clone.
pub struct TileLease {
    owner: Arc<ForeverTerrain>,
    key: TileKey,
}
impl TileLease {
    pub const fn key(&self) -> TileKey {
        self.key
    }
}
impl fmt::Debug for TileLease {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TileLease { .. }")
    }
}
impl Drop for TileLease {
    fn drop(&mut self) {
        let Ok(mut state) = self.owner.state.write() else {
            return;
        };
        let Ok(index) = state.position(self.key) else {
            state.input_changed = true;
            return;
        };
        if state.tiles[index].consumers == 0 || state.leases == 0 {
            state.input_changed = true;
            return;
        }
        state.tiles[index].consumers -= 1;
        state.leases -= 1;
    }
}
