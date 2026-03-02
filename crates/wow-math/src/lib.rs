//! WoW-specific 3D math utilities.
//!
//! This crate wraps the [`glam`] crate and provides coordinate-system helpers,
//! map/grid math, bounding-box primitives, and distance/angle utilities that
//! match the conventions used by the WoW 3.4.x (WotLK) client and TrinityCore.

pub use glam::Vec3;

use std::f32::consts::TAU;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Size of one grid cell in yards (533.33333 yd).
pub const GRID_SIZE: f32 = 533.33333;

/// Number of grid cells along one axis.
pub const MAX_GRID_COUNT: u32 = 64;

/// Total map size in yards (GRID_SIZE * 64).
pub const MAP_SIZE: f32 = GRID_SIZE * MAX_GRID_COUNT as f32;

/// Half the total map size — the map spans [-MAP_HALFSIZE, +MAP_HALFSIZE].
pub const MAP_HALFSIZE: f32 = MAP_SIZE / 2.0;

/// The grid ID at the center of the map.
pub const CENTER_GRID_ID: u32 = 32;

/// Size of one *cell* inside a grid tile (GRID_SIZE / 8).
pub const CELL_SIZE: f32 = GRID_SIZE / 8.0;

/// Total number of cells along one axis (64 grids × 8 cells each = 512).
pub const MAX_CELL_COUNT: u32 = MAX_GRID_COUNT * 8;

/// Sentinel value used to represent "no valid height".
pub const INVALID_HEIGHT: f32 = -100000.0;

// ---------------------------------------------------------------------------
// Angle helpers
// ---------------------------------------------------------------------------

/// Normalize an angle into the range `[0, 2π)`.
///
/// Handles negative angles and angles greater than 2π.
#[inline]
pub fn normalize_angle(angle: f32) -> f32 {
    let mut a = angle % TAU;
    if a < 0.0 {
        a += TAU;
    }
    a
}

// ---------------------------------------------------------------------------
// Distance helpers
// ---------------------------------------------------------------------------

/// Euclidean distance between two points projected onto the XZ plane (2D).
///
/// WoW uses an XZY coordinate system where Y is the vertical axis, so a "2D
/// distance" typically ignores Y.
#[inline]
pub fn distance_2d(a: Vec3, b: Vec3) -> f32 {
    let dx = a.x - b.x;
    let dz = a.z - b.z;
    (dx * dx + dz * dz).sqrt()
}

/// Squared Euclidean distance between two points in full 3D space.
///
/// Useful for distance comparisons without the cost of a square root.
#[inline]
pub fn distance_sq(a: Vec3, b: Vec3) -> f32 {
    let d = a - b;
    d.x * d.x + d.y * d.y + d.z * d.z
}

// ---------------------------------------------------------------------------
// Map coordinate helpers
// ---------------------------------------------------------------------------

/// Convert a world-space position to the grid indices (column, row) inside the
/// 64×64 grid.
///
/// The map spans `[-MAP_HALFSIZE, +MAP_HALFSIZE]` in both X and Y.
/// Grid index 0 corresponds to the positive edge and 63 to the negative edge,
/// matching the TrinityCore convention.
///
/// # Panics
///
/// Does **not** panic, but callers should verify the input is in range with
/// [`is_valid_map_coord`] first.  Out-of-range inputs are clamped to `[0, 63]`.
#[inline]
pub fn world_to_grid(x: f32, y: f32) -> (u32, u32) {
    let grid_x = ((MAP_HALFSIZE - x) / GRID_SIZE) as u32;
    let grid_y = ((MAP_HALFSIZE - y) / GRID_SIZE) as u32;

    let grid_x = grid_x.min(MAX_GRID_COUNT - 1);
    let grid_y = grid_y.min(MAX_GRID_COUNT - 1);

    (grid_x, grid_y)
}

/// Given a grid index and a world-space coordinate, return the cell index
/// (0–7) within that grid for both axes.
///
/// Each grid tile is divided into 8×8 cells.
#[inline]
pub fn grid_to_cell(grid_x: u32, grid_y: u32, x: f32, y: f32) -> (u32, u32) {
    let grid_origin_x = MAP_HALFSIZE - (grid_x as f32 * GRID_SIZE);
    let grid_origin_y = MAP_HALFSIZE - (grid_y as f32 * GRID_SIZE);

    let cell_x = ((grid_origin_x - x) / CELL_SIZE) as u32;
    let cell_y = ((grid_origin_y - y) / CELL_SIZE) as u32;

    let cell_x = cell_x.min(7);
    let cell_y = cell_y.min(7);

    (cell_x, cell_y)
}

/// Returns `true` if `(x, y)` falls inside the valid map area.
#[inline]
pub fn is_valid_map_coord(x: f32, y: f32) -> bool {
    x.is_finite()
        && y.is_finite()
        && x.abs() <= MAP_HALFSIZE
        && y.abs() <= MAP_HALFSIZE
}

/// Returns `true` if `(x, y, z)` falls inside the valid map area and `z` is
/// reasonable.
#[inline]
pub fn is_valid_map_coord_z(x: f32, y: f32, z: f32) -> bool {
    is_valid_map_coord(x, y) && z.is_finite() && z.abs() <= MAP_HALFSIZE
}

// ---------------------------------------------------------------------------
// BoundingBox
// ---------------------------------------------------------------------------

/// Axis-aligned bounding box defined by its minimum and maximum corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    /// The minimum corner (smallest x, y, z).
    pub min: Vec3,
    /// The maximum corner (largest x, y, z).
    pub max: Vec3,
}

impl BoundingBox {
    /// Create a new bounding box from two corner points.
    #[inline]
    pub fn new(min: Vec3, max: Vec3) -> Self {
        Self { min, max }
    }

    /// Returns `true` if `point` is inside this bounding box (inclusive).
    #[inline]
    pub fn contains(&self, point: Vec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// Returns `true` if this box overlaps with `other` on all three axes.
    #[inline]
    pub fn intersects(&self, other: &BoundingBox) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::{FRAC_PI_2, PI, TAU};

    // -- normalize_angle ---------------------------------------------------

    #[test]
    fn normalize_angle_positive_within_range() {
        let a = normalize_angle(1.0);
        assert!((a - 1.0).abs() < 1e-6);
    }

    #[test]
    fn normalize_angle_zero() {
        let a = normalize_angle(0.0);
        assert!(a.abs() < 1e-6);
    }

    #[test]
    fn normalize_angle_full_turn() {
        // TAU should wrap to ~0
        let a = normalize_angle(TAU);
        assert!(a < 1e-4, "expected ~0, got {a}");
    }

    #[test]
    fn normalize_angle_negative() {
        let a = normalize_angle(-FRAC_PI_2);
        let expected = TAU - FRAC_PI_2;
        assert!(
            (a - expected).abs() < 1e-5,
            "expected {expected}, got {a}"
        );
    }

    #[test]
    fn normalize_angle_large_positive() {
        let a = normalize_angle(TAU * 3.0 + 1.0);
        assert!((a - 1.0).abs() < 1e-4, "expected ~1.0, got {a}");
    }

    #[test]
    fn normalize_angle_large_negative() {
        let a = normalize_angle(-TAU * 2.0 - PI);
        let expected = PI;
        assert!(
            (a - expected).abs() < 1e-4,
            "expected {expected}, got {a}"
        );
    }

    // -- distance ----------------------------------------------------------

    #[test]
    fn distance_2d_same_point() {
        let p = Vec3::new(1.0, 2.0, 3.0);
        assert!((distance_2d(p, p)).abs() < 1e-6);
    }

    #[test]
    fn distance_2d_ignores_y() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(3.0, 999.0, 4.0);
        let d = distance_2d(a, b);
        assert!(
            (d - 5.0).abs() < 1e-5,
            "expected 5.0, got {d}"
        );
    }

    #[test]
    fn distance_sq_basic() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 6.0, 3.0);
        // (3^2 + 4^2 + 0^2) = 25
        let d = distance_sq(a, b);
        assert!(
            (d - 25.0).abs() < 1e-5,
            "expected 25.0, got {d}"
        );
    }

    // -- world_to_grid / grid_to_cell --------------------------------------

    #[test]
    fn world_to_grid_center() {
        // World origin (0, 0) → center grid (32, 32)
        let (gx, gy) = world_to_grid(0.0, 0.0);
        assert_eq!(gx, CENTER_GRID_ID);
        assert_eq!(gy, CENTER_GRID_ID);
    }

    #[test]
    fn world_to_grid_positive_edge() {
        // Just inside the positive edge → grid 0
        let (gx, _) = world_to_grid(MAP_HALFSIZE - 1.0, 0.0);
        assert_eq!(gx, 0);
    }

    #[test]
    fn world_to_grid_negative_edge() {
        // Just inside the negative edge → grid 63
        let (gx, _) = world_to_grid(-MAP_HALFSIZE + 1.0, 0.0);
        assert_eq!(gx, MAX_GRID_COUNT - 1);
    }

    #[test]
    fn grid_to_cell_origin() {
        let (gx, gy) = world_to_grid(0.0, 0.0);
        let (cx, cy) = grid_to_cell(gx, gy, 0.0, 0.0);
        // The origin sits at the edge of grid 32 so cell should be 0
        assert!(cx <= 7);
        assert!(cy <= 7);
    }

    // -- is_valid_map_coord ------------------------------------------------

    #[test]
    fn valid_origin() {
        assert!(is_valid_map_coord(0.0, 0.0));
    }

    #[test]
    fn valid_edge() {
        assert!(is_valid_map_coord(MAP_HALFSIZE, MAP_HALFSIZE));
    }

    #[test]
    fn invalid_outside() {
        assert!(!is_valid_map_coord(MAP_HALFSIZE + 1.0, 0.0));
    }

    #[test]
    fn invalid_nan() {
        assert!(!is_valid_map_coord(f32::NAN, 0.0));
    }

    #[test]
    fn invalid_inf() {
        assert!(!is_valid_map_coord(f32::INFINITY, 0.0));
    }

    #[test]
    fn valid_z_origin() {
        assert!(is_valid_map_coord_z(0.0, 0.0, 0.0));
    }

    #[test]
    fn invalid_z_nan() {
        assert!(!is_valid_map_coord_z(0.0, 0.0, f32::NAN));
    }

    #[test]
    fn invalid_z_too_large() {
        assert!(!is_valid_map_coord_z(0.0, 0.0, MAP_HALFSIZE + 1.0));
    }

    // -- BoundingBox -------------------------------------------------------

    #[test]
    fn bbox_contains_inside() {
        let bb = BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        assert!(bb.contains(Vec3::new(0.5, 0.5, 0.5)));
    }

    #[test]
    fn bbox_contains_on_edge() {
        let bb = BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        assert!(bb.contains(Vec3::ZERO));
        assert!(bb.contains(Vec3::ONE));
    }

    #[test]
    fn bbox_does_not_contain_outside() {
        let bb = BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        assert!(!bb.contains(Vec3::new(1.5, 0.5, 0.5)));
        assert!(!bb.contains(Vec3::new(-0.1, 0.5, 0.5)));
    }

    #[test]
    fn bbox_intersects_overlap() {
        let a = BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        let b = BoundingBox::new(Vec3::splat(0.5), Vec3::splat(1.5));
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
    }

    #[test]
    fn bbox_intersects_touching() {
        let a = BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        let b = BoundingBox::new(Vec3::ONE, Vec3::splat(2.0));
        assert!(a.intersects(&b));
    }

    #[test]
    fn bbox_no_intersect() {
        let a = BoundingBox::new(Vec3::ZERO, Vec3::ONE);
        let b = BoundingBox::new(Vec3::splat(2.0), Vec3::splat(3.0));
        assert!(!a.intersects(&b));
    }

    #[test]
    fn bbox_intersects_contained() {
        let outer = BoundingBox::new(Vec3::ZERO, Vec3::splat(10.0));
        let inner = BoundingBox::new(Vec3::splat(2.0), Vec3::splat(4.0));
        assert!(outer.intersects(&inner));
        assert!(inner.intersects(&outer));
    }

    // -- Constants sanity checks ------------------------------------------

    #[test]
    fn constants_coherent() {
        assert!((MAP_SIZE - GRID_SIZE * MAX_GRID_COUNT as f32).abs() < 1.0);
        assert!((MAP_HALFSIZE - MAP_SIZE / 2.0).abs() < 1.0);
        assert_eq!(MAX_CELL_COUNT, MAX_GRID_COUNT * 8);
        assert!((CELL_SIZE - GRID_SIZE / 8.0).abs() < 1e-3);
    }
}
