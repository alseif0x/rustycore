// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Per-target distance and corpse visibility facts for the canonical Creature query.

use wow_entities::{Corpse, Player, Unit, UnitVisibilityContext, WorldObject};
use wow_map::{Map, NoopGridLifecycle, NoopTerrainGridLoader};

const GHOST_VISIBILITY_GHOST_MASK: u32 = 0x2;

pub(super) fn unit_context(
    player: &Player,
    target: &Unit,
    viewpoint: &WorldObject,
    map: &Map<NoopTerrainGridLoader, NoopGridLifecycle>,
    corpse: Option<&Corpse>,
) -> UnitVisibilityContext {
    let seer = player.unit();
    let seer_world = seer.world();
    let target_world = target.world();
    let sight_range = seer_world.get_sight_range(Some(target_world), map);

    // C++ Object.cpp:1543-1555 checks the target's visibility against the
    // Player's visibility mask (not its detect mask). Finding the corpse
    // enables the corpse distance path even when that corpse is too far to
    // grant the ghost visibility exception.
    let corpse = corpse.filter(|_| {
        target.server_side_ghost_visibility()
            & seer.server_side_ghost_visibility()
            & GHOST_VISIBILITY_GHOST_MASK
            == 0
    });

    let corpse_visibility = corpse.is_some_and(|corpse| {
        corpse
            .world()
            .is_within_dist(seer_world, sight_range, false, true, true)
            && corpse
                .world()
                .is_within_dist(target_world, sight_range, false, true, true)
    });
    let distance_allowed =
        corpse.is_some() || viewpoint.is_within_dist(target_world, sight_range, false, true, true);

    UnitVisibilityContext {
        distance_allowed,
        corpse_visibility,
        player_group_visibility: false,
    }
}

pub(super) fn canonical_viewpoint<'a>(
    player: &'a Player,
    map: &'a Map<NoopTerrainGridLoader, NoopGridLifecycle>,
) -> &'a WorldObject {
    let farsight = player.active_data().farsight_object;
    if !farsight.is_empty()
        && let Some(viewpoint) =
            map.map_object_by_kind(farsight, wow_entities::represented_seer_kinds_like_cpp())
        && viewpoint.object().is_in_world()
        && viewpoint.map_id() == map.map_id()
        && viewpoint.instance_id() == map.instance_id()
    {
        return viewpoint;
    }

    player.unit().world()
}
