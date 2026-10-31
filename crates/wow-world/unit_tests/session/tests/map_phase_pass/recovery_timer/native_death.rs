use super::*;
use crate::session::mailbox::{
    CommittedCreatureMeleeAdmission, NativePlayerDeathAdmission, NativePlayerDeathReply,
    RunCommittedCreatureMeleeRequest, RunNativePlayerDeathRequest, SessionPassKind,
};
use wow_entities::{PlayerDeathOperationContext, PlayerDeathOperationPhase};

#[tokio::test]
async fn native_death_rail_preserves_earlier_nonlethal_presentation_before_lethal_prefix() {
    use wow_packet::ServerPacket;
    let (mut session, recovery, _peer, send) = logged_in_recovery_session();
    crate::native_death_test_fixtures::prepare(&mut session);
    assert!(session.core.admission.accept_phase_authority(
        SessionPassKind::Map,
        recovery.coordinator_id,
        recovery.tick_epoch
    ));
    let phase = session
        .current_map_phase_admission_for_test_like_cpp(
            recovery.coordinator_id,
            recovery.tick_epoch,
            recovery.diff_ms,
        )
        .unwrap();
    let attacker = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        1,
        0,
        0,
        9001,
        91_001,
    );
    session.core.client_visible_guids_like_cpp.insert(attacker);
    let revision = session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Alive);
            player.unit_mut().set_max_health(1_000);
            player.unit_mut().set_health(900);
            player.unit().health_state_revision_like_cpp()
        })
        .unwrap();
    for _ in send.try_iter() {}
    let mut command = crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand {
        attacker_guid: attacker,
        victim_guid: phase.handle.guid(),
        map_id: phase.map_key.map_id as u16,
        instance_id: phase.map_key.instance_id,
        damage: 100,
        over_damage: -1,
        target_level: 1,
        victim_health_after: 900,
        victim_health_state_revision_after: revision,
        hit_info: 0,
        victim_state: 0,
        original_damage: 100,
        absorbed: 0,
        mana_spent: 0,
        absorb_consumptions: Vec::new(),
        split_combat_log_packets: Vec::new(),
        self_share_health_updates: Vec::new(),
    };
    let (response_tx, response_rx) = flume::bounded(1);
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();
    session
        .run_requested_session_phase_like_cpp(
            SessionPhaseRequestLikeCpp::CommittedCreatureMelee(RunCommittedCreatureMeleeRequest {
                admission: CommittedCreatureMeleeAdmission {
                    map_phase: phase,
                    map_tick_epoch: recovery.map_tick_epoch,
                },
                command: command.clone(),
                attack_start: Some(crate::session::mailbox::CreatureAttackStartLikeCppCommand {
                    attacker_guid: attacker,
                    victim_guid: phase.handle.guid(),
                    previous_victim_guid: None,
                    map_id: phase.map_key.map_id as u16,
                    instance_id: phase.map_key.instance_id,
                    packet_already_broadcast: true,
                }),
                attack_start_packets: vec![
                    wow_packet::packets::combat::AIReaction {
                        unit_guid: attacker,
                        reaction: wow_constants::creature::AiReaction::Hostile,
                    }
                    .to_bytes(),
                    wow_packet::packets::combat::AttackStart {
                        attacker,
                        victim: phase.handle.guid(),
                    }
                    .to_bytes(),
                ],
                presentation_stage:
                    crate::session::mailbox::CreatureMeleePresentationStageLikeCpp::Whole,
                permit: SessionPhasePermitLikeCpp::new_like_cpp(),
                response_tx,
            }),
            &catalogs,
        )
        .await;
    assert!(response_rx.try_recv().unwrap().completed);
    let operation = session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .begin_native_death_operation(PlayerDeathOperationContext::new(attacker, false))
                .unwrap()
        })
        .unwrap();
    command.damage = 900;
    command.original_damage = 900;
    command.over_damage = 0;
    command.victim_health_after = 0;
    command.victim_health_state_revision_after = 0;
    let (response_tx, response_rx) = flume::bounded(1);
    session
        .run_requested_session_phase_like_cpp(
            SessionPhaseRequestLikeCpp::NativePlayerDeath(RunNativePlayerDeathRequest {
                admission: NativePlayerDeathAdmission {
                    map_phase: phase,
                    map_tick_epoch: recovery.map_tick_epoch,
                    operation,
                    pvp_death: false,
                },
                permit: SessionPhasePermitLikeCpp::new_like_cpp(),
                response_tx,
                creature_melee_prefix: Some(command),
                attack_start: None,
                attack_start_packets: Vec::new(),
            }),
            &catalogs,
        )
        .await;
    assert_eq!(response_rx.try_recv().unwrap().health_after, Some(0));
    // This fixture collects both connection writers through the same output
    // receiver. The sequence proves synchronous enqueue/callpoint order only;
    // it does not prove network delivery order across instance and realm.
    let mut observed = Vec::new();
    for bytes in send.try_iter() {
        let mut packet = wow_packet::WorldPacket::from_bytes(&bytes);
        match packet.server_opcode() {
            Some(wow_constants::ServerOpcodes::AiReaction) => observed.push("reaction"),
            Some(wow_constants::ServerOpcodes::AttackStart) => observed.push("start"),
            Some(wow_constants::ServerOpcodes::AttackerStateUpdate) => observed.push("hit"),
            Some(wow_constants::ServerOpcodes::HealthUpdate) => {
                packet.skip_opcode();
                assert_eq!(packet.read_packed_guid().unwrap(), phase.handle.guid());
                assert_eq!(packet.read_int64().unwrap(), 900);
                observed.push("health");
            }
            Some(wow_constants::ServerOpcodes::StandStateUpdate) => observed.push("death body"),
            _ => {}
        }
    }
    assert_eq!(
        observed,
        ["reaction", "start", "hit", "health", "hit", "death body"]
    );
}

fn native_request_fixture() -> (
    WorldSession,
    NativePlayerDeathAdmission,
    flume::Sender<WorldPacket>,
    flume::Receiver<Vec<u8>>,
) {
    let (mut session, recovery, packet_tx, send) = logged_in_recovery_session();
    crate::native_death_test_fixtures::prepare(&mut session);
    assert!(session.core.admission.accept_phase_authority(
        SessionPassKind::Map,
        recovery.coordinator_id,
        recovery.tick_epoch
    ));
    let phase = session
        .current_map_phase_admission_for_test_like_cpp(
            recovery.coordinator_id,
            recovery.tick_epoch,
            recovery.diff_ms,
        )
        .unwrap();
    let operation = session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Alive);
            player.unit_mut().set_max_health(1_000);
            player.unit_mut().set_health(1_000);
            player
                .begin_native_death_operation(PlayerDeathOperationContext::new(
                    phase.handle.guid(),
                    false,
                ))
                .unwrap()
        })
        .unwrap();
    (
        session,
        NativePlayerDeathAdmission {
            map_phase: phase,
            map_tick_epoch: recovery.map_tick_epoch,
            operation,
            pvp_death: true,
        },
        packet_tx,
        send,
    )
}

async fn run_native_request(
    session: &mut WorldSession,
    admission: NativePlayerDeathAdmission,
    permit: Arc<SessionPhasePermitLikeCpp>,
) -> NativePlayerDeathReply {
    let (response_tx, response_rx) = flume::bounded(1);
    let catalogs = session.session_handler_catalogs_for_test_like_cpp();
    let request = SessionPhaseRequestLikeCpp::NativePlayerDeath(RunNativePlayerDeathRequest {
        admission,
        permit,
        response_tx,
        creature_melee_prefix: None,
        attack_start: None,
        attack_start_packets: Vec::new(),
    });
    assert!(
        session
            .run_requested_session_phase_like_cpp(request, &catalogs)
            .await
            .is_none()
    );
    response_rx.try_recv().unwrap()
}

#[tokio::test]
async fn native_death_rail_runs_multiple_operations_in_one_map_epoch_and_requires_acknowledgement()
{
    let (mut session, mut admission, _peer, _send) = native_request_fixture();
    for index in 0..2 {
        let permit = SessionPhasePermitLikeCpp::new_like_cpp();
        let result = run_native_request(&mut session, admission, Arc::clone(&permit)).await;
        assert_eq!(result.outcome, SessionPhasePassOutcomeLikeCpp::Ran);
        assert_eq!(result.health_after, Some(0));
        assert_eq!(
            permit.state_like_cpp(),
            crate::session::mailbox::SessionPhasePermitStateLikeCpp::Completed
        );
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                let completed = player.native_death_operation_snapshot().unwrap();
                assert_eq!(completed.id(), admission.operation);
                assert_eq!(completed.phase(), PlayerDeathOperationPhase::Complete);
                assert!(player.retire_completed_native_death_operation(admission.operation));
                if index == 0 {
                    player
                        .unit_mut()
                        .set_death_state(wow_constants::DeathState::Alive);
                    player.unit_mut().set_health(1_000);
                    let previous = admission.operation;
                    admission.operation = player
                        .begin_native_death_operation(PlayerDeathOperationContext::new(
                            admission.map_phase.handle.guid(),
                            false,
                        ))
                        .unwrap();
                    assert_ne!(previous, admission.operation);
                }
            })
            .unwrap();
    }
}

#[tokio::test]
async fn native_death_rail_revocation_retains_ready_owner_without_health_effects() {
    let (mut session, admission, _peer, _send) = native_request_fixture();
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    permit.revoke_before_start_like_cpp();
    let result = run_native_request(&mut session, admission, permit).await;
    assert_eq!(
        result.outcome,
        SessionPhasePassOutcomeLikeCpp::RevokedBeforeStart
    );
    assert_eq!(result.health_after, None);
    session
        .core
        .with_owned_player_like_cpp(|player| {
            assert_eq!(player.unit().data().health, 1_000);
            assert_eq!(
                player.native_death_operation_snapshot().unwrap().phase(),
                PlayerDeathOperationPhase::Ready
            );
        })
        .unwrap();
}

#[tokio::test]
async fn native_death_rail_refuses_changed_residence_and_preserves_ready_operation() {
    let (mut session, mut admission, _peer, _send) = native_request_fixture();
    admission.map_phase.residence_revision += 1;
    let result = run_native_request(
        &mut session,
        admission,
        SessionPhasePermitLikeCpp::new_like_cpp(),
    )
    .await;
    assert_eq!(
        result.outcome,
        SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart
    );
    assert_eq!(result.health_after, None);
    assert!(
        session
            .core
            .with_owned_player_like_cpp(|player| player.has_unresolved_native_death_operation())
            .unwrap()
    );
}

#[tokio::test]
async fn native_death_rail_body_failure_never_acknowledges_the_native_operation() {
    let (mut session, admission, _peer, _send) = native_request_fixture();
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            let mut spells = player.spell_runtime_like_cpp().clone();
            spells.set_row_authority_like_cpp(false, false);
            player.replace_spell_runtime_like_cpp(spells);
        })
        .unwrap();
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let result = run_native_request(&mut session, admission, Arc::clone(&permit)).await;
    assert_eq!(result.outcome, SessionPhasePassOutcomeLikeCpp::Ran);
    assert_eq!(result.health_after, None);
    assert_eq!(
        permit.state_like_cpp(),
        crate::session::mailbox::SessionPhasePermitStateLikeCpp::CompletedUnavailable
    );
    assert!(
        session
            .core
            .with_owned_player_like_cpp(|player| player.has_unresolved_native_death_operation())
            .unwrap()
    );
}
