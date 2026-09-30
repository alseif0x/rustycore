//! The original nonblocking release queue fixture and its capacity regression.
use super::recovery_support::*;
use wow_packet::packets::update::UnitDataValuesDeltaUpdate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreatureLootReleaseCommandQueueOutcomeLikeCpp {
    Queued,
    Retrying,
    Disconnected,
}

fn queue_creature_loot_release_command_reliably_like_cpp(
    command_tx: &flume::Sender<SessionCommand>,
    command: SessionCommand,
) -> CreatureLootReleaseCommandQueueOutcomeLikeCpp {
    match command_tx.try_send(command) {
        Ok(()) => CreatureLootReleaseCommandQueueOutcomeLikeCpp::Queued,
        Err(flume::TrySendError::Disconnected(_)) => {
            CreatureLootReleaseCommandQueueOutcomeLikeCpp::Disconnected
        }
        Err(flume::TrySendError::Full(command)) => {
            let command_tx = command_tx.clone();
            // Never await another session from the source session loop: two
            // full queues could otherwise wait on each other forever. The
            // detached retry retains the exact command until capacity opens;
            // receiver-side authority/lifecycle gates coalesce its meaning to
            // the current corpse generation and reject stale respawn reuse.
            tokio::spawn(async move {
                if command_tx.send_async(command).await.is_err() {
                    tracing::debug!(
                        "loot-release DynamicFlags retry ended after target session disconnected"
                    );
                }
            });
            CreatureLootReleaseCommandQueueOutcomeLikeCpp::Retrying
        }
    }
}

#[tokio::test]
async fn creature_loot_release_command_retries_without_blocking_source_like_cpp() {
    let (command_tx, command_rx) = flume::bounded(1);
    command_tx
        .send(SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp)
        .unwrap();
    let creature_guid = test_creature_guid(19_121);
    assert_eq!(
        queue_creature_loot_release_command_reliably_like_cpp(
            &command_tx,
            SessionCommand::SendCreatureLootReleaseValuesUpdateLikeCpp(
                wow_world::session::mailbox::SendCreatureLootReleaseValuesUpdateLikeCppCommand {
                    creature_guid,
                    map_id: 0,
                    instance_id: 0,
                    unit_values_update: UnitDataValuesDeltaUpdate::default(),
                    authority: None,
                },
            ),
        ),
        CreatureLootReleaseCommandQueueOutcomeLikeCpp::Retrying,
        "a full peer queue must schedule retry without blocking this session"
    );
    assert!(matches!(
        command_rx.recv().unwrap(),
        SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp
    ));
    let queued = tokio::time::timeout(Duration::from_secs(1), command_rx.recv_async())
        .await
        .expect("detached retry should enqueue after capacity opens")
        .unwrap();
    assert!(matches!(
        queued,
        SessionCommand::SendCreatureLootReleaseValuesUpdateLikeCpp(command)
            if command.creature_guid == creature_guid
    ));
}
