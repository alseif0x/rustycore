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
        wow_loot::materialize_loot_pools(loot, player_guid, personal, |looter, existing_guid| {
            let loot_guid = match existing_guid {
                Some(guid) => guid,
                None => self.next_represented_loot_object_guid_like_cpp(owner_guid)?,
            };
            let coins = self
                .represented_personal_loot_money
                .get(&(owner_guid, looter))
                .copied()
                .unwrap_or(0);
            Some((loot_guid, coins))
        })
    }

    pub(super) fn represented_loot_can_be_opened_by_player_like_cpp(
        &self,
        loot_guid: ObjectGuid,
        loot: &CreatureLoot,
        player_guid: ObjectGuid,
    ) -> bool {
        if !loot.allowed_looters.contains(&player_guid) {
            return false;
        }

        if self.represented_loot_money_for_player_like_cpp(loot_guid, loot, player_guid) > 0 {
            return true;
        }

        loot_can_be_opened_by_player_like_cpp(loot, player_guid)
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
        let Some(player_guid) = self.player_guid() else {
            return MasterLootGiveResult::TargetMismatch;
        };
        let Some(registry) = self.player_registry() else {
            return MasterLootGiveResult::TargetMismatch;
        };
        let Some(command_address) = registry.control_address(target) else {
            return MasterLootGiveResult::TargetMismatch;
        };

        let (result_tx, result_rx) = flume::bounded(1);
        let command = SessionCommand::MasterLootGive(MasterLootGiveCommand {
            master_guid: player_guid,
            loot_owner: owner_guid,
            loot_obj,
            loot_list_id,
            dungeon_encounter_id,
            entry,
            claim,
            result_tx,
        });

        if command_address.try_send(command).is_err() {
            return MasterLootGiveResult::TargetMismatch;
        }

        timeout(REMOTE_MASTER_LOOT_COMMAND_TIMEOUT, result_rx.recv_async())
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or(MasterLootGiveResult::TargetMismatch)
    }

    pub(super) fn mark_represented_master_loot_item_removed_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        target: ObjectGuid,
    ) {
        {
            let Some(loot) = self.loot_table.get_mut(&owner_guid) else {
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
        if self.player_guid() == Some(target) {
            return true;
        }

        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        self.player_registry()
            .and_then(|registry| {
                registry.loot_delivery_recipient(target, self.player_map_id_like_cpp(), instance_id)
            })
            .is_some()
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

        let loot = self.loot_table.get(&owner_guid)?;
        if loot.loot_method != LOOT_METHOD_MASTER_LIKE_CPP || !is_master_looter {
            return None;
        }

        Some(MasterLootCandidateList {
            loot_obj: loot.loot_guid,
            players: loot.allowed_looters.clone(),
        })
    }
}
