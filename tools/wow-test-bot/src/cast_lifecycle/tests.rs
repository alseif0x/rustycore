use super::*;

mod movement_controls;

fn expected(rule: ExpectedRule) -> ExpectedFact {
    ExpectedFact {
        rule,
        matched: false,
        event_sequence: None,
        detail: String::new(),
    }
}

fn failed_body(cast: Guid) -> Vec<u8> {
    let mut bytes = cast.packed();
    for value in [133i32, 77, 63, 0, 0] {
        bytes.extend(value.to_le_bytes());
    }
    bytes
}

#[test]
fn failure_correlates_server_id_only_after_prepare() {
    let mut event = decode_event(
        1,
        Connection::Instance,
        SMSG_CAST_FAILED,
        &failed_body(guid(42)),
    );
    assert_eq!(event.client_cast_id, None);
    let mut mapping = guid(41).packed();
    mapping.extend(guid(42).packed());
    let evidence = Evidence {
        events: vec![decode_event(
            0,
            Connection::Instance,
            SMSG_SPELL_PREPARE,
            &mapping,
        )],
        ..Default::default()
    };
    correlate_event(&evidence, &mut event);
    assert_eq!(event.client_cast_id, Some(guid(41)));
}

#[test]
fn go_after_failure_cannot_pass_an_ordered_failure_expectation() {
    let mut evidence = Evidence {
        events: vec![
            decode_event(
                0,
                Connection::Instance,
                SMSG_CAST_FAILED,
                &failed_body(guid(42)),
            ),
            decode_event(
                1,
                Connection::Instance,
                SMSG_SPELL_GO,
                &spell_cast_body(guid(42), None, true),
            ),
        ],
        expected_rules: vec![expected(ExpectedRule::CastFailed {
            client_cast_id: None,
            server_cast_id: Some(guid(42)),
            spell_id: 133,
            reason: Some(63),
        })],
        ..Default::default()
    };
    evaluate(&mut evidence);
    assert!(evidence.expected_rules[0].matched);
    assert!(!evidence.passed);
    assert!(
        evidence
            .failure
            .as_deref()
            .unwrap()
            .contains("unexpected plan-bound spell_go")
    );
}

#[test]
fn duplicate_start_and_wrong_connection_are_rejected() {
    let rule = ExpectedRule::Start {
        metadata: None,
        client_cast_id: None,
        server_cast_id: Some(guid(42)),
        spell_id: 133,
        target: TargetExpectation::Empty,
        cast_flags: Some(2),
        cast_flags_ex: Some(0),
    };
    for (connections, message) in [
        (
            vec![Connection::Instance, Connection::Instance],
            "duplicate Start",
        ),
        (vec![Connection::Realm], "wrong socket"),
    ] {
        let mut evidence = Evidence {
            events: connections
                .into_iter()
                .enumerate()
                .map(|(sequence, connection)| {
                    decode_event(
                        sequence,
                        connection,
                        SMSG_SPELL_START,
                        &spell_cast_body(guid(42), None, false),
                    )
                })
                .collect(),
            expected_rules: vec![expected(rule.clone())],
            ..Default::default()
        };
        evaluate(&mut evidence);
        assert!(!evidence.passed, "{message}");
    }
    let mut empty = Evidence::default();
    evaluate(&mut empty);
    assert!(
        !empty.passed,
        "process exit and an empty report are not acceptance"
    );
}

#[test]
fn selected_visual_original_id_and_power_are_acceptance_fields() {
    let mut evidence = Evidence {
        events: vec![decode_event(
            0,
            Connection::Instance,
            SMSG_SPELL_START,
            &spell_cast_body(guid(42), None, false),
        )],
        expected_rules: vec![expected(ExpectedRule::Start {
            client_cast_id: None,
            server_cast_id: Some(guid(42)),
            spell_id: 133,
            target: TargetExpectation::Empty,
            cast_flags: Some(2),
            cast_flags_ex: Some(0),
            metadata: Some(CastMetadataExpectation {
                visual_id: Some(77),
                original_cast_id: Some(guid(0)),
                remaining_power: Some(Vec::new()),
                ..Default::default()
            }),
        })],
        ..Default::default()
    };
    evaluate(&mut evidence);
    assert!(evidence.passed);
    evidence.events[0].visual_id = Some(0);
    evaluate(&mut evidence);
    assert!(
        !evidence.passed,
        "correct opcode/identity cannot hide the wrong visual"
    );
}

#[test]
fn plans_distinguish_unproven_observation_and_reject_reused_client_ids() {
    let observer: Plan =
        serde_json::from_str(r#"{"observe_only":true,"timeout_ms":100,"actions":[],"expect":[]}"#)
            .unwrap();
    assert!(plan::validate_plan(&observer).is_ok());
    let empty: Plan =
        serde_json::from_str(r#"{"timeout_ms":100,"actions":[],"expect":[]}"#).unwrap();
    assert!(plan::validate_plan(&empty).is_err());
    let duplicate: Plan = serde_json::from_str(
        r#"{
        "actions":[
            {"action":"cast","spell_id":133,"cast_id":{"low":1,"high":0}},
            {"action":"cast","spell_id":133,"cast_id":{"low":1,"high":0}}
        ],
        "expect":[{"event":"prepare","client_cast_id":{"low":1,"high":0}}]
    }"#,
    )
    .unwrap();
    assert!(plan::validate_plan(&duplicate).is_err());
}

fn guid(low: u64) -> Guid {
    Guid { low, high: 0 }
}

fn spell_cast_body(cast_id: Guid, target: Option<Guid>, is_go: bool) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend(guid(9).packed());
    body.extend(guid(9).packed());
    body.extend(cast_id.packed());
    body.extend(Guid { low: 0, high: 0 }.packed());
    body.extend(133i32.to_le_bytes());
    body.extend(77u32.to_le_bytes());
    body.extend(0x2u32.to_le_bytes());
    body.extend(0u32.to_le_bytes());
    body.extend(1u32.to_le_bytes());
    body.extend(0u32.to_le_bytes());
    body.extend(0f32.to_le_bytes());
    body.push(0);
    body.extend(0u32.to_le_bytes());
    body.extend(0u32.to_le_bytes());
    body.extend(0u32.to_le_bytes());
    body.push(0);
    body.extend(Guid { low: 0, high: 0 }.packed());
    body.extend([0u8; 10]);
    let mut target_header = [0u8; 5];
    if let Some(target) = target {
        write_msb_bits(&mut target_header, 0, 28, 2);
        body.extend(target_header);
        body.extend(target.packed());
    } else {
        body.extend(target_header);
        body.extend(Guid { low: 0, high: 0 }.packed());
    }
    body.extend(Guid { low: 0, high: 0 }.packed());
    if is_go {
        body.push(0);
    }
    body
}

#[test]
fn cast_request_preserves_empty_and_unit_target_wire_shapes() {
    let empty = build_cast_spell_payload(133, guid(1), None);
    let unit = build_cast_spell_payload(133, guid(1), Some(guid(22)));
    let target_offset = empty.len() - 9;
    assert_eq!(&empty[target_offset..target_offset + 5], &[0; 5]);
    assert_eq!(&unit[target_offset..target_offset + 5], &[0, 0, 0, 0x20, 0]);
    assert_ne!(empty, unit);
}

#[test]
fn pending_and_active_cancel_requests_use_distinct_cpp_opcodes_and_bodies() {
    let active = build_cancel_cast_payload(Guid { low: 1, high: 0 }, 133);
    assert_eq!(CMSG_CANCEL_QUEUED_SPELL, 0x3182);
    assert_eq!(CMSG_CANCEL_CAST, 0x329F);
    assert_eq!(build_cancel_queued_spell_payload(), Vec::<u8>::new());
    assert_eq!(active, vec![1, 0, 1, 133, 0, 0, 0]);
}

#[test]
fn parser_decodes_prepare_identity_and_optional_sections() {
    let client = guid(41);
    let server = guid(42);
    let mut prepare = client.packed();
    prepare.extend(server.packed());
    let mut evidence = Evidence::default();
    evidence.events.push(decode_event(
        0,
        Connection::Instance,
        SMSG_SPELL_PREPARE,
        &prepare,
    ));
    let mut start = spell_cast_body(server, Some(guid(22)), false);
    let event = decode_event(1, Connection::Instance, SMSG_SPELL_START, &start);
    assert_eq!(event.spell_id, Some(133));
    assert_eq!(event.server_cast_id, Some(server));
    assert_eq!(
        event.target.as_ref().map(|target| target.unit),
        Some(guid(22))
    );
    assert!(event.error.is_none());
    start.clear();
    let empty_go = decode_event(
        2,
        Connection::Instance,
        SMSG_SPELL_GO,
        &spell_cast_body(server, None, true),
    );
    assert_eq!(empty_go.full_combat_log, Some(false));
    assert!(
        empty_go
            .target
            .as_ref()
            .is_some_and(|target| target.flags == 0)
    );
    assert!(evidence.events[0].error.is_none());
}

#[test]
fn parser_decodes_remaining_power_and_target_points_in_cpp_order() {
    let server = guid(42);
    let mut body = Vec::new();
    body.extend(guid(9).packed());
    body.extend(guid(9).packed());
    body.extend(server.packed());
    body.extend(Guid { low: 0, high: 0 }.packed());
    body.extend(133i32.to_le_bytes());
    body.extend(77u32.to_le_bytes());
    body.extend(0x100u32.to_le_bytes());
    body.extend(0u32.to_le_bytes());
    body.extend(123u32.to_le_bytes());
    body.extend(0u32.to_le_bytes());
    body.extend(0f32.to_le_bytes());
    body.push(0);
    body.extend(0u32.to_le_bytes());
    body.extend(0u32.to_le_bytes());
    body.extend(0u32.to_le_bytes());
    body.push(0);
    body.extend(Guid { low: 0, high: 0 }.packed());
    let mut counts = [0u8; 10];
    write_msb_bits(&mut counts, 48, 9, 1);
    write_msb_bits(&mut counts, 58, 16, 1);
    body.extend(counts);
    body.extend([0u8; 5]);
    body.extend(Guid { low: 0, high: 0 }.packed());
    body.extend(Guid { low: 0, high: 0 }.packed());
    body.extend((-250i32).to_le_bytes());
    body.push(3u8);
    body.extend(Guid { low: 0, high: 0 }.packed());
    body.extend(1.25f32.to_le_bytes());
    body.extend((-2.5f32).to_le_bytes());
    body.extend(3.75f32.to_le_bytes());
    body.push(0);

    let event = decode_event(0, Connection::Instance, SMSG_SPELL_GO, &body);
    assert!(event.error.is_none());
    assert_eq!(event.remaining_power.len(), 1);
    assert_eq!(event.remaining_power[0].amount, -250);
    assert_eq!(event.remaining_power[0].power_type, 3);
    assert_eq!(event.target_points.len(), 1);
    assert_eq!(event.target_points[0].x, 1.25);
    assert_eq!(event.target_points[0].y, -2.5);
    assert_eq!(event.target_points[0].z, 3.75);
}

#[test]
fn expected_rules_require_prepare_bound_identity_and_order() {
    let client = guid(41);
    let server = guid(42);
    let mut prepare = client.packed();
    prepare.extend(server.packed());
    let events = vec![
        decode_event(0, Connection::Instance, SMSG_SPELL_PREPARE, &prepare),
        decode_event(
            1,
            Connection::Instance,
            SMSG_SPELL_START,
            &spell_cast_body(server, None, false),
        ),
        decode_event(
            2,
            Connection::Instance,
            SMSG_SPELL_GO,
            &spell_cast_body(server, None, true),
        ),
    ];
    let mut evidence = Evidence {
        events,
        expected_rules: vec![
            ExpectedFact {
                rule: ExpectedRule::Prepare {
                    client_cast_id: client,
                    server_cast_id: Some(server),
                },
                matched: false,
                event_sequence: None,
                detail: String::new(),
            },
            ExpectedFact {
                rule: ExpectedRule::Start {
                    metadata: None,
                    client_cast_id: Some(client),
                    server_cast_id: None,
                    spell_id: 133,
                    target: TargetExpectation::Empty,
                    cast_flags: Some(2),
                    cast_flags_ex: Some(0),
                },
                matched: false,
                event_sequence: None,
                detail: String::new(),
            },
            ExpectedFact {
                rule: ExpectedRule::Go {
                    metadata: None,
                    client_cast_id: Some(client),
                    server_cast_id: None,
                    spell_id: 133,
                    target: TargetExpectation::Empty,
                    cast_flags: Some(2),
                    cast_flags_ex: Some(0),
                    hit_targets: Some(0),
                    miss_targets: Some(0),
                    full_combat_log: Some(false),
                },
                matched: false,
                event_sequence: None,
                detail: String::new(),
            },
        ],
        ..Evidence::default()
    };
    let identities = prepare_identity_map(&evidence.events);
    for event in &mut evidence.events {
        if event.client_cast_id.is_none() {
            event.client_cast_id = identities
                .get(&event.server_cast_id.unwrap_or(Guid { low: 0, high: 0 }))
                .copied();
        }
    }
    evaluate(&mut evidence);
    assert!(evidence.passed);
    assert_eq!(
        evidence
            .expected_rules
            .iter()
            .map(|rule| rule.event_sequence)
            .collect::<Vec<_>>(),
        vec![Some(0), Some(1), Some(2)]
    );
}

#[test]
fn observer_without_prepare_requires_explicit_server_cast_id() {
    let server = guid(42);
    let event = decode_event(
        0,
        Connection::Instance,
        SMSG_SPELL_START,
        &spell_cast_body(server, None, false),
    );
    let mut evidence = Evidence {
        events: vec![event],
        expected_rules: vec![ExpectedFact {
            rule: ExpectedRule::Start {
                metadata: None,
                client_cast_id: None,
                server_cast_id: Some(server),
                spell_id: 133,
                target: TargetExpectation::Empty,
                cast_flags: Some(2),
                cast_flags_ex: Some(0),
            },
            matched: false,
            event_sequence: None,
            detail: String::new(),
        }],
        ..Evidence::default()
    };
    evaluate(&mut evidence);
    assert!(evidence.passed);
    assert_eq!(evidence.expected_rules[0].event_sequence, Some(0));

    evidence.expected_rules[0].rule = ExpectedRule::Start {
        metadata: None,
        client_cast_id: Some(guid(41)),
        server_cast_id: None,
        spell_id: 133,
        target: TargetExpectation::Empty,
        cast_flags: Some(2),
        cast_flags_ex: Some(0),
    };
    evaluate(&mut evidence);
    assert!(!evidence.passed);
    assert!(evidence.expected_rules[0].event_sequence.is_none());
}

#[test]
fn failure_packets_keep_distinct_reason_widths_and_identity() {
    let caster = guid(9);
    let cast = guid(41);
    let mut failure = caster.packed();
    failure.extend(cast.packed());
    failure.extend(133i32.to_le_bytes());
    failure.extend(77u32.to_le_bytes());
    failure.extend(0x1234u16.to_le_bytes());
    let failure_event = decode_event(0, Connection::Instance, SMSG_SPELL_FAILURE, &failure);
    assert_eq!(failure_event.reason, Some(0x1234));
    let mut other = caster.packed();
    other.extend(cast.packed());
    other.extend(133u32.to_le_bytes());
    other.extend(77u32.to_le_bytes());
    other.push(0x56);
    let other_event = decode_event(1, Connection::Instance, SMSG_SPELL_FAILED_OTHER, &other);
    assert_eq!(other_event.reason, Some(0x56));
    assert_eq!(other_event.cast_id, Some(cast));
    assert_eq!(
        other_event.client_cast_id, None,
        "a failure alone cannot reveal the client ID"
    );
}

fn prepare_body(client: Guid, server: Guid) -> Vec<u8> {
    let mut bytes = client.packed();
    bytes.extend(server.packed());
    bytes
}

fn spell_failure_body(cast: Guid, reason: u16) -> Vec<u8> {
    let mut bytes = guid(9).packed();
    bytes.extend(cast.packed());
    bytes.extend(133i32.to_le_bytes());
    bytes.extend(77u32.to_le_bytes());
    bytes.extend(reason.to_le_bytes());
    bytes
}

fn start_rule(server: Guid) -> ExpectedRule {
    ExpectedRule::Start {
        metadata: None,
        client_cast_id: None,
        server_cast_id: Some(server),
        spell_id: 133,
        target: TargetExpectation::Empty,
        cast_flags: Some(2),
        cast_flags_ex: Some(0),
    }
}

fn plan_rejected(json: &str) -> bool {
    match serde_json::from_str::<Plan>(json) {
        Ok(plan) => plan::validate_plan(&plan).is_err(),
        Err(_) => true,
    }
}

/// Positive shape for every scripted action and every expectation variant, then
/// the bounded/identity rejections the evaluator depends on.
#[test]
fn plan_validation_accepts_the_action_matrix_and_rejects_malformed_plans() {
    let accepted: Plan = serde_json::from_str(
        r#"{
        "timeout_ms": 3000,
        "actions": [
            {"action":"cast","spell_id":133,"cast_id":{"low":41,"high":0}},
            {"action":"wait","milliseconds":400},
            {"action":"cast","spell_id":133,"cast_id":{"low":42,"high":0},"target":{"low":22,"high":0}},
            {"action":"cancel","phase":"active","cast_id":{"low":42,"high":0},"spell_id":133},
            {"action":"cancel","phase":"pending","cast_id":{"low":41,"high":0},"spell_id":133}
        ],
        "expect": [
            {"event":"prepare","client_cast_id":{"low":41,"high":0},"server_cast_id":{"low":90,"high":0}},
            {"event":"start","client_cast_id":{"low":41,"high":0},"spell_id":133,"target":"empty"},
            {"event":"go","server_cast_id":{"low":90,"high":0},"spell_id":133,"target":{"unit":{"low":22,"high":0}}},
            {"event":"cast_failed","client_cast_id":{"low":42,"high":0},"spell_id":133,"reason":63},
            {"event":"spell_failure","server_cast_id":{"low":90,"high":0},"spell_id":133,"reason":40},
            {"event":"spell_failed_other","server_cast_id":{"low":90,"high":0},"spell_id":133,"reason":8}
        ]
    }"#,
    )
    .expect("the documented action matrix must parse");
    plan::validate_plan(&accepted).expect("the documented action matrix must validate");
    assert_eq!(accepted.actions.len(), 5);
    assert_eq!(accepted.expect.len(), 6);
    assert!(!accepted.observe_only);
    assert!(matches!(
        accepted.actions[3],
        Action::Cancel {
            phase: CancelPhase::Active,
            ..
        }
    ));
    assert!(matches!(
        accepted.actions[4],
        Action::Cancel {
            phase: CancelPhase::Pending,
            ..
        }
    ));
    let defaulted: Plan = serde_json::from_str(
        r#"{"actions":[],"expect":[{"event":"prepare","client_cast_id":{"low":1,"high":0}}]}"#,
    )
    .unwrap();
    plan::validate_plan(&defaulted).unwrap();
    assert_eq!(defaulted.timeout_ms, DEFAULT_PLAN_TIMEOUT_MS);

    let expect_one = r#""expect":[{"event":"prepare","client_cast_id":{"low":1,"high":0}}]"#;
    for (label, json) in [
        (
            "zero timeout",
            format!(r#"{{"timeout_ms":0,"actions":[],{expect_one}}}"#),
        ),
        (
            "timeout above the bounded maximum",
            format!(
                r#"{{"timeout_ms":{},"actions":[],{expect_one}}}"#,
                MAX_PLAN_TIMEOUT_MS + 1
            ),
        ),
        (
            "empty cast id",
            format!(
                r#"{{"actions":[{{"action":"cast","spell_id":133,"cast_id":{{"low":0,"high":0}}}}],{expect_one}}}"#
            ),
        ),
        (
            "non-positive cast spell id",
            format!(
                r#"{{"actions":[{{"action":"cast","spell_id":0,"cast_id":{{"low":1,"high":0}}}}],{expect_one}}}"#
            ),
        ),
        (
            "explicit but empty unit target",
            format!(
                r#"{{"actions":[{{"action":"cast","spell_id":133,"cast_id":{{"low":1,"high":0}},"target":{{"low":0,"high":0}}}}],{expect_one}}}"#
            ),
        ),
        (
            "cancel without a spell id",
            format!(
                r#"{{"actions":[{{"action":"cancel","phase":"active","cast_id":{{"low":1,"high":0}},"spell_id":0}}],{expect_one}}}"#
            ),
        ),
        (
            "cancel without a cast id",
            format!(
                r#"{{"actions":[{{"action":"cancel","phase":"pending","cast_id":{{"low":0,"high":0}},"spell_id":133}}],{expect_one}}}"#
            ),
        ),
        (
            "wait above the bounded maximum",
            format!(
                r#"{{"actions":[{{"action":"wait","milliseconds":{}}}],{expect_one}}}"#,
                MAX_WAIT_MS + 1
            ),
        ),
        (
            "observe_only with actions",
            r#"{"observe_only":true,"actions":[{"action":"wait","milliseconds":10}],"expect":[]}"#
                .to_string(),
        ),
        (
            "observe_only with expectations",
            format!(r#"{{"observe_only":true,"actions":[],{expect_one}}}"#),
        ),
        (
            "prepare without a client cast id",
            r#"{"actions":[],"expect":[{"event":"prepare","client_cast_id":{"low":0,"high":0}}]}"#
                .to_string(),
        ),
        (
            "start without any identity",
            r#"{"actions":[],"expect":[{"event":"start","spell_id":133,"target":"empty"}]}"#
                .to_string(),
        ),
        (
            "go without a positive spell id",
            r#"{"actions":[],"expect":[{"event":"go","server_cast_id":{"low":1,"high":0},"spell_id":0,"target":"empty"}]}"#
                .to_string(),
        ),
        (
            "unknown plan field",
            format!(r#"{{"unexpected":1,"actions":[],{expect_one}}}"#),
        ),
        (
            "unknown action",
            format!(r#"{{"actions":[{{"action":"jump"}}],{expect_one}}}"#),
        ),
        (
            "unknown expect event",
            r#"{"actions":[],"expect":[{"event":"channel","spell_id":133}]}"#.to_string(),
        ),
    ] {
        assert!(plan_rejected(&json), "{label} must be rejected");
    }
}

/// `observe_only` collection is never acceptance, even if its facts happen to
/// satisfy expectations that the loader would have refused.
#[test]
fn observe_only_collection_never_reports_pass() {
    let server = guid(42);
    let events = vec![decode_event(
        0,
        Connection::Instance,
        SMSG_SPELL_START,
        &spell_cast_body(server, None, false),
    )];
    let mut accepted = Evidence {
        events: events.clone(),
        expected_rules: vec![expected(start_rule(server))],
        logout_confirmed: true,
        ..Evidence::default()
    };
    evaluate(&mut accepted);
    assert!(accepted.passed, "the caster contrast case must pass");

    let mut observed = Evidence {
        observe_only: true,
        events,
        expected_rules: vec![expected(start_rule(server))],
        logout_confirmed: true,
        ..Evidence::default()
    };
    evaluate(&mut observed);
    assert!(observed.expected_rules[0].matched);
    assert!(!observed.passed, "collection is never acceptance");

    let mut collected = Evidence {
        observe_only: true,
        logout_confirmed: true,
        ..Evidence::default()
    };
    evaluate(&mut collected);
    assert!(!collected.passed);
}

/// A logout on the wrong socket, with a payload, or absent altogether fails the
/// run even when every cast expectation matched.
#[test]
fn missing_or_misrouted_logout_complete_fails_the_run() {
    assert_eq!(
        logout_complete_route(Connection::Realm, &[]).unwrap(),
        "realm"
    );
    let instance = logout_complete_route(Connection::Instance, &[])
        .unwrap_err()
        .to_string();
    assert!(instance.contains("instance connection"), "{instance}");
    let payload = logout_complete_route(Connection::Realm, &[0])
        .unwrap_err()
        .to_string();
    assert!(payload.contains("empty body"), "{payload}");

    let server = guid(42);
    let base = Evidence {
        events: vec![decode_event(
            0,
            Connection::Instance,
            SMSG_SPELL_START,
            &spell_cast_body(server, None, false),
        )],
        expected_rules: vec![expected(start_rule(server))],
        ..Evidence::default()
    };
    let mut missing = base.clone();
    missing.logout_request_completed = true;
    missing.finish_completed = true;
    finalize_logout(&mut missing);
    assert!(missing.expected_rules[0].matched);
    assert!(!missing.passed);
    assert_eq!(
        missing.failure.as_deref(),
        Some("normal logout was not confirmed")
    );

    let mut confirmed = Evidence {
        finish_completed: true,
        logout_request_completed: true,
        logout_confirmed: true,
        logout_route: Some("realm".to_string()),
        ..base
    };
    finalize_logout(&mut confirmed);
    assert!(confirmed.passed);

    let mut not_written = confirmed.clone();
    not_written.logout_request_completed = false;
    finalize_finish(&mut not_written);
    assert!(!not_written.passed);
    assert!(
        not_written
            .failure
            .as_deref()
            .unwrap()
            .contains("logout request was not locally written")
    );
}

fn correlated(events: Vec<ObservedEvent>, expected_rules: Vec<ExpectedFact>) -> Evidence {
    let mut evidence = Evidence {
        events,
        expected_rules,
        logout_confirmed: true,
        ..Evidence::default()
    };
    // Reproduce the per-packet correlation `collect_until` performs at runtime.
    for index in 0..evidence.events.len() {
        let mut event = evidence.events[index].clone();
        correlate_event(&evidence, &mut event);
        evidence.events[index] = event;
    }
    evidence
}

/// The hardening rejects every plan-bound cast packet that no expectation
/// consumed, and only those: ambient traffic for another identity still passes.
#[test]
fn unexpected_plan_bound_cast_events_fail_while_unrelated_traffic_passes() {
    let client = guid(41);
    let server = guid(42);
    let rules = || {
        vec![
            expected(ExpectedRule::Prepare {
                client_cast_id: client,
                server_cast_id: Some(server),
            }),
            expected(ExpectedRule::Go {
                metadata: None,
                client_cast_id: Some(client),
                server_cast_id: None,
                spell_id: 133,
                target: TargetExpectation::Empty,
                cast_flags: Some(2),
                cast_flags_ex: Some(0),
                hit_targets: Some(0),
                miss_targets: Some(0),
                full_combat_log: Some(false),
            }),
        ]
    };
    let prepare = || {
        decode_event(
            0,
            Connection::Instance,
            SMSG_SPELL_PREPARE,
            &prepare_body(client, server),
        )
    };
    let go = |sequence| {
        decode_event(
            sequence,
            Connection::Instance,
            SMSG_SPELL_GO,
            &spell_cast_body(server, None, true),
        )
    };

    let mut clean = correlated(vec![prepare(), go(1)], rules());
    evaluate(&mut clean);
    assert!(clean.passed, "{:?}", clean.failure);

    let mut extra_start = correlated(
        vec![
            prepare(),
            go(1),
            decode_event(
                2,
                Connection::Instance,
                SMSG_SPELL_START,
                &spell_cast_body(server, None, false),
            ),
        ],
        rules(),
    );
    evaluate(&mut extra_start);
    assert!(!extra_start.passed);
    assert!(
        extra_start
            .failure
            .as_deref()
            .unwrap()
            .contains("unexpected plan-bound spell_start"),
        "{:?}",
        extra_start.failure
    );

    let mut duplicate_mapping = correlated(
        vec![
            prepare(),
            go(1),
            decode_event(
                2,
                Connection::Instance,
                SMSG_SPELL_PREPARE,
                &prepare_body(client, guid(43)),
            ),
        ],
        rules(),
    );
    evaluate(&mut duplicate_mapping);
    assert!(!duplicate_mapping.passed);
    assert!(
        duplicate_mapping
            .failure
            .as_deref()
            .unwrap()
            .contains("unexpected plan-bound spell_prepare"),
        "{:?}",
        duplicate_mapping.failure
    );

    // A cast for an identity this plan never bound is ambient traffic.
    let mut unrelated = correlated(
        vec![
            prepare(),
            decode_event(
                1,
                Connection::Instance,
                SMSG_SPELL_START,
                &spell_cast_body(guid(99), None, false),
            ),
            go(2),
        ],
        rules(),
    );
    evaluate(&mut unrelated);
    assert!(unrelated.passed, "{:?}", unrelated.failure);
}

/// Only the pre-preparation CastFailed echoes the client CastID, and only for an
/// ID this plan actually requested; no other failure may seed the mapping.
#[test]
fn cast_failed_identity_is_never_inferred_from_an_unplanned_id() {
    let evidence = Evidence {
        actions: vec![Action::Cast {
            spell_id: 133,
            cast_id: guid(41),
            target: None,
        }],
        ..Evidence::default()
    };
    let mut planned = decode_event(
        0,
        Connection::Instance,
        SMSG_CAST_FAILED,
        &failed_body(guid(41)),
    );
    correlate_event(&evidence, &mut planned);
    assert_eq!(planned.client_cast_id, Some(guid(41)));

    let mut unplanned = decode_event(
        1,
        Connection::Instance,
        SMSG_CAST_FAILED,
        &failed_body(guid(77)),
    );
    correlate_event(&evidence, &mut unplanned);
    assert_eq!(
        unplanned.client_cast_id, None,
        "an arbitrary failure cannot invent a client mapping"
    );

    let mut failure = decode_event(
        2,
        Connection::Instance,
        SMSG_SPELL_FAILURE,
        &spell_failure_body(guid(41), 40),
    );
    correlate_event(&evidence, &mut failure);
    assert_eq!(
        failure.client_cast_id, None,
        "SpellFailure carries the server CastID; only SpellPrepare binds it"
    );

    let prepared = Evidence {
        events: vec![decode_event(
            0,
            Connection::Instance,
            SMSG_SPELL_PREPARE,
            &prepare_body(guid(41), guid(42)),
        )],
        ..Evidence::default()
    };
    let mut bound = decode_event(
        1,
        Connection::Instance,
        SMSG_SPELL_FAILURE,
        &spell_failure_body(guid(42), 40),
    );
    correlate_event(&prepared, &mut bound);
    assert_eq!(bound.client_cast_id, Some(guid(41)));
}

/// Truncated, over-long and structurally wrong payloads are retained as decode
/// errors and can never be accepted.
#[test]
fn decode_failures_are_reported_and_block_acceptance() {
    let truncated = decode_event(
        0,
        Connection::Instance,
        SMSG_SPELL_PREPARE,
        &guid(41).packed(),
    );
    assert!(
        truncated.error.is_some(),
        "a Prepare without the server CastID cannot decode"
    );

    let mut trailing_body = failed_body(guid(41));
    trailing_body.push(0);
    let trailing = decode_event(1, Connection::Instance, SMSG_CAST_FAILED, &trailing_body);
    assert!(
        trailing
            .error
            .as_deref()
            .is_some_and(|error| error.contains("trailing")),
        "{:?}",
        trailing.error
    );

    let server = guid(42);
    let mut padded = spell_cast_body(server, None, false);
    let counts_start = padded.len() - 19;
    write_msb_bits(&mut padded[counts_start..counts_start + 10], 76, 4, 1);
    let padding = decode_event(2, Connection::Instance, SMSG_SPELL_START, &padded);
    assert!(
        padding
            .error
            .as_deref()
            .is_some_and(|error| error.contains("padding is nonzero")),
        "{:?}",
        padding.error
    );

    let mut lying_counts = spell_cast_body(server, None, false);
    let counts_start = lying_counts.len() - 19;
    write_msb_bits(&mut lying_counts[counts_start..counts_start + 10], 0, 16, 1);
    let short = decode_event(3, Connection::Instance, SMSG_SPELL_START, &lying_counts);
    assert!(short.error.is_some(), "a missing hit target cannot decode");

    let mut evidence = Evidence {
        events: vec![
            decode_event(
                0,
                Connection::Instance,
                SMSG_SPELL_START,
                &spell_cast_body(server, None, false),
            ),
            trailing,
        ],
        expected_rules: vec![expected(start_rule(server))],
        logout_confirmed: true,
        ..Evidence::default()
    };
    evaluate(&mut evidence);
    assert!(evidence.expected_rules[0].matched);
    assert!(!evidence.passed);
    assert!(
        evidence
            .failure
            .as_deref()
            .unwrap()
            .contains("cannot accept undecoded event"),
        "{:?}",
        evidence.failure
    );
}

/// Every optional SpellCastData section in C++ order, including the reflect
/// miss status, rune block, ammo fields and the full combat log tail.
#[test]
fn optional_cast_sections_decode_runes_ammo_miss_status_and_full_log() {
    let server = guid(42);
    let mut body = Vec::new();
    body.extend(guid(9).packed()); // CasterGUID
    body.extend(guid(9).packed()); // CasterUnit
    body.extend(server.packed()); // CastID
    body.extend(guid(7).packed()); // OriginalCastID
    body.extend(133i32.to_le_bytes()); // SpellID
    body.extend(77u32.to_le_bytes()); // Visual
    body.extend(0x2u32.to_le_bytes()); // CastFlags
    body.extend(0u32.to_le_bytes()); // CastFlagsEx
    body.extend(1500u32.to_le_bytes()); // CastTime
    body.extend(0u32.to_le_bytes()); // MissileTrajectory.TravelTime
    body.extend(0f32.to_le_bytes()); // MissileTrajectory.Pitch
    body.push(0); // DestLocSpellCastIndex
    body.extend(0u32.to_le_bytes()); // Immunities.School
    body.extend(0u32.to_le_bytes()); // Immunities.Value
    body.extend(0u32.to_le_bytes()); // Predict.Points
    body.push(0); // Predict.Type
    body.extend(Guid { low: 0, high: 0 }.packed()); // Predict.BeaconGUID
    let mut counts = [0u8; 10];
    write_msb_bits(&mut counts, 0, 16, 1); // HitTargets
    write_msb_bits(&mut counts, 16, 16, 1); // MissTargets
    write_msb_bits(&mut counts, 32, 16, 1); // MissStatus
    write_msb_bits(&mut counts, 48, 9, 1); // RemainingPower
    write_msb_bits(&mut counts, 57, 1, 1); // RemainingRunes present
    write_msb_bits(&mut counts, 74, 1, 1); // AmmoDisplayID present
    write_msb_bits(&mut counts, 75, 1, 1); // AmmoInventoryType present
    body.extend(counts);
    let mut header = [0u8; 5];
    write_msb_bits(&mut header, 0, 28, 0x2); // TARGET_FLAG_UNIT
    body.extend(header);
    body.extend(guid(22).packed()); // Target.Unit
    body.extend(Guid { low: 0, high: 0 }.packed()); // Target.Item
    body.extend(guid(22).packed()); // HitTargets[0]
    body.extend(guid(23).packed()); // MissTargets[0]
    body.push(11); // MissStatus[0].Reason == SPELL_MISS_REFLECT
    body.push(2); // MissStatus[0].ReflectStatus
    body.extend((-50i32).to_le_bytes()); // RemainingPower[0].Cost
    body.push(4); // RemainingPower[0].Type
    body.push(1); // RuneData.Start
    body.push(2); // RuneData.Count
    body.extend(2u32.to_le_bytes()); // RuneData.Cooldowns size
    body.extend([9u8, 8]);
    body.extend(1234i32.to_le_bytes()); // AmmoDisplayID
    body.extend(5i32.to_le_bytes()); // AmmoInventoryType
    body.push(1); // FullCombatLog bit
    body.extend(1000i64.to_le_bytes()); // Health
    body.extend(11i32.to_le_bytes()); // AttackPower
    body.extend(12i32.to_le_bytes()); // SpellPower
    body.extend(13i32.to_le_bytes()); // Armor
    let mut log_counts = [0u8; 2];
    write_msb_bits(&mut log_counts, 0, 9, 1);
    body.extend(log_counts);
    body.extend(0i32.to_le_bytes()); // PowerData[0].PowerType
    body.extend(90i32.to_le_bytes()); // PowerData[0].Amount
    body.extend(50i32.to_le_bytes()); // PowerData[0].Cost

    let event = decode_event(0, Connection::Instance, SMSG_SPELL_GO, &body);
    assert_eq!(event.error, None, "{:?}", event.error);
    assert_eq!(event.original_cast_id, Some(guid(7)));
    assert_eq!(event.cast_time_ms, Some(1500));
    assert_eq!(event.hit_targets, vec![guid(22)]);
    assert_eq!(event.miss_targets, vec![guid(23)]);
    assert_eq!(event.miss_statuses.len(), 1);
    assert_eq!(event.miss_statuses[0].reason, 11);
    assert_eq!(event.miss_statuses[0].reflect_status, Some(2));
    assert_eq!(
        event.remaining_power,
        vec![PowerFact {
            amount: -50,
            power_type: 4
        }]
    );
    let runes = event.remaining_runes.as_ref().expect("rune block");
    assert_eq!(
        (runes.start, runes.count, runes.cooldowns.as_slice()),
        (1, 2, &[9u8, 8][..])
    );
    assert_eq!(event.ammo_display_id, Some(1234));
    assert_eq!(event.ammo_inventory_type, Some(5));
    assert_eq!(event.full_combat_log, Some(true));
    assert_eq!(event.full_log_power_count, Some(1));
    assert_eq!(
        event
            .target
            .as_ref()
            .map(|target| (target.flags, target.unit)),
        Some((0x2, guid(22)))
    );

    // The same bytes must not satisfy an "empty target" or hit/miss expectation.
    let mut evidence = Evidence {
        events: vec![event],
        expected_rules: vec![expected(ExpectedRule::Go {
            metadata: None,
            client_cast_id: None,
            server_cast_id: Some(server),
            spell_id: 133,
            target: TargetExpectation::Empty,
            cast_flags: Some(2),
            cast_flags_ex: Some(0),
            hit_targets: Some(1),
            miss_targets: Some(1),
            full_combat_log: Some(true),
        })],
        logout_confirmed: true,
        ..Evidence::default()
    };
    evaluate(&mut evidence);
    assert!(!evidence.passed, "a unit target cannot match `empty`");
}

/// The bot exit code and final-state report never treat an unproven or
/// observation-only cast evidence block as a successful run.
#[test]
fn cast_lifecycle_run_result_requires_a_proven_pass() {
    let logged_in = BotRunResult {
        world_auth: true,
        enum_characters: true,
        player_login_verified: true,
        ..BotRunResult::default()
    };
    let mut result = BotRunResult {
        cast_lifecycle: Some(Evidence::default()),
        ..logged_in
    };
    assert!(!result.success(false, false, false));

    // An observer capture whose facts satisfy every rule still evaluates to
    // `passed: false`, so the run can never be reported as successful.
    let server = guid(42);
    let mut observed = Evidence {
        observe_only: true,
        events: vec![decode_event(
            0,
            Connection::Instance,
            SMSG_SPELL_START,
            &spell_cast_body(server, None, false),
        )],
        expected_rules: vec![expected(start_rule(server))],
        logout_confirmed: true,
        ..Evidence::default()
    };
    evaluate(&mut observed);
    assert!(observed.expected_rules[0].matched);
    result.cast_lifecycle = Some(observed);
    assert!(
        !result.success(false, false, false),
        "an observer capture is never a successful run"
    );

    result.cast_lifecycle = Some(Evidence {
        passed: true,
        logout_confirmed: true,
        ..Evidence::default()
    });
    assert!(result.success(false, false, false));

    result.player_login_verified = false;
    assert!(
        !result.success(false, false, false),
        "cast acceptance still requires a verified login"
    );
}

fn exact_packet_fact(connection: Connection, opcode: u16, body: &[u8]) -> ExpectedFact {
    expected(ExpectedRule::Packet {
        connection,
        opcode,
        body_hex: hex::encode(body),
    })
}

#[test]
fn death_action_builders_preserve_target_wire_bytes() {
    assert_eq!(
        build_action_packet(&Action::ReleaseSpirit {
            check_instance: true
        }),
        (CMSG_RELEASE_SPIRIT, vec![0x80])
    );
    assert_eq!(
        build_action_packet(&Action::ReleaseSpirit {
            check_instance: false
        }),
        (CMSG_RELEASE_SPIRIT, vec![0x00])
    );

    let nonempty = guid(0x0102);
    let packed_nonempty = vec![0x03, 0x00, 0x02, 0x01];
    assert_eq!(
        build_action_packet(&Action::ReclaimCorpse { corpse: nonempty }),
        (CMSG_RECLAIM_CORPSE, packed_nonempty.clone())
    );
    assert_eq!(
        build_action_packet(&Action::ReclaimCorpse {
            corpse: Guid { low: 0, high: 0 }
        }),
        (CMSG_RECLAIM_CORPSE, vec![0, 0])
    );
    assert_eq!(
        build_action_packet(&Action::SpiritHealerActivate {
            healer: Guid { low: 0, high: 0 }
        }),
        (CMSG_SPIRIT_HEALER_ACTIVATE, vec![0, 0])
    );
    assert_eq!(
        build_action_packet(&Action::ResurrectResponse {
            offerer: nonempty,
            response: 0x0102_0304,
        }),
        (
            CMSG_RESURRECT_RESPONSE,
            vec![0x03, 0x00, 0x02, 0x01, 0x04, 0x03, 0x02, 0x01]
        )
    );
    assert_eq!(
        build_action_packet(&Action::ResurrectResponse {
            offerer: Guid { low: 0, high: 0 },
            response: 2,
        }),
        (CMSG_RESURRECT_RESPONSE, vec![0, 0, 2, 0, 0, 0])
    );
    assert_eq!(
        build_action_packet(&Action::SelfRes { spell_id: 21_169 }),
        (CMSG_SELF_RES, vec![0xB1, 0x52, 0, 0])
    );
    assert_eq!(
        build_action_packet(&Action::AttackSwing { target: nonempty }),
        (CMSG_ATTACK_SWING, packed_nonempty)
    );
}

#[test]
fn death_plan_keeps_server_negative_inputs_and_validates_only_wire_fields() {
    let plan: Plan = serde_json::from_str(
        r#"{
            "actions":[
                {"action":"release_spirit","check_instance":true},
                {"action":"reclaim_corpse","corpse":{"low":0,"high":0}},
                {"action":"spirit_healer_activate","healer":{"low":0,"high":0}},
                {"action":"resurrect_response","offerer":{"low":0,"high":0},"response":4294967295},
                {"action":"self_res","spell_id":21169},
                {"action":"attack_swing","target":{"low":0,"high":0}}
            ],
            "expect":[{"event":"packet","connection":"realm","opcode":9860,"body_hex":""}]
        }"#,
    )
    .unwrap();
    plan::validate_plan(&plan).unwrap();
    assert_eq!(plan.finish_policy, FinishPolicy::Logout);

    for bad_spell_id in [0, -1] {
        let json = format!(
            r#"{{"actions":[{{"action":"self_res","spell_id":{bad_spell_id}}}],"expect":[{{"event":"packet","connection":"realm","opcode":9860,"body_hex":""}}]}}"#
        );
        assert!(plan_rejected(&json), "self_res spell id {bad_spell_id}");
    }
    let explicit_disconnect: Plan = serde_json::from_str(
        r#"{"finish_policy":"disconnect","actions":[],"expect":[{"event":"packet","connection":"realm","opcode":9860,"body_hex":""}]}"#,
    )
    .unwrap();
    assert_eq!(explicit_disconnect.finish_policy, FinishPolicy::Disconnect);
}

#[test]
fn exact_packet_plan_hex_is_bounded_and_auth_control_opcodes_are_forbidden() {
    let valid = r#"{"actions":[],"expect":[{"event":"packet","connection":"realm","opcode":9860,"body_hex":""}]}"#;
    let parsed: Plan = serde_json::from_str(valid).unwrap();
    plan::validate_plan(&parsed).unwrap();

    for (label, body_hex, opcode) in [
        ("truncated byte", "abc", 9939),
        ("non-hex body", "zz", 9939),
        ("auth challenge", "", 0x3048),
        ("encrypted mode", "", 0x3049),
        ("connect-to key", "", 0x304D),
        ("unsupported gameplay opcode", "", 0xFFFF),
    ] {
        let json = format!(
            r#"{{"actions":[],"expect":[{{"event":"packet","connection":"realm","opcode":{opcode},"body_hex":"{body_hex}"}}]}}"#
        );
        assert!(
            plan_rejected(&json),
            "{label} must be rejected before login"
        );
    }

    let oversized_hex = "00".repeat(64 * 1024 + 1);
    let oversized_plan = format!(
        r#"{{"actions":[],"expect":[{{"event":"packet","connection":"instance","opcode":9939,"body_hex":"{oversized_hex}"}}]}}"#
    );
    assert!(
        plan_rejected(&oversized_plan),
        "oversized packet bodies are bounded"
    );
}

#[test]
fn exact_packet_expectations_match_per_route_without_cross_route_ordering() {
    let realm_body = [0x80, 0x01];
    let instance_body = [0x02, 0x03];
    let mut evidence = Evidence {
        events: vec![
            decode_event(
                0,
                Connection::Realm,
                RAW_SMSG_DEATH_RELEASE_LOC,
                &realm_body,
            ),
            decode_event(
                1,
                Connection::Instance,
                RAW_SMSG_AURA_UPDATE,
                &instance_body,
            ),
        ],
        expected_rules: vec![
            exact_packet_fact(Connection::Instance, RAW_SMSG_AURA_UPDATE, &instance_body),
            exact_packet_fact(Connection::Realm, RAW_SMSG_DEATH_RELEASE_LOC, &realm_body),
        ],
        ..Evidence::default()
    };
    evaluate(&mut evidence);
    assert!(
        evidence.passed,
        "opposite cross-route arrival is not a gate"
    );
    assert_eq!(
        evidence
            .expected_rules
            .iter()
            .map(|fact| fact.event_sequence)
            .collect::<Vec<_>>(),
        vec![Some(1), Some(0)]
    );
    assert_eq!(evidence.events[0].body_hex, "8001");
}

#[test]
fn exact_packet_expectations_reject_wrong_route_duplicates_and_route_order() {
    let body = [0x10, 0x20];
    let opcode = RAW_SMSG_DEATH_RELEASE_LOC;
    let mut wrong_route = Evidence {
        events: vec![decode_event(0, Connection::Instance, opcode, &body)],
        expected_rules: vec![exact_packet_fact(Connection::Realm, opcode, &body)],
        ..Evidence::default()
    };
    evaluate(&mut wrong_route);
    assert!(!wrong_route.passed);
    assert!(!wrong_route.expected_rules[0].matched);
    assert!(
        wrong_route
            .failure
            .as_deref()
            .unwrap()
            .contains("wrong-route")
    );

    let mut duplicate = Evidence {
        events: vec![
            decode_event(0, Connection::Realm, opcode, &body),
            decode_event(1, Connection::Realm, opcode, &body),
        ],
        expected_rules: vec![exact_packet_fact(Connection::Realm, opcode, &body)],
        ..Evidence::default()
    };
    evaluate(&mut duplicate);
    assert!(!duplicate.passed);
    assert!(duplicate.failure.as_deref().unwrap().contains("duplicate"));

    let first = [0x01];
    let second = [0x02];
    let mut out_of_order = Evidence {
        events: vec![
            decode_event(0, Connection::Realm, opcode, &second),
            decode_event(1, Connection::Realm, opcode, &first),
        ],
        expected_rules: vec![
            exact_packet_fact(Connection::Realm, opcode, &first),
            exact_packet_fact(Connection::Realm, opcode, &second),
        ],
        ..Evidence::default()
    };
    evaluate(&mut out_of_order);
    assert!(out_of_order.expected_rules[0].matched);
    assert!(!out_of_order.expected_rules[1].matched);
    assert!(!out_of_order.passed);
}

#[test]
fn trace_keeps_write_attempt_failure_and_redacts_control_bodies() {
    let mut evidence = Evidence::default();
    record_action_boundary(
        &mut evidence,
        Some(0),
        "release_spirit",
        TraceKind::ActionStarted,
        None,
    );
    let sent = record_send_attempt(
        &mut evidence,
        Connection::Instance,
        Some(0),
        "release_spirit",
        CMSG_RELEASE_SPIRIT,
        &[0x80],
    );
    record_write_failed(&mut evidence, sent, "connection reset during local write");
    record_action_boundary(
        &mut evidence,
        Some(0),
        "release_spirit",
        TraceKind::ActionFailed,
        Some("connection reset during local write".to_string()),
    );
    evidence.failure = Some("action failed: connection reset during local write".to_string());

    let packet = &evidence.sent_packets[0];
    assert!(packet.write_attempted);
    assert!(!packet.write_completed);
    assert_eq!(
        packet.write_error.as_deref(),
        Some("connection reset during local write")
    );
    assert_eq!(packet.body_hex, "80");
    assert_eq!(
        evidence
            .trace
            .iter()
            .map(|entry| entry.kind.clone())
            .collect::<Vec<_>>(),
        vec![
            TraceKind::ActionStarted,
            TraceKind::SendAttempted,
            TraceKind::LocalWriteFailed,
            TraceKind::ActionFailed,
        ]
    );
    assert_eq!(packet.connection_sequence, 0);
    assert_eq!(packet.observation_sequence, 1);
    let failure_report = serde_json::to_string(&evidence).unwrap();
    assert!(failure_report.contains("\"write_attempted\":true"));
    assert!(failure_report.contains("\"write_completed\":false"));
    assert!(failure_report.contains("\"body_hex\":\"80\""));

    let completed = record_send_attempt(
        &mut evidence,
        Connection::Realm,
        Some(1),
        "resurrect_response",
        CMSG_RESURRECT_RESPONSE,
        &[0, 0, 0, 0, 0, 0],
    );
    record_write_completed(&mut evidence, completed);
    assert!(evidence.sent_packets[completed].write_attempted);
    assert!(evidence.sent_packets[completed].write_completed);
    assert_eq!(evidence.sent_packets[completed].write_error, None);
    assert_eq!(evidence.trace[4].kind, TraceKind::SendAttempted);
    assert_eq!(evidence.trace[5].kind, TraceKind::LocalWriteCompleted);

    let mut observed = Evidence::default();
    assert_eq!(
        record_observed_packet(
            &mut observed,
            Connection::Realm,
            RAW_SMSG_DEATH_RELEASE_LOC,
            &[0xA1],
        ),
        Some(0)
    );
    assert_eq!(
        record_observed_packet(
            &mut observed,
            Connection::Realm,
            RAW_SMSG_CORPSE_RECLAIM_DELAY,
            &[0xB2],
        ),
        Some(1)
    );
    assert_eq!(
        record_observed_packet(
            &mut observed,
            Connection::Instance,
            RAW_SMSG_AURA_UPDATE,
            &[0xC3],
        ),
        Some(2)
    );
    assert_eq!(observed.events[0].body_hex, "a1");
    assert_eq!(observed.events[0].connection, Connection::Realm);
    assert_eq!(observed.events[0].connection_sequence, 0);
    assert_eq!(observed.events[0].observation_sequence, 0);
    assert_eq!(observed.events[1].connection_sequence, 1);
    assert_eq!(observed.events[1].observation_sequence, 1);
    assert_eq!(observed.events[2].connection, Connection::Instance);
    assert_eq!(observed.events[2].connection_sequence, 0);
    assert_eq!(observed.events[2].observation_sequence, 2);
    assert_eq!(
        observed.packet_payload_encoding,
        "logical_decrypted_uncompressed"
    );
    assert_eq!(observed.merged_observation_order, "client_poll_arrival");
    assert!(!observed.cross_connection_order_is_server_enqueue);

    let secret_payload = [0xDE, 0xAD, 0xBE, 0xEF];
    let mut privacy = Evidence::default();
    for (index, opcode) in [
        REDACTED_SMSG_AUTH_CHALLENGE,
        REDACTED_SMSG_AUTH_FAILED,
        REDACTED_SMSG_AUTH_RESPONSE,
        REDACTED_SMSG_ENTER_ENCRYPTED_MODE,
        REDACTED_SMSG_CONNECT_TO,
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(
            record_observed_packet(&mut privacy, Connection::Realm, opcode, &secret_payload,),
            None
        );
        assert_eq!(privacy.trace[index].sequence, index);
        assert_eq!(privacy.trace[index].connection_sequence, Some(index));
    }
    assert!(privacy.events.is_empty());
    assert_eq!(privacy.trace.len(), 5);
    assert_eq!(privacy.trace[0].kind, TraceKind::PacketRedacted);
    assert!(privacy.trace[0].body_redacted);
    assert_eq!(privacy.trace[0].connection, Some(Connection::Realm));
    assert_eq!(privacy.trace[0].connection_sequence, Some(0));
    assert!(privacy.trace.iter().all(|entry| entry.opcode.is_some()));
    let report_json = serde_json::to_string(&privacy).unwrap();
    assert!(!report_json.contains("deadbeef"));
    assert!(!report_json.contains(&format!("{:x}", Sha256::digest(secret_payload))));
}

#[test]
fn finish_policies_keep_default_logout_and_disconnect_claims_distinct() {
    assert_eq!(FinishPolicy::default(), FinishPolicy::Logout);
    let body = [0x44];
    let mut disconnected = Evidence {
        finish_policy: FinishPolicy::Disconnect,
        local_disconnect_recorded: true,
        finish_completed: true,
        logout_confirmed: true,
        logout_route: Some("realm".to_string()),
        events: vec![decode_event(
            0,
            Connection::Realm,
            RAW_SMSG_DEATH_RELEASE_LOC,
            &body,
        )],
        expected_rules: vec![exact_packet_fact(
            Connection::Realm,
            RAW_SMSG_DEATH_RELEASE_LOC,
            &body,
        )],
        ..Evidence::default()
    };
    finalize_finish(&mut disconnected);
    assert!(disconnected.passed);
    assert!(disconnected.local_disconnect_recorded);
    assert!(!disconnected.logout_confirmed);
    assert_eq!(disconnected.logout_route, None);
    assert!(disconnected.finish_completed);
}
