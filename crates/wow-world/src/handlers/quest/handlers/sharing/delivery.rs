//! Receiver quest-share command delivery.
use super::*;

impl WorldSession {

    pub(crate) fn handle_send_repeatable_turn_in_request_items_command_like_cpp(
        &mut self,
        command: SendRepeatableTurnInRequestItemsLikeCppCommand,
    ) {
        self.send_repeatable_turn_in_request_items_like_cpp(command.sender_guid, &command.quest);
    }

    pub(crate) fn handle_set_quest_sharing_info_and_send_details_command_like_cpp(
        &mut self,
        command: SetQuestSharingInfoAndSendDetailsCommand,
    ) {
        let Some(receiver_guid) = self.player_guid() else {
            return;
        };

        self.set_represented_pending_quest_sharing_like_cpp(command.sender_guid, command.quest.id);
        self.send_represented_quest_giver_quest_details_like_cpp(
            receiver_guid,
            &command.quest,
            false,
        );
    }
}
