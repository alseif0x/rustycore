//! Quest packets and updates published to the client.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn send_represented_duel_requested_to_opponent_like_cpp(
        &self,
        opponent_guid: ObjectGuid,
        arbiter_guid: ObjectGuid,
        packet_bytes: Vec<u8>,
    ) {
        self.try_send_connected_player_command_like_cpp(
            opponent_guid,
            SessionCommand::SendRepresentedDuelRequestedLikeCpp(
                crate::session::mailbox::SendRepresentedDuelRequestedLikeCppCommand {
                    arbiter_guid,
                    packet_bytes,
                },
            ),
        );
    }
    pub(in crate::session) fn send_represented_prepared_quest_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
        menu_items: Vec<RepresentedPreparedQuestMenuItemLikeCpp>,
    ) {
        if let [menu_item] = menu_items.as_slice() {
            info!(
                ?source_guid,
                quest_id = menu_item.quest.id,
                quest_icon = menu_item.quest_icon,
                starter = menu_item.has_starter_relation,
                involved = menu_item.has_involved_relation,
                title = menu_item.quest.log_title.as_str(),
                "Sending represented single prepared quest like C++"
            );
            self.send_represented_prepared_single_quest_like_cpp(source_guid, menu_item, true);
            return;
        }

        info!(
            ?source_guid,
            quests = ?self.represented_quest_menu_item_log_rows_like_cpp(&menu_items),
            "Sending represented QuestGiverQuestList like C++"
        );
        self.send_packet(&QuestGiverQuestList {
            guid: source_guid,
            greeting: String::new(),
            greet_emote_delay: 0,
            greet_emote_type: 0,
            quests: menu_items
                .iter()
                .map(|item| self.quest_list_entry_from_menu_item_like_cpp(item))
                .collect(),
        });
    }
    pub(in crate::session) fn send_represented_prepared_single_quest_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
        menu_item: &RepresentedPreparedQuestMenuItemLikeCpp,
        auto_launched: bool,
    ) {
        let quest = &menu_item.quest;

        if menu_item.quest_icon == QUEST_MENU_ICON_COMPLETE_LIKE_CPP {
            let can_complete = self.can_reward_quest_represented_bounded_like_cpp(quest);
            info!(
                ?source_guid,
                quest_id = quest.id,
                can_complete,
                auto_launched,
                title = quest.log_title.as_str(),
                "Sending represented QuestGiverRequestItems from complete icon like C++"
            );
            self.send_represented_quest_giver_request_items_with_completion_like_cpp(
                source_guid,
                quest,
                can_complete,
                auto_launched,
            );
            return;
        }

        if !menu_item.has_starter_relation && !menu_item.has_involved_relation {
            // C++ would `SendCloseGossip()`. This represented seam has no close-gossip
            // packet helper, and relation-derived menu items should not reach this branch.
            return;
        }

        // C++ may auto-accept here; this represented-partial seam intentionally performs no
        // quest log mutation and only chooses the outbound packet.
        if quest.is_turn_in_like_cpp()
            && quest.is_repeatable()
            && !quest.is_daily_or_weekly_like_cpp()
            && !quest.is_monthly_like_cpp()
        {
            info!(
                ?source_guid,
                quest_id = quest.id,
                auto_launched,
                title = quest.log_title.as_str(),
                "Sending represented QuestGiverRequestItems for repeatable turn-in like C++"
            );
            let can_complete =
                self.can_complete_repeatable_quest_represented_bounded_like_cpp(quest);
            self.send_represented_quest_giver_request_items_with_completion_like_cpp(
                source_guid,
                quest,
                can_complete,
                auto_launched,
            );
        } else if quest.is_turn_in_like_cpp()
            && !quest.is_daily_or_weekly_like_cpp()
            && !quest.is_monthly_like_cpp()
        {
            let can_complete = self.can_reward_quest_represented_bounded_like_cpp(quest);
            info!(
                ?source_guid,
                quest_id = quest.id,
                can_complete,
                auto_launched,
                title = quest.log_title.as_str(),
                "Sending represented QuestGiverRequestItems for turn-in like C++"
            );
            self.send_represented_quest_giver_request_items_with_completion_like_cpp(
                source_guid,
                quest,
                can_complete,
                auto_launched,
            );
        } else {
            info!(
                ?source_guid,
                quest_id = quest.id,
                auto_launched,
                title = quest.log_title.as_str(),
                "Sending represented QuestGiverQuestDetails like C++"
            );
            self.send_represented_quest_giver_quest_details_like_cpp(
                source_guid,
                quest,
                auto_launched,
            );
        }
    }
}
