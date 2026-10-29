//! JSON action plan and structured report types for cast lifecycle QA.

use super::*;
use serde::{Deserialize, Deserializer, Serialize};
use std::path::Path;

const MAX_EXPECTED_PACKET_BODY_BYTES: usize = 64 * 1024;

/// A packed ObjectGuid represented in plan files and reports without losing
/// the original client cast identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) struct Guid {
    pub low: u64,
    pub high: u64,
}

impl Guid {
    pub(crate) fn empty(self) -> bool {
        self.low == 0 && self.high == 0
    }

    pub(crate) fn packed(self) -> Vec<u8> {
        build_packed_guid(self.low, self.high)
    }
}

/// A creature target may stay an explicit packed GUID or be selected from
/// retained world-update evidence at the action's send boundary.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(untagged)]
pub(crate) enum CreatureTarget {
    Guid(Guid),
    Observed(ObservedCreatureSelector),
}

impl<'de> Deserialize<'de> for CreatureTarget {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct StrictGuid {
            low: u64,
            high: u64,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Input {
            Observed(ObservedCreatureSelector),
            Guid(StrictGuid),
        }

        Ok(match Input::deserialize(deserializer)? {
            Input::Observed(selector) => Self::Observed(selector),
            Input::Guid(guid) => Self::Guid(Guid {
                low: guid.low,
                high: guid.high,
            }),
        })
    }
}

/// A bounded search request against creature CREATEs actually observed by
/// this client, scoped to one world map and a finite position radius.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObservedCreatureSelector {
    pub(crate) entry: u32,
    pub(crate) map_id: u16,
    pub(crate) near_xyz: [f32; 3],
    pub(crate) radius: f32,
}

impl ObservedCreatureSelector {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.entry == 0 || self.entry > 0x7F_FFFF {
            bail!("entry must fit the nonzero creature GUID entry field");
        }
        if self.map_id > 0x1FFF {
            bail!("map_id must fit the creature GUID map field");
        }
        if !self.near_xyz.into_iter().all(f32::is_finite)
            || !self.radius.is_finite()
            || self.radius <= 0.0
        {
            bail!("near_xyz must be finite and radius must be finite and positive");
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Options {
    pub(crate) source_path: String,
    pub(crate) plan: Plan,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Plan {
    /// Optional in-memory character counter override for a single-account
    /// cast-lifecycle run. The private bot credential file is never rewritten.
    #[serde(default)]
    pub(crate) character_guid: Option<u64>,
    #[serde(default)]
    pub(crate) observe_only: bool,
    #[serde(default = "default_timeout_ms")]
    pub(crate) timeout_ms: u64,
    #[serde(default)]
    pub(crate) actions: Vec<Action>,
    #[serde(default)]
    pub(crate) expect: Vec<ExpectedRule>,
    #[serde(default)]
    pub(crate) finish_policy: FinishPolicy,
}

fn default_timeout_ms() -> u64 {
    DEFAULT_PLAN_TIMEOUT_MS
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Action {
    /// `target = null` writes the C++ empty target shape.  A unit target sets
    /// only TARGET_FLAG_UNIT and writes the supplied packed GUID.
    Cast {
        spell_id: i32,
        cast_id: Guid,
        #[serde(default)]
        target: Option<Guid>,
    },
    /// Active uses CancelCast with its identity; pending uses the empty
    /// CancelQueuedSpell packet. IDs still name the pending request in the plan.
    Cancel {
        phase: CancelPhase,
        cast_id: Guid,
        spell_id: u32,
    },
    /// CMSG_REPOP_REQUEST carries one MSB-first CheckInstance bit.
    ReleaseSpirit {
        check_instance: bool,
    },
    /// The server currently ignores CorpseGUID, so every packed GUID value is
    /// intentionally representable for negative protocol cases.
    ReclaimCorpse {
        corpse: Guid,
    },
    SpiritHealerActivate {
        healer: CreatureTarget,
    },
    ResurrectResponse {
        offerer: Guid,
        response: u32,
    },
    SelfRes {
        spell_id: i32,
    },
    AttackSwing {
        target: CreatureTarget,
    },
    /// One finite client movement packet. The opcode comes from the closed
    /// `MovementKind` set; control flags remain owned by the live controller.
    Movement {
        kind: MovementKind,
        #[serde(default)]
        time_ms: u32,
        x: f32,
        y: f32,
        z: f32,
        orientation: f32,
        #[serde(default)]
        fall: Option<FallData>,
    },
    Wait {
        milliseconds: u64,
    },
    WaitForPacket {
        connection: Connection,
        opcode: u16,
        timeout_ms: u64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MovementKind {
    Heartbeat,
    StartForward,
    Stop,
    FallLand,
}

/// Optional `MovementInfo::jump` data. Supplying it sets FALLING and writes
/// `fallTime`, `zspeed`, the direction bit, then its three floats.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct FallData {
    pub(crate) fall_time_ms: u32,
    pub(crate) z_speed: f32,
    pub(crate) sin_angle: f32,
    pub(crate) cos_angle: f32,
    pub(crate) xy_speed: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) enum FinishPolicy {
    #[default]
    Logout,
    Disconnect,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CancelPhase {
    Pending,
    Active,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ExpectedRule {
    Packet {
        connection: Connection,
        opcode: u16,
        body_hex: String,
    },
    Prepare {
        client_cast_id: Guid,
        #[serde(default)]
        server_cast_id: Option<Guid>,
    },
    Start {
        #[serde(default)]
        metadata: Option<CastMetadataExpectation>,
        /// Set for a caster session; the observer session normally has no
        /// Prepare packet and must use `server_cast_id` instead.
        #[serde(default)]
        client_cast_id: Option<Guid>,
        #[serde(default)]
        server_cast_id: Option<Guid>,
        spell_id: i32,
        target: TargetExpectation,
        #[serde(default)]
        cast_flags: Option<u32>,
        #[serde(default)]
        cast_flags_ex: Option<u32>,
    },
    Go {
        #[serde(default)]
        metadata: Option<CastMetadataExpectation>,
        /// Set for a caster session; the observer session normally has no
        /// Prepare packet and must use `server_cast_id` instead.
        #[serde(default)]
        client_cast_id: Option<Guid>,
        #[serde(default)]
        server_cast_id: Option<Guid>,
        spell_id: i32,
        target: TargetExpectation,
        #[serde(default)]
        cast_flags: Option<u32>,
        #[serde(default)]
        cast_flags_ex: Option<u32>,
        #[serde(default)]
        hit_targets: Option<usize>,
        #[serde(default)]
        miss_targets: Option<usize>,
        #[serde(default)]
        full_combat_log: Option<bool>,
    },
    CastFailed {
        #[serde(default)]
        client_cast_id: Option<Guid>,
        #[serde(default)]
        server_cast_id: Option<Guid>,
        spell_id: i32,
        #[serde(default)]
        reason: Option<i32>,
    },
    SpellFailure {
        #[serde(default)]
        client_cast_id: Option<Guid>,
        #[serde(default)]
        server_cast_id: Option<Guid>,
        spell_id: i32,
        #[serde(default)]
        reason: Option<u16>,
    },
    SpellFailedOther {
        #[serde(default)]
        client_cast_id: Option<Guid>,
        #[serde(default)]
        server_cast_id: Option<Guid>,
        spell_id: i32,
        #[serde(default)]
        reason: Option<u8>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetExpectation {
    Empty,
    Unit(Guid),
    CorpseAlly(Guid),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub(crate) struct CastMetadataExpectation {
    pub visual_id: Option<u32>,
    pub original_cast_id: Option<Guid>,
    pub cast_time_ms: Option<u32>,
    pub remaining_power: Option<Vec<PowerFact>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Evidence {
    pub passed: bool,
    pub observe_only: bool,
    pub plan_path: String,
    pub timeout_ms: u64,
    pub actions: Vec<Action>,
    pub expected_rules: Vec<ExpectedFact>,
    pub events: Vec<ObservedEvent>,
    #[serde(default)]
    pub target_selections: Vec<TargetSelectionFact>,
    pub sent_packets: Vec<SentPacket>,
    pub trace: Vec<TraceEntry>,
    pub packet_payload_encoding: String,
    pub merged_observation_order: String,
    pub cross_connection_order_is_server_enqueue: bool,
    pub finish_policy: FinishPolicy,
    pub logout_request_completed: bool,
    pub logout_confirmed: bool,
    pub logout_route: Option<String>,
    pub local_disconnect_recorded: bool,
    pub finish_completed: bool,
    pub failure: Option<String>,
    pub movement_state: Option<MovementStateFact>,
    #[serde(skip)]
    pub(crate) connection_sequences: ConnectionSequences,
}

impl Default for Evidence {
    fn default() -> Self {
        Self {
            passed: false,
            observe_only: false,
            plan_path: String::new(),
            timeout_ms: 0,
            actions: Vec::new(),
            expected_rules: Vec::new(),
            events: Vec::new(),
            target_selections: Vec::new(),
            sent_packets: Vec::new(),
            trace: Vec::new(),
            packet_payload_encoding: "logical_decrypted_uncompressed".to_string(),
            merged_observation_order: "client_poll_arrival".to_string(),
            cross_connection_order_is_server_enqueue: false,
            finish_policy: FinishPolicy::Logout,
            logout_request_completed: false,
            logout_confirmed: false,
            logout_route: None,
            local_disconnect_recorded: false,
            finish_completed: false,
            failure: None,
            movement_state: None,
            connection_sequences: ConnectionSequences::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct MovementStateFact {
    pub(crate) mover: Guid,
    pub(crate) map_id: Option<i32>,
    pub(crate) x: Option<f32>,
    pub(crate) y: Option<f32>,
    pub(crate) z: Option<f32>,
    pub(crate) orientation: Option<f32>,
    pub(crate) flags: u32,
    pub(crate) active_mover_complete_count: u32,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ConnectionSequences {
    pub(crate) instance: usize,
    pub(crate) realm: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ExpectedFact {
    pub rule: ExpectedRule,
    pub matched: bool,
    pub event_sequence: Option<usize>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SentPacket {
    pub sequence: usize,
    pub observation_sequence: usize,
    pub action_index: Option<usize>,
    pub connection: Connection,
    pub connection_sequence: usize,
    pub action: String,
    pub opcode: u16,
    pub bytes: usize,
    pub body_hex: String,
    pub body_sha256: String,
    pub write_attempted: bool,
    pub write_completed: bool,
    pub write_error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Connection {
    Instance,
    Realm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ObservedEvent {
    pub sequence: usize,
    pub observation_sequence: usize,
    pub connection: Connection,
    pub connection_sequence: usize,
    pub opcode: u16,
    pub kind: String,
    pub body_bytes: usize,
    pub body_hex: String,
    pub body_sha256: String,
    pub client_cast_id: Option<Guid>,
    pub server_cast_id: Option<Guid>,
    pub cast_id: Option<Guid>,
    pub caster: Option<Guid>,
    pub caster_unit: Option<Guid>,
    pub original_cast_id: Option<Guid>,
    pub spell_id: Option<i32>,
    pub visual_id: Option<u32>,
    pub cast_flags: Option<u32>,
    pub cast_flags_ex: Option<u32>,
    pub cast_time_ms: Option<u32>,
    pub target: Option<TargetFact>,
    pub hit_targets: Vec<Guid>,
    pub miss_targets: Vec<Guid>,
    pub miss_statuses: Vec<MissStatusFact>,
    pub remaining_power: Vec<PowerFact>,
    pub remaining_runes: Option<RuneFact>,
    pub target_points: Vec<TargetPointFact>,
    pub ammo_display_id: Option<i32>,
    pub ammo_inventory_type: Option<i32>,
    pub full_combat_log: Option<bool>,
    pub full_log_power_count: Option<usize>,
    pub reason: Option<u32>,
    pub failed_arg1: Option<i32>,
    pub failed_arg2: Option<i32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TraceKind {
    ActionStarted,
    TargetSelected,
    ActionSkipped,
    SendAttempted,
    LocalWriteCompleted,
    LocalWriteFailed,
    PacketObserved,
    PacketRedacted,
    ActionCompleted,
    ActionFailed,
    LocalSocketClosed,
}

/// Timeline index shared by actions, writes and packet observations. Packet
/// bodies live in `sent_packets` or `events`; redacted controls have no body
/// or digest and retain only this routing/ordering metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TraceEntry {
    pub sequence: usize,
    pub kind: TraceKind,
    pub action_index: Option<usize>,
    pub action: Option<String>,
    pub connection: Option<Connection>,
    pub connection_sequence: Option<usize>,
    pub opcode: Option<u16>,
    pub sent_packet_sequence: Option<usize>,
    pub event_sequence: Option<usize>,
    pub body_bytes: Option<usize>,
    pub body_redacted: bool,
    pub detail: Option<String>,
}

/// A selector resolution tied to the exact retained object-update event that
/// supplied the GUID sent by one scripted action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TargetSelectionFact {
    pub action_index: usize,
    pub action: String,
    pub selector: ObservedCreatureSelector,
    pub guid: Guid,
    pub source_event_sequence: usize,
    pub source_connection: Connection,
    pub source_connection_sequence: usize,
    pub source_event_opcode: u16,
    pub source_event_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TargetFact {
    pub flags: u32,
    pub unit: Guid,
    pub item: Guid,
    pub src_location: Option<TargetPointFact>,
    pub dst_location: Option<TargetPointFact>,
    pub orientation: Option<f32>,
    pub map_id: Option<i32>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TargetPointFact {
    pub transport: Guid,
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct PowerFact {
    pub amount: i32,
    pub power_type: i8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RuneFact {
    pub start: u8,
    pub count: u8,
    pub cooldowns: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MissStatusFact {
    pub reason: u8,
    pub reflect_status: Option<u8>,
}

pub(crate) fn load(path: &Path) -> Result<Options> {
    let source_path = path.display().to_string();
    let plan: Plan = serde_json::from_slice(
        &std::fs::read(path).with_context(|| format!("read cast lifecycle plan {source_path}"))?,
    )
    .with_context(|| format!("parse cast lifecycle plan {source_path}"))?;
    validate_plan(&plan)?;
    Ok(Options { source_path, plan })
}

pub(crate) async fn run(
    bot: config::BotConfig,
    dungeon_id: u32,
    lfg_secs: u64,
    options: Options,
) -> Result<BotRunResult> {
    super::run_bot_with_void_storage(
        bot,
        dungeon_id,
        lfg_secs,
        false,
        false,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        Some(options),
        None,
    )
    .await
}

pub(crate) fn failure_result(
    bot: &config::BotConfig,
    dungeon_id: u32,
    failure: String,
) -> BotRunResult {
    BotRunResult {
        account: bot.account.clone(),
        account_id: bot.account_id,
        character_guid: bot.character_guid,
        dungeon_id,
        role: bot.lfg_role,
        cast_lifecycle: Some(Evidence {
            failure: Some(failure),
            ..Evidence::default()
        }),
        ..BotRunResult::default()
    }
}

pub(super) fn validate_plan(plan: &Plan) -> Result<()> {
    if plan.timeout_ms == 0 || plan.timeout_ms > MAX_PLAN_TIMEOUT_MS {
        bail!(
            "cast lifecycle timeout_ms must be in 1..={MAX_PLAN_TIMEOUT_MS}, got {}",
            plan.timeout_ms
        );
    }
    if plan.actions.len() > MAX_ACTIONS {
        bail!(
            "cast lifecycle action count {} exceeds the bounded maximum {MAX_ACTIONS}",
            plan.actions.len()
        );
    }
    if plan.expect.len() > MAX_EXPECTATIONS {
        bail!(
            "cast lifecycle expectation count {} exceeds the bounded maximum {MAX_EXPECTATIONS}",
            plan.expect.len()
        );
    }
    if plan
        .character_guid
        .is_some_and(|counter| counter == 0 || counter > OBJECT_GUID_COUNTER_MASK)
    {
        bail!(
            "character_guid override must be a nonzero 40-bit counter (1..={OBJECT_GUID_COUNTER_MASK})"
        );
    }
    // An explicit collection-only observer is useful for later paired analysis;
    // its empty expectation set can never pass the evaluator.
    if plan.observe_only && (!plan.actions.is_empty() || !plan.expect.is_empty()) {
        bail!(
            "observe_only requires empty actions and expectations; pair its facts with the caster capture externally"
        );
    }
    if plan.expect.is_empty() && !plan.observe_only {
        bail!("cast lifecycle plan requires at least one explicit expect rule");
    }
    let mut client_ids = std::collections::HashSet::new();
    for action in &plan.actions {
        match action {
            Action::Cast {
                spell_id,
                cast_id,
                target,
            } => {
                if *spell_id <= 0 || cast_id.empty() {
                    bail!("cast action requires a positive spell_id and non-empty cast_id");
                }
                if !client_ids.insert(*cast_id) {
                    bail!(
                        "cast actions require distinct client CastIDs for unambiguous correlation"
                    );
                }
                if target.is_some_and(|guid| guid.empty()) {
                    bail!("cast action target must be non-empty when provided");
                }
            }
            Action::Cancel {
                cast_id, spell_id, ..
            } => {
                if *spell_id == 0 || cast_id.empty() {
                    bail!("cancel action requires a positive spell_id and non-empty cast_id");
                }
            }
            Action::Wait { milliseconds } if *milliseconds > MAX_WAIT_MS => {
                bail!("wait milliseconds exceeds bounded maximum {MAX_WAIT_MS}");
            }
            Action::WaitForPacket {
                opcode, timeout_ms, ..
            } => {
                if *timeout_ms == 0 || *timeout_ms > MAX_WAIT_MS {
                    bail!(
                        "wait_for_packet timeout_ms must be in 1..={MAX_WAIT_MS}, got {timeout_ms}"
                    );
                }
                if !supported_wait_packet_opcode(*opcode) {
                    bail!(
                        "wait_for_packet opcode 0x{opcode:04X} is unsupported or reserved for authentication/control traffic"
                    );
                }
            }
            Action::SelfRes { spell_id } if *spell_id <= 0 => {
                bail!("self_res action requires a positive int32 spell_id");
            }
            Action::SpiritHealerActivate { healer } => {
                validate_creature_target(healer, "spirit_healer_activate.healer")?;
            }
            Action::AttackSwing { target } => {
                validate_creature_target(target, "attack_swing.target")?;
            }
            Action::Movement {
                kind,
                x,
                y,
                z,
                orientation,
                fall,
                ..
            } => {
                if ![x, y, z, orientation]
                    .into_iter()
                    .all(|value| value.is_finite())
                {
                    bail!("movement action coordinates and orientation must be finite");
                }
                if *kind == MovementKind::FallLand && fall.is_none() {
                    bail!("fall_land requires finite fall data");
                }
                if let Some(fall) = fall {
                    if fall.fall_time_ms == 0
                        || fall.fall_time_ms > MAX_WAIT_MS as u32
                        || ![fall.z_speed, fall.sin_angle, fall.cos_angle, fall.xy_speed]
                            .into_iter()
                            .all(f32::is_finite)
                    {
                        bail!(
                            "movement fall data requires a bounded positive time and finite floats"
                        );
                    }
                }
            }
            Action::Wait { .. }
            | Action::ReleaseSpirit { .. }
            | Action::ReclaimCorpse { .. }
            | Action::ResurrectResponse { .. }
            | Action::SelfRes { .. } => {}
        }
    }
    for rule in &plan.expect {
        if let ExpectedRule::Packet {
            opcode, body_hex, ..
        } = rule
        {
            if !supported_exact_packet_opcode(*opcode) {
                bail!(
                    "packet expect opcode 0x{opcode:04X} is unsupported or reserved for authentication/control traffic"
                );
            }
            validate_packet_body_hex(body_hex)?;
            continue;
        }
        match rule {
            ExpectedRule::Prepare { client_cast_id, .. } if (*client_cast_id).empty() => {
                bail!("prepare expect rule requires a non-empty client_cast_id");
            }
            ExpectedRule::Start {
                client_cast_id,
                server_cast_id,
                ..
            }
            | ExpectedRule::Go {
                client_cast_id,
                server_cast_id,
                ..
            }
            | ExpectedRule::CastFailed {
                client_cast_id,
                server_cast_id,
                ..
            }
            | ExpectedRule::SpellFailure {
                client_cast_id,
                server_cast_id,
                ..
            }
            | ExpectedRule::SpellFailedOther {
                client_cast_id,
                server_cast_id,
                ..
            } if (*client_cast_id).is_none_or(|guid| guid.empty())
                && (*server_cast_id).is_none_or(|guid| guid.empty()) =>
            {
                bail!("expect rule requires client_cast_id or server_cast_id");
            }
            _ => {}
        }
        match rule {
            ExpectedRule::Start { spell_id, .. }
            | ExpectedRule::Go { spell_id, .. }
            | ExpectedRule::CastFailed { spell_id, .. }
            | ExpectedRule::SpellFailure { spell_id, .. }
            | ExpectedRule::SpellFailedOther { spell_id, .. }
                if *spell_id <= 0 =>
            {
                bail!("expect rule requires a positive spell_id");
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_creature_target(target: &CreatureTarget, field: &str) -> Result<()> {
    let CreatureTarget::Observed(selector) = target else {
        // Literal GUIDs intentionally retain their full negative-test range.
        return Ok(());
    };
    selector
        .validate()
        .with_context(|| format!("invalid {field} observed-creature selector"))
}

fn validate_packet_body_hex(body_hex: &str) -> Result<()> {
    if body_hex.len() % 2 != 0 {
        bail!("packet expect body_hex must contain complete bytes");
    }
    let byte_count = body_hex.len() / 2;
    if byte_count > MAX_EXPECTED_PACKET_BODY_BYTES {
        bail!(
            "packet expect body_hex is {byte_count} bytes; maximum is {MAX_EXPECTED_PACKET_BODY_BYTES}"
        );
    }
    if !body_hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("packet expect body_hex must contain only hexadecimal digits");
    }
    hex::decode(body_hex).context("decode packet expect body_hex")?;
    Ok(())
}

pub(super) fn supported_exact_packet_opcode(opcode: u16) -> bool {
    matches!(
        opcode,
        RAW_SMSG_AREA_SPIRIT_HEALER_TIME
            | RAW_SMSG_ATTACKER_STATE_UPDATE
            | RAW_SMSG_ATTACK_START
            | RAW_SMSG_ATTACK_STOP
            | RAW_SMSG_ATTACK_SWING_ERROR
            | RAW_SMSG_AURA_UPDATE
            | SMSG_CAST_FAILED
            | RAW_SMSG_CORPSE_LOCATION
            | RAW_SMSG_CORPSE_RECLAIM_DELAY
            | RAW_SMSG_DEATH_RELEASE_LOC
            | RAW_SMSG_DURABILITY_DAMAGE_DEATH
            | RAW_SMSG_FORCED_DEATH_UPDATE
            | SMSG_LOGOUT_COMPLETE
            | RAW_SMSG_PRE_RESSURECT
            | RAW_SMSG_REATTACH_RESURRECT
            | RAW_SMSG_RESURRECT_REQUEST
            | SMSG_SPELL_FAILED_OTHER
            | SMSG_SPELL_FAILURE
            | SMSG_SPELL_GO
            | SMSG_SPELL_PREPARE
            | SMSG_SPELL_START
            | RAW_SMSG_UPDATE_OBJECT
    )
}

pub(super) fn supported_wait_packet_opcode(opcode: u16) -> bool {
    supported_exact_packet_opcode(opcode)
        || matches!(
            opcode,
            SMSG_LOGIN_VERIFY_WORLD
                | SMSG_MOVE_ROOT
                | SMSG_MOVE_UNROOT
                | SMSG_MOVE_SET_LAND_WALK
                | SMSG_MOVE_SET_RUN_SPEED
                | SMSG_MOVE_SET_SWIM_SPEED
                | SMSG_MOVE_SET_WATER_WALK
                | SMSG_MOVE_TELEPORT
                | SMSG_NEW_WORLD
                | SMSG_RESUME_TOKEN
                | SMSG_SUSPEND_TOKEN
                | SMSG_TRANSFER_PENDING
        )
}
