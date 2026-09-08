//! Scripted player cast QA façade.

use super::*;

mod plan;
mod report;
mod wire;

pub(super) use plan::*;
use report::*;
use wire::*;

const CMSG_CAST_SPELL: u16 = 0x329C;
const CMSG_CANCEL_CAST: u16 = 0x329F;
const CMSG_CANCEL_QUEUED_SPELL: u16 = 0x3182;
const SMSG_SPELL_PREPARE: u16 = 0x2C35;
const SMSG_SPELL_GO: u16 = 0x2C36;
const SMSG_SPELL_START: u16 = 0x2C37;
const SMSG_SPELL_FAILURE: u16 = 0x2C50;
const SMSG_SPELL_FAILED_OTHER: u16 = 0x2C52;
const SMSG_CAST_FAILED: u16 = 0x2C54;

const MAX_ACTIONS: usize = 128;
const MAX_EXPECTATIONS: usize = 256;
const MAX_PLAN_TIMEOUT_MS: u64 = 10 * 60 * 1000;
const DEFAULT_PLAN_TIMEOUT_MS: u64 = 5_000;
const MAX_WAIT_MS: u64 = 10 * 60 * 1000;
const LOGOUT_OBSERVATION_MS: u64 = 30_000;

/// Run the scripted client actions and bounded observer window.  Failures are
/// retained in `Evidence` so the JSON report remains useful even when a live
/// socket closes halfway through a cast sequence.
pub(super) async fn execute(
    options: &Options,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
) -> Result<Evidence> {
    let mut evidence = Evidence {
        observe_only: options.plan.observe_only,
        plan_path: options.source_path.clone(),
        timeout_ms: options.plan.timeout_ms,
        actions: options.plan.actions.clone(),
        expected_rules: options
            .plan
            .expect
            .iter()
            .cloned()
            .map(|rule| ExpectedFact {
                rule,
                matched: false,
                event_sequence: None,
                detail: "not evaluated".to_string(),
            })
            .collect(),
        ..Evidence::default()
    };
    let deadline = tokio::time::Instant::now() + Duration::from_millis(options.plan.timeout_ms);

    for action in &options.plan.actions {
        if tokio::time::Instant::now() >= deadline {
            evidence.failure = Some("script exceeded timeout before the next action".to_string());
            break;
        }
        let action_result = match action {
            Action::Cast {
                spell_id,
                cast_id,
                target,
            } => {
                let body = build_cast_spell_payload(*spell_id, *cast_id, *target);
                let result = send_encrypted_packet(stream, crypt, CMSG_CAST_SPELL, &body).await;
                if result.is_ok() {
                    record_sent(&mut evidence, "cast", CMSG_CAST_SPELL, &body);
                }
                result
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
                let result = send_encrypted_packet(stream, crypt, opcode, &body).await;
                if result.is_ok() {
                    record_sent(
                        &mut evidence,
                        match phase {
                            CancelPhase::Pending => "cancel_pending",
                            CancelPhase::Active => "cancel_active",
                        },
                        opcode,
                        &body,
                    );
                }
                result
            }
            Action::Wait { milliseconds } => {
                let wait_deadline = (tokio::time::Instant::now()
                    + Duration::from_millis(*milliseconds))
                .min(deadline);
                collect_until(wait_deadline, stream, crypt, inflater, realm, &mut evidence).await
            }
        };
        if let Err(error) = action_result {
            evidence.failure = Some(format!("action failed: {error}"));
            break;
        }
    }

    if evidence.failure.is_none() {
        if let Err(error) =
            collect_until(deadline, stream, crypt, inflater, realm, &mut evidence).await
        {
            evidence.failure = Some(format!("observer failed: {error}"));
        }
    }
    evaluate(&mut evidence);
    Ok(evidence)
}

pub(super) async fn run_after_login(
    options: Options,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
    result: &mut BotRunResult,
) {
    if let Err(error) = drain_login_streams(stream, crypt, inflater, realm, result).await {
        let mut evidence = Evidence {
            observe_only: options.plan.observe_only,
            plan_path: options.source_path.clone(),
            timeout_ms: options.plan.timeout_ms,
            actions: options.plan.actions.clone(),
            expected_rules: options
                .plan
                .expect
                .iter()
                .cloned()
                .map(|rule| ExpectedFact {
                    rule,
                    matched: false,
                    event_sequence: None,
                    detail: "login drain failed before evaluation".to_string(),
                })
                .collect(),
            ..Evidence::default()
        };
        evidence.failure = Some(format!("login stream drain failed: {error}"));
        result.cast_lifecycle = Some(evidence);
        return;
    }
    let mut evidence = match execute(&options, stream, crypt, inflater, realm).await {
        Ok(evidence) => evidence,
        Err(error) => Evidence {
            plan_path: options.source_path.clone(),
            failure: Some(error.to_string()),
            ..Evidence::default()
        },
    };
    finish(stream, crypt, inflater, realm, &mut evidence).await;
    result.cast_lifecycle = Some(evidence);
}

/// Send the normal logout request and close both encrypted sockets without any
/// database query.  Cast QA owns packet evidence only; fixture pairing and
/// durable offline checks remain an external runtime responsibility.
pub(super) async fn finish(
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
    evidence: &mut Evidence,
) {
    if let Err(error) = send_encrypted_packet(stream, crypt, CMSG_LOGOUT_REQUEST, &[0]).await {
        evidence
            .failure
            .get_or_insert_with(|| format!("logout request failed: {error}"));
    } else {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(LOGOUT_OBSERVATION_MS);
        if let Err(error) = collect_until(deadline, stream, crypt, inflater, realm, evidence).await
        {
            evidence
                .failure
                .get_or_insert_with(|| format!("logout observation failed: {error}"));
        }
    }
    let _ = stream.shutdown().await;
    if let Some(realm) = realm.as_mut() {
        let _ = realm.stream.shutdown().await;
    }
    finalize_logout(evidence);
}

#[cfg(test)]
mod tests;
