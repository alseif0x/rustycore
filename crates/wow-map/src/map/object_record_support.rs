// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private helpers for map object records, cell membership and removal cleanup.

use super::*;

pub(super) fn is_active_object_like_cpp(kind: AccessorObjectKind, object: &WorldObject) -> bool {
    kind == AccessorObjectKind::Player || object.is_active()
}

pub(super) fn remove_from_map_in_world_eligible_type_like_cpp(kind: AccessorObjectKind) -> bool {
    matches!(
        kind,
        AccessorObjectKind::Player
            | AccessorObjectKind::Creature
            | AccessorObjectKind::Pet
            | AccessorObjectKind::Corpse
            | AccessorObjectKind::GameObject
            | AccessorObjectKind::Transport
    )
}

pub(super) fn remove_list_grid_kind_like_cpp(kind: AccessorObjectKind) -> Option<GridObjectKind> {
    match kind {
        AccessorObjectKind::Creature | AccessorObjectKind::Pet => Some(GridObjectKind::Creature),
        AccessorObjectKind::GameObject | AccessorObjectKind::Transport => {
            Some(GridObjectKind::GameObject)
        }
        AccessorObjectKind::DynamicObject => Some(GridObjectKind::DynamicObject),
        AccessorObjectKind::AreaTrigger => Some(GridObjectKind::AreaTrigger),
        AccessorObjectKind::Corpse => Some(GridObjectKind::Corpse),
        AccessorObjectKind::SceneObject => Some(GridObjectKind::SceneObject),
        AccessorObjectKind::Conversation => Some(GridObjectKind::Conversation),
        AccessorObjectKind::Player => None,
    }
}

pub(super) fn switch_list_unit_kind_like_cpp(kind: AccessorObjectKind) -> bool {
    matches!(kind, AccessorObjectKind::Creature | AccessorObjectKind::Pet)
}

pub(super) fn set_record_temp_world_object_like_cpp(record: ObjectMut<'_>, on: bool) {
    match record.kind() {
        AccessorObjectKind::Creature => {
            if let Some(creature) = record.creature_mut() {
                creature.set_temp_world_object_like_cpp(on);
            }
        }
        AccessorObjectKind::Pet => {
            if let Some(pet) = record.pet_mut() {
                pet.creature_mut().set_temp_world_object_like_cpp(on);
            }
        }
        _ => {}
    }
}

pub(super) fn map_record_is_world_object_like_cpp(record: ObjectRef<'_>) -> bool {
    if record.object().is_world_object() {
        return true;
    }
    if let Some(creature) = record.creature() {
        return creature.is_temp_world_object();
    }
    if let Some(pet) = record.pet() {
        return pet.creature().is_temp_world_object();
    }
    false
}

pub(super) fn cleanup_map_object_record_before_delete_like_cpp(
    record: ObjectMut<'_>,
    kind: AccessorObjectKind,
    creature_second_cleanup: bool,
) -> usize {
    match kind {
        AccessorObjectKind::Creature => record.creature_mut().map_or(0, |creature| {
            if !creature_second_cleanup {
                creature.set_destroyed_object(true);
            }
            creature.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Pet => record.pet_mut().map_or(0, |pet| {
            if !creature_second_cleanup {
                pet.creature_mut().set_destroyed_object(true);
            }
            pet.creature_mut().cleanup_before_delete();
            1
        }),
        AccessorObjectKind::GameObject => record.game_object_mut().map_or(0, |game_object| {
            game_object.set_destroyed_object(true);
            game_object.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Transport => record.transport_mut().map_or(0, |transport| {
            transport.game_object_mut().set_destroyed_object(true);
            let _removed_static_passengers = transport.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::DynamicObject => {
            record.dynamic_object_mut().map_or(0, |dynamic_object| {
                dynamic_object.set_destroyed_object(true);
                dynamic_object.cleanup_before_delete();
                1
            })
        }
        AccessorObjectKind::AreaTrigger => record.area_trigger_mut().map_or(0, |area_trigger| {
            area_trigger.set_destroyed_object(true);
            area_trigger.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Corpse => record.corpse_mut().map_or(0, |corpse| {
            corpse.set_destroyed_object(true);
            corpse.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::SceneObject => record.scene_object_mut().map_or(0, |scene_object| {
            scene_object.set_destroyed_object(true);
            scene_object.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Conversation => record.conversation_mut().map_or(0, |conversation| {
            conversation.set_destroyed_object(true);
            conversation.cleanup_before_delete();
            1
        }),
        AccessorObjectKind::Player => {
            // No typed represented `CleanupsBeforeDelete` exists for Player in this
            // bounded map remove-list seam. Preserve at least the base
            // `WorldObject::SetDestroyedObject(true)` mutation and report no
            // represented cleanup.
            record.object_mut().object_mut().set_destroyed_object(true);
            0
        }
    }
}

pub(super) fn insert_object_guid_in_cell_like_cpp(
    cell: &mut Cell,
    kind: AccessorObjectKind,
    is_world_object: bool,
    guid: ObjectGuid,
) {
    match kind {
        AccessorObjectKind::Player => {
            cell.world_objects.players.insert(guid);
        }
        AccessorObjectKind::Creature | AccessorObjectKind::Pet => {
            if is_world_object {
                cell.world_objects.creatures.insert(guid);
            } else {
                cell.grid_objects.creatures.insert(guid);
            }
        }
        AccessorObjectKind::GameObject | AccessorObjectKind::Transport => {
            cell.grid_objects.gameobjects.insert(guid);
        }
        AccessorObjectKind::DynamicObject => {
            if is_world_object {
                cell.world_objects.dynamic_objects.insert(guid);
            } else {
                cell.grid_objects.dynamic_objects.insert(guid);
            }
        }
        AccessorObjectKind::AreaTrigger => {
            cell.grid_objects.area_triggers.insert(guid);
        }
        AccessorObjectKind::Corpse => {
            if is_world_object {
                cell.world_objects.corpses.insert(guid);
            } else {
                cell.grid_objects.corpses.insert(guid);
            }
        }
        AccessorObjectKind::SceneObject => {
            cell.grid_objects.scene_objects.insert(guid);
        }
        AccessorObjectKind::Conversation => {
            cell.grid_objects.conversations.insert(guid);
        }
    }
}

fn remove_object_guid_from_cell_like_cpp<Terrain, Lifecycle>(
    map: &mut Map<Terrain, Lifecycle>,
    grid: GridCoord,
    cell: &Cell,
    kind: AccessorObjectKind,
    is_world_object: bool,
    guid: ObjectGuid,
) -> bool
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    let Some(ngrid) = map.get_ngrid_mut(grid) else {
        return false;
    };
    let Some(local_cell) = ngrid.get_grid_type_mut(cell.cell_x(), cell.cell_y()) else {
        return false;
    };

    match kind {
        AccessorObjectKind::Player => local_cell.world_objects.players.remove(&guid),
        AccessorObjectKind::Creature | AccessorObjectKind::Pet => {
            if is_world_object {
                local_cell.world_objects.creatures.remove(&guid)
            } else {
                local_cell.grid_objects.creatures.remove(&guid)
            }
        }
        AccessorObjectKind::GameObject | AccessorObjectKind::Transport => {
            local_cell.grid_objects.gameobjects.remove(&guid)
        }
        AccessorObjectKind::DynamicObject => {
            if is_world_object {
                local_cell.world_objects.dynamic_objects.remove(&guid)
            } else {
                local_cell.grid_objects.dynamic_objects.remove(&guid)
            }
        }
        AccessorObjectKind::AreaTrigger => local_cell.grid_objects.area_triggers.remove(&guid),
        AccessorObjectKind::Corpse => {
            if is_world_object {
                local_cell.world_objects.corpses.remove(&guid)
            } else {
                local_cell.grid_objects.corpses.remove(&guid)
            }
        }
        AccessorObjectKind::SceneObject => local_cell.grid_objects.scene_objects.remove(&guid),
        AccessorObjectKind::Conversation => local_cell.grid_objects.conversations.remove(&guid),
    }
}

/// Invalidates async claims before a terminal typed object lifetime is dropped.
///
/// C++ destroys `Creature::loot` / `GameObject::loot` together with the typed
/// object. Rust leases can retain the shared authority after that drop, so the
/// backing allocation must become permanently detached first.
pub(super) fn detach_typed_loot_authority_like_cpp(record: ObjectMut<'_>) {
    match record.kind() {
        AccessorObjectKind::Creature => {
            if let Some(creature) = record.creature_mut() {
                creature.loot_authority_like_cpp().detach_like_cpp();
            }
        }
        AccessorObjectKind::GameObject => {
            if let Some(game_object) = record.game_object_mut() {
                game_object.loot_authority_like_cpp().detach_like_cpp();
            }
        }
        _ => {}
    }
}

/// A same-GUID whole-entity refresh may replace the record while deliberately
/// retaining the exact object lifetime. Do not detach the shared authority in
/// that case; only a distinct backing allocation is displaced terminally.
pub(super) fn typed_loot_authorities_share_storage_like_cpp(
    previous: ObjectRef<'_>,
    replacement: ObjectRef<'_>,
) -> bool {
    match (previous.kind(), replacement.kind()) {
        (AccessorObjectKind::Creature, AccessorObjectKind::Creature) => previous
            .creature()
            .zip(replacement.creature())
            .is_some_and(|(previous, replacement)| {
                previous
                    .loot_authority_like_cpp()
                    .shares_storage_like_cpp(replacement.loot_authority_like_cpp())
            }),
        (AccessorObjectKind::GameObject, AccessorObjectKind::GameObject) => previous
            .game_object()
            .zip(replacement.game_object())
            .is_some_and(|(previous, replacement)| {
                previous
                    .loot_authority_like_cpp()
                    .shares_storage_like_cpp(replacement.loot_authority_like_cpp())
            }),
        _ => false,
    }
}
