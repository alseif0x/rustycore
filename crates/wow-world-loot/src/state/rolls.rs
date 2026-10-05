use super::{LootState, REMOTE_MASTER_LOOT_COMMAND_TIMEOUT};
use tokio::time::timeout;
use wow_core::ObjectGuid;
use wow_loot::LootClaimLease;
use wow_packet::packets::loot::{LootEntry, LootRoll};
use wow_world_core::session::mailbox::{
    LootRollStoreWinnerCommand, LootRollVoteCommand, MasterLootGiveResult, SessionCommand,
};
use wow_world_core::session::{HubRef, ItemValuationCatalogsLikeCpp};

impl LootState {
    pub fn route_represented_remote_loot_roll_vote_to_owner_like_cpp(
        &self,
        hub: HubRef<'_>,
        roll: &LootRoll,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(registry) = hub.core.player_registry() else {
            return false;
        };
        let Some(pass_on_group_loot) = self.resolved_pass_on_group_loot_like_cpp(hub) else {
            return false;
        };

        let instance_id = hub
            .core
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let Some((registration, roll_identity)) = registry.loot_roll_owner(
            player_guid,
            hub.core.player_map_id_like_cpp(),
            instance_id,
            roll.loot_obj,
            roll.loot_list_id,
        ) else {
            return false;
        };

        registry
            .try_send_current_command(
                registration,
                SessionCommand::LootRollVote(LootRollVoteCommand {
                    voter_guid: player_guid,
                    loot_obj: roll.loot_obj,
                    loot_list_id: roll.loot_list_id,
                    roll_type: roll.roll_type,
                    pass_on_group_loot,
                    roll_identity,
                }),
            )
            .is_ok()
    }

    pub async fn request_represented_remote_loot_roll_winner_store_like_cpp(
        &self,
        hub: HubRef<'_>,
        target: ObjectGuid,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        dungeon_encounter_id: u32,
        entries: Vec<LootEntry>,
        is_disenchant: bool,
        claim: Option<LootClaimLease>,
    ) -> MasterLootGiveResult {
        let Some(registry) = hub.core.player_registry() else {
            return MasterLootGiveResult::TargetMismatch;
        };
        let Some(command_address) = registry.control_address(target) else {
            return MasterLootGiveResult::TargetMismatch;
        };

        let (result_tx, result_rx) = flume::bounded(1);
        let command = SessionCommand::LootRollStoreWinner(LootRollStoreWinnerCommand {
            loot_owner: owner_guid,
            loot_obj,
            loot_list_id,
            dungeon_encounter_id,
            entries,
            is_disenchant,
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

    pub fn represented_loot_roll_disenchant_skill_required_like_cpp(
        &self,
        hub: HubRef<'_>,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        item_id: u32,
    ) -> Option<u16> {
        let template = hub
            .catalogs
            .item_stats_store()
            .and_then(|store| store.random_property_template(item_id))?;
        hub.catalogs
            .item_disenchant_loot_with_catalogs_like_cpp(
                item_valuation,
                item_id,
                template.quality as u32,
                u32::from(template.item_level),
                true,
            )
            .map(|(_, skill_required)| skill_required)
    }
}
