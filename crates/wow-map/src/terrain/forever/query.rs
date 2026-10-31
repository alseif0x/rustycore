// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: checked readiness and composed static-height selection.
// Required behavior: TrinityCore 02245dcd245e7433e524577656177723d3e4992e,
// TerrainMgr.cpp TerrainInfo::GetStaticHeight/GetGrid; G3D/g3dmath.h fuzzyGe.
use super::*;
pub use forever_vmap::store::{INVALID_HEIGHT as NO_HEIGHT, ObjectHit};

fn finite(position: [f32; 3]) -> Result<(), Error> {
    if position.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(Error::NonFinite)
    }
}
fn axis(value: f32) -> Result<u32, Error> {
    if !value.is_finite() {
        return Err(Error::NonFinite);
    }
    let grid = 32.0_f32 - value / 533.333_3_f32;
    if !(0.0..64.0).contains(&grid) {
        return Err(Error::Coordinates);
    }
    Ok(grid as u32)
}
impl TileKey {
    /// Physical MAPS/VMAP grid, after map-ID phase resolution by the caller.
    pub fn for_position(map: u32, x: f32, y: f32) -> Result<Self, Error> {
        Self::new(map, axis(x)?, axis(y)?)
    }
}
impl State {
    fn tile(&self, key: TileKey) -> Result<&Tile, Error> {
        self.position(key)
            .map(|i| &self.tiles[i])
            .map_err(|_| Error::Unprepared)
    }
    fn segment_ready(&self, map: u32, start: [f32; 3], end: [f32; 3]) -> Result<(), Error> {
        let a = TileKey::for_position(map, start[0], start[1])?;
        let b = TileKey::for_position(map, end[0], end[1])?;
        for x in a.x.min(b.x)..=a.x.max(b.x) {
            for y in a.y.min(b.y)..=a.y.max(b.y) {
                if !self.tile(TileKey { map, x, y })?.collision_loaded {
                    return Err(Error::Unprepared);
                }
            }
        }
        Ok(())
    }
}

// Subtraction is f32 before promotion, and fuzzyGe is strictly greater than
// b-eps, not >=. Keep native float subtraction order for the distance comparison.
pub(super) fn select_height(grid: f32, vmap: f32, z: f32) -> Result<f32, Error> {
    if !grid.is_finite() || !vmap.is_finite() || !z.is_finite() {
        return Err(Error::NonFinite);
    }
    let threshold = f64::from(grid - 0.05_f32);
    let z64 = f64::from(z);
    let map = if z64 > threshold - 0.000_000_5_f64 * (z64.abs() + 1.0) {
        grid
    } else {
        NO_HEIGHT
    };
    if vmap > crate::grid_map::INVALID_HEIGHT {
        if map > crate::grid_map::INVALID_HEIGHT {
            let dm = (map - z).abs();
            let dv = (vmap - z).abs();
            if !dm.is_finite() || !dv.is_finite() {
                return Err(Error::NonFinite);
            }
            if vmap > map || dm > dv {
                return Ok(vmap);
            }
            return Ok(map);
        }
        return Ok(vmap);
    }
    Ok(map)
}

impl ForeverTerrain {
    pub fn grid_height(&self, map: u32, x: f32, y: f32) -> Result<f32, Error> {
        let state = self.state.read().map_err(|_| Error::LockPoisoned)?;
        state.healthy(map)?;
        let tile = state.tile(TileKey::for_position(map, x, y)?)?;
        let height = tile
            .ground
            .as_ref()
            .map_or(NO_HEIGHT, |g| g.get_height(x, y));
        if !height.is_finite() {
            return Err(Error::NonFinite);
        }
        Ok(height)
    }
    /// Caller supplies the final probe Z; no collision-height/0.5 offset is added.
    /// Positive infinity is the admitted unlimited collision search distance.
    pub fn static_height(
        &self,
        map: u32,
        position: [f32; 3],
        check_vmap: bool,
        maximum: f32,
        budget: &mut QueryBudget,
    ) -> Result<f32, Error> {
        let state = self.state.read().map_err(|_| Error::LockPoisoned)?;
        state.healthy(map)?;
        finite(position)?;
        if maximum.is_nan() || maximum < 0.0 {
            return Err(Error::Coordinates);
        }
        let tile = state.tile(TileKey::for_position(map, position[0], position[1])?)?;
        let grid = tile
            .ground
            .as_ref()
            .map_or(NO_HEIGHT, |g| g.get_height(position[0], position[1]));
        let vmap = if check_vmap && state.store.options().enable_height {
            if !tile.collision_loaded {
                return Err(Error::Unprepared);
            }
            state
                .store
                .height_world(map, position, maximum, budget)?
                .unwrap_or(NO_HEIGHT)
        } else {
            NO_HEIGHT
        };
        select_height(grid, vmap, position[2])
    }
    pub fn line_of_sight(
        &self,
        map: u32,
        start: [f32; 3],
        end: [f32; 3],
        ignore_m2: bool,
        budget: &mut QueryBudget,
    ) -> Result<bool, Error> {
        let state = self.state.read().map_err(|_| Error::LockPoisoned)?;
        state.healthy(map)?;
        finite(start)?;
        finite(end)?;
        TileKey::for_position(map, start[0], start[1])?;
        TileKey::for_position(map, end[0], end[1])?;
        if state.store.options().enable_line_of_sight {
            state.segment_ready(map, start, end)?;
        }
        Ok(state
            .store
            .line_of_sight_world(map, start, end, ignore_m2, budget)?)
    }
    pub fn object_hit(
        &self,
        map: u32,
        start: [f32; 3],
        end: [f32; 3],
        modify: f32,
        budget: &mut QueryBudget,
    ) -> Result<ObjectHit, Error> {
        let state = self.state.read().map_err(|_| Error::LockPoisoned)?;
        state.healthy(map)?;
        finite(start)?;
        finite(end)?;
        if !modify.is_finite() {
            return Err(Error::NonFinite);
        }
        TileKey::for_position(map, start[0], start[1])?;
        TileKey::for_position(map, end[0], end[1])?;
        if state.store.options().enable_line_of_sight {
            state.segment_ready(map, start, end)?;
        }
        Ok(state
            .store
            .object_hit_world(map, start, end, modify, budget)?)
    }
}
