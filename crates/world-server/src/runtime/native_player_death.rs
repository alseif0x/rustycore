//! The current combat future's required Session rail call. No Map guard
//! crosses delivery, and a timeout cannot acknowledge a running native body.

use super::map_session_pass::{
    MapSessionPassSummaryLikeCpp, PassReply, await_one_admitted_pass_like_cpp,
};
use std::{sync::Arc, time::Duration};
use wow_world::session::directory::PlayerRegistration;
use wow_world::session::directory::PlayerRegistry;
use wow_world::session::mailbox::{
    CommittedCreatureMeleeAdmission, MapPhaseAdmissionLikeCpp, NativePlayerDeathAdmission,
    RunCommittedCreatureMeleeRequest, RunNativePlayerDeathRequest, SessionPhasePassOutcomeLikeCpp,
    SessionPhasePermitLikeCpp, SessionPhaseRequestLikeCpp,
};
use wow_world::session::{
    CommittedMapCreatureMelee, MapPlayerCombatCall, NativePlayerDeathFailure,
    PendingMapPlayerDeath, SharedCanonicalMapManager,
};

pub(super) async fn run_map_player_combat_call(
    call: MapPlayerCombatCall,
    manager: &SharedCanonicalMapManager,
    registry: &PlayerRegistry,
    coordinator_id: u64,
    tick_epoch: u64,
    map_tick_epoch: u64,
    diff_ms: u32,
    ack_timeout: Duration,
) -> Result<(), NativePlayerDeathFailure> {
    match call {
        MapPlayerCombatCall::RuntimeEvents {
            events,
            producer_map_key,
            producer_map_incarnation,
        } => publish_required_combat_events_like_cpp(
            &events,
            producer_map_key,
            producer_map_incarnation,
            map_tick_epoch,
            None,
            manager,
            registry,
        )
        .map_err(NativePlayerDeathFailure::from),
        MapPlayerCombatCall::NativeDeath(pending) => finish_map_player_death(
            pending,
            manager,
            registry,
            coordinator_id,
            tick_epoch,
            map_tick_epoch,
            diff_ms,
            ack_timeout,
        )
        .await
        .map(|_| ()),
        MapPlayerCombatCall::CommittedCreatureMelee(pending) => {
            present_committed_creature_melee(
                pending,
                manager,
                registry,
                coordinator_id,
                tick_epoch,
                map_tick_epoch,
                diff_ms,
                ack_timeout,
            )
            .await
        }
    }
}

#[derive(Clone, Copy)]
struct FrozenCombatPacketRecipientLikeCpp {
    registration: PlayerRegistration,
    handle: wow_map::PlayerHandle,
    map_key: wow_map::MapKey,
    residence_revision: u64,
    visible_source: Option<wow_core::ObjectGuid>,
}

#[derive(Clone, Copy)]
enum CombatPacketConnectionLikeCpp {
    Instance,
    Realm,
}

fn required_combat_packet_connection_like_cpp(
    event: &wow_world_core::runtime_plan::RuntimeEvent,
) -> Result<CombatPacketConnectionLikeCpp, &'static str> {
    use wow_constants::ServerOpcodes;

    // TrinityCore Opcodes.cpp routes HealthUpdate on REALM; the combat logs,
    // absorb and attack prelude frames, and object values use INSTANCE.
    match wow_packet::WorldPacket::from_bytes(&event.packet_bytes).server_opcode() {
        Some(ServerOpcodes::HealthUpdate) => Ok(CombatPacketConnectionLikeCpp::Realm),
        Some(
            ServerOpcodes::AiReaction
            | ServerOpcodes::AttackStart
            | ServerOpcodes::AttackStop
            | ServerOpcodes::AttackerStateUpdate
            | ServerOpcodes::SpellAbsorbLog
            | ServerOpcodes::SpellMissLog
            | ServerOpcodes::SpellNonMeleeDamageLog
            | ServerOpcodes::UpdateObject,
        ) => Ok(CombatPacketConnectionLikeCpp::Instance),
        _ => Err("Required combat event has an unsupported server opcode"),
    }
}

/// Publish only the ordered melee callback events. Unlike ordinary runtime
/// visibility events, these bytes must enter their socket queues before the
/// combat future begins the next native body or presentation stage.
fn publish_required_combat_events_like_cpp(
    events: &[wow_world_core::runtime_plan::RuntimeEvent],
    producer_map_key: wow_map::MapKey,
    producer_map_incarnation: u64,
    map_tick_epoch: u64,
    excluded_player: Option<wow_core::ObjectGuid>,
    manager: &SharedCanonicalMapManager,
    registry: &PlayerRegistry,
) -> Result<(), &'static str> {
    use wow_map::MapTickCoordinationStateLikeCpp;
    use wow_world_core::runtime_plan::RecipientRule;

    for event in events {
        let connection = required_combat_packet_connection_like_cpp(event)?;
        let (registrations, map_broadcast, nearby_filter, player_source_broadcast) =
            match &event.recipients {
                RecipientRule::MapBroadcastVisible {
                    map_id,
                    instance_id,
                } => {
                    if u32::from(*map_id) != producer_map_key.map_id
                        || *instance_id != producer_map_key.instance_id
                    {
                        return Err("Required combat broadcast left its admitted Map");
                    }
                    let registrations = registry
                        .runtime_recipients()
                        .into_iter()
                        .filter(|recipient| {
                            recipient.map_id == *map_id
                                && recipient.instance_id == *instance_id
                                && Some(recipient.guid) != excluded_player
                        })
                        .map(|recipient| recipient.registration)
                        .collect::<Vec<_>>();
                    (registrations, true, None, event.source_guid.is_player())
                }
                RecipientRule::NearbyVisibleDurable {
                    source_guid,
                    map_id,
                    instance_id,
                    source_position,
                    range,
                    required_3d,
                    ..
                } => {
                    if *source_guid != event.source_guid {
                        return Err("Required combat prelude source changed");
                    }
                    if u32::from(*map_id) != producer_map_key.map_id
                        || *instance_id != producer_map_key.instance_id
                    {
                        return Err("Required combat prelude left its admitted Map");
                    }
                    let registrations = registry
                        .runtime_recipients()
                        .into_iter()
                        .filter(|recipient| {
                            recipient.map_id == *map_id
                                && recipient.instance_id == *instance_id
                                && Some(recipient.guid) != excluded_player
                        })
                        .map(|recipient| recipient.registration)
                        .collect::<Vec<_>>();
                    (
                        registrations,
                        true,
                        Some((*source_position, *range, *required_3d)),
                        false,
                    )
                }
                RecipientRule::ExplicitPlayer(guid) => {
                    if !guid.is_player() {
                        return Err("Required combat explicit recipient is not a Player");
                    }
                    // The in-scope ExplicitPlayer producers are split/share
                    // HealthUpdates sent directly to their caster, so they do
                    // not use the MapBroadcast HaveAtClient selection.
                    let registrations = if Some(*guid) == excluded_player {
                        Vec::new()
                    } else {
                        registry
                            .runtime_recipient(*guid)
                            .map(|recipient| vec![recipient.registration])
                            .unwrap_or_default()
                    };
                    (registrations, false, None, false)
                }
                RecipientRule::SelfOnly => {
                    // The admitted melee event producers use MapBroadcastVisible,
                    // NearbyVisibleDurable, and ExplicitPlayer. Fail closed if
                    // that contract changes.
                    return Err("Required combat publisher does not accept SelfOnly events");
                }
                RecipientRule::NearbyVisible { .. }
                | RecipientRule::NearbyVisibleDurableSpellCast { .. } => {
                    return Err("Required combat event used an unsupported recipient rule");
                }
            };

        let frozen_recipients = {
            let manager = manager
                .lock()
                .map_err(|_| "Required combat Map lock is unavailable")?;
            if manager.tick_coordination_like_cpp()
                != MapTickCoordinationStateLikeCpp::AwaitingSessions(map_tick_epoch)
                || manager.map_incarnation_like_cpp(producer_map_key)
                    != Some(producer_map_incarnation)
            {
                return Err("Required combat producer Map admission changed before publication");
            }
            let broadcast_map = if map_broadcast {
                Some(
                    manager
                        .find_map(producer_map_key.map_id, producer_map_key.instance_id)
                        .ok_or("Required combat broadcast Map disappeared before publication")?
                        .map(),
                )
            } else {
                None
            };
            let player_source_observers = if player_source_broadcast {
                Some(
                    broadcast_map
                        .ok_or("Required combat source Map disappeared before publication")?
                        .selected_aura_update_observers(event.source_guid)
                        .into_iter()
                        .filter(|guid| *guid != event.source_guid)
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            };
            registrations
                .into_iter()
                .filter_map(|registration| {
                    let guid = registration.guid();
                    let (handle, residence_revision) =
                        manager.current_player_admission_like_cpp(guid)?;
                    let (map_key, current_revision) =
                        manager.player_active_residence_revision_like_cpp(handle)?;
                    if current_revision != residence_revision
                        || (map_broadcast && map_key != producer_map_key)
                    {
                        return None;
                    }
                    if player_source_broadcast
                        && guid != event.source_guid
                        && !player_source_observers
                            .as_ref()
                            .is_some_and(|observers| observers.contains(&guid))
                    {
                        return None;
                    }
                    if map_broadcast
                        && (event.source_guid.is_creature() || nearby_filter.is_some())
                        && !broadcast_map.is_some_and(|map| {
                            let Some(player) = map.get_typed_player(guid) else {
                                return false;
                            };
                            let player_world = player.unit().world();
                            let player_position = player_world.position();
                            if let Some((source_position, range, required_3d)) = nearby_filter {
                                let dx = player_position.x - source_position.x;
                                let dy = player_position.y - source_position.y;
                                let dz = player_position.z - source_position.z;
                                let distance_sq = if required_3d {
                                    dx * dx + dy * dy + dz * dz
                                } else {
                                    dx * dx + dy * dy
                                };
                                if distance_sq > range * range {
                                    return false;
                                }
                            }
                            if !event.source_guid.is_creature() {
                                return true;
                            }
                            map.with_creature_like_cpp(event.source_guid, |creature| {
                                wow_world::creature_message_to_set_target_allows_like_cpp(
                                    creature,
                                    true,
                                    producer_map_key.map_id,
                                    producer_map_key.instance_id,
                                    &player_position,
                                    player_world.phase_shift(),
                                    false,
                                )
                            })
                            .unwrap_or(false)
                        })
                    {
                        return None;
                    }
                    Some(FrozenCombatPacketRecipientLikeCpp {
                        registration,
                        handle,
                        map_key,
                        residence_revision,
                        visible_source: if player_source_broadcast && guid == event.source_guid {
                            None
                        } else {
                            map_broadcast.then_some(event.source_guid)
                        },
                    })
                })
                .collect::<Vec<_>>()
        };

        for recipient in frozen_recipients {
            let Some((registration, _)) = registry.selected_packet_recipient_at_residence(
                recipient.registration.guid(),
                recipient.handle,
                recipient.map_key,
                recipient.residence_revision,
                recipient.visible_source,
            ) else {
                continue;
            };
            if registration != recipient.registration {
                continue;
            }
            let result = match connection {
                CombatPacketConnectionLikeCpp::Instance => {
                    registry.try_send_current_packet(registration, event.packet_bytes.clone())
                }
                CombatPacketConnectionLikeCpp::Realm => {
                    registry.send_current_realm_packet(registration, event.packet_bytes.clone())
                }
            };
            if let Err(error) = result {
                // C++ WorldSession::SendPacket is best-effort (void): a vanished
                // recipient socket skips that packet and does not cancel Unit
                // combat. Map admission and unsupported event contracts remain
                // errors above; only this optional per-recipient queue write is
                // ignored here.
                tracing::debug!(
                    recipient = ?recipient.registration.guid(),
                    ?error,
                    "Skipping combat packet for unavailable recipient"
                );
            }
        }
    }
    Ok(())
}

async fn present_committed_creature_melee(
    pending: CommittedMapCreatureMelee,
    manager: &SharedCanonicalMapManager,
    registry: &PlayerRegistry,
    coordinator_id: u64,
    tick_epoch: u64,
    map_tick_epoch: u64,
    diff_ms: u32,
    ack_timeout: Duration,
) -> Result<(), NativePlayerDeathFailure> {
    let expected_handle = pending.handle;
    let expected_map_key = pending.map_key;
    let expected_map_incarnation = pending.map_incarnation;
    let expected_residence_revision = pending.residence_revision;
    publish_required_combat_events_like_cpp(
        &pending.attack_start_events,
        pending.map_key,
        pending.map_incarnation,
        map_tick_epoch,
        Some(pending.handle.guid()),
        manager,
        registry,
    )
    .map_err(NativePlayerDeathFailure::from)?;
    let attack_start_packets = pending
        .attack_start_events
        .into_iter()
        .map(|event| event.packet_bytes)
        .collect();
    let address = registry
        .session_phase_address_like_cpp(pending.handle.guid())
        .ok_or("Committed melee Session is unaddressable")?;
    let admission = CommittedCreatureMeleeAdmission {
        map_phase: MapPhaseAdmissionLikeCpp {
            coordinator_id,
            tick_epoch,
            phase: wow_handler::PacketUpdatePhase::Map,
            registration: address.registration(),
            handle: pending.handle,
            map_key: pending.map_key,
            map_incarnation: pending.map_incarnation,
            residence_revision: pending.residence_revision,
            diff_ms,
        },
        map_tick_epoch,
    };
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let (response_tx, response_rx) = flume::bounded(1);
    if address
        .try_send(SessionPhaseRequestLikeCpp::CommittedCreatureMelee(
            RunCommittedCreatureMeleeRequest {
                admission,
                command: pending.command,
                presentation_stage: pending.presentation_stage,
                attack_start: pending.attack_start,
                attack_start_packets,
                permit: Arc::clone(&permit),
                response_tx,
            },
        ))
        .is_err()
    {
        permit.revoke_before_start_like_cpp();
        return Err("Committed melee rail refused delivery".into());
    }
    let mut summary = MapSessionPassSummaryLikeCpp::default();
    let reply = await_one_admitted_pass_like_cpp(
        &mut summary,
        pending.map_key,
        tick_epoch,
        &permit,
        &response_rx,
        ack_timeout,
        |reply: &wow_world::session::mailbox::CommittedCreatureMeleeReply| {
            (reply.admission == admission).then_some(PassReply {
                tick_epoch,
                outcome: reply.outcome,
                dispatched: 0,
                stopped_at_ineligible_head: false,
                blocks_tick_tail: !reply.completed,
            })
        },
    )
    .await;
    if reply.is_some_and(|reply| {
        reply.admission == admission
            && reply.completed
            && reply.outcome == SessionPhasePassOutcomeLikeCpp::Ran
    }) && permit.state_like_cpp()
        == wow_world::session::mailbox::SessionPhasePermitStateLikeCpp::Completed
    {
        let manager = manager.lock().map_err(|_| NativePlayerDeathFailure {
            reason: "Committed melee completion Map lock is unavailable",
            unresolved_permit: None,
            native_handle: Some(expected_handle),
        })?;
        if manager.map_incarnation_like_cpp(expected_map_key) != Some(expected_map_incarnation)
            || manager.current_player_admission_like_cpp(expected_handle.guid())
                != Some((expected_handle, expected_residence_revision))
            || manager.player_active_residence_revision_like_cpp(expected_handle)
                != Some((expected_map_key, expected_residence_revision))
        {
            return Err(NativePlayerDeathFailure {
                reason: "Committed melee Player residence changed during callback",
                unresolved_permit: None,
                native_handle: Some(expected_handle),
            });
        }
        Ok(())
    } else {
        Err(NativePlayerDeathFailure {
            reason: "Required melee presentation did not complete",
            unresolved_permit: summary.unresolved_permits.into_iter().next(),
            native_handle: None,
        })
    }
}

pub(super) async fn finish_map_player_death(
    pending: PendingMapPlayerDeath,
    manager: &SharedCanonicalMapManager,
    registry: &PlayerRegistry,
    coordinator_id: u64,
    tick_epoch: u64,
    map_tick_epoch: u64,
    diff_ms: u32,
    ack_timeout: Duration,
) -> Result<u64, NativePlayerDeathFailure> {
    let fail = |reason, unresolved_permit| NativePlayerDeathFailure {
        reason,
        unresolved_permit,
        native_handle: Some(pending.handle),
    };
    let address = registry
        .session_phase_address_like_cpp(pending.handle.guid())
        .ok_or_else(|| fail("Native death Session is unaddressable", None))?;
    {
        let manager = manager
            .lock()
            .map_err(|_| fail("Native death Map lock is unavailable", None))?;
        if manager.tick_coordination_like_cpp()
            != wow_map::MapTickCoordinationStateLikeCpp::AwaitingSessions(map_tick_epoch)
            || manager.map_incarnation_like_cpp(pending.map_key) != Some(pending.map_incarnation)
            || manager.player_active_residence_revision_like_cpp(pending.handle)
                != Some((pending.map_key, pending.residence_revision))
        {
            return Err(fail("Native death producer admission changed", None));
        }
    }
    let admission = NativePlayerDeathAdmission {
        map_phase: MapPhaseAdmissionLikeCpp {
            coordinator_id,
            tick_epoch,
            phase: wow_handler::PacketUpdatePhase::Map,
            registration: address.registration(),
            handle: pending.handle,
            map_key: pending.map_key,
            map_incarnation: pending.map_incarnation,
            residence_revision: pending.residence_revision,
            diff_ms,
        },
        map_tick_epoch,
        operation: pending.operation,
        pvp_death: pending.pvp_death,
    };
    publish_required_combat_events_like_cpp(
        &pending.attack_start_events,
        pending.map_key,
        pending.map_incarnation,
        map_tick_epoch,
        Some(pending.handle.guid()),
        manager,
        registry,
    )
    .map_err(NativePlayerDeathFailure::from)?;
    let attack_start_packets = pending
        .attack_start_events
        .into_iter()
        .map(|event| event.packet_bytes)
        .collect();
    let permit = SessionPhasePermitLikeCpp::new_like_cpp();
    let (response_tx, response_rx) = flume::bounded(1);
    if address
        .try_send(SessionPhaseRequestLikeCpp::NativePlayerDeath(
            RunNativePlayerDeathRequest {
                admission,
                permit: Arc::clone(&permit),
                response_tx,
                creature_melee_prefix: pending.creature_melee_prefix,
                attack_start: pending.attack_start,
                attack_start_packets,
            },
        ))
        .is_err()
    {
        permit.revoke_before_start_like_cpp();
        return Err(fail("Native death rail refused delivery", None));
    }
    let mut summary = MapSessionPassSummaryLikeCpp::default();
    let reply = await_one_admitted_pass_like_cpp(
        &mut summary,
        pending.map_key,
        tick_epoch,
        &permit,
        &response_rx,
        ack_timeout,
        |reply: &wow_world::session::mailbox::NativePlayerDeathReply| {
            (reply.admission == admission).then_some(PassReply {
                tick_epoch,
                outcome: reply.outcome,
                dispatched: 0,
                stopped_at_ineligible_head: false,
                blocks_tick_tail: reply.health_after.is_none(),
            })
        },
    )
    .await;
    let Some(reply) = reply.filter(|reply| {
        reply.admission == admission
            && reply.outcome == SessionPhasePassOutcomeLikeCpp::Ran
            && permit.state_like_cpp()
                == wow_world::session::mailbox::SessionPhasePermitStateLikeCpp::Completed
    }) else {
        return Err(fail(
            "Required native death did not complete",
            summary.unresolved_permits.into_iter().next(),
        ));
    };
    let health = reply
        .health_after
        .ok_or_else(|| fail("Native death result is unavailable", None))?;
    let mut manager = manager
        .lock()
        .map_err(|_| fail("Native death Map lock is unavailable after callback", None))?;
    if manager.map_incarnation_like_cpp(pending.map_key) != Some(pending.map_incarnation)
        || manager.player_active_residence_revision_like_cpp(pending.handle)
            != Some((pending.map_key, pending.residence_revision))
    {
        return Err(fail(
            "Native death Player handle or Map residence changed during callback",
            None,
        ));
    }
    let acknowledged = manager.with_player_mut_like_cpp(pending.handle, |player| {
        player.retire_completed_native_death_operation(pending.operation)
    }) == Some(true);
    if !acknowledged {
        return Err(fail("Native death operation acknowledgement changed", None));
    }
    Ok(health)
}
