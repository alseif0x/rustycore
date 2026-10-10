// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The three production owners attempted together for one admitted transition.

use super::*;

// ── Production-shape composition ─────────────────────────────────────────────
//
// The isolated path also owns the *composition* entry the F6-8D2 acceptance
// requires: the three production owners are attempted together for one admitted
// transition, and exactly one of them executes. Production never calls this
// entry; the candidates it evaluates are the real production gates —
// `RuntimeTickOwner` for the legacy and session arms — plus the single-owner
// claim for the canonical arm.

/// What the session-owned arm produced when it was admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IsolatedSessionArmOutcomeLikeCpp {
    pub publications: usize,
}

/// What the legacy-owned arm produced when it was admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IsolatedLegacyArmOutcomeLikeCpp {
    pub skipped_owner_not_global: bool,
    pub creatures_seen: usize,
    pub due_casts: usize,
    pub swing_damage: u64,
    pub publications: usize,
}

/// One composed attempt at one admitted transition.
#[derive(Debug, Clone)]
pub struct IsolatedCreatureExecutionCompositionOutcomeLikeCpp {
    /// The one owner admitted to execute, or `None` when all three refused.
    pub admitted_owner: Option<CreatureExecutionOwnerLikeCpp>,
    /// Every owner that was attempted and refused, with its reason label.
    pub refusals: Vec<(CreatureExecutionOwnerLikeCpp, &'static str)>,
    pub canonical: Option<IsolatedCreatureExecutionOutcomeLikeCpp>,
    pub legacy: Option<IsolatedLegacyArmOutcomeLikeCpp>,
    pub session: Option<IsolatedSessionArmOutcomeLikeCpp>,
}

impl IsolatedCreatureExecutionCompositionOutcomeLikeCpp {
    /// Publication events of the one admitted owner.
    #[must_use]
    pub fn publication_events_like_cpp(&self) -> usize {
        self.canonical.as_ref().map_or(
            0,
            IsolatedCreatureExecutionOutcomeLikeCpp::publication_events_like_cpp,
        ) + self.legacy.map_or(0, |legacy| legacy.publications)
            + self.session.map_or(0, |session| session.publications)
    }
}

/// Attempt the canonical, legacy and session owners together and execute
/// exactly one of them.
///
/// The admission rule is the production rule, made explicit. `canonical_armed`
/// is the policy input the admitted tick supplies — today production holds
/// `RuntimeTickOwner::GlobalLegacy` and therefore never arms it; D3 arms it
/// exclusively:
///
/// * an **armed** canonical transition supersedes the other two owners. The
///   canonical arm executes when the admission fence and the single-owner claim
///   both admit it, and the legacy and session owners are refused with
///   `canonical-admitted-claim`;
/// * an armed transition the canonical owner cannot execute is **closed**: the
///   fence refused it (the admitted incarnation or record is gone) or another
///   owner already holds its claim. No other owner may run over it, because
///   running one would execute the same admitted diff twice;
/// * otherwise the legacy global owner executes when
///   `RuntimeTickOwner::GlobalLegacy` is set (the production default);
/// * otherwise the session owner executes when `RuntimeTickOwner::Session` is
///   set and a session arm was supplied (the per-session configuration).
///
/// The refused owners are reported with the gate that refused them, and none of
/// them is touched: an inactive owner produces zero effects.
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn run_isolated_admitted_creature_execution_composition_like_cpp(
    admission: &AdmittedCreatureExecutionLikeCpp,
    canonical_armed: bool,
    canonical_map_manager: &SharedCanonicalMapManager,
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    lease: &SharedCreatureExecutionLeaseLikeCpp,
    tick_owner: crate::map_manager::RuntimeTickOwner,
    legacy_config: &LegacyCreatureAggroConfigLikeCpp,
    session_arm: Option<&mut dyn FnMut() -> IsolatedSessionArmOutcomeLikeCpp>,
) -> IsolatedCreatureExecutionCompositionOutcomeLikeCpp {
    let mut outcome = IsolatedCreatureExecutionCompositionOutcomeLikeCpp {
        admitted_owner: None,
        refusals: Vec::new(),
        canonical: None,
        legacy: None,
        session: None,
    };

    if canonical_armed {
        let canonical = run_admitted_creature_execution_isolated_like_cpp(
            admission,
            canonical_map_manager,
            lease,
        );
        if canonical.executed_like_cpp() {
            outcome.admitted_owner = Some(CreatureExecutionOwnerLikeCpp::CanonicalAdmitted);
            outcome.refusals.push((
                CreatureExecutionOwnerLikeCpp::LegacyGlobal,
                "canonical-admitted-claim",
            ));
            outcome.refusals.push((
                CreatureExecutionOwnerLikeCpp::Session,
                "canonical-admitted-claim",
            ));
            outcome.canonical = Some(canonical);
            return outcome;
        }
        // The transition is closed for this tick: the fence refused the admitted
        // incarnation/record, or the claim is already held. Falling through to a
        // second owner would execute the same admitted diff twice.
        outcome.refusals.push((
            CreatureExecutionOwnerLikeCpp::CanonicalAdmitted,
            canonical
                .admission
                .refusal_label_like_cpp()
                .unwrap_or("canonical-admitted"),
        ));
        outcome.refusals.push((
            CreatureExecutionOwnerLikeCpp::LegacyGlobal,
            "canonical-transition-closed",
        ));
        outcome.refusals.push((
            CreatureExecutionOwnerLikeCpp::Session,
            "canonical-transition-closed",
        ));
        outcome.canonical = Some(canonical);
        return outcome;
    }

    outcome.refusals.push((
        CreatureExecutionOwnerLikeCpp::CanonicalAdmitted,
        "transition-not-armed",
    ));

    match tick_owner {
        crate::map_manager::RuntimeTickOwner::GlobalLegacy => {
            let legacy = run_legacy_creature_execution_arm_like_cpp(
                legacy_map_manager,
                canonical_map_manager,
                admission,
                legacy_config,
            );
            if legacy.skipped_owner_not_global {
                outcome.refusals.push((
                    CreatureExecutionOwnerLikeCpp::LegacyGlobal,
                    "owner-not-global-legacy",
                ));
            } else {
                outcome.admitted_owner = Some(CreatureExecutionOwnerLikeCpp::LegacyGlobal);
                outcome
                    .refusals
                    .push((CreatureExecutionOwnerLikeCpp::Session, "owner-not-session"));
                outcome.legacy = Some(legacy);
                return outcome;
            }
        }
        crate::map_manager::RuntimeTickOwner::Session
        | crate::map_manager::RuntimeTickOwner::CanonicalMap => {
            outcome.refusals.push((
                CreatureExecutionOwnerLikeCpp::LegacyGlobal,
                "owner-not-global-legacy",
            ));
        }
    }

    match (tick_owner, session_arm) {
        (crate::map_manager::RuntimeTickOwner::Session, Some(session_arm)) => {
            let session = session_arm();
            outcome.admitted_owner = Some(CreatureExecutionOwnerLikeCpp::Session);
            outcome.session = Some(session);
        }
        _ => outcome
            .refusals
            .push((CreatureExecutionOwnerLikeCpp::Session, "owner-not-session")),
    }
    outcome
}

/// The legacy global arm: the production legacy phases for the same diff.
///
/// The arm is the real production entry set — movement, spell and melee — so a
/// second owner is observable through the same clock, due-cast and swing
/// observables the canonical arm mutates.
fn run_legacy_creature_execution_arm_like_cpp(
    legacy_map_manager: &crate::map_manager::SharedMapManager,
    canonical_map_manager: &SharedCanonicalMapManager,
    admission: &AdmittedCreatureExecutionLikeCpp,
    legacy_config: &LegacyCreatureAggroConfigLikeCpp,
) -> IsolatedLegacyArmOutcomeLikeCpp {
    let movement = run_legacy_creature_movement_tick_once_like_cpp(
        legacy_map_manager,
        Some(canonical_map_manager),
        &MMapRuntimeConfigLikeCpp::default(),
        None,
        &std::collections::HashMap::new(),
        admission.diff_ms,
    );
    if movement.skipped_owner_not_global {
        return IsolatedLegacyArmOutcomeLikeCpp {
            skipped_owner_not_global: true,
            ..Default::default()
        };
    }
    let spell = run_legacy_creature_spell_tick_once_like_cpp(
        legacy_map_manager,
        Some(canonical_map_manager),
        legacy_config,
    );
    let melee = run_legacy_creature_melee_tick_once_like_cpp(
        legacy_map_manager,
        Some(canonical_map_manager),
        legacy_config,
    );
    IsolatedLegacyArmOutcomeLikeCpp {
        skipped_owner_not_global: false,
        creatures_seen: movement.creatures_seen,
        due_casts: spell.canonical_cast_preconditions_passed,
        swing_damage: melee
            .commands
            .iter()
            .map(|command| u64::from(command.damage))
            .sum(),
        publications: movement.plan.events.len()
            + spell.plan.events.len()
            + melee.plan.events.len(),
    }
}
