use std::sync::Arc;

use tracing::warn;

use crate::{HomebindPersistenceJobLikeCpp, SessionLifecycleState};

impl SessionLifecycleState {
    pub fn queue_player_homebind_persistence_like_cpp(
        &mut self,
        port: Arc<dyn wow_persistence::PlayerLifecyclePortLikeCpp>,
        request: wow_persistence::PlayerHomebindPersistenceRequestLikeCpp,
        guid_counter: u64,
    ) -> Result<(), tokio::sync::mpsc::error::SendError<HomebindPersistenceJobLikeCpp>> {
        let persistence_tx = self
            .homebind_persistence_tx_like_cpp
            .get_or_insert_with(|| {
                let (tx, mut rx) =
                    tokio::sync::mpsc::unbounded_channel::<HomebindPersistenceJobLikeCpp>();
                tokio::spawn(async move {
                    while let Some(job) = rx.recv().await {
                        match job.port.persist_homebind_like_cpp(job.request).await {
                            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. } => {}
                            wow_persistence::PersistenceOutcomeLikeCpp::Failed { reason }
                            | wow_persistence::PersistenceOutcomeLikeCpp::Unknown { reason } => {
                                warn!(
                                    player_guid = job.guid_counter,
                                    "failed to update represented player homebind: {reason}"
                                );
                            }
                        }
                    }
                });
                tx
            });
        persistence_tx.send(HomebindPersistenceJobLikeCpp {
            port,
            request,
            guid_counter,
        })
    }
}
