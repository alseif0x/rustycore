// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Post-AddToWorld activation, visibility boundary and canonical spawn indexes.

use super::*;

impl<Terrain, Lifecycle> Map<Terrain, Lifecycle>
where
    Terrain: TerrainGridLoader,
    Lifecycle: GridLifecycle,
{
    pub fn add_to_active_like_cpp(&mut self, guid: ObjectGuid) -> AddToActiveOutcomeLikeCpp {
        let Some(record) = self.map_object_record(guid) else {
            return AddToActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::MissingRecord,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        };
        if record.kind() == AccessorObjectKind::Player {
            return AddToActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::PlayerUnsupported,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        }
        if !is_active_object_like_cpp(record.kind(), record.object()) {
            return AddToActiveOutcomeLikeCpp {
                guid,
                status: ActiveNonPlayerMutationStatusLikeCpp::NotActiveObject,
                inserted_in_active_set: false,
                removed_from_active_set: false,
                spawn_id_zero_or_unsupported: false,
                unload_lock: None,
            };
        }

        let location = self.active_respawn_location_like_cpp(guid);
        let inserted_in_active_set = self.active_non_players_like_cpp.insert(guid);
        let unload_lock = location.map(|location| {
            self.mutate_unload_active_lock_for_respawn_location_like_cpp(location, true)
        });
        AddToActiveOutcomeLikeCpp {
            guid,
            status: ActiveNonPlayerMutationStatusLikeCpp::Mutated,
            inserted_in_active_set,
            removed_from_active_set: false,
            spawn_id_zero_or_unsupported: unload_lock.is_none(),
            unload_lock,
        }
    }

    pub(super) fn represent_add_to_map_post_add_to_world_tail_like_cpp(
        &mut self,
        kind: AccessorObjectKind,
        guid: ObjectGuid,
        active_object: bool,
    ) -> Option<AddToMapPostAddToWorldOutcomeLikeCpp> {
        let pending_move_state = match kind {
            AccessorObjectKind::Creature => {
                if self
                    .map_object_record(guid)
                    .is_some_and(|record| record.creature().is_some())
                {
                    self.creature_move_states.remove(&guid)
                } else {
                    return None;
                }
            }
            AccessorObjectKind::GameObject => {
                if self
                    .map_object_record(guid)
                    .is_some_and(|record| record.game_object().is_some())
                {
                    self.gameobject_move_states.remove(&guid)
                } else {
                    return None;
                }
            }
            _ => return None,
        };

        if pending_move_state.is_some() {
            match kind {
                AccessorObjectKind::Creature => {
                    self.creatures_to_move.retain(|queued| *queued != guid)
                }
                AccessorObjectKind::GameObject => {
                    self.gameobjects_to_move.retain(|queued| *queued != guid);
                }
                _ => {}
            }
        }

        let add_to_active = active_object.then(|| self.add_to_active_like_cpp(guid));

        let mut set_true = false;
        let mut set_false = false;
        let final_is_new_object = if let Some(mut record) = self.entity_world.get_mut(&guid) {
            record.reborrow().object_mut().object_mut().set_is_new_object(true);
            set_true = true;
            record.reborrow().object_mut().object_mut().set_is_new_object(false);
            set_false = true;
            record.object().object().is_new_object()
        } else {
            false
        };

        Some(AddToMapPostAddToWorldOutcomeLikeCpp {
            initialize_object_represented: true,
            pending_move_state_cleared: pending_move_state.is_some(),
            no_pending_move_state: pending_move_state.is_none(),
            add_to_active_represented: add_to_active.is_some(),
            add_to_active_skipped_runtime_gap: false,
            add_to_active,
            set_is_new_object_true: set_true,
            update_object_visibility_on_create_represented: true,
            update_object_visibility_on_create_runtime_gap: true,
            set_is_new_object_false: set_false,
            final_is_new_object,
        })
    }

    pub(in crate::map) fn index_map_object_record_by_spawn_id_like_cpp(&mut self, record: ObjectRef<'_>) {
        if let Some(creature) = record.creature() {
            let spawn_id = creature.spawn_id();
            if spawn_id != 0 {
                self.creatures_by_spawn_id
                    .entry(spawn_id)
                    .or_default()
                    .insert(creature.guid());
            }
            return;
        }

        if let Some(gameobject) = record.game_object() {
            let spawn_id = gameobject.spawn_id();
            if spawn_id != 0 {
                self.gameobjects_by_spawn_id
                    .entry(spawn_id)
                    .or_default()
                    .insert(gameobject.world().guid());
            }
            return;
        }

        if let Some(area_trigger) = record.area_trigger() {
            let spawn_id = area_trigger.spawn_id();
            if spawn_id != 0 {
                self.area_triggers_by_spawn_id
                    .entry(spawn_id)
                    .or_default()
                    .insert(area_trigger.world().guid());
            }
        }
    }

    pub(in crate::map) fn unindex_map_object_record_by_spawn_id_like_cpp(&mut self, record: ObjectRef<'_>) {
        if let Some(creature) = record.creature() {
            remove_spawn_id_index_entry_like_cpp(
                &mut self.creatures_by_spawn_id,
                creature.spawn_id(),
                creature.guid(),
            );
            return;
        }

        if let Some(gameobject) = record.game_object() {
            remove_spawn_id_index_entry_like_cpp(
                &mut self.gameobjects_by_spawn_id,
                gameobject.spawn_id(),
                gameobject.world().guid(),
            );
            return;
        }

        if let Some(area_trigger) = record.area_trigger() {
            remove_spawn_id_index_entry_like_cpp(
                &mut self.area_triggers_by_spawn_id,
                area_trigger.spawn_id(),
                area_trigger.world().guid(),
            );
        }
    }

    pub fn active_non_players_count_like_cpp(&self) -> usize {
        self.active_non_players_like_cpp.len()
    }

    pub fn is_active_non_player_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.active_non_players_like_cpp.contains(&guid)
    }

    pub(in crate::map) fn represented_active_non_player_sources_like_cpp(&self) -> Vec<ObjectGuid> {
        let mut guids: Vec<_> = self
            .active_non_players_like_cpp
            .iter()
            .copied()
            .filter(|guid| self.object_is_in_world(*guid))
            .collect();
        sort_dedup(&mut guids);
        guids
    }

}
