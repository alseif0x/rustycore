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
    let mut next_instance_sequence = None;
    let mut next_realm_sequence = None;
    let mut all_matched = evidence.failure.is_none();
    let mut matched_sequences = std::collections::HashSet::new();
    let events = evidence.events.as_slice();
    let game_object_selections = evidence.game_object_selections.as_slice();
    for fact in &mut evidence.expected_rules {
        let connection = match &fact.rule {
            ExpectedRule::Packet { connection, .. } => *connection,
            _ => Connection::Instance,
        };
        let after = match connection {
            Connection::Instance => next_instance_sequence,
            Connection::Realm => next_realm_sequence,
        };
        let found = events.iter().find(|event| {
            event.connection == connection
                && after.is_none_or(|sequence| event.sequence > sequence)
                && event.error.is_none()
                && matches_rule(&fact.rule, event, events, game_object_selections)
        });
        if let Some(event) = found {
            fact.matched = true;
            fact.event_sequence = Some(event.sequence);
            fact.detail = format!("matched ordered {connection:?} event {}", event.sequence);
            match connection {
                Connection::Instance => next_instance_sequence = Some(event.sequence),
                Connection::Realm => next_realm_sequence = Some(event.sequence),
            }
            matched_sequences.insert(event.sequence);
        } else {
            fact.matched = false;
            fact.event_sequence = None;
            fact.detail = format!("no ordered {connection:?} decoded event satisfied this rule");
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
    let exact_packet_opcodes = evidence
        .expected_rules
        .iter()
        .filter_map(|fact| match &fact.rule {
            ExpectedRule::Packet { opcode, .. } => Some(*opcode),
            ExpectedRule::LootResponse { .. } => Some(crate::loot_race::SMSG_LOOT_RESPONSE),
            ExpectedRule::LootMoneyNotify { .. } => Some(crate::loot_race::SMSG_LOOT_MONEY_NOTIFY),
            ExpectedRule::LootCoinRemoved {} => Some(crate::loot_race::SMSG_COIN_REMOVED),
            ExpectedRule::LootRelease {} => Some(SMSG_LOOT_RELEASE),
            _ => None,
        })
        .collect::<std::collections::HashSet<_>>();
    if let Some(event) = evidence.events.iter().find(|event| {
        exact_packet_opcodes.contains(&event.opcode) && !matched_sequences.contains(&event.sequence)
    }) {
        all_matched = false;
        evidence.failure.get_or_insert_with(|| {
            format!(
                "unexpected, duplicate, or wrong-route exact packet 0x{:04X} at event {} on {:?}",
                event.opcode, event.sequence, event.connection
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

/// Apply the finish-policy proof requirement before evaluating the final report.
pub(super) fn finalize_finish(evidence: &mut Evidence) {
    match evidence.finish_policy {
        FinishPolicy::Logout => {
            if !evidence.logout_request_completed {
                evidence.failure.get_or_insert_with(|| {
                    "normal logout request was not locally written".to_string()
                });
            }
            if !evidence.logout_confirmed {
                evidence
                    .failure
                    .get_or_insert_with(|| "normal logout was not confirmed".to_string());
            }
            if evidence.logout_route.as_deref() != Some("realm") {
                evidence
                    .failure
                    .get_or_insert_with(|| "normal logout was not confirmed on realm".to_string());
            }
            if !evidence.finish_completed {
                evidence.failure.get_or_insert_with(|| {
                    "normal logout finish did not complete locally".to_string()
                });
            }
        }
        FinishPolicy::Disconnect => {
            evidence.logout_confirmed = false;
            evidence.logout_route = None;
            if !evidence.local_disconnect_recorded || !evidence.finish_completed {
                evidence
                    .failure
                    .get_or_insert_with(|| "local disconnect was not recorded".to_string());
            }
        }
    }
    evaluate(evidence);
}

/// Kept as a small test helper for the original default logout contract.
pub(super) fn finalize_logout(evidence: &mut Evidence) {
    evidence.finish_policy = FinishPolicy::Logout;
    finalize_finish(evidence);
}

fn event_is_plan_bound(
    event: &ObservedEvent,
    expected_rules: &[ExpectedFact],
    identities: &HashMap<Guid, Guid>,
) -> bool {
    expected_rules.iter().any(|fact| match &fact.rule {
        ExpectedRule::Packet { .. }
        | ExpectedRule::LootResponse { .. }
        | ExpectedRule::LootMoneyNotify { .. }
        | ExpectedRule::LootCoinRemoved {}
        | ExpectedRule::LootRelease {} => false,
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

fn matches_rule(
    rule: &ExpectedRule,
    event: &ObservedEvent,
    events: &[ObservedEvent],
    game_object_selections: &[GameObjectSelectionFact],
) -> bool {
    if let ExpectedRule::Packet {
        connection,
        opcode,
        body_hex,
    } = rule
    {
        return event.connection == *connection
            && event.opcode == *opcode
            && hex::decode(body_hex).is_ok_and(|expected| {
                hex::decode(&event.body_hex).is_ok_and(|actual| actual == expected)
            });
    }
    // Classic Opcodes.cpp registers all six observed cast opcodes on INSTANCE.
    if event.connection != Connection::Instance {
        return false;
    }
    match rule {
        ExpectedRule::Packet { .. } => unreachable!("packet rules handled before cast route gate"),
        ExpectedRule::LootResponse { coins, item_entry } => {
            if event.opcode != crate::loot_race::SMSG_LOOT_RESPONSE {
                return false;
            }
            let Ok(payload) = hex::decode(&event.body_hex) else {
                return false;
            };
            let Ok(response) = crate::loot_race::parse_loot_response(&payload) else {
                return false;
            };
            let Some(selection) = game_object_selections.first() else {
                return false;
            };
            response.failure_reason == CXX_DEFAULT_LOOT_RESPONSE_FAILURE_REASON
                && response.acquired
                && (response.owner_low, response.owner_high)
                    == (selection.guid.low, selection.guid.high)
                && crate::loot_race::validate_loot_object_guid_like_cpp(
                    response.loot_low,
                    response.loot_high,
                    crate::loot_race::RACE_GAMEOBJECT_MAP_ID,
                    realm_id(),
                )
                .is_ok()
                && response.coins == *coins
                && response.items.len() == 1
                && response.items[0].item_entry == *item_entry
                && response.items[0].quantity == 1
        }
        ExpectedRule::LootMoneyNotify { money, money_mod } => {
            event.opcode == crate::loot_race::SMSG_LOOT_MONEY_NOTIFY
                && hex::decode(&event.body_hex)
                    .ok()
                    .and_then(|payload| wire::parse_loot_money_notify(&payload).ok())
                    .is_some_and(|notify| {
                        notify.money == *money
                            && notify.money_mod == *money_mod
                            && notify.sole_looter
                    })
        }
        ExpectedRule::LootCoinRemoved {} => {
            event.opcode == crate::loot_race::SMSG_COIN_REMOVED
                && hex::decode(&event.body_hex)
                    .ok()
                    .and_then(|payload| wire::parse_loot_coin_removed(&payload).ok())
                    .is_some_and(|coin_guid| {
                        first_loot_response_before(events, event.sequence).is_some_and(|response| {
                            (coin_guid.low, coin_guid.high)
                                == (response.loot_low, response.loot_high)
                        })
                    })
        }
        ExpectedRule::LootRelease {} => {
            event.opcode == SMSG_LOOT_RELEASE
                && hex::decode(&event.body_hex)
                    .ok()
                    .and_then(|payload| wire::parse_loot_release(&payload).ok())
                    .is_some_and(|(loot_guid, owner_guid)| {
                        first_loot_response_before(events, event.sequence).is_some_and(|response| {
                            (loot_guid.low, loot_guid.high)
                                == (response.owner_low, response.owner_high)
                                && !owner_guid.empty()
                        })
                    })
        }
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

fn first_loot_response_before(
    events: &[ObservedEvent],
    sequence: usize,
) -> Option<crate::loot_race::ParsedLootResponse> {
    let event = events
        .iter()
        .filter(|event| {
            event.sequence < sequence
                && event.connection == Connection::Instance
                && event.opcode == crate::loot_race::SMSG_LOOT_RESPONSE
                && event.error.is_none()
        })
        .min_by_key(|event| event.sequence)?;
    let payload = hex::decode(&event.body_hex).ok()?;
    crate::loot_race::parse_loot_response(&payload).ok()
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
        TargetExpectation::CorpseAlly(guid) => {
            actual.flags & 0x8000 != 0 && actual.unit == *guid && actual.item.empty()
        }
    }
}
