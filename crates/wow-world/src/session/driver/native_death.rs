//! Session-owned continuation awaited by the current Map combat future.

use super::{SessionHandlerCatalogsLikeCpp, WorldSession};
use crate::session::mailbox::{
    CommittedCreatureMeleeReply, NativePlayerDeathReply, RunCommittedCreatureMeleeRequest,
    RunNativePlayerDeathRequest, SessionPhaseClaimLikeCpp, SessionPhasePassOutcomeLikeCpp,
};

pub(super) fn run_requested_committed_creature_melee(
    session: &mut WorldSession,
    request: RunCommittedCreatureMeleeRequest,
) {
    let admission = request.admission;
    let reply = |outcome, completed| CommittedCreatureMeleeReply {
        admission,
        outcome,
        completed,
    };
    if !session
        .core
        .is_map_combat_admission_current(&admission.map_phase, admission.map_tick_epoch)
        || request.command.victim_guid != admission.map_phase.handle.guid()
        || u32::from(request.command.map_id) != admission.map_phase.map_key.map_id
        || request.command.instance_id != admission.map_phase.map_key.instance_id
        || (request.presentation_stage
            == crate::session::mailbox::CreatureMeleePresentationStageLikeCpp::HealthOnly
            && request.command.over_damage >= 0)
        || (request.attack_start.is_none() && !request.attack_start_packets.is_empty())
        || request
            .attack_start
            .as_ref()
            .is_some_and(|start| !attack_start_matches(start, &request.command))
    {
        request.permit.refuse_before_start_like_cpp();
        let _ = request.response_tx.try_send(reply(
            SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart,
            false,
        ));
        return;
    }
    let outcome = match request.permit.claim_like_cpp() {
        SessionPhaseClaimLikeCpp::Claimed => None,
        SessionPhaseClaimLikeCpp::Revoked => {
            Some(SessionPhasePassOutcomeLikeCpp::RevokedBeforeStart)
        }
        SessionPhaseClaimLikeCpp::AlreadyResolved(_) => {
            Some(SessionPhasePassOutcomeLikeCpp::AlreadyResolved)
        }
    };
    if let Some(outcome) = outcome {
        let _ = request.response_tx.try_send(reply(outcome, false));
        return;
    }
    if let Some(start) = request.attack_start {
        present_attack_start_prelude(session, start, &request.attack_start_packets);
    }
    let completed = match wow_world_entities::present_committed_creature_melee_hit_at_stage_like_cpp(
        session,
        request.command,
        request.presentation_stage,
    ) {
        Ok(()) => {
            request.permit.complete_like_cpp();
            true
        }
        Err(error) => {
            tracing::error!(%error, "Required melee callback could not complete");
            request.permit.interrupt_after_start_like_cpp();
            false
        }
    };
    let _ = request
        .response_tx
        .try_send(reply(SessionPhasePassOutcomeLikeCpp::Ran, completed));
}

pub(super) async fn run_requested_native_player_death(
    session: &mut WorldSession,
    request: RunNativePlayerDeathRequest,
    catalogs: &SessionHandlerCatalogsLikeCpp,
) {
    let admission = request.admission;
    let reply = |outcome, health_after| NativePlayerDeathReply {
        admission,
        outcome,
        health_after,
    };
    if (request.creature_melee_prefix.is_none()
        && (request.attack_start.is_some() || !request.attack_start_packets.is_empty()))
        || (request.attack_start.is_none() && !request.attack_start_packets.is_empty())
        || !session
            .core
            .is_native_player_death_admission_current(&admission)
    {
        request.permit.refuse_before_start_like_cpp();
        let _ = request.response_tx.try_send(reply(
            SessionPhasePassOutcomeLikeCpp::RefusedBeforeStart,
            None,
        ));
        return;
    }
    let outcome = match request.permit.claim_like_cpp() {
        SessionPhaseClaimLikeCpp::Claimed => None,
        SessionPhaseClaimLikeCpp::Revoked => {
            Some(SessionPhasePassOutcomeLikeCpp::RevokedBeforeStart)
        }
        SessionPhaseClaimLikeCpp::AlreadyResolved(_) => {
            Some(SessionPhasePassOutcomeLikeCpp::AlreadyResolved)
        }
    };
    if let Some(outcome) = outcome {
        let _ = request.response_tx.try_send(reply(outcome, None));
        return;
    }
    if let Some(prefix) = request.creature_melee_prefix.as_ref() {
        if prefix.victim_guid != admission.map_phase.handle.guid()
            || session
                .core
                .with_owned_player_like_cpp(|player| {
                    player
                        .native_death_operation_snapshot()
                        .map(|operation| operation.context().attacker_guid())
                })
                .flatten()
                != Some(prefix.attacker_guid)
            || request
                .attack_start
                .as_ref()
                .is_some_and(|start| !attack_start_matches(start, prefix))
        {
            request.permit.complete_unavailable();
            let _ = request
                .response_tx
                .try_send(reply(SessionPhasePassOutcomeLikeCpp::Ran, None));
            return;
        }
        if let Some(start) = request.attack_start {
            present_attack_start_prelude(session, start, &request.attack_start_packets);
        }
        if let Err(error) =
            wow_world_entities::publish_creature_melee_prefix_like_cpp(session, prefix)
        {
            tracing::error!(%error, "Native death pre-Kill callback could not complete");
            request.permit.complete_unavailable();
            let _ = request
                .response_tx
                .try_send(reply(SessionPhasePassOutcomeLikeCpp::Ran, None));
            return;
        }
    }
    let result = wow_world_application::finish_native_player_death_like_cpp(
        session,
        admission.operation,
        admission.pvp_death,
        wow_world_application::ResurrectionOperationCatalogs {
            item_guid_generator: catalogs.id_generators.item.as_ref(),
            creature_spawns: catalogs.creature_spawns.as_ref(),
            graveyards: Some(catalogs.graveyards.as_ref()),
        },
    )
    .await;
    let health_after = match result {
        Ok(health) => {
            request.permit.complete_like_cpp();
            Some(health)
        }
        Err(error) => {
            tracing::error!(%error, operation = ?admission.operation,
                "Required Map Player death could not complete; retaining native operation");
            request.permit.complete_unavailable();
            None
        }
    };
    let _ = request
        .response_tx
        .try_send(reply(SessionPhasePassOutcomeLikeCpp::Ran, health_after));
}

fn attack_start_matches(
    start: &crate::session::mailbox::CreatureAttackStartLikeCppCommand,
    hit: &crate::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand,
) -> bool {
    start.attacker_guid == hit.attacker_guid
        && start.victim_guid == hit.victim_guid
        && start.map_id == hit.map_id
        && start.instance_id == hit.instance_id
}

fn present_attack_start_prelude(
    session: &mut WorldSession,
    mut start: crate::session::mailbox::CreatureAttackStartLikeCppCommand,
    packets: &[Vec<u8>],
) {
    let visible = session
        .core
        .client_visible_guids_like_cpp
        .contains(&start.attacker_guid);
    if !packets.is_empty() {
        start.packet_already_broadcast = true;
    }
    wow_world_entities::handle_creature_attack_start_like_cpp_command_like_cpp(session, start);
    if visible {
        for packet in packets {
            session.core.send_raw_packet(packet);
        }
    }
}
