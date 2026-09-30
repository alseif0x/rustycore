//! Loot-roll routing and ballots at the application boundary.
use super::*;

impl WorldSession {
    pub(in crate::handlers::loot) fn route_represented_remote_loot_roll_vote_to_owner_like_cpp(
        &self,
        roll: &LootRoll,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(registry) = self.player_registry() else {
            return false;
        };
        let Some(pass_on_group_loot) = self.resolved_pass_on_group_loot_like_cpp() else {
            return false;
        };

        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let Some((registration, roll_identity)) = registry.loot_roll_owner(
            player_guid,
            self.player_map_id_like_cpp(),
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

    #[cfg(test)]
    pub(in crate::handlers::loot) async fn represented_player_vote_on_loot_roll_like_cpp(
        &mut self,
        roll: &LootRoll,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        self.represented_player_vote_on_loot_roll_with_generator_like_cpp(
            generator.as_ref(),
            &item_valuation,
            roll,
            player_guid,
        )
        .await
    }

    pub(in crate::handlers::loot) async fn represented_player_vote_on_loot_roll_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        roll: &LootRoll,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(pass_on_group_loot) = self.resolved_pass_on_group_loot_like_cpp() else {
            return false;
        };
        self.represented_player_vote_on_loot_roll_with_pass_state_and_generator_like_cpp(
            item_guid_generator,
            item_valuation,
            roll,
            player_guid,
            pass_on_group_loot,
        )
        .await
    }

    #[cfg(test)]
    pub(in crate::handlers::loot) async fn represented_player_vote_on_loot_roll_with_pass_state_like_cpp(
        &mut self,
        roll: &LootRoll,
        player_guid: ObjectGuid,
        pass_on_group_loot: bool,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        let item_valuation = self.item_valuation_catalogs_for_test_like_cpp();
        self.represented_player_vote_on_loot_roll_with_pass_state_and_generator_like_cpp(
            generator.as_ref(),
            &item_valuation,
            roll,
            player_guid,
            pass_on_group_loot,
        )
        .await
    }

    pub(in crate::handlers::loot) async fn represented_player_vote_on_loot_roll_with_pass_state_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        roll: &LootRoll,
        player_guid: ObjectGuid,
        pass_on_group_loot: bool,
    ) -> bool {
        let roll_key = (roll.loot_obj, roll.loot_list_id);
        let Some(roll_state) = self.represented_loot_rolls.get(&roll_key).cloned() else {
            return false;
        };
        if self
            .represented_current_loot_roll_authority_like_cpp(&roll_state)
            .is_none()
        {
            self.cancel_represented_loot_roll_generation_mismatch_like_cpp(roll_key, &roll_state);
            return true;
        }

        if pass_on_group_loot {
            return false;
        }

        let owner_guid = roll_state.owner_guid;

        let Some(loot) = self.loot_table.get(&owner_guid) else {
            return false;
        };
        if !matches!(
            loot.loot_method,
            LOOT_METHOD_GROUP_LIKE_CPP | LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP
        ) {
            return false;
        }
        let loot_guid = loot.loot_guid;
        let dungeon_encounter_id = loot.dungeon_encounter_id as i32;

        let Some(entry) = loot.items.iter().find(|entry| {
            entry.loot_list_id == roll.loot_list_id
                && entry.flags.blocked
                && entry.has_allowed_looter_like_cpp(player_guid)
        }) else {
            return false;
        };
        let entry = entry.clone();

        let Some(stored_roll_number) = wow_loot::RollBallots::prepare_vote(roll.roll_type, || {
            self.represented_urand_u32_like_cpp(1, 100) as u8
        }) else {
            return false;
        };
        let roll_number = if roll.roll_type == ROLL_VOTE_NEED_LIKE_CPP {
            0
        } else {
            -1
        };

        let Some(state) = self
            .represented_loot_rolls
            .get_mut(&(loot_guid, roll.loot_list_id))
        else {
            return false;
        };
        if !state.ballots.record_vote(player_guid, roll.roll_type, stored_roll_number) {
            return false;
        }

        let packet = LootRollBroadcast {
            loot_obj: loot_guid,
            player: player_guid,
            roll: roll_number,
            roll_type: roll.roll_type,
            item: loot_roll_broadcast_item_like_cpp(&entry, LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP),
            autopassed: false,
            off_spec: false,
            dungeon_encounter_id,
        };

        let finish = state.ballots.finished_winner();
        let finished_state = finish.as_ref().map(|_| state.clone());
        self.update_represented_loot_roll_vote_criteria_like_cpp(player_guid, roll.roll_type);
        self.broadcast_represented_loot_roll_packet_like_cpp(&packet, &entry, None);
        if let Some(winner) = finish {
            self.finish_represented_loot_roll_like_cpp(
                item_guid_generator,
                item_valuation,
                loot_guid,
                roll.loot_list_id,
                &entry,
                winner,
                finished_state.as_ref(),
            )
            .await;
        }
        true
    }
}
