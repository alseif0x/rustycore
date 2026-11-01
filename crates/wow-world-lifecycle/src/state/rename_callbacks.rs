use flume::r#async::SendFut;
use wow_core::ObjectGuid;

use crate::RenameOutcome;

use super::SessionLifecycleState;

impl SessionLifecycleState {
    pub fn character_rename_process_ready_like_cpp(&mut self) {
        self.character_rename_callbacks.process_ready();
    }

    pub fn character_rename_has_worker_failure_like_cpp(&self) -> bool {
        self.character_rename_callbacks.has_worker_failure()
    }

    pub fn character_rename_pending_delivery_count_like_cpp(&self) -> usize {
        self.character_rename_callbacks.pending_delivery_count()
    }

    pub fn character_rename_pending_result_count_like_cpp(&self) -> usize {
        self.character_rename_callbacks.pending_result_count()
    }

    pub fn character_rename_pending_result_at_like_cpp(
        &self,
        index: usize,
    ) -> (ObjectGuid, &RenameOutcome) {
        self.character_rename_callbacks.pending_result_at(index)
    }

    pub fn character_rename_enqueue_pending_delivery_like_cpp(
        &mut self,
        delivery: SendFut<'static, Vec<u8>>,
    ) {
        self.character_rename_callbacks
            .enqueue_pending_delivery(delivery);
    }

    pub fn character_rename_poll_pending_deliveries_like_cpp(&mut self) -> bool {
        self.character_rename_callbacks.poll_pending_deliveries()
    }

    pub async fn character_rename_finish_like_cpp(&mut self) -> bool {
        self.character_rename_callbacks.finish().await
    }
}
