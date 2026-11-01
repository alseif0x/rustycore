// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: checked production-library consumer, no server activation.
use std::sync::Arc;
use wow_entities::{LineOfSightQuery, WorldObjectHeightQuery};
use wow_map::terrain::forever::{
    Error, ForeverTerrain, Ground, ObjectHit, QueryBudget, Stats, TileKey, TileLease,
};

/// Shared Forever spatial library capability. Map IDs are already phase-resolved;
/// the caller must prepare outside entity/map/session locks and retain leases.
/// No background load, second cache, dynamic-object LOS or legacy unchecked trait.
/// The Forever account-phase server does not instantiate this capability yet.
#[derive(Clone, Debug)]
pub struct LiveForeverTerrain {
    owner: Arc<ForeverTerrain>,
}
impl LiveForeverTerrain {
    pub fn new(owner: Arc<ForeverTerrain>) -> Self {
        Self { owner }
    }
    pub fn prepare(&self, key: TileKey, ground: Ground<'_>) -> Result<TileLease, Error> {
        self.owner.prepare(key, ground)
    }
    pub fn cleanup_unreferenced(&self) -> Result<usize, Error> {
        self.owner.cleanup_unreferenced()
    }
    pub fn verify_source(&self) -> Result<(), Error> {
        self.owner.verify_source()
    }
    pub fn stats(&self) -> Result<Stats, Error> {
        self.owner.stats()
    }
    pub fn grid_height(&self, map: u32, x: f32, y: f32) -> Result<f32, Error> {
        self.owner.grid_height(map, x, y)
    }
    pub fn static_height(
        &self,
        map: u32,
        position: [f32; 3],
        query: WorldObjectHeightQuery,
        budget: &mut QueryBudget,
    ) -> Result<f32, Error> {
        self.owner
            .static_height(map, position, query.vmap, query.distance_to_search, budget)
    }
    /// Static VMAP portion only. Already-resolved endpoint positions are forwarded
    /// once; the check_dynamic option belongs to the eventual dynamic-tree owner.
    pub fn static_line_of_sight(
        &self,
        map: u32,
        query: LineOfSightQuery<'_>,
        ignore_m2: bool,
        budget: &mut QueryBudget,
    ) -> Result<bool, Error> {
        let a = query.from.position;
        let b = query.to.position;
        self.owner
            .line_of_sight(map, [a.x, a.y, a.z], [b.x, b.y, b.z], ignore_m2, budget)
    }
    pub fn static_object_hit(
        &self,
        map: u32,
        start: [f32; 3],
        end: [f32; 3],
        modify: f32,
        budget: &mut QueryBudget,
    ) -> Result<ObjectHit, Error> {
        self.owner.object_hit(map, start, end, modify, budget)
    }
}
