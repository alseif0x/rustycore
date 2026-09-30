// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Removal from formation and active-object membership.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub(super) fn remove_creature_from_formation_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<CreatureRemoveFormationOutcomeLikeCpp> {
        let (spawn_id, leader_spawn_id) = self
            .map_object_record(guid)
            .filter(|record| record.kind() == AccessorObjectKind::Creature)
            .and_then(|record| record.creature())
            .filter(|creature| creature.unit().world().object().is_in_world())
            .and_then(|creature| {
                let leader_spawn_id = creature.formation_info_like_cpp()?.leader_spawn_id;
                Some((creature.spawn_id(), leader_spawn_id))
            })?;

        let Some(group) = self
            .creature_group_holder_like_cpp
            .get_mut(&leader_spawn_id)
        else {
            return Some(CreatureRemoveFormationOutcomeLikeCpp {
                guid,
                spawn_id,
                leader_spawn_id: Some(leader_spawn_id),
                had_group: false,
                removed_member: false,
                removed_group: false,
                remaining_members: 0,
            });
        };

        let removed_member = group.remove(&guid);
        let remaining_members = group.len();
        let removed_group = remaining_members == 0;
        if removed_group {
            self.creature_group_holder_like_cpp.remove(&leader_spawn_id);
        }

        Some(CreatureRemoveFormationOutcomeLikeCpp {
            guid,
            spawn_id,
            leader_spawn_id: Some(leader_spawn_id),
            had_group: true,
            removed_member,
            removed_group,
            remaining_members,
        })
    }

    pub fn remove_from_active_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> RemoveFromActiveOutcomeLikeCpp {
        let Some(record) = self.map_object_record(guid) else {
            return RemoveFromActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::MissingRecord,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        };
        if record.kind() == AccessorObjectKind::Player {
            return RemoveFromActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::PlayerUnsupported,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        }
        if !is_active_object_like_cpp(record.kind(), record.object()) {
            return RemoveFromActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::NotActiveObject,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        }

        let location = self.active_respawn_location_like_cpp(guid);
        let removed_from_active_set = self.active_non_players_like_cpp.remove(&guid);
        let unload_lock = location.map(|location| {
            self.mutate_unload_active_lock_for_respawn_location_like_cpp(location, false)
        });
        RemoveFromActiveOutcomeLikeCpp {
            guid,
            status: ActiveNonPlayerMutationStatusLikeCpp::Mutated,
            inserted_in_active_set: false,
            removed_from_active_set,
            spawn_id_zero_or_unsupported: unload_lock.is_none(),
            unload_lock,
        }
    }
}
