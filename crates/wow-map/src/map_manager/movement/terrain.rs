//! Terrain operations of movement.
//!
//! Height normalization shares the private engine's lazy scalar stages.

use super::*;

impl WorldCreature {
    /// C++ `PathGenerator::CreateFilter` + `PathGenerator::UpdateFilter`
    /// (`PathGenerator.cpp:648-698`) derive the Detour query filter from the
    /// *owner*, never from a constant: `Creature::CanWalk()` adds `NAV_GROUND`,
    /// `Creature::CanEnterWater()` adds `NAV_WATER | NAV_MAGMA_SLIME`, and
    /// `Unit::IsInCombat() || Creature::IsInEvadeMode()` adds
    /// `NAV_GROUND_STEEP`.
    ///
    /// Boundary: `UpdateFilter` also ORs in
    /// `Map::GetForceEnabled/DisabledNavMeshFilterFlags()` and, while the owner
    /// `IsInWater()/IsUnderWater()`, `GetNavTerrain()` from
    /// `Map::GetLiquidStatus`. Neither map-level source exists in the Rust
    /// runtime yet, so those stay at their neutral values here.
    pub fn path_query_filter_context_like_cpp(&self) -> PathQueryFilterContext {
        // C++ `Unit::IsInCombat()` is `HasUnitFlag(UNIT_FLAG_IN_COMBAT)`, and C++
        // really does set that flag on entering combat. RustyCore's
        // `Creature::enter_ai_combat` sets the AI state and the attacking GUID
        // but not the client-visible flag, so reading the flag alone would leave
        // every chasing creature without `NAV_GROUND_STEEP`. Both signals are
        // consulted, so the filter is correct today and still correct once the
        // flag itself is maintained.
        //
        // Boundary: that missing `UNIT_FLAG_IN_COMBAT` is a separate parity
        // defect with client-visible UpdateField consequences; it is not fixed
        // here.
        let in_combat = self
            .creature
            .unit()
            .unit_flags_like_cpp()
            .contains(wow_constants::unit::UnitFlags::IN_COMBAT)
            || self.creature.is_in_combat();
        PathQueryFilterContext::creature(
            self.creature.can_walk_like_cpp(),
            self.creature.can_enter_water_like_cpp(),
            in_combat,
            self.creature.is_in_evade_mode_like_cpp(),
        )
    }

    pub(super) fn allowed_position_z_caps_like_cpp(&self) -> AllowedPositionZCaps {
        let hover_offset = if self
            .creature
            .movement_flags_like_cpp()
            .contains(MovementFlag::HOVER)
        {
            self.creature.unit().data().hover_height
        } else {
            0.0
        };
        AllowedPositionZCaps {
            on_transport: false,
            can_fly: self.creature.can_fly_like_cpp(),
            can_swim: self.creature.can_swim_like_cpp(),
            hover_offset,
        }
    }

    pub(in crate::map_manager) fn normalize_path_position_z_like_cpp(
        &self,
        point: Position,
        terrain: Option<&LiveTerrainHeights>,
    ) -> Position {
        pending::normalize_position_sync(self, point, terrain)
    }

    pub(super) fn path_generator_from_detour_for_creature_like_cpp(
        &self,
        destination: Position,
        detour_path: &DetourPolyPath,
        force_destination: bool,
        terrain: Option<&LiveTerrainHeights>,
    ) -> PathGenerator {
        path_generator_from_detour_with_normalizer_like_cpp(
            self.position(),
            destination,
            detour_path,
            force_destination,
            |point| self.normalize_path_position_z_like_cpp(point, terrain),
        )
    }
}
