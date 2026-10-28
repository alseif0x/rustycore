//! Scripted post-login player lifecycle QA façade.

use super::*;

mod movement;
mod plan;
mod report;
mod targets;
mod wire;

pub(crate) use movement::Controller;
#[cfg(test)]
use movement::Outbound;
use movement::{
    SMSG_LOGIN_VERIFY_WORLD, SMSG_MOVE_ROOT, SMSG_MOVE_SET_LAND_WALK, SMSG_MOVE_SET_RUN_SPEED,
    SMSG_MOVE_SET_SWIM_SPEED, SMSG_MOVE_SET_WATER_WALK, SMSG_MOVE_TELEPORT, SMSG_MOVE_UNROOT,
    SMSG_NEW_WORLD, SMSG_RESUME_TOKEN, SMSG_SUSPEND_TOKEN, SMSG_TRANSFER_PENDING,
};
pub(super) use plan::*;
use report::*;
use targets::resolve_action_targets;
use wire::*;

const CMSG_CAST_SPELL: u16 = 0x329C;
const CMSG_CANCEL_CAST: u16 = 0x329F;
const CMSG_CANCEL_QUEUED_SPELL: u16 = 0x3182;
const CMSG_RELEASE_SPIRIT: u16 = 0x3526;
const CMSG_RECLAIM_CORPSE: u16 = 0x34DB;
const CMSG_SPIRIT_HEALER_ACTIVATE: u16 = 0x34AF;
const CMSG_RESURRECT_RESPONSE: u16 = 0x3685;
const CMSG_SELF_RES: u16 = 0x3531;
const CMSG_ATTACK_SWING: u16 = 0x3255;
const SMSG_SPELL_PREPARE: u16 = 0x2C35;
const SMSG_SPELL_GO: u16 = 0x2C36;
const SMSG_SPELL_START: u16 = 0x2C37;
const SMSG_SPELL_FAILURE: u16 = 0x2C50;
const SMSG_SPELL_FAILED_OTHER: u16 = 0x2C52;
const SMSG_CAST_FAILED: u16 = 0x2C54;
const REDACTED_SMSG_AUTH_CHALLENGE: u16 = 0x3048;
const REDACTED_SMSG_AUTH_FAILED: u16 = 0x256C;
const REDACTED_SMSG_AUTH_RESPONSE: u16 = 0x256D;
const REDACTED_SMSG_ENTER_ENCRYPTED_MODE: u16 = 0x3049;
const REDACTED_SMSG_CONNECT_TO: u16 = 0x304D;

// Exact packet expectations are restricted to this 3.4.3 death/combat output
// set plus the existing spell and LogoutComplete packets (Opcodes.h/Opcodes.cpp).
const RAW_SMSG_AREA_SPIRIT_HEALER_TIME: u16 = 0x2740;
const RAW_SMSG_ATTACKER_STATE_UPDATE: u16 = 0x2952;
const RAW_SMSG_ATTACK_START: u16 = 0x293D;
const RAW_SMSG_ATTACK_STOP: u16 = 0x293E;
const RAW_SMSG_ATTACK_SWING_ERROR: u16 = 0x294C;
const RAW_SMSG_AURA_UPDATE: u16 = 0x2C1F;
const RAW_SMSG_CORPSE_LOCATION: u16 = 0x264F;
const RAW_SMSG_CORPSE_RECLAIM_DELAY: u16 = 0x274A;
const RAW_SMSG_DEATH_RELEASE_LOC: u16 = 0x26D3;
const RAW_SMSG_DURABILITY_DAMAGE_DEATH: u16 = 0x2745;
const RAW_SMSG_FORCED_DEATH_UPDATE: u16 = 0x26D4;
const RAW_SMSG_PRE_RESSURECT: u16 = 0x276B;
const RAW_SMSG_REATTACH_RESURRECT: u16 = 0x274B;
const RAW_SMSG_RESURRECT_REQUEST: u16 = 0x257E;
const RAW_SMSG_UPDATE_OBJECT: u16 = 0x27CB;

const MAX_ACTIONS: usize = 128;
const MAX_EXPECTATIONS: usize = 256;
const MAX_PLAN_TIMEOUT_MS: u64 = 10 * 60 * 1000;
const DEFAULT_PLAN_TIMEOUT_MS: u64 = 5_000;
const MAX_WAIT_MS: u64 = 10 * 60 * 1000;
const LOGOUT_OBSERVATION_MS: u64 = 30_000;

/// Run scripted client actions and a bounded observer window. Failures remain
/// in `Evidence`, including a failed local write after its send attempt.
pub(super) async fn execute(
    controller: &mut Controller,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
) -> Result<()> {
    let actions = controller.options.plan.actions.clone();
    let deadline =
        tokio::time::Instant::now() + Duration::from_millis(controller.options.plan.timeout_ms);

    for (action_index, action) in actions.iter().enumerate() {
        if tokio::time::Instant::now() >= deadline {
            record_action_boundary(
                &mut controller.evidence,
                Some(action_index),
                action_name(action),
                TraceKind::ActionSkipped,
                Some("script deadline expired before action start".to_string()),
            );
            controller.evidence.failure =
                Some("script exceeded timeout before the next action".to_string());
            break;
        }
        let action_name = action_name(action);
        record_action_boundary(
            &mut controller.evidence,
            Some(action_index),
            action_name,
            TraceKind::ActionStarted,
            None,
        );
        let action_result = match action {
            Action::Cast {
                spell_id,
                cast_id,
                target,
            } => {
                let body = build_cast_spell_payload(*spell_id, *cast_id, *target);
                send_traced_packet(
                    stream,
                    crypt,
                    &mut controller.evidence,
                    Connection::Instance,
                    Some(action_index),
                    action_name,
                    CMSG_CAST_SPELL,
                    &body,
                )
                .await
            }
            Action::Cancel {
                phase,
                cast_id,
                spell_id,
            } => {
                let (opcode, body) = match phase {
                    CancelPhase::Pending => (
                        CMSG_CANCEL_QUEUED_SPELL,
                        build_cancel_queued_spell_payload(),
                    ),
                    CancelPhase::Active => (
                        CMSG_CANCEL_CAST,
                        build_cancel_cast_payload(*cast_id, *spell_id),
                    ),
                };
                send_traced_packet(
                    stream,
                    crypt,
                    &mut controller.evidence,
                    Connection::Instance,
                    Some(action_index),
                    action_name,
                    opcode,
                    &body,
                )
                .await
            }
            Action::ReleaseSpirit { .. }
            | Action::ReclaimCorpse { .. }
            | Action::SpiritHealerActivate { .. }
            | Action::ResurrectResponse { .. }
            | Action::SelfRes { .. }
            | Action::AttackSwing { .. } => {
                match resolve_action_targets(&mut controller.evidence, action_index, action)
                    .map(|resolved| build_action_packet(&resolved))
                {
                    Ok((opcode, body)) => {
                        send_traced_packet(
                            stream,
                            crypt,
                            &mut controller.evidence,
                            Connection::Instance,
                            Some(action_index),
                            action_name,
                            opcode,
                            &body,
                        )
                        .await
                    }
                    Err(error) => Err(error),
                }
            }
            Action::Movement { .. } => match controller.movement_action(action) {
                Ok((opcode, body)) => {
                    send_traced_packet(
                        stream,
                        crypt,
                        &mut controller.evidence,
                        Connection::Instance,
                        Some(action_index),
                        action_name,
                        opcode,
                        &body,
                    )
                    .await
                }
                Err(error) => Err(error),
            },
            Action::Wait { milliseconds } => {
                let wait_deadline = (tokio::time::Instant::now()
                    + Duration::from_millis(*milliseconds))
                .min(deadline);
                collect_until(wait_deadline, stream, crypt, inflater, realm, controller).await
            }
        };
        if let Err(error) = action_result {
            record_action_boundary(
                &mut controller.evidence,
                Some(action_index),
                action_name,
                TraceKind::ActionFailed,
                Some(error.to_string()),
            );
            controller.evidence.failure = Some(format!("action failed: {error}"));
            break;
        }
        record_action_boundary(
            &mut controller.evidence,
            Some(action_index),
            action_name,
            TraceKind::ActionCompleted,
            None,
        );
    }

    if controller.evidence.failure.is_none() {
        if let Err(error) =
            collect_until(deadline, stream, crypt, inflater, realm, controller).await
        {
            controller.evidence.failure = Some(format!("observer failed: {error}"));
        }
    }
    evaluate(&mut controller.evidence);
    Ok(())
}

pub(super) async fn run_after_login(
    mut controller: Controller,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
    result: &mut BotRunResult,
) {
    if let Err(error) =
        drain_login_streams_with_controller(stream, crypt, inflater, realm, result, &mut controller)
            .await
    {
        abort_login(
            controller,
            format!("login stream drain failed: {error}"),
            stream,
            realm,
            result,
        )
        .await;
        return;
    }
    if let Err(error) = execute(&mut controller, stream, crypt, inflater, realm).await {
        controller.evidence.failure = Some(error.to_string());
    }
    if let Some(failure) = controller.evidence.failure.clone() {
        abort_login(controller, failure, stream, realm, result).await;
        return;
    }
    finish(stream, crypt, inflater, realm, &mut controller).await;
    result.cast_lifecycle = Some(controller.evidence);
}

/// Preserve controller observations and close open sockets without sending
/// another client packet after a login-frame or movement-control failure.
pub(crate) async fn abort_login(
    mut controller: Controller,
    failure: String,
    stream: &mut TcpStream,
    realm: &mut Option<EncryptedWorldConnection>,
    result: &mut BotRunResult,
) {
    controller.evidence.failure.get_or_insert(failure);
    let primary_connection = if realm.is_some() {
        Connection::Instance
    } else {
        Connection::Realm
    };
    let primary_close = stream.shutdown().await;
    record_socket_close(
        &mut controller.evidence,
        primary_connection,
        primary_close.as_ref().err(),
    );
    let mut sockets_closed = primary_close.is_ok();
    if let Some(realm) = realm.as_mut() {
        let realm_close = realm.stream.shutdown().await;
        sockets_closed &= realm_close.is_ok();
        record_socket_close(
            &mut controller.evidence,
            Connection::Realm,
            realm_close.as_ref().err(),
        );
        if let Err(error) = realm_close {
            append_failure(
                &mut controller.evidence,
                format!("realm socket shutdown failed: {error}"),
            );
        }
    }
    controller.evidence.local_disconnect_recorded = sockets_closed;
    controller.evidence.finish_completed = false;
    if let Err(error) = primary_close {
        append_failure(
            &mut controller.evidence,
            format!("{primary_connection:?} socket shutdown failed: {error}"),
        );
    }
    preserve_controller_failure(controller, result, "login/controller failure".to_string());
}

pub(crate) fn preserve_controller_failure(
    mut controller: Controller,
    result: &mut BotRunResult,
    failure: String,
) {
    controller.evidence.failure.get_or_insert(failure);
    result.cast_lifecycle = Some(controller.evidence);
}

fn append_failure(evidence: &mut Evidence, detail: String) {
    if let Some(failure) = evidence.failure.as_mut() {
        failure.push_str("; ");
        failure.push_str(&detail);
    } else {
        evidence.failure = Some(detail);
    }
}

/// Apply the selected finish policy and close both encrypted sockets without
/// querying persistence. Disconnect records local close only; it does not
/// claim a server logout or a saved database result.
pub(super) async fn finish(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
    controller: &mut Controller,
) {
    match controller.evidence.finish_policy {
        FinishPolicy::Logout => {
            record_action_boundary(
                &mut controller.evidence,
                None,
                "finish_logout",
                TraceKind::ActionStarted,
                None,
            );
            if let Err(error) = send_traced_packet(
                stream,
                crypt,
                &mut controller.evidence,
                Connection::Instance,
                None,
                "finish_logout",
                CMSG_LOGOUT_REQUEST,
                &[0],
            )
            .await
            {
                controller
                    .evidence
                    .failure
                    .get_or_insert_with(|| format!("logout request failed: {error}"));
                record_action_boundary(
                    &mut controller.evidence,
                    None,
                    "finish_logout",
                    TraceKind::ActionFailed,
                    Some(error.to_string()),
                );
            } else {
                controller.evidence.logout_request_completed = true;
                let deadline =
                    tokio::time::Instant::now() + Duration::from_millis(LOGOUT_OBSERVATION_MS);
                if let Err(error) =
                    collect_until(deadline, stream, crypt, inflater, realm, controller).await
                {
                    controller
                        .evidence
                        .failure
                        .get_or_insert_with(|| format!("logout observation failed: {error}"));
                    record_action_boundary(
                        &mut controller.evidence,
                        None,
                        "finish_logout",
                        TraceKind::ActionFailed,
                        Some(error.to_string()),
                    );
                } else {
                    record_action_boundary(
                        &mut controller.evidence,
                        None,
                        "finish_logout",
                        TraceKind::ActionCompleted,
                        None,
                    );
                }
            }
        }
        FinishPolicy::Disconnect => {
            record_action_boundary(
                &mut controller.evidence,
                None,
                "finish_disconnect",
                TraceKind::ActionStarted,
                None,
            );
        }
    }
    let instance_close = stream.shutdown().await;
    record_socket_close(
        &mut controller.evidence,
        Connection::Instance,
        instance_close.as_ref().err(),
    );
    let has_realm_socket = realm.is_some();
    let mut realm_close_error = None;
    if let Some(realm) = realm.as_mut() {
        let close = realm.stream.shutdown().await;
        realm_close_error = close.as_ref().err().map(ToString::to_string);
        record_socket_close(
            &mut controller.evidence,
            Connection::Realm,
            close.as_ref().err(),
        );
    }
    let sockets_closed = instance_close.is_ok() && has_realm_socket && realm_close_error.is_none();
    if !sockets_closed {
        controller.evidence.failure.get_or_insert_with(|| {
            let close_error = instance_close
                .err()
                .map(|error| error.to_string())
                .or(realm_close_error)
                .or_else(|| (!has_realm_socket).then(|| "realm socket was absent".to_string()))
                .unwrap_or_else(|| "unknown close error".to_string());
            format!("local socket close failed: {close_error}")
        });
    }
    match controller.evidence.finish_policy {
        FinishPolicy::Logout => {
            controller.evidence.finish_completed =
                controller.evidence.logout_confirmed && sockets_closed;
            finalize_finish(&mut controller.evidence);
        }
        FinishPolicy::Disconnect => {
            controller.evidence.local_disconnect_recorded = sockets_closed;
            controller.evidence.finish_completed = sockets_closed;
            if sockets_closed {
                record_action_boundary(
                    &mut controller.evidence,
                    None,
                    "finish_disconnect",
                    TraceKind::ActionCompleted,
                    Some("local sockets closed; logout and persistence are unclaimed".to_string()),
                );
            } else {
                record_action_boundary(
                    &mut controller.evidence,
                    None,
                    "finish_disconnect",
                    TraceKind::ActionFailed,
                    Some("one or more local sockets did not close cleanly".to_string()),
                );
            }
            finalize_finish(&mut controller.evidence);
        }
    }
}

fn action_name(action: &Action) -> &'static str {
    match action {
        Action::Cast { .. } => "cast",
        Action::Cancel {
            phase: CancelPhase::Pending,
            ..
        } => "cancel_pending",
        Action::Cancel {
            phase: CancelPhase::Active,
            ..
        } => "cancel_active",
        Action::ReleaseSpirit { .. } => "release_spirit",
        Action::ReclaimCorpse { .. } => "reclaim_corpse",
        Action::SpiritHealerActivate { .. } => "spirit_healer_activate",
        Action::ResurrectResponse { .. } => "resurrect_response",
        Action::SelfRes { .. } => "self_res",
        Action::AttackSwing { .. } => "attack_swing",
        Action::Movement { kind, .. } => match kind {
            MovementKind::Heartbeat => "movement_heartbeat",
            MovementKind::StartForward => "movement_start_forward",
            MovementKind::Stop => "movement_stop",
            MovementKind::FallLand => "movement_fall_land",
        },
        Action::Wait { .. } => "wait",
    }
}

#[cfg(test)]
mod tests;
