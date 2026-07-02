//! Static VMap line-of-sight adapter.
//!
//! C++ anchors:
//! - `Map::isInLineOfSight` (`server/game/Maps/Map.cpp`) checks static VMAP
//!   first and rejects immediately when it is blocked.
//! - `VMapManager2::isInLineOfSight` (`common/Collision/Management/VMapManager2.cpp`)
//!   converts world coordinates to the VMAP internal representation before
//!   querying `StaticMapTree::isInLineOfSight`.
//! - `StaticMapTree::isInLineOfSight` (`common/Collision/Maps/MapTree.cpp`)
//!   returns false when a BIH ray intersection hits blocking geometry.

use std::fmt;

use wow_core::Position;

/// C++ `VMapManager2::convertPositionToInternalRep`: half of the 64x64 grid map.
const VMAP_INTERNAL_MID: f32 = 0.5 * 64.0 * 533.333_333_33;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VMapPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl VMapPosition {
    #[must_use]
    pub const fn from_world(position: Position) -> Self {
        Self {
            x: position.x,
            y: position.y,
            z: position.z,
        }
    }

    #[must_use]
    pub fn to_internal_rep_like_cpp(self) -> Self {
        Self {
            x: VMAP_INTERNAL_MID - self.x,
            y: VMAP_INTERNAL_MID - self.y,
            z: self.z,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VMapLineOfSightEndpoint {
    pub world: VMapPosition,
    pub internal: VMapPosition,
}

impl VMapLineOfSightEndpoint {
    #[must_use]
    pub fn from_world(position: Position) -> Self {
        let world = VMapPosition::from_world(position);
        Self {
            world,
            internal: world.to_internal_rep_like_cpp(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VMapLineOfSightQuery {
    pub map_id: u32,
    pub from: VMapLineOfSightEndpoint,
    pub to: VMapLineOfSightEndpoint,
}

impl VMapLineOfSightQuery {
    #[must_use]
    pub fn from_world(map_id: u32, from: Position, to: Position) -> Self {
        Self {
            map_id,
            from: VMapLineOfSightEndpoint::from_world(from),
            to: VMapLineOfSightEndpoint::from_world(to),
        }
    }

    #[must_use]
    pub fn same_internal_position_like_cpp(&self) -> bool {
        self.from.internal == self.to.internal
    }
}

/// Static VMAP LOS provider. This is intentionally narrower than the full C++
/// `VMapManager2`: loading, model ownership, height, liquid, and object-hit
/// position remain separate future slices.
pub trait StaticVMapLineOfSightProvider: fmt::Debug + Send + Sync {
    fn is_in_line_of_sight(&self, query: VMapLineOfSightQuery) -> bool;
}

pub type SharedStaticVMapLineOfSightProvider = std::sync::Arc<dyn StaticVMapLineOfSightProvider>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vmap_los_query_converts_world_coordinates_to_internal_rep_like_cpp() {
        let query = VMapLineOfSightQuery::from_world(
            0,
            Position::new(100.0, -200.0, 30.0, 1.0),
            Position::new(-50.0, 25.0, 40.0, 2.0),
        );

        assert_eq!(query.map_id, 0);
        assert_eq!(
            query.from.world,
            VMapPosition {
                x: 100.0,
                y: -200.0,
                z: 30.0
            }
        );
        assert!((query.from.internal.x - (VMAP_INTERNAL_MID - 100.0)).abs() < f32::EPSILON);
        assert!((query.from.internal.y - (VMAP_INTERNAL_MID + 200.0)).abs() < f32::EPSILON);
        assert_eq!(query.from.internal.z, 30.0);
        assert!((query.to.internal.x - (VMAP_INTERNAL_MID + 50.0)).abs() < f32::EPSILON);
        assert!((query.to.internal.y - (VMAP_INTERNAL_MID - 25.0)).abs() < f32::EPSILON);
        assert_eq!(query.to.internal.z, 40.0);
    }

    #[test]
    fn same_internal_position_matches_cpp_vmap_manager_short_circuit() {
        let query = VMapLineOfSightQuery::from_world(
            1,
            Position::new(1.0, 2.0, 3.0, 0.0),
            Position::new(1.0, 2.0, 3.0, 5.0),
        );

        assert!(query.same_internal_position_like_cpp());
    }
}
