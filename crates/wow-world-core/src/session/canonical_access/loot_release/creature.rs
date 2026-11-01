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

/// The one corpse lifecycle transition shared by the viewed and unviewed
/// release completions.
fn looted_creature_lifecycle_like_cpp(
    whole_object_fully_skinned: bool,
    corpse_decay_looted_rate: f32,
    fanout_publishes: bool,
) -> impl FnOnce(&mut wow_entities::Creature) -> (Option<(u32, u32)>, wow_entities::UnitValuesUpdate)
{
    move |creature: &mut wow_entities::Creature| {
        crate::creature_projection::remove_lootable_dynamic_flag_like_cpp(creature);
        let marked = if !creature.is_alive() {
            let corpse_decay_secs = looted_corpse_decay_secs_like_cpp(
                whole_object_fully_skinned,
                creature.corpse_delay(),
                creature.ignore_corpse_decay_ratio(),
                corpse_decay_looted_rate,
            );
            if !crate::creature_projection::all_loot_removed_from_corpse_like_cpp(
                creature,
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
        (
            marked,
            loot_release_values_for_fanout_like_cpp(creature, fanout_publishes),
        )
    }
}

/// The creature VALUES a loot release fans out to every viewer.
///
/// #1263 F6-8D3b-3: the fanout (`SendCreatureLootReleaseValuesUpdateLikeCpp`)
/// is the one publisher of the viewer-dependent DynamicFlags it carries (C++
/// `ForceUpdateFieldChange(DynamicFlags)` → one per-viewer VALUES at the next
/// `Map::SendObjectUpdates`). Since the root mutates the canonical incarnation
/// (D3b-2), its DynamicFlags mark would otherwise make the map's VALUES step
/// republish the same field, raw, to the same clients.
///
/// #1263 F6-8E: the mark is taken only when the fanout publishes to every
/// viewer, which needs the object-owned authority (each queued receiver
/// checks the update against it and refuses an authority-less one). Without
/// it the mark stays, and the map's VALUES step publishes the forced field to
/// every viewer, as C++ `Map::SendObjectUpdates` does; the authority-less
/// viewer projection preserves the canonical flag, the same value.
pub(crate) fn loot_release_values_for_fanout_like_cpp(
    creature: &mut wow_entities::Creature,
    fanout_publishes: bool,
) -> wow_entities::UnitValuesUpdate {
    let update = creature.unit().values_update();
    if fanout_publishes {
        creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .take_dynamic_flags_change_like_cpp();
    }
    update
}

impl LootReleaseOwnerAccessLikeCpp<'_> {
    pub fn canonical_creature_is_fully_looted_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<bool> {
        self.core
            .mutate_canonical_creature_by_guid_like_cpp(guid, |creature| {
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
        let apply_lifecycle = looted_creature_lifecycle_like_cpp(
            whole_object_fully_skinned,
            corpse_decay_looted_rate,
            observation.is_some(),
        );
        if let Some((authority, object_generation, lifecycle_revision)) = observation {
            self.core
                .mutate_world_creature_if_fully_looted_observation_like_cpp(
                    guid,
                    authority,
                    object_generation,
                    lifecycle_revision,
                    apply_lifecycle,
                )
        } else {
            self.core.mutate_world_creature(guid, apply_lifecycle)
        }
    }

    /// Durable-claim completion for an owner whose last viewer already closed:
    /// the same corpse lifecycle transition, serialized under the unviewed
    /// fully-looted observation instead of the viewed one.
    pub fn finish_unviewed_looted_creature_like_cpp(
        &mut self,
        guid: ObjectGuid,
        whole_object_fully_skinned: bool,
        corpse_decay_looted_rate: f32,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
    ) -> Option<(Option<(u32, u32)>, wow_entities::UnitValuesUpdate)> {
        self.core
            .mutate_world_creature_if_unviewed_fully_looted_observation_like_cpp(
                guid,
                authority,
                object_generation,
                lifecycle_revision,
                looted_creature_lifecycle_like_cpp(
                    whole_object_fully_skinned,
                    corpse_decay_looted_rate,
                    true,
                ),
            )
    }
}
