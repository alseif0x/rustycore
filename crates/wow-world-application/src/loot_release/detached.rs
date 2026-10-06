// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::*;
use tracing::info;

impl LootReleaseCxLikeCpp<'_> {
    /// C++ durable-claim completion for an owner whose viewers already closed.
    ///
    /// The committed snapshot is replayed into the represented cache and the
    /// same release transitions run, but every guarded map/lifecycle mutation
    /// is serialized under the *unviewed* fully-looted observation, matching
    /// `set_canonical_gameobject_loot_state_if_unviewed_fully_looted_observation`
    /// and the unviewed corpse/creature paths.
    #[allow(clippy::too_many_arguments)]
    pub fn release_detached_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        authority: OwnedLootAuthority,
        selected_generation: u64,
        snapshot_loot: CreatureLoot,
        whole_object_fully_skinned: bool,
        object_generation: u64,
        lifecycle_revision: u64,
    ) {
        self.loot
            .insert_cached_loot_for_owner_like_cpp(owner_guid, snapshot_loot.clone());
        self.loot
            .insert_cached_loot_generation_like_cpp(owner_guid, selected_generation);

        if owner_guid.is_game_object() {
            let release = AuthoritativeLootReleaseLikeCpp {
                authority: authority.clone(),
                selected_generation,
                loot: snapshot_loot,
                whole_object_fully_looted: true,
                whole_object_fully_skinned,
                object_generation,
                lifecycle_revision,
                require_no_viewers: true,
            };
            self.apply_represented_gameobject_loot_release_like_cpp(
                owner_guid,
                player_guid,
                true,
                true,
                Some(&release),
            );
            let _ = self.queue_chest_gameobject_state_refresh_for_same_map_like_cpp(owner_guid);
            self.hide_represented_gameobject_for_player_after_loot_release_like_cpp(owner_guid);
            let go_type = self
                .world_entities
                .represented_gameobject_use_state_like_cpp(owner_guid)
                .and_then(|state| state.go_type)
                .map(u32::from);
            if go_type == Some(GAMEOBJECT_TYPE_GATHERING_NODE) {
                self.send_gathering_node_loot_release_dynamic_flags_update_like_cpp(owner_guid);
            }
            self.loot.remove_cached_loot_for_owner_like_cpp(owner_guid);
            return;
        }

        if owner_guid.is_corpse() {
            self.owner
                .transitions_like_cpp()
                .remove_canonical_corpse_lootable_dynamic_flag_if_unviewed_fully_looted_observation_like_cpp(
                    owner_guid,
                    &authority,
                    object_generation,
                    lifecycle_revision,
                );
            self.loot.remove_cached_loot_for_owner_like_cpp(owner_guid);
            return;
        }

        if !owner_guid.is_creature_or_vehicle() {
            return;
        }

        let corpse_decay_looted_rate = self.stats_inputs.corpse_decay_looted_rate_like_cpp();
        let lifecycle_update = self.owner.finish_unviewed_looted_creature_like_cpp(
            owner_guid,
            whole_object_fully_skinned,
            corpse_decay_looted_rate,
            &authority,
            object_generation,
            lifecycle_revision,
        );
        self.loot.remove_cached_loot_for_owner_like_cpp(owner_guid);
        if let Some((_, values_update)) = lifecycle_update.as_ref() {
            self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                owner_guid,
                values_update,
                Some(&authority),
            );
        }
        let marked = lifecycle_update.and_then(|(marked, _)| marked);
        if let Some((entry, corpse_decay_secs)) = marked {
            info!(
                "Creature {:?} (entry {}) fully looted after durable claim — despawning in {}s",
                owner_guid, entry, corpse_decay_secs
            );
        }
    }
}
