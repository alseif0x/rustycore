// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::LootReleaseOwnerAccessLikeCpp;
use wow_core::ObjectGuid;
use wow_loot::OwnedLootAuthority;

pub fn looted_corpse_decay_secs_like_cpp(
    is_fully_skinned: bool,
    corpse_delay_secs: u32,
    ignore_decay_ratio: bool,
    corpse_decay_looted_rate: f32,
) -> u32 {
    if is_fully_skinned {
        return 0;
    }

    let rate = if ignore_decay_ratio {
        1.0
    } else {
        corpse_decay_looted_rate.max(0.0)
    };
    ((corpse_delay_secs as f32) * rate) as u32
}

impl LootReleaseOwnerAccessLikeCpp<'_> {
    pub fn canonical_creature_is_fully_looted_like_cpp(&mut self, guid: ObjectGuid) -> Option<bool> {
        self.core.mutate_canonical_creature_by_guid_like_cpp(guid, |creature| {
            creature.is_fully_looted_like_cpp()
        })
    }

    pub fn finish_looted_creature_like_cpp(
        &mut self,
        guid: ObjectGuid,
        whole_object_fully_skinned: bool,
        corpse_decay_looted_rate: f32,
        observation: Option<(&OwnedLootAuthority, u64, u64)>,
    ) -> Option<(Option<(u32, u32)>, wow_entities::UnitValuesUpdate)> {
        let apply_lifecycle = |creature: &mut crate::map_manager::WorldCreature| {
            creature.remove_lootable_dynamic_flag_like_cpp();
            let marked = if !creature.is_alive() {
                let corpse_decay_secs = looted_corpse_decay_secs_like_cpp(
                    whole_object_fully_skinned,
                    creature.corpse_delay_secs_like_cpp(),
                    creature.ignore_corpse_decay_ratio_like_cpp(),
                    corpse_decay_looted_rate,
                );
                if !creature.all_loot_removed_from_corpse_like_cpp(
                    corpse_decay_looted_rate,
                    whole_object_fully_skinned,
                ) {
                    // C++ returns without resetting an already-expired
                    // corpse. The lifecycle mirror must remain expired too.
                    None
                } else {
                    Some((creature.entry(), corpse_decay_secs))
                }
            } else {
                None
            };
            (marked, creature.creature.unit().values_update())
        };
        if let Some((authority, object_generation, lifecycle_revision)) = observation {
            self.mutate_world_creature_if_fully_looted_observation_like_cpp(
                guid, authority, object_generation, lifecycle_revision, apply_lifecycle,
            )
        } else {
            self.core.mutate_world_creature(guid, apply_lifecycle)
        }
    }

    fn mutate_world_creature_if_fully_looted_observation_like_cpp<F, R>(
        &mut self,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut crate::map_manager::WorldCreature) -> R,
    {
        let (map_id, instance_id) = self.core.current_legacy_runtime_map_key_like_cpp();
        let manager = self.core.map_manager.as_ref().cloned()?;
        let guarded_result = {
            let mut manager = manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let creature = manager.find_creature_mut(map_id, instance_id, guid)?;
            if !creature
                .creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(authority)
            {
                return None;
            }
            authority.with_fully_looted_lifecycle_observation_like_cpp(
                object_generation,
                lifecycle_revision,
                || {
                    let result = f(creature);
                    (result, creature.creature.clone())
                },
            )
        }?;
        let (result, creature) = guarded_result;
        self.core.sync_canonical_creature_entity_like_cpp(creature);
        Some(result)
    }

    fn mutate_world_creature_if_unviewed_fully_looted_observation_like_cpp<F, R>(
        &mut self,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut crate::map_manager::WorldCreature) -> R,
    {
        let (map_id, instance_id) = self.core.current_legacy_runtime_map_key_like_cpp();
        let manager = self.core.map_manager.as_ref().cloned()?;
        let guarded_result = {
            let mut manager = manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let creature = manager.find_creature_mut(map_id, instance_id, guid)?;
            if !creature
                .creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(authority)
            {
                return None;
            }
            authority.with_unviewed_fully_looted_lifecycle_observation_like_cpp(
                object_generation,
                lifecycle_revision,
                || {
                    let result = f(creature);
                    (result, creature.creature.clone())
                },
            )
        }?;
        let (result, creature) = guarded_result;
        self.core.sync_canonical_creature_entity_like_cpp(creature);
        Some(result)
    }
}
