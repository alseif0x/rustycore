//! Session-facing effects for the finalization application coordinator.
//! No transaction grouping or scheduling policy is defined by this adapter.
//! The coordinator body moved to `wow_world_application::session_lifecycle`
//! (#1363 slice 13b); the public entry stays here for the World callers.
use crate::finalization::{FinalizationMode, FinalizationReport, SessionFinalization};
#[cfg(test)]
use crate::finalization::{FinalizationOutcome, FinalizationStep};
use crate::session::WorldSession;
#[cfg(test)]
use wow_packet::packets::misc::LogoutComplete;

impl WorldSession {
    pub fn finalization_report_like_cpp(&self) -> Option<FinalizationReport> {
        self.lifecycle
            .finalization()
            .map(SessionFinalization::report)
    }

    /// Cancellation leaves an in-flight obligation, never a rollback receipt.
    pub fn interrupt_finalization_like_cpp(&mut self) -> Option<FinalizationReport> {
        if let Some(operation) = self.lifecycle.finalization_mut() {
            operation.interrupt();
        }
        self.finalization_report_like_cpp()
    }

    pub async fn finalize_session_with_generator_like_cpp(
        &mut self,
        mode: FinalizationMode,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> FinalizationReport {
        wow_world_application::finalize_session_with_generator_like_cpp(
            self,
            mode,
            item_guid_generator,
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::future::Future;

    #[tokio::test]
    async fn logout_publication_uses_realm_and_preserves_backpressure_and_failure() {
        let (_, input) = flume::bounded(1);
        let (output, instance) = flume::bounded(1);
        let mut session = WorldSession::new(
            585,
            "LogoutRoute".into(),
            0,
            2,
            9,
            54261,
            vec![0; 40],
            "enUS".into(),
            input,
            output,
            crate::session::registry::build_dispatch_table(),
        );
        let (realm_tx, realm) = flume::bounded(1);
        realm_tx.send(vec![0]).unwrap();
        session.install_realm_send_channel_for_test(realm_tx);
        let generator = wow_core::ObjectGuidGenerator::new(wow_core::guid::HighGuid::Item, 1);
        assert_eq!(
            session
                .execute_finalization_step(
                    FinalizationStep::LogoutPublication,
                    FinalizationMode::CharacterSelection,
                    &generator,
                )
                .await,
            FinalizationOutcome::Unavailable,
            "missing instance fence cannot authorize completion"
        );
        let fence = wow_network::SocketWriteFenceLikeCpp::default();
        session
            .core
            .transport
            .connection
            .set_send_write_fence_like_cpp(fence.clone());
        let mut publication = Box::pin(session.execute_finalization_step(
            FinalizationStep::LogoutPublication,
            FinalizationMode::CharacterSelection,
            &generator,
        ));
        assert!(
            std::future::poll_fn(|cx| {
                std::task::Poll::Ready(publication.as_mut().poll(cx).is_pending())
            })
            .await
        );
        let marker = instance.try_recv().expect("instance writer fence");
        assert_eq!(
            realm.len(),
            1,
            "completion cannot precede instance write acknowledgement"
        );
        assert!(fence.acknowledge_marker_like_cpp(&marker));
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(
                publication.as_mut().poll(cx).is_pending()
            ))
            .await,
            "realm backpressure remains after the instance fence"
        );
        assert_eq!(realm.try_recv().unwrap(), vec![0]);
        assert_eq!(publication.await, FinalizationOutcome::Applied);
        assert_eq!(
            realm.try_recv().unwrap(),
            wow_packet::ServerPacket::to_bytes(&LogoutComplete)
        );
        drop(realm);
        drop(instance);
        assert_eq!(
            session
                .execute_finalization_step(
                    FinalizationStep::LogoutPublication,
                    FinalizationMode::TimedLogout,
                    &generator,
                )
                .await,
            FinalizationOutcome::Unavailable
        );
    }
}
