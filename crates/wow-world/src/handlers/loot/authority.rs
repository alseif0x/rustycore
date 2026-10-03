// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot permission, recipient eligibility and master-loot authority.

use super::*;

impl WorldSession {
    pub(super) fn represented_loot_authority_pools_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        personal: bool,
    ) -> Option<(Option<CreatureLoot>, HashMap<ObjectGuid, CreatureLoot>)> {
        if !personal {
            return Some((Some(loot), HashMap::new()));
        }

        let mut looters = loot.allowed_looters.clone();
        if looters.is_empty() && !player_guid.is_empty() {
            looters.push(player_guid);
        }
        looters.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
        looters.dedup();

        let mut personal_loot = HashMap::new();
        for (index, looter) in looters.into_iter().enumerate() {
            let mut pool = loot.clone();
            if index != 0 {
                pool.loot_guid = self.next_represented_loot_object_guid_like_cpp(owner_guid)?;
            }
            pool.coins = self
                .loot
                .personal_loot_money_for_owner_and_player_like_cpp(owner_guid, looter)
                .copied()
                .unwrap_or(0);
            pool.allowed_looters = vec![looter];
            pool.players_looting.retain(|viewer| *viewer == looter);
            pool.items.retain(|entry| {
                entry.allowed_looters.is_empty() || entry.allowed_looters.contains(&looter)
            });
            for entry in &mut pool.items {
                entry.allowed_looters = vec![looter];
            }
            rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(&mut pool);
            personal_loot.insert(looter, pool);
        }

        Some((None, personal_loot))
    }

    pub(super) async fn request_represented_remote_master_loot_give_like_cpp(
        &self,
        target: ObjectGuid,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        dungeon_encounter_id: u32,
        entry: LootEntry,
        claim: Option<LootClaimLease>,
    ) -> MasterLootGiveResult {
        let (state, hub) = crate::session::split_loot_ref(self);
        state
            .request_represented_remote_master_loot_give_like_cpp(
                hub,
                target,
                owner_guid,
                loot_obj,
                loot_list_id,
                dungeon_encounter_id,
                entry,
                claim,
            )
            .await
    }

    pub(super) fn mark_represented_master_loot_item_removed_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        target: ObjectGuid,
    ) {
        {
            let Some(loot) = self.loot.cached_loot_for_owner_mut_like_cpp(owner_guid) else {
                return;
            };

            let Some(entry) = loot.items.get_mut(loot_list_id as usize) else {
                return;
            };

            let was_unlooted = !entry.is_looted_for_player_like_cpp(target);
            if !was_unlooted {
                return;
            }

            entry.quantity = 0;
            entry.mark_looted_for_player_like_cpp(target);
            loot.unlooted_count = loot.unlooted_count.saturating_sub(1);
        }

        self.refresh_represented_loot_owner_canonical_summary_like_cpp(owner_guid, target);
        self.send_packet(&LootRemoved {
            owner: owner_guid,
            loot_obj,
            loot_list_id,
        });
    }

    pub(super) fn represented_master_loot_target_exists_like_cpp(
        &self,
        target: ObjectGuid,
    ) -> bool {
        let (state, hub) = crate::session::split_loot_ref(self);
        state.represented_master_loot_target_exists_like_cpp(hub, target)
    }

    pub(super) fn represented_master_loot_target_eligible_like_cpp(
        &self,
        target: ObjectGuid,
    ) -> bool {
        let Some(group_guid) = self.resolved_group_guid_like_cpp() else {
            return false;
        };

        let Some(group_registry) = self.group_registry() else {
            return false;
        };

        group_registry
            .get(&group_guid)
            .is_some_and(|group| group.members.contains(&target))
    }

    pub(super) fn represented_master_loot_can_store_error_like_cpp(
        &self,
        target: ObjectGuid,
        item_id: u32,
        count: u32,
    ) -> Option<u8> {
        if self.player_guid() != Some(target) {
            return None;
        }

        let Some((result, _, _)) = self.plan_store_new_direct_inventory_item(item_id, count) else {
            return Some(LOOT_ERROR_MASTER_OTHER_LIKE_CPP);
        };

        master_loot_error_for_inventory_result_like_cpp(result)
    }

    pub(super) fn represented_master_loot_candidate_list_like_cpp(
        &self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> Option<MasterLootCandidateList> {
        let is_master_looter = if let (Some(group_guid), Some(registry)) =
            (self.resolved_group_guid_like_cpp(), self.group_registry())
        {
            registry.get(&group_guid).is_some_and(|group| {
                group.loot_method == LOOT_METHOD_MASTER_LIKE_CPP
                    && group.master_looter_guid == player_guid
            })
        } else {
            false
        };

        let loot = self.loot.cached_loot_for_owner_like_cpp(owner_guid)?;
        if loot.loot_method != LOOT_METHOD_MASTER_LIKE_CPP || !is_master_looter {
            return None;
        }

        Some(MasterLootCandidateList {
            loot_obj: loot.loot_guid,
            players: loot.allowed_looters.clone(),
        })
    }
}
