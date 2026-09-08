//! Ordered, identity-bound acceptance for observed Player cast packets.

use super::*;
use std::collections::HashMap;

pub(super) fn correlate_event(evidence: &Evidence, event: &mut ObservedEvent) {
    if event.client_cast_id.is_some() {
        return;
    }
    let Some(cast_id) = event.server_cast_id.or(event.cast_id) else {
        return;
    };
    let identities = prepare_identity_map(&evidence.events);
    event.client_cast_id = identities.get(&cast_id).copied().or_else(|| {
        (event.kind == "cast_failed" && evidence.actions.iter().any(|action| {
            matches!(action, Action::Cast { cast_id: client, .. } if *client == cast_id)
        })).then_some(cast_id)
    });
}

pub(super) fn prepare_identity_map(events: &[ObservedEvent]) -> HashMap<Guid, Guid> {
    events
        .iter()
        .filter_map(|event| {
            (event.kind == "spell_prepare")
                .then(|| Some((event.server_cast_id?, event.client_cast_id?)))
                .flatten()
        })
        .collect()
}

pub(super) fn evaluate(evidence: &mut Evidence) {
    let mut next_sequence = 0usize;
    let mut all_matched = evidence.failure.is_none();
    let mut matched_sequences = std::collections::HashSet::new();
    for fact in &mut evidence.expected_rules {
        let found = evidence
            .events
            .iter()
            .skip(next_sequence)
            .find(|event| event.error.is_none() && matches_rule(&fact.rule, event));
        if let Some(event) = found {
            fact.matched = true;
            fact.event_sequence = Some(event.sequence);
            fact.detail = format!("matched ordered event {}", event.sequence);
            next_sequence = event.sequence + 1;
            matched_sequences.insert(event.sequence);
        } else {
            fact.matched = false;
            fact.event_sequence = None;
            fact.detail = "no ordered decoded event satisfied this identity/spell rule".to_string();
            all_matched = false;
        }
    }

    // The ordered matcher above intentionally permits unrelated packets between
    // expected facts.  A cast packet carrying an identity from this plan is
    // different: an extra Start/Go would make a cancellation or failure appear
    // successful even though the spell was launched.  Reject every plan-bound
    // cast event that was not consumed by its expectation. This also rejects
    // duplicate mappings and failures rather than silently accepting them. The
    // complete Prepare map is used here so a caster's client id also binds to
    // later packets; observer captures can use their explicit server id.
    let identities = prepare_identity_map(&evidence.events);
    let unexpected = evidence.events.iter().find(|event| {
        matches!(
            event.kind.as_str(),
            "spell_prepare"
                | "spell_start"
                | "spell_go"
                | "cast_failed"
                | "spell_failure"
                | "spell_failed_other"
        ) && event_is_plan_bound(event, &evidence.expected_rules, &identities)
            && !matched_sequences.contains(&event.sequence)
    });
    if let Some(event) = unexpected {
        all_matched = false;
        evidence.failure.get_or_insert_with(|| {
            format!(
                "unexpected plan-bound {} event at sequence {}",
                event.kind, event.sequence
            )
        });
    }
    if let Some(event) = evidence.events.iter().find(|event| event.error.is_some()) {
        evidence.failure.get_or_insert_with(|| {
            format!(
                "cannot accept undecoded event {}: {:?}",
                event.sequence, event.error
            )
        });
    }
    // An `observe_only` capture has no expectations by construction, but the
    // rule is stated here as well: collection is never acceptance, so no
    // observer report may be relabelled as a pass.
    evidence.passed = all_matched
        && !evidence.observe_only
        && !evidence.expected_rules.is_empty()
        && evidence.failure.is_none();
}

/// Normal logout is part of the scenario contract: an unconfirmed empty REALM
/// SMSG_LOGOUT_COMPLETE fails the run even when every cast expectation matched.
pub(super) fn finalize_logout(evidence: &mut Evidence) {
    if !evidence.logout_confirmed {
        evidence
            .failure
            .get_or_insert_with(|| "normal logout was not confirmed".to_string());
    }
    evaluate(evidence);
}

fn event_is_plan_bound(
    event: &ObservedEvent,
    expected_rules: &[ExpectedFact],
    identities: &HashMap<Guid, Guid>,
) -> bool {
    expected_rules.iter().any(|fact| match &fact.rule {
        ExpectedRule::Prepare {
            client_cast_id,
            server_cast_id,
        } => identity_matches_either(event, Some(*client_cast_id), *server_cast_id, identities),
        ExpectedRule::Start {
            client_cast_id,
            server_cast_id,
            ..
        } => identity_matches_either(event, *client_cast_id, *server_cast_id, identities),
        ExpectedRule::Go {
            client_cast_id,
            server_cast_id,
            ..
        } => identity_matches_either(event, *client_cast_id, *server_cast_id, identities),
        ExpectedRule::CastFailed {
            client_cast_id,
            server_cast_id,
            ..
        } => identity_matches_either(event, *client_cast_id, *server_cast_id, identities),
        ExpectedRule::SpellFailure {
            client_cast_id,
            server_cast_id,
            ..
        } => identity_matches_either(event, *client_cast_id, *server_cast_id, identities),
        ExpectedRule::SpellFailedOther {
            client_cast_id,
            server_cast_id,
            ..
        } => identity_matches_either(event, *client_cast_id, *server_cast_id, identities),
    })
}

fn identity_matches_either(
    event: &ObservedEvent,
    expected_client: Option<Guid>,
    expected_server: Option<Guid>,
    identities: &HashMap<Guid, Guid>,
) -> bool {
    let event_server = event.server_cast_id.or(event.cast_id);
    let event_client = event
        .client_cast_id
        .or_else(|| event_server.and_then(|server| identities.get(&server).copied()));
    let event_server = event_server.or_else(|| {
        event.client_cast_id.and_then(|client| {
            identities
                .iter()
                .find_map(|(server, mapped_client)| (*mapped_client == client).then_some(*server))
        })
    });
    expected_client.is_some_and(|expected| event_client == Some(expected))
        || expected_server.is_some_and(|expected| event_server == Some(expected))
}

fn matches_rule(rule: &ExpectedRule, event: &ObservedEvent) -> bool {
    // Classic Opcodes.cpp registers all six observed cast opcodes on INSTANCE.
    if event.connection != Connection::Instance {
        return false;
    }
    match rule {
        ExpectedRule::Prepare {
            client_cast_id,
            server_cast_id,
        } => {
            event.kind == "spell_prepare"
                && event.client_cast_id == Some(*client_cast_id)
                && (*server_cast_id).is_none_or(|expected| event.server_cast_id == Some(expected))
        }
        ExpectedRule::Start {
            metadata,
            client_cast_id,
            server_cast_id,
            spell_id,
            target,
            cast_flags,
            cast_flags_ex,
        } => {
            event.kind == "spell_start"
                && metadata_matches(metadata.as_ref(), event)
                && identity_matches(event, *client_cast_id, *server_cast_id)
                && event.spell_id == Some(*spell_id)
                && target_matches(target, event.target.as_ref())
                && (*cast_flags).is_none_or(|expected| event.cast_flags == Some(expected))
                && (*cast_flags_ex).is_none_or(|expected| event.cast_flags_ex == Some(expected))
        }
        ExpectedRule::Go {
            metadata,
            client_cast_id,
            server_cast_id,
            spell_id,
            target,
            cast_flags,
            cast_flags_ex,
            hit_targets,
            miss_targets,
            full_combat_log,
        } => {
            event.kind == "spell_go"
                && metadata_matches(metadata.as_ref(), event)
                && identity_matches(event, *client_cast_id, *server_cast_id)
                && event.spell_id == Some(*spell_id)
                && target_matches(target, event.target.as_ref())
                && (*cast_flags).is_none_or(|expected| event.cast_flags == Some(expected))
                && (*cast_flags_ex).is_none_or(|expected| event.cast_flags_ex == Some(expected))
                && (*hit_targets).is_none_or(|expected| event.hit_targets.len() == expected)
                && (*miss_targets).is_none_or(|expected| event.miss_targets.len() == expected)
                && (*full_combat_log).is_none_or(|expected| event.full_combat_log == Some(expected))
        }
        ExpectedRule::CastFailed {
            client_cast_id,
            server_cast_id,
            spell_id,
            reason,
        } => {
            event.kind == "cast_failed"
                && identity_matches(event, *client_cast_id, *server_cast_id)
                && event.spell_id == Some(*spell_id)
                && (*reason).is_none_or(|expected| event.reason == Some(expected as u32))
        }
        ExpectedRule::SpellFailure {
            client_cast_id,
            server_cast_id,
            spell_id,
            reason,
        } => {
            event.kind == "spell_failure"
                && identity_matches(event, *client_cast_id, *server_cast_id)
                && event.spell_id == Some(*spell_id)
                && (*reason).is_none_or(|expected| event.reason == Some(expected as u32))
        }
        ExpectedRule::SpellFailedOther {
            client_cast_id,
            server_cast_id,
            spell_id,
            reason,
        } => {
            event.kind == "spell_failed_other"
                && identity_matches(event, *client_cast_id, *server_cast_id)
                && event.spell_id == Some(*spell_id)
                && (*reason).is_none_or(|expected| event.reason == Some(expected as u32))
        }
    }
}

fn identity_matches(
    event: &ObservedEvent,
    expected_client: Option<Guid>,
    expected_server: Option<Guid>,
) -> bool {
    let observed_server = event.server_cast_id.or(event.cast_id);
    expected_client.is_none_or(|expected| event.client_cast_id == Some(expected))
        && expected_server.is_none_or(|expected| observed_server == Some(expected))
}

fn metadata_matches(expected: Option<&CastMetadataExpectation>, event: &ObservedEvent) -> bool {
    expected.is_none_or(|expected| {
        expected
            .visual_id
            .is_none_or(|value| event.visual_id == Some(value))
            && expected
                .original_cast_id
                .is_none_or(|value| event.original_cast_id == Some(value))
            && expected
                .cast_time_ms
                .is_none_or(|value| event.cast_time_ms == Some(value))
            && expected
                .remaining_power
                .as_ref()
                .is_none_or(|value| &event.remaining_power == value)
    })
}

fn target_matches(expected: &TargetExpectation, actual: Option<&TargetFact>) -> bool {
    let Some(actual) = actual else { return false };
    match expected {
        TargetExpectation::Empty => actual.flags == 0 && actual.unit.empty() && actual.item.empty(),
        TargetExpectation::Unit(guid) => {
            actual.flags & 0x2 != 0 && actual.unit == *guid && actual.item.empty()
        }
    }
}
