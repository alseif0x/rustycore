// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Deterministic position resolution for GameObject summons.

use super::DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP;
use crate::coords::normalize_map_coord;
use wow_core::Position;

/// Position resolver for C++ `WorldObject::SummonGameObject(entry, x, y, z, ang, ...)`.
///
/// C++ anchors:
/// - `Object.cpp:2096-2105`: if `x == y == z == 0`, call
///   `GetClosePoint(x, y, z, GetCombatReach())` and use the summoner orientation.
/// - `Object.cpp:3341-3408`: `GetClosePoint` delegates to `GetNearPoint(nullptr,
///   distance2d + size, orientation)`, whose 2D calculation adds the summoner
///   combat reach again when `searcher == nullptr`.
///
/// Scope: this represents the deterministic 2D coordinate/orientation branch
/// and map-coordinate normalization. Height correction, collision detection,
/// LOS fallback search and map-vmap terrain queries remain runtime gaps.
pub fn world_object_summon_gameobject_position_from_coords_like_cpp(
    summoner_position: Position,
    summoner_combat_reach: f32,
    x: f32,
    y: f32,
    z: f32,
    angle: f32,
) -> WorldObjectSummonGameObjectPositionOutcomeLikeCpp {
    if x == 0.0 && y == 0.0 && z == 0.0 {
        let reach = summoner_combat_reach.max(0.0);
        let distance = reach + reach;
        let mut resolved_x = summoner_position.x + distance * summoner_position.orientation.cos();
        let mut resolved_y = summoner_position.y + distance * summoner_position.orientation.sin();
        let before_normalize_x = resolved_x;
        let before_normalize_y = resolved_y;
        normalize_map_coord(&mut resolved_x);
        normalize_map_coord(&mut resolved_y);
        return WorldObjectSummonGameObjectPositionOutcomeLikeCpp {
            position: Position::new(
                resolved_x,
                resolved_y,
                summoner_position.z,
                summoner_position.orientation,
            ),
            close_point_fallback_used: true,
            normalized_map_coords: resolved_x != before_normalize_x
                || resolved_y != before_normalize_y,
            collision_los_adjustment_represented: false,
        };
    }

    WorldObjectSummonGameObjectPositionOutcomeLikeCpp {
        position: Position::new(x, y, z, angle),
        close_point_fallback_used: false,
        normalized_map_coords: false,
        collision_los_adjustment_represented: false,
    }
}

/// Position resolver for C++ `Spell::EffectSummonObjectWild`.
///
/// C++ anchors:
/// - `SpellEffects.cpp:2946-2954`: explicit destination wins; otherwise call
///   `m_caster->GetClosePoint(..., DEFAULT_PLAYER_BOUNDING_RADIUS)` and use
///   `target->GetOrientation()`.
/// - `ObjectDefines.h:39`: `DEFAULT_PLAYER_BOUNDING_RADIUS`.
/// - `Object.cpp:3341-3408`: `GetClosePoint` delegates to `GetNearPoint`
///   with `searcher == nullptr`, so 2D distance is caster combat reach plus
///   the provided size.
///
/// Scope: this represents deterministic 2D fallback and map-coordinate
/// normalization. `focusObject` selection, height correction, collision, LOS
/// search and terrain queries remain caller/runtime gaps.
pub fn spell_effect_summon_object_wild_position_like_cpp(
    caster_position: Position,
    caster_combat_reach: f32,
    target_orientation: f32,
    explicit_destination: Option<Position>,
) -> SpellEffectSummonObjectWildPositionOutcomeLikeCpp {
    if let Some(position) = explicit_destination {
        return SpellEffectSummonObjectWildPositionOutcomeLikeCpp {
            position,
            explicit_destination_used: true,
            close_point_fallback_used: false,
            normalized_map_coords: false,
            focus_object_orientation_represented: target_orientation != caster_position.orientation,
            collision_los_adjustment_represented: false,
        };
    }

    let distance = caster_combat_reach.max(0.0) + DEFAULT_PLAYER_BOUNDING_RADIUS_LIKE_CPP;
    let mut resolved_x = caster_position.x + distance * caster_position.orientation.cos();
    let mut resolved_y = caster_position.y + distance * caster_position.orientation.sin();
    let before_normalize_x = resolved_x;
    let before_normalize_y = resolved_y;
    normalize_map_coord(&mut resolved_x);
    normalize_map_coord(&mut resolved_y);

    SpellEffectSummonObjectWildPositionOutcomeLikeCpp {
        position: Position::new(
            resolved_x,
            resolved_y,
            caster_position.z,
            target_orientation,
        ),
        explicit_destination_used: false,
        close_point_fallback_used: true,
        normalized_map_coords: resolved_x != before_normalize_x || resolved_y != before_normalize_y,
        focus_object_orientation_represented: target_orientation != caster_position.orientation,
        collision_los_adjustment_represented: false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpellEffectSummonObjectWildPositionOutcomeLikeCpp {
    pub position: Position,
    pub explicit_destination_used: bool,
    pub close_point_fallback_used: bool,
    pub normalized_map_coords: bool,
    pub focus_object_orientation_represented: bool,
    pub collision_los_adjustment_represented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorldObjectSummonGameObjectPositionOutcomeLikeCpp {
    pub position: Position,
    pub close_point_fallback_used: bool,
    pub normalized_map_coords: bool,
    pub collision_los_adjustment_represented: bool,
}
