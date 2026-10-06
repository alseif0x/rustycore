use super::{LootState, REMOTE_MASTER_LOOT_COMMAND_TIMEOUT};
use tokio::time::timeout;
use wow_core::ObjectGuid;
use wow_loot::{
    CreatureLoot, LootClaimLease, OwnedLootAuthority, loot_can_be_opened_by_player_like_cpp,
};
use wow_packet::packets::loot::LootEntry;
use wow_world_core::session::HubRef;
use wow_world_core::session::mailbox::{
    MasterLootGiveCommand, MasterLootGiveResult, SessionCommand,
};

impl LootState {
    pub fn represented_loot_can_be_opened_by_player_like_cpp(
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

    pub async fn request_represented_remote_master_loot_give_like_cpp(
        &self,
        hub: HubRef<'_>,
        target: ObjectGuid,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        dungeon_encounter_id: u32,
        entry: LootEntry,
        claim: Option<LootClaimLease>,
    ) -> MasterLootGiveResult {
        let Some(player_guid) = hub.core.player_guid() else {
            return MasterLootGiveResult::TargetMismatch;
        };
        let Some(registry) = hub.core.player_registry() else {
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

    pub fn represented_master_loot_target_exists_like_cpp(
        &self,
        hub: HubRef<'_>,
        target: ObjectGuid,
    ) -> bool {
        if hub.core.player_guid() == Some(target) {
            return true;
        }

        let instance_id = hub
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        hub.core
            .player_registry()
            .and_then(|registry| {
                registry.loot_delivery_recipient(
                    target,
                    hub.core.player_map_id_like_cpp(),
                    instance_id,
                )
            })
            .is_some()
    }
}
