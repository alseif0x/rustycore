//! Creature-target resolution from the cast runner's retained packet evidence.

use super::*;
use anyhow::{bail, Context, Result};

#[derive(Debug)]
struct SelectedCreature {
    guid: Guid,
    distance: f32,
    source_event_sequence: usize,
    source_connection: Connection,
    source_connection_sequence: usize,
    source_event_opcode: u16,
    source_event_sha256: String,
}

/// Resolve only at the action boundary, after all earlier observations have
/// entered the same evidence timeline used by the report.
pub(super) fn resolve_action_targets(
    evidence: &mut Evidence,
    action_index: usize,
    action: &Action,
) -> Result<Action> {
    match action {
        Action::SpiritHealerActivate {
            healer: CreatureTarget::Observed(selector),
        } => {
            let selected = select_observed_creature(&evidence.events, selector)?;
            record_selection(evidence, action_index, action, *selector, &selected);
            Ok(Action::SpiritHealerActivate {
                healer: CreatureTarget::Guid(selected.guid),
            })
        }
        Action::AttackSwing {
            target: CreatureTarget::Observed(selector),
        } => {
            let selected = select_observed_creature(&evidence.events, selector)?;
            record_selection(evidence, action_index, action, *selector, &selected);
            Ok(Action::AttackSwing {
                target: CreatureTarget::Guid(selected.guid),
            })
        }
        _ => Ok(action.clone()),
    }
}

fn record_selection(
    evidence: &mut Evidence,
    action_index: usize,
    action: &Action,
    selector: ObservedCreatureSelector,
    selected: &SelectedCreature,
) {
    let name = action_name(action).to_string();
    evidence.target_selections.push(TargetSelectionFact {
        action_index,
        action: name.clone(),
        selector,
        guid: selected.guid,
        source_event_sequence: selected.source_event_sequence,
        source_connection: selected.source_connection,
        source_connection_sequence: selected.source_connection_sequence,
        source_event_opcode: selected.source_event_opcode,
        source_event_sha256: selected.source_event_sha256.clone(),
    });
    evidence.trace.push(TraceEntry {
        sequence: evidence.trace.len(),
        kind: TraceKind::TargetSelected,
        action_index: Some(action_index),
        action: Some(name),
        connection: Some(selected.source_connection),
        connection_sequence: Some(selected.source_connection_sequence),
        opcode: Some(selected.source_event_opcode),
        sent_packet_sequence: None,
        event_sequence: Some(selected.source_event_sequence),
        body_bytes: None,
        body_redacted: false,
        detail: Some(format!(
            "selected observed creature GUID low={} high={} at distance {:.3} from event {} sha256={}",
            selected.guid.low,
            selected.guid.high,
            selected.distance,
            selected.source_event_sequence,
            selected.source_event_sha256,
        )),
    });
}

fn select_observed_creature(
    events: &[ObservedEvent],
    selector: &ObservedCreatureSelector,
) -> Result<SelectedCreature> {
    selector
        .validate()
        .context("invalid observed-creature selector")?;
    let barrier = events
        .iter()
        .filter(|event| {
            (event.connection == Connection::Instance && event.opcode == SMSG_LOGIN_VERIFY_WORLD)
                || (event.connection == Connection::Realm && event.opcode == SMSG_NEW_WORLD)
        })
        .max_by_key(|event| event.sequence)
        .context("observed-creature selector requires a login or worldport map barrier")?;
    if let Some(error) = &barrier.error {
        bail!(
            "latest world map barrier {} is malformed: {error}",
            barrier.sequence
        );
    }
    let barrier_payload =
        hex::decode(&barrier.body_hex).context("decode the latest login/worldport map barrier")?;
    let map_bytes: [u8; 4] = barrier_payload
        .get(..4)
        .context("latest login/worldport map barrier is shorter than its map ID")?
        .try_into()
        .expect("four-byte map ID slice");
    let barrier_map = i32::from_le_bytes(map_bytes);
    if !(0..=0x1FFF).contains(&barrier_map) {
        bail!("latest world map barrier has unsupported map ID {barrier_map}");
    }
    let barrier_map = barrier_map as u16;
    if selector.map_id != barrier_map {
        bail!(
            "observed-creature selector map {} differs from current map {}",
            selector.map_id,
            barrier_map
        );
    }

    let mut best: Option<SelectedCreature> = None;
    for event in events.iter().filter(|event| {
        event.sequence > barrier.sequence
            && event.connection == Connection::Instance
            && event.opcode == RAW_SMSG_UPDATE_OBJECT
    }) {
        let payload = hex::decode(&event.body_hex)
            .with_context(|| format!("decode retained object update event {}", event.sequence))?;
        let candidates = find_creature_guids_near_position_in_update_object(
            &payload,
            barrier_map,
            selector.entry,
            selector.near_xyz[0],
            selector.near_xyz[1],
            selector.near_xyz[2],
            selector.radius,
            None,
        );
        for candidate in candidates {
            let dx = candidate.x - selector.near_xyz[0];
            let dy = candidate.y - selector.near_xyz[1];
            let dz = candidate.z - selector.near_xyz[2];
            let selected = SelectedCreature {
                guid: Guid {
                    low: candidate.low,
                    high: candidate.high,
                },
                distance: (dx * dx + dy * dy + dz * dz).sqrt(),
                source_event_sequence: event.sequence,
                source_connection: event.connection,
                source_connection_sequence: event.connection_sequence,
                source_event_opcode: event.opcode,
                source_event_sha256: event.body_sha256.clone(),
            };
            if best
                .as_ref()
                .is_none_or(|current| precedes(&selected, current))
            {
                best = Some(selected);
            }
        }
    }
    best.context("no matching creature was observed after the latest map barrier")
}

fn precedes(candidate: &SelectedCreature, current: &SelectedCreature) -> bool {
    match candidate.distance.total_cmp(&current.distance) {
        std::cmp::Ordering::Less => true,
        std::cmp::Ordering::Greater => false,
        std::cmp::Ordering::Equal => {
            (
                candidate.source_event_sequence,
                candidate.guid.low,
                candidate.guid.high,
            ) < (
                current.source_event_sequence,
                current.guid.low,
                current.guid.high,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    const MAP_ID: u16 = 530;
    const CREATURE_ENTRY: u32 = 15_274;

    fn selector(map_id: u16, near_xyz: [f32; 3], radius: f32) -> ObservedCreatureSelector {
        ObservedCreatureSelector {
            entry: CREATURE_ENTRY,
            map_id,
            near_xyz,
            radius,
        }
    }

    fn login_payload(map_id: u16) -> Vec<u8> {
        let mut payload = i32::from(map_id).to_le_bytes().to_vec();
        for value in [10.0f32, 20.0, 30.0, 0.0] {
            payload.extend(value.to_le_bytes());
        }
        payload.extend(0u32.to_le_bytes());
        payload
    }

    fn login_event(sequence: usize, map_id: u16) -> ObservedEvent {
        let payload = login_payload(map_id);
        decode_event(
            sequence,
            Connection::Instance,
            SMSG_LOGIN_VERIFY_WORLD,
            &payload,
        )
    }

    fn worldport_event(sequence: usize, map_id: u16) -> ObservedEvent {
        let mut payload = i32::from(map_id).to_le_bytes().to_vec();
        for value in [10.0f32, 20.0, 30.0, 0.0] {
            payload.extend(value.to_le_bytes());
        }
        payload.extend((-1i32).to_le_bytes());
        payload.extend((-1i32).to_le_bytes());
        payload.extend(2u32.to_le_bytes());
        payload.extend([0u8; 12]);
        decode_event(sequence, Connection::Realm, SMSG_NEW_WORLD, &payload)
    }

    fn object_update_event(sequence: usize, payload: &[u8]) -> ObservedEvent {
        decode_event(
            sequence,
            Connection::Instance,
            RAW_SMSG_UPDATE_OBJECT,
            payload,
        )
    }

    fn creature(map_id: u16, counter: u64, xyz: [f32; 3]) -> Vec<u8> {
        crate::tests::rested_xp_create_object_fixture(
            map_id,
            CREATURE_ENTRY,
            counter,
            xyz[0],
            xyz[1],
            xyz[2],
        )
    }

    fn valid_plan(actions: Vec<Action>, character_guid: Option<u64>) -> Plan {
        Plan {
            character_guid,
            observe_only: false,
            timeout_ms: 100,
            actions,
            expect: vec![ExpectedRule::Packet {
                connection: Connection::Instance,
                opcode: SMSG_SPELL_PREPARE,
                body_hex: String::new(),
            }],
            finish_policy: FinishPolicy::Disconnect,
        }
    }

    #[test]
    fn observed_targets_use_initial_and_later_visible_login_updates() {
        let point = [10.0, 20.0, 30.0];
        let initial = creature(MAP_ID, 41, point);
        let initial_selected = select_observed_creature(
            &[login_event(0, MAP_ID), object_update_event(1, &initial)],
            &selector(MAP_ID, point, 4.0),
        )
        .unwrap();
        assert_eq!(initial_selected.guid.low, 41);
        assert_eq!(initial_selected.source_event_sequence, 1);

        // The first login burst and a later ghost-visibility update remain in
        // one ordered evidence stream; the later, closer healer is selected.
        let login_update = creature(MAP_ID, 42, [14.0, 20.0, 30.0]);
        let later_ghost_update = creature(MAP_ID, 43, [10.5, 20.0, 30.0]);
        let events = [
            login_event(0, MAP_ID),
            object_update_event(1, &login_update),
            object_update_event(2, &later_ghost_update),
        ];
        let selected = select_observed_creature(&events, &selector(MAP_ID, point, 5.0)).unwrap();
        assert_eq!(selected.guid.low, 43);
        assert_eq!(selected.source_event_sequence, 2);
    }

    #[test]
    fn target_resolution_fails_without_observation_or_for_stale_maps() {
        let point = [10.0, 20.0, 30.0];
        assert!(
            select_observed_creature(&[login_event(0, MAP_ID)], &selector(MAP_ID, point, 4.0),)
                .unwrap_err()
                .to_string()
                .contains("no matching creature")
        );

        let update = creature(MAP_ID, 51, point);
        let before_worldport = [login_event(0, MAP_ID), object_update_event(1, &update)];
        assert!(
            select_observed_creature(&before_worldport, &selector(MAP_ID + 1, point, 4.0),)
                .unwrap_err()
                .to_string()
                .contains("differs from current map")
        );

        let destination_update = creature(MAP_ID + 1, 52, point);
        let after_map_change = [
            login_event(0, MAP_ID),
            object_update_event(1, &update),
            worldport_event(2, MAP_ID + 1),
            object_update_event(3, &destination_update),
        ];
        let selected =
            select_observed_creature(&after_map_change, &selector(MAP_ID + 1, point, 4.0)).unwrap();
        assert_eq!(selected.guid.low, 52);
        assert_eq!(selected.source_event_sequence, 3);
        assert!(
            select_observed_creature(&after_map_change, &selector(MAP_ID, point, 4.0),).is_err()
        );
    }

    #[test]
    fn equal_distance_selection_is_deterministic_and_keeps_event_provenance() {
        let point = [10.0, 20.0, 30.0];
        let mut update = creature(MAP_ID, 72, point);
        update.extend(creature(MAP_ID, 71, point));
        let events = vec![login_event(0, MAP_ID), object_update_event(1, &update)];
        let action = Action::AttackSwing {
            target: CreatureTarget::Observed(selector(MAP_ID, point, 4.0)),
        };
        let mut evidence = Evidence {
            events,
            ..Evidence::default()
        };

        let resolved = resolve_action_targets(&mut evidence, 3, &action).unwrap();
        assert!(matches!(
            resolved,
            Action::AttackSwing {
                target: CreatureTarget::Guid(Guid { low: 71, .. })
            }
        ));
        let fact = evidence
            .target_selections
            .first()
            .expect("selection report");
        assert_eq!(fact.action_index, 3);
        assert_eq!(fact.guid.low, 71);
        assert_eq!(fact.source_event_sequence, 1);
        assert_eq!(fact.source_connection, Connection::Instance);
        assert_eq!(fact.source_event_opcode, RAW_SMSG_UPDATE_OBJECT);
        assert_eq!(fact.source_event_sha256, evidence.events[1].body_sha256);
        let trace = evidence
            .trace
            .iter()
            .find(|entry| entry.kind == TraceKind::TargetSelected)
            .expect("selection trace");
        assert_eq!(trace.event_sequence, Some(fact.source_event_sequence));
        assert_eq!(trace.opcode, Some(RAW_SMSG_UPDATE_OBJECT));
        assert!(trace.detail.as_deref().unwrap().contains("low=71"));

        let json = serde_json::to_value(&evidence).unwrap();
        assert_eq!(json["target_selections"][0]["guid"]["low"], 71);
        assert_eq!(json["target_selections"][0]["source_event_sequence"], 1);
    }

    #[test]
    fn selector_and_character_override_validation_reject_malformed_inputs() {
        let valid = selector(MAP_ID, [10.0, 20.0, 30.0], 4.0);
        let mut plan = valid_plan(
            vec![Action::SpiritHealerActivate {
                healer: CreatureTarget::Observed(valid),
            }],
            Some(OBJECT_GUID_COUNTER_MASK),
        );
        assert!(plan::validate_plan(&plan).is_ok());

        for counter in [0, OBJECT_GUID_COUNTER_MASK + 1] {
            plan.character_guid = Some(counter);
            assert!(plan::validate_plan(&plan).is_err());
        }
        plan.character_guid = None;
        for malformed in [
            ObservedCreatureSelector { entry: 0, ..valid },
            ObservedCreatureSelector {
                radius: 0.0,
                ..valid
            },
            ObservedCreatureSelector {
                near_xyz: [f32::NAN, 20.0, 30.0],
                ..valid
            },
        ] {
            plan.actions[0] = Action::SpiritHealerActivate {
                healer: CreatureTarget::Observed(malformed),
            };
            assert!(plan::validate_plan(&plan).is_err());
        }

        let malformed_json: std::result::Result<Action, _> = serde_json::from_str(
            r#"{"action":"attack_swing","target":{"entry":15274,"map_id":530,"near_xyz":[10.0,20.0],"radius":4.0}}"#,
        );
        assert!(malformed_json.is_err());
        let ambiguous_json: std::result::Result<Action, _> = serde_json::from_str(
            r#"{"action":"attack_swing","target":{"low":1,"high":0,"entry":15274,"map_id":530,"near_xyz":[10.0,20.0,30.0],"radius":4.0}}"#,
        );
        assert!(ambiguous_json.is_err());

        for action_json in [
            r#"{"action":"spirit_healer_activate","healer":{"entry":15274,"map_id":530,"near_xyz":[10.0,20.0,30.0],"radius":4.0}}"#,
            r#"{"action":"attack_swing","target":{"entry":15274,"map_id":530,"near_xyz":[10.0,20.0,30.0],"radius":4.0}}"#,
        ] {
            let action: Action = serde_json::from_str(action_json).unwrap();
            assert!(matches!(
                action,
                Action::SpiritHealerActivate {
                    healer: CreatureTarget::Observed(_)
                } | Action::AttackSwing {
                    target: CreatureTarget::Observed(_)
                }
            ));
        }
    }

    async fn socket_pair() -> (tokio::net::TcpStream, tokio::net::TcpStream) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let client = tokio::net::TcpStream::connect(listener.local_addr().unwrap())
            .await
            .unwrap();
        let server = listener.accept().await.unwrap().0;
        (client, server)
    }

    #[tokio::test]
    async fn plan_execution_writes_only_a_resolved_observed_target() {
        let point = [10.0, 20.0, 30.0];
        let update = creature(MAP_ID, 81, point);
        let target_selector = selector(MAP_ID, point, 4.0);
        let mut options_plan = valid_plan(
            vec![Action::SpiritHealerActivate {
                healer: CreatureTarget::Observed(target_selector),
            }],
            None,
        );
        options_plan.timeout_ms = 100;
        options_plan.expect = vec![ExpectedRule::Packet {
            connection: Connection::Instance,
            opcode: RAW_SMSG_UPDATE_OBJECT,
            body_hex: hex::encode(&update),
        }];
        plan::validate_plan(&options_plan).unwrap();
        let mut controller = Controller::new(
            Options {
                source_path: "observed-target-plan-test".to_string(),
                plan: options_plan,
            },
            15,
            1,
        );
        controller
            .record_and_process_packet(
                Connection::Instance,
                SMSG_LOGIN_VERIFY_WORLD,
                &login_payload(MAP_ID),
            )
            .unwrap();
        controller
            .record_and_process_packet(Connection::Instance, RAW_SMSG_UPDATE_OBJECT, &update)
            .unwrap();

        let (mut client, mut server) = socket_pair().await;
        let mut crypt = WorldCrypt::new(&[0; 16]);
        let mut inflater = ServerPacketInflater::default();
        let mut realm = None;
        execute(
            &mut controller,
            &mut client,
            &mut crypt,
            &mut inflater,
            &mut realm,
        )
        .await
        .unwrap();

        let mut header = [0u8; 16];
        tokio::time::timeout(
            std::time::Duration::from_secs(1),
            server.read_exact(&mut header),
        )
        .await
        .unwrap()
        .unwrap();
        let size = u32::from_le_bytes(header[..4].try_into().unwrap()) as usize;
        let tag: [u8; 12] = header[4..].try_into().unwrap();
        let mut ciphertext = vec![0u8; size];
        server.read_exact(&mut ciphertext).await.unwrap();
        let plaintext = WorldCrypt::new(&[0; 16])
            .decrypt_client(&ciphertext, &tag, &[])
            .unwrap();
        assert_eq!(
            u16::from_le_bytes([plaintext[0], plaintext[1]]),
            CMSG_SPIRIT_HEALER_ACTIVATE
        );
        let (low, high) = create_creature_guid_raw(MAP_ID, CREATURE_ENTRY, 81);
        assert_eq!(&plaintext[2..], build_packed_guid(low, high).as_slice());
        assert!(controller.evidence.passed);
        assert_eq!(controller.evidence.sent_packets.len(), 1);
        assert!(controller.evidence.sent_packets[0].write_completed);
        assert_eq!(
            controller.evidence.target_selections[0].guid,
            Guid { low, high }
        );
        assert_eq!(
            controller.evidence.target_selections[0].source_event_sequence,
            1
        );

        let mut blocked_plan = valid_plan(
            vec![Action::SpiritHealerActivate {
                healer: CreatureTarget::Observed(selector(MAP_ID, point, 4.0)),
            }],
            None,
        );
        blocked_plan.timeout_ms = 20;
        let mut blocked = Controller::new(
            Options {
                source_path: "unresolved-target-plan-test".to_string(),
                plan: blocked_plan,
            },
            15,
            1,
        );
        let login_payload = login_payload(MAP_ID);
        blocked
            .record_and_process_packet(
                Connection::Instance,
                SMSG_LOGIN_VERIFY_WORLD,
                &login_payload,
            )
            .unwrap();
        let (mut blocked_client, mut _blocked_server) = socket_pair().await;
        let mut blocked_crypt = WorldCrypt::new(&[0; 16]);
        let mut blocked_inflater = ServerPacketInflater::default();
        let mut blocked_realm = None;
        execute(
            &mut blocked,
            &mut blocked_client,
            &mut blocked_crypt,
            &mut blocked_inflater,
            &mut blocked_realm,
        )
        .await
        .unwrap();
        assert!(blocked.evidence.failure.is_some());
        assert!(blocked
            .evidence
            .failure
            .as_deref()
            .unwrap()
            .contains("no matching creature"));
        assert!(blocked.evidence.sent_packets.is_empty());
        assert!(!blocked
            .evidence
            .trace
            .iter()
            .any(|entry| entry.kind == TraceKind::SendAttempted));
    }
}
