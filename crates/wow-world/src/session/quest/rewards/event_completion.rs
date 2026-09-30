//! event completion operations at the existing Quest application boundary.

use super::*;

impl WorldSession {
    pub fn set_game_event_quest_complete_sender_like_cpp(
        &mut self,
        sender: flume::Sender<GameEventQuestCompleteCommandLikeCpp>,
    ) {
        self.directory.game_event_quest_complete_tx = Some(sender);
    }
    pub async fn notify_game_event_quest_complete_like_cpp(
        &self,
        quest_id: u32,
    ) -> GameEventQuestCompleteClientOutcomeLikeCpp {
        let Some(sender) = self.directory.game_event_quest_complete_tx.as_ref() else {
            return GameEventQuestCompleteClientOutcomeLikeCpp::SenderMissing { quest_id };
        };

        let (response_tx, response_rx) = flume::bounded(1);
        let command = GameEventQuestCompleteCommandLikeCpp {
            quest_id,
            response_tx,
        };
        if sender.try_send(command).is_err() {
            return GameEventQuestCompleteClientOutcomeLikeCpp::SendFailed { quest_id };
        }

        match tokio::time::timeout(Duration::from_millis(250), response_rx.recv_async()).await {
            Ok(Ok(response)) => GameEventQuestCompleteClientOutcomeLikeCpp::Ok(response),
            Ok(Err(_)) => {
                GameEventQuestCompleteClientOutcomeLikeCpp::ResponseChannelClosed { quest_id }
            }
            Err(_) => GameEventQuestCompleteClientOutcomeLikeCpp::ResponseTimeout { quest_id },
        }
    }
}
