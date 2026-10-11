//! Runtime event routing and delivery for the canonical creature runtime.

use super::*;

// ── Runtime candidate routing + delivery ────────────────────────────────────
//
// These functions started as Slice 4A.1b infrastructure; since #1263 F6-8D3b
// they deliver the admitted canonical creature executor's deferred output
// (`canonical_creature_runtime.rs`) after the canonical guard is released.
// C++ anchors: `Object.cpp : WorldObject::SendMessageToSet` (~1746-1764),
// `GridNotifiersImpl.h : MessageDistDeliverer::Visit(PlayerMapType&)` (~43-46),
// `GridNotifiers.h : MessageDistDeliverer::SendPacket`.

/// Summary returned by [`deliver_runtime_plan_like_cpp`], testable without I/O.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RuntimeDeliverySummaryLikeCpp {
    /// Total `RuntimeEvent`s processed.
    pub events_seen: usize,
    /// Total candidate sessions evaluated across all events.
    pub candidates_seen: usize,
    /// Commands successfully enqueued (`try_send` succeeded).
    pub candidates_queued: usize,
    /// Candidates rejected because their map_id did not match the event.
    pub candidates_skipped_wrong_map: usize,
    /// Candidates rejected because their instance_id did not match the event.
    pub candidates_skipped_wrong_instance: usize,
    /// Candidates rejected because `is_in_world == false`.
    pub candidates_skipped_not_in_world: usize,
    /// Candidates rejected because they were out of distance range.
    pub candidates_skipped_distance: usize,
    /// Candidates rejected because the source was not in their `HaveAtClient`
    /// set at the moment the message was resolved.
    pub candidates_skipped_not_visible: usize,
    /// `SelfOnly` events skipped (no broadcast; session delivers its own packets).
    pub self_only_skipped: usize,
    /// `try_send` calls that returned `Err` (channel full or disconnected).
    pub send_failed: usize,
}

/// Summary for map-wide creature-visibility refresh commands.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RuntimeVisibilityRefreshDeliverySummaryLikeCpp {
    /// Total candidate sessions evaluated.
    pub candidates_seen: usize,
    /// Commands successfully enqueued (`try_send` succeeded).
    pub candidates_queued: usize,
    /// Candidates rejected because their map_id did not match.
    pub candidates_skipped_wrong_map: usize,
    /// Candidates rejected because their instance_id did not match.
    pub candidates_skipped_wrong_instance: usize,
    /// Candidates rejected because `is_in_world == false`.
    pub candidates_skipped_not_in_world: usize,
    /// `try_send` calls that returned `Err` (channel full or disconnected).
    pub send_failed: usize,
}

/// Summary for the map-owned `SendObjectUpdates` VALUES publication rail.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct CanonicalMapObjectValuesDeliverySummaryLikeCpp {
    pub updates_seen: usize,
    pub candidates_seen: usize,
    pub candidates_queued: usize,
    pub candidates_skipped_wrong_map: usize,
    pub candidates_skipped_wrong_instance: usize,
    pub candidates_skipped_not_in_world: usize,
    pub candidates_skipped_not_visible: usize,
    pub send_failed: usize,
}

/// Deliver typed map VALUES updates after the canonical map lock is released.
///
/// C++ `Map::SendObjectUpdates` builds one update for every player that has the
/// source in its `HaveAtClient` set. The map producer has already snapshotted
/// and cleared the source masks; this adapter owns only recipient selection and
/// queueing, with the Session command applying the final incarnation/visibility
/// gate. No synchronous map or persistence guard may surround this function.
pub(crate) fn deliver_canonical_map_object_values_updates_like_cpp(
    updates: &[CanonicalMapObjectValuesUpdateLikeCpp],
    registry: &wow_world::session::directory::PlayerRegistry,
) -> CanonicalMapObjectValuesDeliverySummaryLikeCpp {
    let mut summary = CanonicalMapObjectValuesDeliverySummaryLikeCpp {
        updates_seen: updates.len(),
        ..Default::default()
    };
    let recipients = registry.runtime_recipients();
    for update in updates {
        for recipient in &recipients {
            summary.candidates_seen += 1;
            if !recipient.is_in_world {
                summary.candidates_skipped_not_in_world += 1;
                continue;
            }
            if recipient.map_id != update.map_id {
                summary.candidates_skipped_wrong_map += 1;
                continue;
            }
            if recipient.instance_id != update.instance_id {
                summary.candidates_skipped_wrong_instance += 1;
                continue;
            }
            let recipient_has_object = if update.object_guid.is_mo_transport() {
                recipient
                    .committed_visible_transports
                    .contains(&update.object_guid)
            } else {
                recipient.committed_visibility.contains(&update.object_guid)
            };
            if !recipient_has_object {
                summary.candidates_skipped_not_visible += 1;
                continue;
            }

            let command = SessionCommand::SendVisibleObjectValuesUpdate(
                SendVisibleObjectValuesUpdateCommand {
                    object_guid: update.object_guid,
                    map_id: update.map_id,
                    packet_bytes: update.packet_bytes.clone(),
                    unit_values_update: update.unit_values_update.clone(),
                },
            );
            match registry.try_send_current_command(recipient.registration, command) {
                Ok(()) => summary.candidates_queued += 1,
                Err(_) => summary.send_failed += 1,
            }
        }
    }
    summary
}

/// Summary for explicit victim-session creature melee commands.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RuntimeCreatureMeleeDeliverySummaryLikeCpp {
    /// Total already-resolved creature melee commands processed.
    pub commands_seen: usize,
    /// Victim sessions found in [`PlayerRegistry`] and evaluated.
    pub candidates_seen: usize,
    /// Commands successfully enqueued (`try_send` succeeded).
    pub candidates_queued: usize,
    /// Commands whose victim player was not present in the registry.
    pub candidates_skipped_missing_victim: usize,
    /// Candidates rejected because their map_id did not match the command.
    pub candidates_skipped_wrong_map: usize,
    /// Candidates rejected because their instance_id did not match the command.
    pub candidates_skipped_wrong_instance: usize,
    /// Candidates rejected because `is_in_world == false`.
    pub candidates_skipped_not_in_world: usize,
    /// `try_send` calls that returned `Err` (channel full or disconnected).
    pub send_failed: usize,
}

/// Summary for explicit victim-session creature attack-start commands.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct RuntimeCreatureAttackStartDeliverySummaryLikeCpp {
    /// Total already-resolved creature aggro commands processed.
    pub commands_seen: usize,
    /// Victim sessions found in [`PlayerRegistry`] and evaluated.
    pub candidates_seen: usize,
    /// Commands successfully enqueued (`try_send` succeeded).
    pub candidates_queued: usize,
    /// Commands whose victim player was not present in the registry.
    pub candidates_skipped_missing_victim: usize,
    /// Candidates rejected because their map_id did not match the command.
    pub candidates_skipped_wrong_map: usize,
    /// Candidates rejected because their instance_id did not match the command.
    pub candidates_skipped_wrong_instance: usize,
    /// Candidates rejected because `is_in_world == false`.
    pub candidates_skipped_not_in_world: usize,
    /// Candidates rejected because the victim was no longer alive at delivery time.
    pub candidates_skipped_dead: usize,
    /// `try_send` calls that returned `Err` (channel full or disconnected).
    pub send_failed: usize,
}

impl RuntimeVisibilityRefreshDeliverySummaryLikeCpp {
    fn merge(&mut self, other: Self) {
        self.candidates_seen += other.candidates_seen;
        self.candidates_queued += other.candidates_queued;
        self.candidates_skipped_wrong_map += other.candidates_skipped_wrong_map;
        self.candidates_skipped_wrong_instance += other.candidates_skipped_wrong_instance;
        self.candidates_skipped_not_in_world += other.candidates_skipped_not_in_world;
        self.send_failed += other.send_failed;
    }
}

/// Collect candidate sessions from `registry` for one `RuntimeEvent` and push
/// [`SessionCommand::SendIfVisibleLikeCpp`] commands via `try_send`.
///
/// Gates applied here (cheap, without session lock):
/// - `is_in_world` — mirrors C++ `Player::IsInWorld()`.
/// - `map_id` / `instance_id` — mirrors C++ `InSamePhase` + map check.
/// - Distance — 2D (`required_3d == false`) or 3D (`required_3d == true`),
///   mirroring `MessageDistDeliverer::Visit` range parameter.
///
/// The final HaveAtClient gate is applied per-session in
/// `handle_send_if_visible_like_cpp_command_like_cpp`.
///
/// **No guards are held during `try_send`**: candidates are collected into a
/// `Vec` first, then commands are sent outside the DashMap iteration.
pub(crate) fn resolve_runtime_event_candidates_like_cpp(
    event: &wow_world::runtime_plan::RuntimeEvent,
    registry: &wow_world::session::directory::PlayerRegistry,
    summary: &mut RuntimeDeliverySummaryLikeCpp,
) {
    use wow_world::runtime_plan::RecipientRule;

    match &event.recipients {
        RecipientRule::SelfOnly => {
            // SelfOnly packets are delivered by the owning session directly
            // (e.g. flush_runtime_output).  Never broadcast globally — the
            // owner session is not identified here.  C++ analogy: self-send
            // path inside `WorldObject::SendMessageToSet` skips the
            // `MessageDistDeliverer` entirely for the source player.
            summary.self_only_skipped += 1;
            return;
        }
        RecipientRule::ExplicitPlayer(guid) => {
            // Send to exactly one player session — no distance or in_world filter.
            // Read map_id/instance_id from the registry entry so that the per-session
            // gate 2/3 (player_map_id_like_cpp / instance_id) accepts the command.
            // C++ analogy: SendDirectMessage / explicit-receiver path in
            // WorldObject::SendMessageToSet does NOT apply a map filter on the sender
            // side; the receiver is already known.  We mirror this by populating the
            // command with the *target* session's own map/instance so the gate passes.
            // If the guid is not in the registry we drop silently (session already gone).
            if let Some(candidate) = registry.runtime_recipient(*guid) {
                summary.candidates_seen += 1;
                let cmd = wow_world::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(
                    wow_world::session::mailbox::SendIfVisibleLikeCppCommand {
                        queued_at: Instant::now(),
                        source_guid: event.source_guid,
                        map_id: candidate.map_id,
                        instance_id: candidate.instance_id,
                        packet_bytes: event.packet_bytes.clone(),
                    },
                );
                if registry
                    .try_send_current_command(candidate.registration, cmd)
                    .is_ok()
                {
                    summary.candidates_queued += 1;
                } else {
                    summary.send_failed += 1;
                }
            }
        }
        RecipientRule::MapBroadcastVisible {
            map_id,
            instance_id,
        } => {
            // Single pass: classify each session once, then send outside the
            // DashMap iteration (no guards held during try_send).
            // Mirrors the NearbyVisible pattern above.
            struct Candidate {
                registration: wow_world::session::directory::PlayerRegistration,
                skip_reason: Option<BroadcastSkipReason>,
            }
            enum BroadcastSkipReason {
                NotInWorld,
                WrongMap,
                WrongInstance,
            }
            let candidates: Vec<Candidate> = registry
                .runtime_recipients()
                .into_iter()
                .map(|recipient| {
                    if !recipient.is_in_world {
                        return Candidate {
                            registration: recipient.registration,
                            skip_reason: Some(BroadcastSkipReason::NotInWorld),
                        };
                    }
                    if recipient.map_id != *map_id {
                        return Candidate {
                            registration: recipient.registration,
                            skip_reason: Some(BroadcastSkipReason::WrongMap),
                        };
                    }
                    if recipient.instance_id != *instance_id {
                        return Candidate {
                            registration: recipient.registration,
                            skip_reason: Some(BroadcastSkipReason::WrongInstance),
                        };
                    }
                    Candidate {
                        registration: recipient.registration,
                        skip_reason: None,
                    }
                })
                .collect();

            for candidate in candidates {
                summary.candidates_seen += 1;
                match candidate.skip_reason {
                    Some(BroadcastSkipReason::NotInWorld) => {
                        summary.candidates_skipped_not_in_world += 1;
                    }
                    Some(BroadcastSkipReason::WrongMap) => {
                        summary.candidates_skipped_wrong_map += 1;
                    }
                    Some(BroadcastSkipReason::WrongInstance) => {
                        summary.candidates_skipped_wrong_instance += 1;
                    }
                    None => {
                        let cmd = wow_world::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(
                            wow_world::session::mailbox::SendIfVisibleLikeCppCommand {
                                queued_at: Instant::now(),
                                source_guid: event.source_guid,
                                map_id: *map_id,
                                instance_id: *instance_id,
                                packet_bytes: event.packet_bytes.clone(),
                            },
                        );
                        if registry
                            .try_send_current_command(candidate.registration, cmd)
                            .is_ok()
                        {
                            summary.candidates_queued += 1;
                        } else {
                            summary.send_failed += 1;
                        }
                    }
                }
            }
        }
        RecipientRule::NearbyVisible {
            source_guid: _,
            map_id,
            instance_id,
            source_position,
            range,
            required_3d,
        } => {
            let range_sq = range * range;
            // Collect candidates first (avoid holding guards during try_send).
            struct Candidate {
                registration: wow_world::session::directory::PlayerRegistration,
                skip_reason: Option<SkipReason>,
            }
            enum SkipReason {
                NotInWorld,
                WrongMap,
                WrongInstance,
                Distance,
            }
            let candidates: Vec<Candidate> = registry
                .runtime_recipients()
                .into_iter()
                .map(|recipient| {
                    if !recipient.is_in_world {
                        return Candidate {
                            registration: recipient.registration,
                            skip_reason: Some(SkipReason::NotInWorld),
                        };
                    }
                    if recipient.map_id != *map_id {
                        return Candidate {
                            registration: recipient.registration,
                            skip_reason: Some(SkipReason::WrongMap),
                        };
                    }
                    if recipient.instance_id != *instance_id {
                        return Candidate {
                            registration: recipient.registration,
                            skip_reason: Some(SkipReason::WrongInstance),
                        };
                    }
                    let dist_sq = if *required_3d {
                        let dx = recipient.position.x - source_position.x;
                        let dy = recipient.position.y - source_position.y;
                        let dz = recipient.position.z - source_position.z;
                        dx * dx + dy * dy + dz * dz
                    } else {
                        let dx = recipient.position.x - source_position.x;
                        let dy = recipient.position.y - source_position.y;
                        dx * dx + dy * dy
                    };
                    if dist_sq > range_sq {
                        return Candidate {
                            registration: recipient.registration,
                            skip_reason: Some(SkipReason::Distance),
                        };
                    }
                    Candidate {
                        registration: recipient.registration,
                        skip_reason: None,
                    }
                })
                .collect();

            for candidate in candidates {
                summary.candidates_seen += 1;
                match candidate.skip_reason {
                    Some(SkipReason::NotInWorld) => {
                        summary.candidates_skipped_not_in_world += 1;
                    }
                    Some(SkipReason::WrongMap) => {
                        summary.candidates_skipped_wrong_map += 1;
                    }
                    Some(SkipReason::WrongInstance) => {
                        summary.candidates_skipped_wrong_instance += 1;
                    }
                    Some(SkipReason::Distance) => {
                        summary.candidates_skipped_distance += 1;
                    }
                    None => {
                        let cmd = wow_world::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(
                            wow_world::session::mailbox::SendIfVisibleLikeCppCommand {
                                queued_at: Instant::now(),
                                source_guid: event.source_guid,
                                map_id: *map_id,
                                instance_id: *instance_id,
                                packet_bytes: event.packet_bytes.clone(),
                            },
                        );
                        if registry
                            .try_send_current_command(candidate.registration, cmd)
                            .is_ok()
                        {
                            summary.candidates_queued += 1;
                        } else {
                            summary.send_failed += 1;
                        }
                    }
                }
            }
        }
        RecipientRule::NearbyVisibleDurableSpellCast {
            source_guid,
            map_id,
            instance_id,
            source_position,
            range,
            required_3d,
            basic_go_packet_bytes,
            full_go_packet_bytes,
        } => {
            let range_sq = range * range;
            struct Candidate {
                registration: wow_world::session::directory::PlayerRegistration,
                committed_visibility_like_cpp:
                    wow_world::session::mailbox::SharedClientVisibleGuidsLikeCpp,
                advanced_combat_logging_like_cpp: bool,
                skip_reason: Option<DurableSpellCastSkipReason>,
            }
            enum DurableSpellCastSkipReason {
                NotInWorld,
                WrongMap,
                WrongInstance,
                Distance,
                NotVisible,
            }
            let candidates: Vec<Candidate> = registry
                .runtime_recipients()
                .into_iter()
                .map(|recipient| {
                    let dx = recipient.position.x - source_position.x;
                    let dy = recipient.position.y - source_position.y;
                    let dz = recipient.position.z - source_position.z;
                    let distance_sq = if *required_3d {
                        dx * dx + dy * dy + dz * dz
                    } else {
                        dx * dx + dy * dy
                    };
                    let skip_reason = if !recipient.is_in_world {
                        Some(DurableSpellCastSkipReason::NotInWorld)
                    } else if recipient.map_id != *map_id {
                        Some(DurableSpellCastSkipReason::WrongMap)
                    } else if recipient.instance_id != *instance_id {
                        Some(DurableSpellCastSkipReason::WrongInstance)
                    } else if distance_sq > range_sq {
                        Some(DurableSpellCastSkipReason::Distance)
                    } else if !recipient.committed_visibility.contains(source_guid) {
                        // C++ `MessageDistDeliverer` reads `HaveAtClient` while
                        // `SendSpellGo` runs, so the recipient decision belongs
                        // here and is committed into the queued command.
                        Some(DurableSpellCastSkipReason::NotVisible)
                    } else {
                        None
                    };
                    Candidate {
                        registration: recipient.registration,
                        committed_visibility_like_cpp: recipient.committed_visibility,
                        // C++ `WorldObject::SendCombatLogMessage` reads each
                        // receiver's preference while distributing the cast.
                        advanced_combat_logging_like_cpp: recipient.advanced_combat_logging,
                        skip_reason,
                    }
                })
                .collect();
            for candidate in candidates {
                summary.candidates_seen += 1;
                match candidate.skip_reason {
                    Some(DurableSpellCastSkipReason::NotInWorld) => {
                        summary.candidates_skipped_not_in_world += 1;
                        continue;
                    }
                    Some(DurableSpellCastSkipReason::WrongMap) => {
                        summary.candidates_skipped_wrong_map += 1;
                        continue;
                    }
                    Some(DurableSpellCastSkipReason::WrongInstance) => {
                        summary.candidates_skipped_wrong_instance += 1;
                        continue;
                    }
                    Some(DurableSpellCastSkipReason::Distance) => {
                        summary.candidates_skipped_distance += 1;
                        continue;
                    }
                    Some(DurableSpellCastSkipReason::NotVisible) => {
                        summary.candidates_skipped_not_visible += 1;
                        continue;
                    }
                    None => {}
                }
                let command =
                    wow_world::session::mailbox::SendCreatureSpellCastIfVisibleLikeCppCommand {
                        queued_at: Instant::now(),
                        source_guid: *source_guid,
                        map_id: *map_id,
                        instance_id: *instance_id,
                        start_packet_bytes: event.packet_bytes.clone(),
                        go_packet_bytes: if candidate.advanced_combat_logging_like_cpp {
                            full_go_packet_bytes.clone()
                        } else {
                            basic_go_packet_bytes.clone()
                        },
                        committed_visibility_like_cpp: candidate.committed_visibility_like_cpp,
                    };
                if registry
                    .publish_current_creature_spell_cast_if_visible(candidate.registration, command)
                {
                    summary.candidates_queued += 1;
                } else {
                    summary.send_failed += 1;
                }
            }
        }
        RecipientRule::NearbyVisibleDurable {
            source_guid: _,
            map_id,
            instance_id,
            source_position,
            range,
            required_3d,
        } => {
            let range_sq = range * range;
            struct Candidate {
                registration: wow_world::session::directory::PlayerRegistration,
                eligible: bool,
            }
            let candidates: Vec<Candidate> = registry
                .runtime_recipients()
                .into_iter()
                .map(|recipient| {
                    let dx = recipient.position.x - source_position.x;
                    let dy = recipient.position.y - source_position.y;
                    let dz = recipient.position.z - source_position.z;
                    let distance_sq = if *required_3d {
                        dx * dx + dy * dy + dz * dz
                    } else {
                        dx * dx + dy * dy
                    };
                    Candidate {
                        registration: recipient.registration,
                        eligible: recipient.is_in_world
                            && recipient.map_id == *map_id
                            && recipient.instance_id == *instance_id
                            && distance_sq <= range_sq,
                    }
                })
                .collect();
            for candidate in candidates {
                summary.candidates_seen += 1;
                if !candidate.eligible {
                    summary.candidates_skipped_distance += 1;
                    continue;
                }
                let command = wow_world::session::mailbox::SendIfVisibleLikeCppCommand {
                    queued_at: Instant::now(),
                    source_guid: event.source_guid,
                    map_id: *map_id,
                    instance_id: *instance_id,
                    packet_bytes: event.packet_bytes.clone(),
                };
                if registry.publish_current_send_if_visible(candidate.registration, command) {
                    summary.candidates_queued += 1;
                } else {
                    summary.send_failed += 1;
                }
            }
        }
    }
}

/// Route and deliver all events in `plan` to candidate sessions.
///
/// Returns a [`RuntimeDeliverySummaryLikeCpp`] for test assertions.
/// No blocking sends — backpressure via `try_send` only.
pub(crate) fn deliver_runtime_plan_like_cpp(
    plan: &wow_world::runtime_plan::RuntimePlan,
    registry: &wow_world::session::directory::PlayerRegistry,
) -> RuntimeDeliverySummaryLikeCpp {
    let mut summary = RuntimeDeliverySummaryLikeCpp::default();
    for event in &plan.events {
        summary.events_seen += 1;
        resolve_runtime_event_candidates_like_cpp(event, registry, &mut summary);
    }
    summary
}

/// Ask all sessions on a map instance to recompute map-owned creature visibility.
///
/// This is dormant 4A.3c infrastructure for global create/destroy/respawn
/// delivery. C++ creates/destroys visibility through `Player::UpdateVisibilityOf`
/// (Player.cpp:23138+) rather than by sending a raw packet that bypasses
/// `m_clientGUIDs`. The session command mirrors that seam by forcing each
/// matching session to run its own visibility pass.
///
/// No map locks are held here; registry candidates are cloned before `try_send`.
pub(crate) fn deliver_refresh_visible_world_creatures_like_cpp(
    map_id: u16,
    instance_id: u32,
    registry: &wow_world::session::directory::PlayerRegistry,
) -> RuntimeVisibilityRefreshDeliverySummaryLikeCpp {
    struct Candidate {
        registration: wow_world::session::directory::PlayerRegistration,
        skip_reason: Option<RefreshSkipReason>,
    }
    enum RefreshSkipReason {
        NotInWorld,
        WrongMap,
        WrongInstance,
    }

    let candidates: Vec<Candidate> = registry
        .runtime_recipients()
        .into_iter()
        .map(|recipient| {
            if !recipient.is_in_world {
                return Candidate {
                    registration: recipient.registration,
                    skip_reason: Some(RefreshSkipReason::NotInWorld),
                };
            }
            if recipient.map_id != map_id {
                return Candidate {
                    registration: recipient.registration,
                    skip_reason: Some(RefreshSkipReason::WrongMap),
                };
            }
            if recipient.instance_id != instance_id {
                return Candidate {
                    registration: recipient.registration,
                    skip_reason: Some(RefreshSkipReason::WrongInstance),
                };
            }
            Candidate {
                registration: recipient.registration,
                skip_reason: None,
            }
        })
        .collect();

    let mut summary = RuntimeVisibilityRefreshDeliverySummaryLikeCpp::default();
    for candidate in candidates {
        summary.candidates_seen += 1;
        match candidate.skip_reason {
            Some(RefreshSkipReason::NotInWorld) => {
                summary.candidates_skipped_not_in_world += 1;
            }
            Some(RefreshSkipReason::WrongMap) => {
                summary.candidates_skipped_wrong_map += 1;
            }
            Some(RefreshSkipReason::WrongInstance) => {
                summary.candidates_skipped_wrong_instance += 1;
            }
            None => {
                let cmd = wow_world::session::mailbox::SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(
                    wow_world::session::mailbox::RefreshVisibleWorldCreaturesLikeCppCommand {
                        map_id,
                        instance_id,
                    },
                );
                if registry
                    .try_send_current_command(candidate.registration, cmd)
                    .is_ok()
                {
                    summary.candidates_queued += 1;
                } else {
                    summary.send_failed += 1;
                }
            }
        }
    }
    summary
}

/// Snapshot active player positions for the global creature aggro scan.
///
/// The scan itself is map-owned and runs in `wow-world`; this bridge only
/// collects cheap, copyable receiver state from [`PlayerRegistry`] and drops
/// DashMap guards before the canonical map lock is taken.
#[cfg(test)]
pub(crate) fn collect_legacy_creature_aggro_candidates_like_cpp(
    registry: &wow_world::session::directory::PlayerRegistry,
) -> Vec<wow_world::session::LegacyCreatureAggroCandidateLikeCpp> {
    collect_legacy_creature_aggro_candidates_with_canonical_like_cpp(registry, None)
}

pub(crate) fn collect_legacy_creature_aggro_candidates_with_canonical_like_cpp(
    registry: &wow_world::session::directory::PlayerRegistry,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
) -> Vec<wow_world::session::LegacyCreatureAggroCandidateLikeCpp> {
    let mut candidates: Vec<_> = registry
        .legacy_aggro_candidates()
        .into_iter()
        .map(
            |snapshot| wow_world::session::LegacyCreatureAggroCandidateLikeCpp {
                player_guid: snapshot.player_guid,
                map_id: snapshot.map_id,
                instance_id: snapshot.instance_id,
                map_difficulty_id: 0,
                position: snapshot.position,
                player_visibility_represented: false,
                player_phase_shift: wow_entities::PhaseShift::default(),
                player_visibility_detection:
                    wow_entities::UnitVisibilityDetectionStateLikeCpp::default(),
                player_combat_reach: snapshot.combat_reach,
                player_detected_range_aura_mod: 0.0,
                player_liquid_status_like_cpp: snapshot.liquid_status,
                player_level: snapshot.level,
                player_gray_level: snapshot.gray_level,
                player_unit_flags: snapshot.unit_flags,
                player_unit_flags2: 0,
                player_unit_state: snapshot.unit_state,
                player_is_game_master: snapshot.is_game_master,
                player_is_contested_pvp: false,
                player_faction_template_id: snapshot.faction_template_id,
                player_reputation_standings: Vec::new(),
                player_reputation_state_flags: Vec::new(),
                player_forced_reputation_ranks: snapshot.forced_reputation_ranks,
                player_forced_reputation_faction_ids: Vec::new(),
                player_school_immunity_mask: 0,
                player_damage_immunity_mask: 0,
                player_has_confuse_aura: false,
                player_has_breakable_stun_aura: false,
            },
        )
        .collect();

    if let Some(canonical_map_manager) = canonical_map_manager
        && let Ok(manager) = canonical_map_manager.lock()
    {
        for candidate in &mut candidates {
            let Some(managed) =
                manager.find_map(u32::from(candidate.map_id), candidate.instance_id)
            else {
                continue;
            };
            let Some(player) = managed.map().get_typed_player(candidate.player_guid) else {
                continue;
            };
            candidate.map_difficulty_id = managed.difficulty();
            candidate.player_visibility_represented = true;
            candidate.player_unit_flags2 =
                wow_world::canonical_player_access::canonical_player_unit_flags2_like_cpp(player);
            candidate.player_is_contested_pvp =
                wow_world::canonical_player_access::canonical_player_is_contested_pvp_like_cpp(
                    player,
                );
            candidate.player_reputation_standings =
                wow_world::canonical_player_access::canonical_player_reputation_standings_like_cpp(
                    player,
                );
            candidate.player_reputation_state_flags =
                wow_world::canonical_player_access::canonical_player_reputation_state_flags_like_cpp(
                    player,
                );
            candidate.player_forced_reputation_faction_ids =
                wow_world::canonical_player_access::canonical_player_forced_reputation_faction_ids_like_cpp(
                    player,
                );
            candidate.player_phase_shift = player.unit().world().phase_shift().clone();
            candidate.player_visibility_detection =
                player.unit().visibility_detection_like_cpp().clone();
            candidate.player_detected_range_aura_mod = player.unit().total_aura_modifier_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_DETECTED_RANGE,
            ) as f32;
            candidate.player_school_immunity_mask =
                player.unit().subsystems().auras.aura_school_mask_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_SCHOOL_IMMUNITY,
                );
            candidate.player_damage_immunity_mask =
                player.unit().subsystems().auras.aura_school_mask_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_DAMAGE_IMMUNITY,
                );
            candidate.player_has_confuse_aura = player
                .unit()
                .subsystems()
                .auras
                .has_aura_type_like_cpp(wow_data::spell::aura_types::SPELL_AURA_MOD_CONFUSE);
            candidate.player_has_breakable_stun_aura = player
                .unit()
                .subsystems()
                .auras
                .has_breakable_by_damage_aura_type_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_STUN,
                );
        }
    }

    candidates
}

pub(crate) fn legacy_creature_aggro_config_like_cpp(
    configs: &WorldConfigSet,
) -> wow_world::session::LegacyCreatureAggroConfigLikeCpp {
    let creature_aggro_rate = world_config_f32(configs, "RATE_CREATURE_AGGRO", 1.0);
    wow_world::session::LegacyCreatureAggroConfigLikeCpp {
        expected_stat_store: None,
        no_gray_aggro_above: world_config_u32(configs, "CONFIG_NO_GRAY_AGGRO_ABOVE", 0),
        no_gray_aggro_below: world_config_u32(configs, "CONFIG_NO_GRAY_AGGRO_BELOW", 0),
        creature_aggro_rate,
        max_player_level_config: world_config_u32(configs, "CONFIG_MAX_PLAYER_LEVEL", 80),
        visibility_distance_continents: legacy_visibility_distance_like_cpp(
            "Visibility.Distance.Continents",
            wow_entities::DEFAULT_VISIBILITY_DISTANCE,
            creature_aggro_rate,
        ),
        visibility_distance_instances: legacy_visibility_distance_like_cpp(
            "Visibility.Distance.Instances",
            wow_entities::DEFAULT_VISIBILITY_INSTANCE,
            creature_aggro_rate,
        ),
        visibility_distance_battlegrounds: legacy_visibility_distance_like_cpp(
            "Visibility.Distance.BG",
            533.0,
            creature_aggro_rate,
        ),
        visibility_distance_arenas: legacy_visibility_distance_like_cpp(
            "Visibility.Distance.Arenas",
            533.0,
            creature_aggro_rate,
        ),
        family_assistance_radius: world_config_f32(
            configs,
            "CONFIG_CREATURE_FAMILY_ASSISTANCE_RADIUS",
            10.0,
        ),
        family_assistance_delay_ms: world_config_u32(
            configs,
            "CONFIG_CREATURE_FAMILY_ASSISTANCE_DELAY",
            1_500,
        ),
        faction_template_store: None,
        faction_store: None,
        map_store: None,
        disable_mgr: Some(Arc::new(wow_data::DisableMgrLikeCpp::default())),
        spell_misc_store: None,
        spell_range_store: None,
        spell_duration_store: None,
        spell_cooldowns_store: None,
        spell_category_store: None,
        spell_x_spell_visual_store: None,
        spell_target_restrictions_store: None,
        spell_casting_requirements_store: None,
        spell_aura_restrictions_store: None,
        spell_store: None,
        spell_threat_store: None,
        spell_chain_store: None,
        spell_linked_store: None,
        spell_condition_store: None,
        spell_script_exact_spell_ids_like_cpp: None,
        spell_script_all_rank_root_spell_ids_like_cpp: None,
        legacy_spell_script_spell_ids_like_cpp: None,
        spell_linked_rejected_trigger_spell_ids_like_cpp: None,
        spell_custom_attribute_store: None,
        difficulty_store: None,
        creature_template_lifecycle_store: None,
        chr_races_store: None,
    }
}

pub(crate) fn legacy_visibility_distance_like_cpp(
    key: &str,
    default: f32,
    creature_aggro_rate: f32,
) -> f32 {
    let configured = wow_config::get_value_default::<f32>(key, default);
    let min = 45.0 * creature_aggro_rate;
    if configured < min {
        min
    } else if configured > wow_entities::MAX_VISIBILITY_DISTANCE {
        wow_entities::MAX_VISIBILITY_DISTANCE
    } else {
        configured
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct RuntimePlayerMeleeDeliverySummaryLikeCpp {
    pub commands_seen: usize,
    pub candidates_seen: usize,
    pub candidates_queued: usize,
    pub candidates_skipped_missing_attacker: usize,
    pub candidates_skipped_not_in_world: usize,
    pub candidates_skipped_wrong_map: usize,
    pub candidates_skipped_wrong_instance: usize,
    pub candidates_dropped_durable: usize,
}

/// Who is swinging this frame, from the registry, before any map lock.
///
/// No `in_combat` pre-filter: that field is a session-published mirror, and
/// filtering the phase on it would let a stale mirror silently stop a player's
/// swings from resolving — the failure #28 exists to prevent. The aggro phase
/// already reads every in-world player per tick, so this is not a new cost
/// class.
pub(crate) fn collect_player_melee_attackers_like_cpp(
    registry: &wow_world::session::directory::PlayerRegistry,
    tap_index: &std::collections::HashMap<ObjectGuid, Vec<ObjectGuid>>,
) -> Vec<wow_world::session::PlayerMeleeAttackerSnapshotLikeCpp> {
    registry
        .runtime_recipients()
        .into_iter()
        .filter(|recipient| recipient.is_in_world)
        .map(
            |recipient| wow_world::session::PlayerMeleeAttackerSnapshotLikeCpp {
                registration: recipient.registration,
                player_guid: recipient.guid,
                map_id: recipient.map_id,
                instance_id: recipient.instance_id,
                in_combat_mirror: recipient.in_combat,
                tap_group_guids: tap_index.get(&recipient.guid).cloned().unwrap_or_default(),
            },
        )
        .collect()
}

/// Route each resolution to the one session that swung it.
///
/// The attacker is the sole recipient, matching the session's `send_tx`-only
/// fanout today: no distance or visibility gate, because none existed. It rides
/// the durable rail — a dropped result is a lost kill, and with it the loot.
pub(crate) fn deliver_player_melee_results_like_cpp(
    commands: &[wow_world::session::mailbox::ApplyPlayerMeleeResultLikeCppCommand],
    registry: &wow_world::session::directory::PlayerRegistry,
) -> RuntimePlayerMeleeDeliverySummaryLikeCpp {
    let mut summary = RuntimePlayerMeleeDeliverySummaryLikeCpp::default();
    for command in commands {
        summary.commands_seen += 1;
        let Some(recipient) = registry.runtime_recipient(command.attacker_guid) else {
            summary.candidates_skipped_missing_attacker += 1;
            continue;
        };
        summary.candidates_seen += 1;
        if !recipient.is_in_world {
            summary.candidates_skipped_not_in_world += 1;
            continue;
        }
        if recipient.map_id != command.map_id {
            summary.candidates_skipped_wrong_map += 1;
            continue;
        }
        if recipient.instance_id != command.instance_id {
            summary.candidates_skipped_wrong_instance += 1;
            continue;
        }
        if registry.publish_current_player_melee_result(recipient.registration, command.clone()) {
            summary.candidates_queued += 1;
        } else {
            summary.candidates_dropped_durable += 1;
        }
    }
    summary
}

/// Deliver map-owned creature aggro starts to their exact victim sessions.
///
/// C++ contrast: `CreatureAI::MoveInLineOfSight`/`Creature::CanStartAttack`
/// decides the engagement from map state; `Unit::SendMeleeAttackStart` sends
/// the visible combat-start packet. This helper routes that already-resolved
/// engagement to the victim session outside all map locks. The transition is
/// authoritative and one-shot, so it is published to the session's durable
/// FIFO rail without waiting on its bounded visual-command queue.
pub(crate) fn deliver_creature_attack_start_commands_like_cpp(
    commands: &[wow_world::session::mailbox::CreatureAttackStartLikeCppCommand],
    registry: &wow_world::session::directory::PlayerRegistry,
) -> RuntimeCreatureAttackStartDeliverySummaryLikeCpp {
    struct Candidate {
        registration: wow_world::session::directory::PlayerRegistration,
        map_id: u16,
        instance_id: u32,
        is_in_world: bool,
        is_alive: bool,
    }

    let mut summary = RuntimeCreatureAttackStartDeliverySummaryLikeCpp::default();
    for command in commands {
        summary.commands_seen += 1;

        let Some(candidate) = registry
            .runtime_recipient(command.victim_guid)
            .map(|recipient| Candidate {
                registration: recipient.registration,
                map_id: recipient.map_id,
                instance_id: recipient.instance_id,
                is_in_world: recipient.is_in_world,
                is_alive: recipient.is_alive,
            })
        else {
            summary.candidates_skipped_missing_victim += 1;
            continue;
        };

        summary.candidates_seen += 1;
        if !candidate.is_in_world {
            summary.candidates_skipped_not_in_world += 1;
            continue;
        }
        if !candidate.is_alive {
            summary.candidates_skipped_dead += 1;
            continue;
        }
        if candidate.map_id != command.map_id {
            summary.candidates_skipped_wrong_map += 1;
            continue;
        }
        if candidate.instance_id != command.instance_id {
            summary.candidates_skipped_wrong_instance += 1;
            continue;
        }

        if registry.publish_current_attack_start(candidate.registration, command.clone()) {
            summary.candidates_queued += 1;
        } else {
            summary.send_failed += 1;
        }
    }
    summary
}

/// Apply map-owned creature-vs-creature assistance starts directly to the
/// canonical map. Player victims use their session command because that
/// transition also owns session combat state; a creature victim has no
/// Who shares a tap with each grouped player, for one tick.
///
/// `WorldSession::current_group_member_guids_for_tap_like_cpp` answers this from
/// the session's own `group_guid` mirror plus the registry. The tick owner has
/// no session, so it asks the membership authority directly and builds the
/// whole index once per tick rather than per creature (#28).
///
/// No map lock is held, and the cost is proportional to grouped players. If it
/// ever shows up hot, the follow-up is a published group projection — not
/// re-growing the session mirror.
pub(crate) fn build_tap_group_index_like_cpp(
    group_registry: Option<&Arc<wow_social::group::GroupRegistry>>,
) -> std::collections::HashMap<ObjectGuid, Vec<ObjectGuid>> {
    let mut index = std::collections::HashMap::new();
    let Some(group_registry) = group_registry else {
        return index;
    };
    for group in group_registry.snapshots() {
        for member in &group.members {
            let others: Vec<ObjectGuid> = group
                .members
                .iter()
                .copied()
                .filter(|other| other != member)
                .collect();
            index.insert(*member, others);
        }
    }
    index
}

/// Commit creature attack-start commands through each map's single writer.
///
/// #1263 F6-8D3a-1: the body lives in wow-world so the admitted canonical
/// executor commits through the same transition; this adapter only takes the
/// canonical guard for a caller that does not hold it.
#[cfg(test)]
pub(crate) fn apply_canonical_creature_attack_starts_like_cpp(
    commands: &[wow_world::session::mailbox::CreatureAttackStartLikeCppCommand],
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
) -> Vec<wow_map::MapCommandOutcomeLikeCpp> {
    let Some(manager) = canonical_map_manager else {
        return Vec::new();
    };
    let Ok(mut manager) = manager.lock() else {
        return Vec::new();
    };
    wow_world::session::apply_creature_attack_start_commands_on_manager_like_cpp(
        &mut manager,
        commands,
    )
}

/// Commit creature evade/combat-stop commands through each map's single writer.
/// The body lives in wow-world beside the start commit (#1263 F6-8D3a-1).
#[cfg(test)]
pub(crate) fn apply_canonical_creature_attack_stops_like_cpp(
    commands: &[wow_world::session::mailbox::CreatureAttackStopLikeCppCommand],
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
) -> Vec<wow_map::MapCommandOutcomeLikeCpp> {
    let Some(manager) = canonical_map_manager else {
        return Vec::new();
    };
    let Ok(mut manager) = manager.lock() else {
        return Vec::new();
    };
    wow_world::session::apply_creature_attack_stop_commands_on_manager_like_cpp(
        &mut manager,
        commands,
    )
}

#[cfg(test)]
pub(crate) use wow_world::session::retain_committed_creature_combat_events_like_cpp;

pub(crate) fn deliver_creature_attack_stop_commands_like_cpp(
    commands: &[wow_world::session::mailbox::CreatureAttackStopLikeCppCommand],
    registry: &wow_world::session::directory::PlayerRegistry,
) -> RuntimeCreatureAttackStartDeliverySummaryLikeCpp {
    let mut summary = RuntimeCreatureAttackStartDeliverySummaryLikeCpp::default();
    for command in commands {
        summary.commands_seen += 1;
        let Some(candidate) = registry.runtime_recipient(command.victim_guid) else {
            summary.candidates_skipped_missing_victim += 1;
            continue;
        };
        summary.candidates_seen += 1;
        if !candidate.is_in_world {
            summary.candidates_skipped_not_in_world += 1;
        } else if candidate.map_id != command.map_id {
            summary.candidates_skipped_wrong_map += 1;
        } else if candidate.instance_id != command.instance_id {
            summary.candidates_skipped_wrong_instance += 1;
        // Evade cleanup is a one-shot authoritative transition. Publish it to
        // the durable FIFO rail so bounded visual backpressure cannot
        // drop it or block the global creature tick.
        } else if registry.publish_current_attack_stop(candidate.registration, command.clone()) {
            summary.candidates_queued += 1;
        } else {
            summary.send_failed += 1;
        }
    }
    summary
}

/// Deliver map-owned creature melee results to their exact victim sessions.
///
/// C++ contrast: `Creature::Update` runs `DoMeleeAttackIfReady()` from the
/// map object update phase; `AttackerStateUpdate` resolves the damage once,
/// mutates the victim, then sends the combat packet. The global Rust driver
/// mirrors that by producing final-health commands once from the map owner.
/// This helper only routes those already-resolved results to the victim
/// session. It never holds map locks; because canonical health is already
/// committed, it publishes every resolved swing to the durable FIFO session
/// rail rather than dropping it or blocking the world tick.
pub(crate) fn deliver_creature_melee_damage_commands_like_cpp(
    commands: &[wow_world::session::mailbox::ApplyCreatureMeleeDamageLikeCppCommand],
    registry: &wow_world::session::directory::PlayerRegistry,
) -> RuntimeCreatureMeleeDeliverySummaryLikeCpp {
    struct Candidate {
        registration: wow_world::session::directory::PlayerRegistration,
        map_id: u16,
        instance_id: u32,
        is_in_world: bool,
    }

    let mut summary = RuntimeCreatureMeleeDeliverySummaryLikeCpp::default();
    for command in commands {
        summary.commands_seen += 1;

        let Some(candidate) = registry
            .runtime_recipient(command.victim_guid)
            .map(|recipient| Candidate {
                registration: recipient.registration,
                map_id: recipient.map_id,
                instance_id: recipient.instance_id,
                is_in_world: recipient.is_in_world,
            })
        else {
            summary.candidates_skipped_missing_victim += 1;
            continue;
        };

        summary.candidates_seen += 1;
        if !candidate.is_in_world {
            summary.candidates_skipped_not_in_world += 1;
            continue;
        }
        if candidate.map_id != command.map_id {
            summary.candidates_skipped_wrong_map += 1;
            continue;
        }
        if candidate.instance_id != command.instance_id {
            summary.candidates_skipped_wrong_instance += 1;
            continue;
        }

        if registry.publish_current_melee_damage(candidate.registration, command.clone()) {
            summary.candidates_queued += 1;
        } else {
            summary.send_failed += 1;
        }
    }
    summary
}

/// Snapshot player positions a chasing creature may need this frame.
///
/// C++ `ChaseMovementGenerator` dereferences a live `Unit*`; the Rust creature
/// step has no object accessor, so the same facts are collected here from
/// [`PlayerRegistry`] and every DashMap guard is dropped before the canonical
/// map lock is taken — the pattern the aggro scan already uses.
pub(crate) fn collect_chase_target_snapshots_like_cpp(
    registry: &wow_world::session::directory::PlayerRegistry,
) -> std::collections::HashMap<
    (u16, u32, wow_core::ObjectGuid),
    wow_world::ChaseTargetSnapshotLikeCpp,
> {
    registry
        .runtime_recipients()
        .into_iter()
        .filter_map(|recipient| {
            (recipient.is_in_world && recipient.is_alive).then_some((
                (recipient.map_id, recipient.instance_id, recipient.guid),
                wow_world::ChaseTargetSnapshotLikeCpp {
                    guid: recipient.guid,
                    position: recipient.position,
                    combat_reach: recipient.combat_reach,
                    in_world: true,
                    // C++ `Unit::isInAccessiblePlaceFor` asks `CanEnterWater()`
                    // when the victim `IsInWater()`; the registry already carries
                    // the liquid status the aggro scan uses.
                    in_water: Some(
                        recipient.liquid_status
                            & (wow_world::session::LIQUID_MAP_IN_WATER_LIKE_CPP
                                | wow_world::session::LIQUID_MAP_UNDER_WATER_LIKE_CPP)
                            != 0,
                    ),
                },
            ))
        })
        .collect()
}

/// Wake every in-world session of each map key the lifecycle phase changed.
///
/// The canonical owner (#1263 F6-8D3b-1) calls it with the executor's deferred
/// refresh keys once every guard is released.
pub(crate) fn deliver_creature_lifecycle_refreshes_like_cpp(
    refresh_map_keys: &[(u16, u32)],
    registry: &wow_world::session::directory::PlayerRegistry,
) -> RuntimeVisibilityRefreshDeliverySummaryLikeCpp {
    let mut delivery = RuntimeVisibilityRefreshDeliverySummaryLikeCpp::default();
    for (map_id, instance_id) in refresh_map_keys {
        delivery.merge(deliver_refresh_visible_world_creatures_like_cpp(
            *map_id,
            *instance_id,
            registry,
        ));
    }
    delivery
}

/// Submit the lifecycle phase's respawn statements to the shared writer, in
/// production order, while the caller still holds the respawn mutation-order
/// gate (#1263 F6-8D3b-1).
pub(crate) fn submit_creature_lifecycle_respawn_db_mutations_like_cpp(
    mutations: &mut Vec<wow_persistence::RespawnPersistenceMutationLikeCpp>,
    respawn_db_writer_tx: &RespawnDbWriterSenderLikeCpp,
) -> usize {
    let mut submitted = 0;
    for mutation in mutations.drain(..) {
        if respawn_db_writer_tx.send(mutation).is_err() {
            tracing::error!(
                "Shared respawn DB writer stopped before creature respawn statement submission"
            );
        } else {
            submitted += 1;
        }
    }
    submitted
}

/// Deliver the committed attack starts, then the committed stops, to their
/// victim sessions, as one summary. The canonical owner (#1263 F6-8D3b-1) hands
/// it the executor's committed commands after every guard is released.
pub(crate) fn deliver_committed_creature_combat_commands_like_cpp(
    applied_start_commands: &[wow_world::session::mailbox::CreatureAttackStartLikeCppCommand],
    applied_stop_commands: &[wow_world::session::mailbox::CreatureAttackStopLikeCppCommand],
    registry: &wow_world::session::directory::PlayerRegistry,
) -> RuntimeCreatureAttackStartDeliverySummaryLikeCpp {
    let mut delivery =
        deliver_creature_attack_start_commands_like_cpp(applied_start_commands, registry);
    let stop_delivery =
        deliver_creature_attack_stop_commands_like_cpp(applied_stop_commands, registry);
    delivery.commands_seen += stop_delivery.commands_seen;
    delivery.candidates_seen += stop_delivery.candidates_seen;
    delivery.candidates_queued += stop_delivery.candidates_queued;
    delivery.candidates_skipped_missing_victim += stop_delivery.candidates_skipped_missing_victim;
    delivery.candidates_skipped_wrong_map += stop_delivery.candidates_skipped_wrong_map;
    delivery.candidates_skipped_wrong_instance += stop_delivery.candidates_skipped_wrong_instance;
    delivery.candidates_skipped_not_in_world += stop_delivery.candidates_skipped_not_in_world;
    delivery.candidates_skipped_dead += stop_delivery.candidates_skipped_dead;
    delivery.send_failed += stop_delivery.send_failed;
    delivery
}
