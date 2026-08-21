//! Runtime event routing, delivery, and the legacy map-owned update loop.

use super::*;

// ── Runtime candidate routing + delivery ────────────────────────────────────
//
// These functions started as dormant Slice 4A.1b infrastructure and are now
// reached through the map-owned `RustyCore.LegacyCreatureGlobalRuntime` loop,
// which is enabled by default to match C++ `MapManager::Update`.
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
    event: &wow_world::map_manager::RuntimeEvent,
    registry: &wow_network::PlayerRegistry,
    summary: &mut RuntimeDeliverySummaryLikeCpp,
) {
    use wow_world::map_manager::RecipientRule;

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
                let cmd = wow_network::SessionCommand::SendIfVisibleLikeCpp(
                    wow_network::player_registry::SendIfVisibleLikeCppCommand {
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
                registration: wow_network::PlayerRegistration,
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
                        let cmd = wow_network::SessionCommand::SendIfVisibleLikeCpp(
                            wow_network::player_registry::SendIfVisibleLikeCppCommand {
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
                registration: wow_network::PlayerRegistration,
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
                        let cmd = wow_network::SessionCommand::SendIfVisibleLikeCpp(
                            wow_network::player_registry::SendIfVisibleLikeCppCommand {
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
                registration: wow_network::PlayerRegistration,
                committed_visibility_like_cpp: wow_network::SharedClientVisibleGuidsLikeCpp,
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
                    wow_network::player_registry::SendCreatureSpellCastIfVisibleLikeCppCommand {
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
                registration: wow_network::PlayerRegistration,
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
                let command = wow_network::player_registry::SendIfVisibleLikeCppCommand {
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
    plan: &wow_world::map_manager::RuntimePlan,
    registry: &wow_network::PlayerRegistry,
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
    registry: &wow_network::PlayerRegistry,
) -> RuntimeVisibilityRefreshDeliverySummaryLikeCpp {
    struct Candidate {
        registration: wow_network::PlayerRegistration,
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
                let cmd = wow_network::SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(
                    wow_network::player_registry::RefreshVisibleWorldCreaturesLikeCppCommand {
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
/// DashMap guards before the legacy map lock is taken.
#[cfg(test)]
pub(crate) fn collect_legacy_creature_aggro_candidates_like_cpp(
    registry: &wow_network::PlayerRegistry,
) -> Vec<wow_world::session::LegacyCreatureAggroCandidateLikeCpp> {
    collect_legacy_creature_aggro_candidates_with_canonical_like_cpp(registry, None)
}

pub(crate) fn collect_legacy_creature_aggro_candidates_with_canonical_like_cpp(
    registry: &wow_network::PlayerRegistry,
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
                player_unit_flags2: snapshot.unit_flags2,
                player_unit_state: snapshot.unit_state,
                player_is_game_master: snapshot.is_game_master,
                player_is_contested_pvp: snapshot.is_contested_pvp,
                player_faction_template_id: snapshot.faction_template_id,
                player_reputation_standings: snapshot.reputation_standings,
                player_reputation_state_flags: snapshot.reputation_state_flags,
                player_forced_reputation_ranks: snapshot.forced_reputation_ranks,
                player_forced_reputation_faction_ids: snapshot.forced_reputation_faction_ids,
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
        spell_chain_store: None,
        spell_linked_store: None,
        spell_condition_store: None,
        spell_script_exact_spell_ids_like_cpp: None,
        spell_script_all_rank_root_spell_ids_like_cpp: None,
        legacy_spell_script_spell_ids_like_cpp: None,
        spell_linked_rejected_trigger_spell_ids_like_cpp: None,
        spell_custom_attribute_store: None,
        difficulty_store: None,
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

/// Deliver map-owned creature aggro starts to their exact victim sessions.
///
/// C++ contrast: `CreatureAI::MoveInLineOfSight`/`Creature::CanStartAttack`
/// decides the engagement from map state; `Unit::SendMeleeAttackStart` sends
/// the visible combat-start packet. This helper routes that already-resolved
/// engagement to the victim session outside all map locks. The transition is
/// authoritative and one-shot, so it is published to the session's durable
/// FIFO rail without waiting on its bounded visual-command queue.
pub(crate) fn deliver_creature_attack_start_commands_like_cpp(
    commands: &[wow_network::player_registry::CreatureAttackStartLikeCppCommand],
    registry: &wow_network::PlayerRegistry,
) -> RuntimeCreatureAttackStartDeliverySummaryLikeCpp {
    struct Candidate {
        registration: wow_network::PlayerRegistration,
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
/// `PlayerRegistry` recipient.
pub(crate) fn apply_canonical_creature_attack_starts_like_cpp(
    commands: &[wow_network::player_registry::CreatureAttackStartLikeCppCommand],
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
) -> usize {
    let Some(manager) = canonical_map_manager else {
        return 0;
    };
    let Ok(mut manager) = manager.lock() else {
        return 0;
    };
    let mut applied = 0;
    for command in commands
        .iter()
        .filter(|command| command.victim_guid.is_creature())
    {
        let Some(managed) = manager.find_map_mut(u32::from(command.map_id), command.instance_id)
        else {
            continue;
        };
        let map = managed.map_mut();
        if map.get_typed_creature(command.attacker_guid).is_none()
            || map.get_typed_creature(command.victim_guid).is_none()
        {
            continue;
        }
        if let Some(previous_victim) = command.previous_victim_guid {
            if let Some(player) = map.get_typed_player_mut(previous_victim) {
                player
                    .unit_mut()
                    .remove_attacker_like_cpp(command.attacker_guid);
            } else if let Some(creature) = map.get_typed_creature_mut(previous_victim) {
                creature
                    .unit_mut()
                    .remove_attacker_like_cpp(command.attacker_guid);
            }
        }
        let threat_ref = if let Some(attacker) = map.get_typed_creature_mut(command.attacker_guid) {
            let combat = &mut attacker.unit_mut().subsystems_mut().combat;
            combat.set_in_combat_with(command.victim_guid, false, false);
            combat.add_threat(command.victim_guid, 0.0);
            combat.threat_ref(command.victim_guid).copied()
        } else {
            None
        };
        if let Some(victim) = map.get_typed_creature_mut(command.victim_guid) {
            let combat = &mut victim.unit_mut().subsystems_mut().combat;
            combat.set_in_combat_with(command.attacker_guid, false, false);
            if let Some(threat_ref) = threat_ref {
                combat.put_threatened_by_me_ref(command.attacker_guid, threat_ref);
            }
            victim
                .unit_mut()
                .add_attacker_like_cpp(command.attacker_guid);
        }
        applied += 1;
    }
    applied
}

/// Apply the creature half of evade combat-stop directly to the canonical map.
///
/// C++ `Unit::CombatStop` removes every attacker and clears the corresponding
/// `CombatReference` from both participants. Player participants finish that
/// transition through their session command; creatures have no registry
/// recipient, so the map-owned bridge must purge the pair here.
pub(crate) fn apply_canonical_creature_attack_stops_like_cpp(
    commands: &[wow_network::player_registry::CreatureAttackStopLikeCppCommand],
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
) -> usize {
    let Some(manager) = canonical_map_manager else {
        return 0;
    };
    let Ok(mut manager) = manager.lock() else {
        return 0;
    };
    let mut applied = 0;
    for command in commands
        .iter()
        .filter(|command| command.victim_guid.is_creature())
    {
        let Some(managed) = manager.find_map_mut(u32::from(command.map_id), command.instance_id)
        else {
            continue;
        };
        let map = managed.map_mut();
        if map.get_typed_creature(command.attacker_guid).is_none()
            || map.get_typed_creature(command.victim_guid).is_none()
        {
            continue;
        }
        if let Some(attacker) = map.get_typed_creature_mut(command.attacker_guid) {
            attacker
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_combat_ref_like_cpp(command.victim_guid);
        }
        if let Some(victim) = map.get_typed_creature_mut(command.victim_guid) {
            victim
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_combat_ref_like_cpp(command.attacker_guid);
            victim
                .unit_mut()
                .remove_attacker_like_cpp(command.attacker_guid);
        }
        applied += 1;
    }
    applied
}

pub(crate) fn deliver_creature_attack_stop_commands_like_cpp(
    commands: &[wow_network::player_registry::CreatureAttackStopLikeCppCommand],
    registry: &wow_network::PlayerRegistry,
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
    commands: &[wow_network::player_registry::ApplyCreatureMeleeDamageLikeCppCommand],
    registry: &wow_network::PlayerRegistry,
) -> RuntimeCreatureMeleeDeliverySummaryLikeCpp {
    struct Candidate {
        registration: wow_network::PlayerRegistration,
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
/// [`PlayerRegistry`] and every DashMap guard is dropped before the legacy map
/// lock is taken — the pattern the aggro scan already uses.
pub(crate) fn collect_legacy_chase_target_snapshots_like_cpp(
    registry: &wow_network::PlayerRegistry,
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

/// Run one legacy global creature-movement tick and deliver its runtime plan.
///
/// Production reaches this through the map-owned
/// `RustyCore.LegacyCreatureGlobalRuntime` loop. The tick
/// body itself owns all map-lock ordering; delivery happens afterwards through
/// the already-tested `SendIfVisibleLikeCpp` rail.
pub(crate) fn run_legacy_creature_movement_tick_and_deliver_once_like_cpp(
    legacy_map_manager: &SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    mmap_config: &MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<&WorldMMapPathfinderWorkerLikeCpp>,
    diff_ms: u32,
    registry: &wow_network::PlayerRegistry,
) -> (
    wow_world::session::LegacyCreatureMovementTickOutcomeLikeCpp,
    RuntimeDeliverySummaryLikeCpp,
) {
    let chase_targets = collect_legacy_chase_target_snapshots_like_cpp(registry);
    let outcome = wow_world::session::run_legacy_creature_movement_tick_once_like_cpp(
        legacy_map_manager,
        canonical_map_manager,
        mmap_config,
        mmap_pathfinder,
        &chase_targets,
        diff_ms,
    );
    let delivery = deliver_runtime_plan_like_cpp(&outcome.plan, registry);
    (outcome, delivery)
}

/// Run one legacy global creature lifecycle tick and wake affected sessions.
///
/// Production reaches this through the map-owned
/// `RustyCore.LegacyCreatureGlobalRuntime` loop. The
/// lifecycle body mutates legacy/canonical map state and returns map keys whose
/// sessions need to recompute creature visibility; delivery is map-scoped
/// refresh commands via `try_send`.
pub(crate) fn run_legacy_creature_lifecycle_tick_and_refresh_once_like_cpp(
    legacy_map_manager: &SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    map_store: &wow_data::MapStore,
    now: std::time::Instant,
    registry: &wow_network::PlayerRegistry,
) -> (
    wow_world::session::LegacyCreatureLifecycleTickOutcomeLikeCpp,
    RuntimeVisibilityRefreshDeliverySummaryLikeCpp,
) {
    let outcome = wow_world::session::run_legacy_creature_lifecycle_tick_once_like_cpp(
        legacy_map_manager,
        canonical_map_manager,
        map_store,
        now,
    );
    let mut delivery = RuntimeVisibilityRefreshDeliverySummaryLikeCpp::default();
    for (map_id, instance_id) in &outcome.refresh_map_keys {
        delivery.merge(deliver_refresh_visible_world_creatures_like_cpp(
            *map_id,
            *instance_id,
            registry,
        ));
    }
    (outcome, delivery)
}

/// Run one legacy global creature aggro scan and deliver attack-start commands.
///
/// Production reaches this through the map-owned
/// `RustyCore.LegacyCreatureGlobalRuntime` loop. Candidate
/// player snapshots are collected before taking the legacy map lock; delivery
/// happens after the map-owned aggro result is computed.
pub(crate) fn run_legacy_creature_aggro_tick_and_deliver_once_like_cpp(
    legacy_map_manager: &SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    registry: &wow_network::PlayerRegistry,
    aggro_config: wow_world::session::LegacyCreatureAggroConfigLikeCpp,
) -> (
    wow_world::session::LegacyCreatureAggroTickOutcomeLikeCpp,
    RuntimeCreatureAttackStartDeliverySummaryLikeCpp,
    RuntimeDeliverySummaryLikeCpp,
) {
    let candidates = collect_legacy_creature_aggro_candidates_with_canonical_like_cpp(
        registry,
        canonical_map_manager,
    );
    let outcome = wow_world::session::run_legacy_creature_aggro_tick_once_with_config_like_cpp(
        legacy_map_manager,
        &candidates,
        aggro_config,
    );
    let _ =
        apply_canonical_creature_attack_starts_like_cpp(&outcome.commands, canonical_map_manager);
    let _ = apply_canonical_creature_attack_stops_like_cpp(
        &outcome.stop_commands,
        canonical_map_manager,
    );
    let mut delivery = deliver_creature_attack_start_commands_like_cpp(&outcome.commands, registry);
    let stop_delivery =
        deliver_creature_attack_stop_commands_like_cpp(&outcome.stop_commands, registry);
    delivery.commands_seen += stop_delivery.commands_seen;
    delivery.candidates_seen += stop_delivery.candidates_seen;
    delivery.candidates_queued += stop_delivery.candidates_queued;
    delivery.candidates_skipped_missing_victim += stop_delivery.candidates_skipped_missing_victim;
    delivery.candidates_skipped_wrong_map += stop_delivery.candidates_skipped_wrong_map;
    delivery.candidates_skipped_wrong_instance += stop_delivery.candidates_skipped_wrong_instance;
    delivery.candidates_skipped_not_in_world += stop_delivery.candidates_skipped_not_in_world;
    delivery.candidates_skipped_dead += stop_delivery.candidates_skipped_dead;
    delivery.send_failed += stop_delivery.send_failed;
    let plan_delivery = deliver_runtime_plan_like_cpp(&outcome.plan, registry);
    (outcome, delivery, plan_delivery)
}

/// Run one legacy global creature melee tick and deliver victim commands.
///
/// Production reaches this through the map-owned
/// `RustyCore.LegacyCreatureGlobalRuntime` loop. The tick
/// body mutates canonical victim health and returns final-health commands; this
/// bridge delivers them outside all map locks.
pub(crate) fn run_legacy_creature_melee_tick_and_deliver_once_like_cpp(
    legacy_map_manager: &SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    registry: &wow_network::PlayerRegistry,
) -> (
    wow_world::session::LegacyCreatureMeleeTickOutcomeLikeCpp,
    RuntimeCreatureMeleeDeliverySummaryLikeCpp,
    RuntimeDeliverySummaryLikeCpp,
) {
    let outcome = wow_world::session::run_legacy_creature_melee_tick_once_like_cpp(
        legacy_map_manager,
        canonical_map_manager,
    );
    let delivery = deliver_creature_melee_damage_commands_like_cpp(&outcome.commands, registry);
    let plan_delivery = deliver_runtime_plan_like_cpp(&outcome.plan, registry);
    (outcome, delivery, plan_delivery)
}

pub(crate) fn run_legacy_creature_spell_tick_and_deliver_once_like_cpp(
    legacy_map_manager: &SharedMapManager,
    canonical_map_manager: Option<&wow_world::session::SharedCanonicalMapManager>,
    registry: &wow_network::PlayerRegistry,
    config: &wow_world::session::LegacyCreatureAggroConfigLikeCpp,
) -> (
    wow_world::session::LegacyCreatureSpellTickOutcomeLikeCpp,
    RuntimeDeliverySummaryLikeCpp,
) {
    let outcome = wow_world::session::run_legacy_creature_spell_tick_once_like_cpp(
        legacy_map_manager,
        canonical_map_manager,
        config,
    );
    // START and GO are committed map events and enter every eligible
    // observer's durable FIFO in plan order.
    let plan_delivery = deliver_runtime_plan_like_cpp(&outcome.plan, registry);
    (outcome, plan_delivery)
}

/// Combined single-shot legacy creature runtime bridge.
///
/// This is the production loop body behind the
/// `RustyCore.LegacyCreatureGlobalRuntime` flag and the same body used by the
/// task-boundary tests. It mirrors the current legacy creature tick split while
/// proving that lifecycle refresh and movement fanout can run without holding
/// map locks during channel delivery.
#[derive(Debug)]
pub(crate) struct LegacyCreatureRuntimeTickBridgeOutcomeLikeCpp {
    pub lifecycle: wow_world::session::LegacyCreatureLifecycleTickOutcomeLikeCpp,
    pub lifecycle_delivery: RuntimeVisibilityRefreshDeliverySummaryLikeCpp,
    pub movement: wow_world::session::LegacyCreatureMovementTickOutcomeLikeCpp,
    pub movement_delivery: RuntimeDeliverySummaryLikeCpp,
    pub aggro: wow_world::session::LegacyCreatureAggroTickOutcomeLikeCpp,
    pub aggro_delivery: RuntimeCreatureAttackStartDeliverySummaryLikeCpp,
    pub aggro_plan_delivery: RuntimeDeliverySummaryLikeCpp,
    pub spell: wow_world::session::LegacyCreatureSpellTickOutcomeLikeCpp,
    pub spell_plan_delivery: RuntimeDeliverySummaryLikeCpp,
    pub melee: wow_world::session::LegacyCreatureMeleeTickOutcomeLikeCpp,
    pub melee_delivery: RuntimeCreatureMeleeDeliverySummaryLikeCpp,
    pub melee_plan_delivery: RuntimeDeliverySummaryLikeCpp,
}

pub(crate) fn run_legacy_creature_runtime_tick_and_deliver_once_like_cpp(
    legacy_map_manager: &SharedMapManager,
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    map_store: &wow_data::MapStore,
    mmap_config: &MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<&WorldMMapPathfinderWorkerLikeCpp>,
    aggro_config: wow_world::session::LegacyCreatureAggroConfigLikeCpp,
    diff_ms: u32,
    now: std::time::Instant,
    registry: &wow_network::PlayerRegistry,
    respawn_db_mutation_order: Option<&SharedRespawnDbMutationOrderLikeCpp>,
    respawn_db_writer_tx: Option<&RespawnDbWriterSenderLikeCpp>,
) -> LegacyCreatureRuntimeTickBridgeOutcomeLikeCpp {
    // Hold the same ordering gate as the canonical tick from before the
    // lifecycle mutation until every resulting statement is in the shared
    // mailbox. This makes DB operation order match runtime mutation order.
    let respawn_db_mutation_order_guard = respawn_db_mutation_order.map(|order| {
        order
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    });
    let (mut lifecycle, lifecycle_delivery) =
        run_legacy_creature_lifecycle_tick_and_refresh_once_like_cpp(
            legacy_map_manager,
            canonical_map_manager,
            map_store,
            now,
            registry,
        );
    if let Some(respawn_db_writer_tx) = respawn_db_writer_tx {
        for statement in lifecycle.respawn_db_statements.drain(..) {
            if respawn_db_writer_tx.send(statement).is_err() {
                tracing::error!(
                    "Shared respawn DB writer stopped before legacy respawn statement submission"
                );
            }
        }
    }
    drop(respawn_db_mutation_order_guard);

    let (movement, movement_delivery) = run_legacy_creature_movement_tick_and_deliver_once_like_cpp(
        legacy_map_manager,
        canonical_map_manager,
        mmap_config,
        mmap_pathfinder,
        diff_ms,
        registry,
    );
    let (aggro, aggro_delivery, aggro_plan_delivery) =
        run_legacy_creature_aggro_tick_and_deliver_once_like_cpp(
            legacy_map_manager,
            canonical_map_manager,
            registry,
            aggro_config.clone(),
        );
    let (spell, spell_plan_delivery) = run_legacy_creature_spell_tick_and_deliver_once_like_cpp(
        legacy_map_manager,
        canonical_map_manager,
        registry,
        &aggro_config,
    );
    let (melee, melee_delivery, melee_plan_delivery) =
        run_legacy_creature_melee_tick_and_deliver_once_like_cpp(
            legacy_map_manager,
            canonical_map_manager,
            registry,
        );

    LegacyCreatureRuntimeTickBridgeOutcomeLikeCpp {
        lifecycle,
        lifecycle_delivery,
        movement,
        movement_delivery,
        aggro,
        aggro_delivery,
        aggro_plan_delivery,
        spell,
        spell_plan_delivery,
        melee,
        melee_delivery,
        melee_plan_delivery,
    }
}

/// Spawn the legacy global creature runtime loop.
///
/// C++ contrast: `World::Update` calls `sMapMgr->Update(diff)` and
/// `MapManager::Update` uses `CONFIG_INTERVAL_MAPUPDATE` / `MapUpdateInterval`.
/// This Rust bridge uses the same configured interval and is enabled by default
/// so creature AI is map-owned like C++. Set
/// `RustyCore.LegacyCreatureGlobalRuntime = 0` only for local diagnostics.
///
/// The actual tick is executed via `spawn_blocking` because the legacy manager
/// uses `std::sync::RwLock` and movement may touch synchronous mmap/pathfinding
/// state. Packet fanout still happens outside map locks inside the single-shot
/// bridge.
pub(crate) fn spawn_legacy_creature_runtime_update_loop_like_cpp(
    enabled: bool,
    legacy_map_manager: SharedMapManager,
    canonical_map_manager: SharedCanonicalMapManager,
    map_store: Arc<wow_data::MapStore>,
    mmap_config: MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<Arc<WorldMMapPathfinderWorkerLikeCpp>>,
    aggro_config: wow_world::session::LegacyCreatureAggroConfigLikeCpp,
    tick_interval_ms: u32,
    respawn_db_writer_tx: Option<RespawnDbWriterSenderLikeCpp>,
    respawn_db_mutation_order: SharedRespawnDbMutationOrderLikeCpp,
    respawn_db_producer_stop: SharedRespawnDbProducerStopLikeCpp,
    player_registry: Arc<PlayerRegistry>,
) -> tokio::task::JoinHandle<()> {
    if !enabled {
        return tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(Duration::from_millis(u64::from(tick_interval_ms.max(1))));
            loop {
                interval.tick().await;
                if respawn_db_producer_stop.load(Ordering::Acquire) {
                    break;
                }
            }
        });
    }

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_millis(u64::from(tick_interval_ms.max(1))));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        // Track real wall-clock elapsed between ticks. C++ `World::Update(diff)`
        // / `MapManager::Update(diff)` feed the measured frame diff to
        // `MotionMaster::Update` so `_timer.Update(diff)` advances at real time.
        // The canonical map loop already does this; the legacy creature loop
        // previously passed the constant `tick_interval_ms`, which let the
        // generator timer drift behind the spline's real-clock finalization and
        // delayed wander re-arming in proportion to scheduler lag. Mirror the
        // canonical loop here.
        let mut last_tick = Instant::now();

        loop {
            interval.tick().await;
            let stop_after_tick = respawn_db_producer_stop.load(Ordering::Acquire);
            let now = Instant::now();
            let diff_ms = now
                .duration_since(last_tick)
                .as_millis()
                .min(u128::from(u32::MAX)) as u32;
            last_tick = now;
            if diff_ms == 0 {
                continue;
            }
            let legacy_for_tick = Arc::clone(&legacy_map_manager);
            let canonical_for_tick = Arc::clone(&canonical_map_manager);
            let map_store_for_tick = Arc::clone(&map_store);
            let mmap_config_for_tick = mmap_config.clone();
            let mmap_pathfinder_for_tick = mmap_pathfinder.clone();
            let aggro_config_for_tick = aggro_config.clone();
            let registry_for_tick = Arc::clone(&player_registry);
            let respawn_db_mutation_order_for_tick = Arc::clone(&respawn_db_mutation_order);
            let respawn_db_writer_tx_for_tick = respawn_db_writer_tx.clone();

            let tick_result = tokio::task::spawn_blocking(move || {
                run_legacy_creature_runtime_tick_and_deliver_once_like_cpp(
                    &legacy_for_tick,
                    Some(&canonical_for_tick),
                    map_store_for_tick.as_ref(),
                    &mmap_config_for_tick,
                    mmap_pathfinder_for_tick.as_deref(),
                    aggro_config_for_tick,
                    diff_ms,
                    now,
                    registry_for_tick.as_ref(),
                    Some(&respawn_db_mutation_order_for_tick),
                    respawn_db_writer_tx_for_tick.as_ref(),
                )
            })
            .await;

            let Ok(outcome) = tick_result else {
                tracing::error!("Legacy global creature runtime tick task panicked; stopping loop");
                break;
            };

            let touched_creatures = outcome.lifecycle.corpses_despawned
                + outcome.movement.movement_packets
                + outcome.aggro.aggro_starts
                + outcome.spell.casts_ready
                + outcome.melee.canonical_hits;
            if touched_creatures > 0 {
                debug!(
                    lifecycle_corpses_despawned = outcome.lifecycle.corpses_despawned,
                    lifecycle_respawns_processed = outcome.lifecycle.respawns_processed,
                    lifecycle_refresh_commands = outcome.lifecycle_delivery.candidates_queued,
                    movement_packets = outcome.movement.movement_packets,
                    movement_commands = outcome.movement_delivery.candidates_queued,
                    movement_seen = outcome.movement_delivery.candidates_seen,
                    movement_skipped_not_in_world =
                        outcome.movement_delivery.candidates_skipped_not_in_world,
                    movement_skipped_wrong_map =
                        outcome.movement_delivery.candidates_skipped_wrong_map,
                    movement_skipped_wrong_instance =
                        outcome.movement_delivery.candidates_skipped_wrong_instance,
                    movement_skipped_distance =
                        outcome.movement_delivery.candidates_skipped_distance,
                    movement_send_failed = outcome.movement_delivery.send_failed,
                    aggro_starts = outcome.aggro.aggro_starts,
                    aggro_commands = outcome.aggro_delivery.candidates_queued,
                    aggro_alerts = outcome.aggro.alert_triggers,
                    aggro_movement_interrupts = outcome.aggro.movement_interrupts,
                    aggro_plan_commands = outcome.aggro_plan_delivery.candidates_queued,
                    creature_spell_casts = outcome.spell.casts_ready,
                    creature_spell_plan_events = outcome.spell_plan_delivery.events_seen,
                    creature_spell_plan_commands = outcome.spell_plan_delivery.candidates_queued,
                    creature_spell_plan_skipped_not_in_world =
                        outcome.spell_plan_delivery.candidates_skipped_not_in_world,
                    creature_spell_plan_skipped_wrong_map =
                        outcome.spell_plan_delivery.candidates_skipped_wrong_map,
                    creature_spell_plan_skipped_wrong_instance = outcome
                        .spell_plan_delivery
                        .candidates_skipped_wrong_instance,
                    creature_spell_plan_skipped_distance =
                        outcome.spell_plan_delivery.candidates_skipped_distance,
                    creature_spell_plan_send_failed = outcome.spell_plan_delivery.send_failed,
                    creature_spell_noninstant_unrepresented =
                        outcome.spell.noninstant_casts_unrepresented,
                    creature_spell_effects_unrepresented =
                        outcome.spell.spell_effects_unrepresented,
                    creature_spell_projectiles_unrepresented =
                        outcome.spell.spell_projectiles_unrepresented,
                    creature_spell_visuals_unrepresented =
                        outcome.spell.spell_visuals_unrepresented,
                    creature_spell_range_rejections = outcome.spell.spell_range_rejections,
                    creature_spell_los_rejections = outcome.spell.spell_los_rejections,
                    melee_compatibility_hits = outcome.melee.canonical_hits,
                    melee_outcomes_unrepresented = outcome.melee.melee_outcomes_unrepresented,
                    melee_attacker_incarnation_rejections =
                        outcome.melee.attacker_incarnation_rejections,
                    melee_creature_victim_sync_cas_rejections =
                        outcome.melee.legacy_creature_victim_sync_cas_rejections,
                    melee_commands = outcome.melee_delivery.candidates_queued,
                    melee_plan_commands = outcome.melee_plan_delivery.candidates_queued,
                    "Legacy global creature runtime tick produced visible work"
                );
            }

            if stop_after_tick {
                break;
            }
        }
    })
}
