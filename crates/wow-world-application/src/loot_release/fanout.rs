// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Durable item-loot fanout publication and detached finalization.

use super::*;
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use wow_packet::packets::loot::LootRemoved;
use wow_world_lifecycle::DurableLootItemFanoutLikeCpp;

/// C++ serializes `StoreLootItem` before a later `LootRelease` and notifies
/// synchronously. The already ordered cohort is the union of the pre-commit and
/// committed player-looting snapshots.
pub fn durable_loot_item_fanout_viewers_like_cpp(
    precommit_viewers: &[ObjectGuid],
    committed_viewers: &[ObjectGuid],
) -> HashSet<ObjectGuid> {
    precommit_viewers
        .iter()
        .chain(committed_viewers)
        .copied()
        .collect()
}

impl LootReleaseCxLikeCpp<'_> {
    /// Publish one committed durable item removal to the cohort that observed
    /// the item before the commit, then finish the detached owner release.
    pub fn publish_durable_loot_item_fanout_like_cpp(
        &mut self,
        route: &DurableLootItemFanoutLikeCpp,
    ) -> bool {
        let Some(committed_snapshot) = route.committed_snapshot.get().filter(|snapshot| {
            snapshot.generation == route.authority_generation
                && snapshot.loot.loot_guid == route.loot_obj
        }) else {
            // Never replace the serialization cut with a later authority
            // sample. The latter may include a viewer that opened after the
            // item commit and already received a response without this slot.
            return false;
        };
        if route
            .published
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return true;
        }

        let viewers = durable_loot_item_fanout_viewers_like_cpp(
            &route.precommit_snapshot.loot.players_looting,
            &committed_snapshot.loot.players_looting,
        );
        let Some(entry) = committed_snapshot
            .loot
            .items
            .iter()
            .find(|entry| entry.loot_list_id == route.loot_list_id)
        else {
            return false;
        };
        let allowed_looters = entry
            .allowed_looters
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        let packet = LootRemoved {
            owner: route.owner_guid,
            loot_obj: route.loot_obj,
            loot_list_id: route.loot_list_id,
        };
        let bytes = packet.to_bytes();
        let mut stale_viewers = Vec::new();

        if route.free_for_all {
            let _ = route.source_send_tx.send(bytes);
        } else {
            for viewer in viewers {
                if !allowed_looters.contains(&viewer) {
                    continue;
                }
                if viewer == route.player_guid {
                    let _ = route.source_send_tx.send(bytes.clone());
                    continue;
                }
                let Some(registry) = route.player_registry.as_ref() else {
                    stale_viewers.push(viewer);
                    continue;
                };
                let Some(registration) =
                    registry.loot_delivery_recipient(viewer, route.map_id, route.instance_id)
                else {
                    stale_viewers.push(viewer);
                    continue;
                };
                if registry
                    .send_current_packet(registration, bytes.clone())
                    .is_err()
                {
                    stale_viewers.push(viewer);
                }
            }
        }

        for viewer in stale_viewers {
            let _ = route
                .authority
                .remove_viewer_if_generation_like_cpp(route.authority_generation, viewer);
        }

        self.owner
            .refresh_owned_loot_summary_like_cpp(route.owner_guid);
        if self.player_guid() == Some(route.player_guid) {
            let _ = self
                .reconcile_represented_loot_cache_like_cpp(route.owner_guid, route.player_guid);
        }
        self.finalize_unviewed_durable_loot_owner_like_cpp(route);
        true
    }

    fn finalize_unviewed_durable_loot_owner_like_cpp(
        &mut self,
        route: &DurableLootItemFanoutLikeCpp,
    ) {
        let same_view_still_open = self
            .loot
            .active_loot_view_authority_like_cpp(route.owner_guid)
            .is_some_and(|authority| authority.shares_storage_like_cpp(&route.authority))
            && self
                .loot
                .active_loot_view_generation_like_cpp(route.owner_guid)
                .is_some_and(|generation| *generation == route.authority_generation);
        if same_view_still_open {
            return;
        }
        if !self
            .represented_owned_loot_authority_like_cpp(route.owner_guid)
            .is_some_and(|authority| authority.shares_storage_like_cpp(&route.authority))
        {
            return;
        }
        let Some(observation) = route
            .authority
            .fully_looted_unviewed_lifecycle_observation_like_cpp()
        else {
            return;
        };
        let Some(snapshot) = route
            .authority
            .snapshot_for_player_like_cpp(route.player_guid)
            .filter(|snapshot| snapshot.generation == route.authority_generation)
        else {
            return;
        };

        self.release_detached_owner_like_cpp(
            route.owner_guid,
            route.player_guid,
            route.authority.clone(),
            route.authority_generation,
            snapshot.loot,
            observation.whole_object_fully_skinned,
            observation.object_generation,
            observation.lifecycle_revision,
        );
    }
}
