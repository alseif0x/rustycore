//! JSON action plan and structured report types for cast lifecycle QA.

use super::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

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

#[derive(Debug, Clone)]
pub(crate) struct Options {
    pub(crate) source_path: String,
    pub(crate) plan: Plan,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Plan {
    #[serde(default)]
    pub(crate) observe_only: bool,
    #[serde(default = "default_timeout_ms")]
    pub(crate) timeout_ms: u64,
    #[serde(default)]
    pub(crate) actions: Vec<Action>,
    #[serde(default)]
    pub(crate) expect: Vec<ExpectedRule>,
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
    Wait {
        milliseconds: u64,
    },
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
    pub sent_packets: Vec<SentPacket>,
    pub logout_confirmed: bool,
    pub logout_route: Option<String>,
    pub failure: Option<String>,
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
            sent_packets: Vec::new(),
            logout_confirmed: false,
            logout_route: None,
            failure: None,
        }
    }
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
    pub action: String,
    pub opcode: u16,
    pub bytes: usize,
    pub body_sha256: String,
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
    pub connection: Connection,
    pub opcode: u16,
    pub kind: String,
    pub body_bytes: usize,
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
    // An explicit collection-only observer is useful for later paired analysis;
    // its empty expectation set can never pass the evaluator.
    if plan.observe_only && (!plan.actions.is_empty() || !plan.expect.is_empty()) {
        bail!("observe_only requires empty actions and expectations; pair its facts with the caster capture externally");
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
            Action::Wait { .. } => {}
        }
    }
    for rule in &plan.expect {
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
