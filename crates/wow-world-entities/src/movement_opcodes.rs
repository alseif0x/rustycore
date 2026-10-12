// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Movement packet handlers — CMSG_MOVE_* (#1363 slice 4b; moved from the
//! World `handlers/movement/ops_1.rs`).
//!
//! All movement opcodes map to the same handler logic:
//!   1. Parse MovementInfo from the packet
//!   2. Sanitize movement flags like `Player::ValidateMovementInfo`
//!   3. Validate: GUID must match `Player::GetUnitBeingMoved()`, position must be finite
//!   4. Update server-side mover position when represented
//!   5. Broadcast SMSG_MOVE_UPDATE to nearby visible sessions
//!
//! Reference: C++ `WorldSession::HandleMovementOpcode`. The registrations stay
//! in the application `ApplicationMovement` area registrar.

use tracing::{info, trace, warn};
use wow_constants::ClientOpcodes;
use wow_constants::movement::MovementFlag;
use wow_constants::unit::UnitStandStateType;
use wow_packet::ServerPacket;
use wow_packet::packets::movement::{
    ClientPlayerMovement, MoveSplineDone, MoveUpdate, MovementAckMessage, MovementInfo,
};
use wow_world_core::session::map_admission::PlayerGridLoadResolverLikeCpp;
use wow_world_core::session::{MovementTransportMembershipLikeCpp, ProgressionCatalogsLikeCpp};
use wow_world_core::terrain::zone_and_area_for_position_like_cpp;

use crate::CreatureSpawnCatalogsLikeCpp;
use crate::movement_acks::{
    handle_fall_like_cpp, handle_move_spline_done_taxi_like_cpp,
    record_validated_movement_ack_like_cpp, sanitize_movement_info_represented_like_cpp,
};
use crate::movement_host::PlayerMovementHostLikeCpp;

pub const SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP: u32 = 0x0000_0010;
pub const SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP: u32 = 0x0200_0000;
pub const SPELL_AURA_INTERRUPT_FLAG2_JUMP_LIKE_CPP: u32 = 0x0000_0020;

/// Handle any CMSG_MOVE_* packet.
///
/// Parses MovementInfo, validates it, updates player position,
/// and queues a broadcast to nearby players.
pub async fn handle_movement_with_catalogs_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    area_trigger_catalogs: &H::AreaTriggerCatalogsLikeCpp,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    progression: &ProgressionCatalogsLikeCpp,
    player_grid_loader: &PlayerGridLoadResolverLikeCpp,
    mut pkt: wow_packet::WorldPacket,
) {
    let opcode = pkt.client_opcode();
    let info = match ClientPlayerMovement::read(&mut pkt) {
        Ok(m) => m,
        Err(e) => {
            warn!(
                account = host.core().account_id,
                "Failed to parse movement packet: {e}"
            );
            return;
        }
    };

    handle_movement_info_with_catalogs_like_cpp(
        host,
        area_trigger_catalogs,
        creature_spawn_catalogs,
        progression,
        player_grid_loader,
        opcode,
        info.info,
    )
    .await;
}

pub async fn handle_movement_info_with_catalogs_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    area_trigger_catalogs: &H::AreaTriggerCatalogsLikeCpp,
    creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    progression: &ProgressionCatalogsLikeCpp,
    player_grid_loader: &PlayerGridLoadResolverLikeCpp,
    opcode: Option<ClientOpcodes>,
    mut info: MovementInfo,
) {
    let Some(player_guid) = host.player_guid() else {
        warn!(
            account = host.core().account_id,
            "Movement packet received without loaded player"
        );
        return;
    };
    let Some(mover_guid) = host.hub_ref().player_moved_unit_guid_like_cpp() else {
        warn!(
            account = host.core().account_id,
            "Movement packet received without active mover"
        );
        return;
    };
    let mover_is_player = mover_guid == player_guid;
    if std::env::var_os("RUSTYCORE_LOGIN_TRACE").is_some() {
        info!(
            account = host.core().account_id,
            ?opcode,
            mover = ?info.guid,
            expected_mover = ?mover_guid,
            player = ?player_guid,
            flags = ?info.flags,
            client_time = info.time,
            x = info.position.x,
            y = info.position.y,
            z = info.position.z,
            o = info.position.orientation,
            has_transport = info.transport.is_some(),
            "RUST_LOGIN_TRACE movement_received"
        );
    }

    // MovementHandler.cpp returns before validation while the active
    // Player is being teleported. The ACK/world-port handlers own the
    // pending transition, so movement packets must not clear emotes,
    // mutate position, or publish stale state in this window.
    if mover_is_player
        && (host.hub_ref().near_teleport_pending_like_cpp()
            || host.hub_ref().represented_far_teleport_pending_like_cpp())
    {
        trace!(
            account = host.core().account_id,
            ?mover_guid,
            "Ignoring movement while Player teleport is pending"
        );
        return;
    }

    // C++ calls Player::ValidateMovementInfo before rejecting mismatched
    // GUIDs or invalid positions, then broadcasts only the sanitized state.
    let movement_validation = sanitize_movement_info_represented_like_cpp(host, &mut info);
    if !movement_validation.removed_flags.is_empty() {
        for rule in movement_validation
            .stripped_rules
            .iter()
            .copied()
            .filter(|rule| rule.removes_flags_like_cpp())
        {
            host.hub_ref().trace_anticheat_violation_like_cpp(
                rule.trace_rule_name_like_cpp(),
                opcode,
                "strip",
            );
        }
        trace!(
            account = host.core().account_id,
            removed = ?movement_validation.removed_flags,
            rules = ?movement_validation.stripped_rules,
            "MovementInfo flags sanitized before position update and broadcast"
        );
    }

    if info.guid != mover_guid {
        host.hub_ref().trace_anticheat_violation_like_cpp(
            "HandleMovementOpcode.GuidMismatch",
            opcode,
            "reject",
        );
        warn!(
            account = host.core().account_id,
            "Movement GUID mismatch: expected {:?}, got {:?}", mover_guid, info.guid
        );
        return;
    }

    let pos = info.position;
    if !pos.is_valid_map_coord_like_cpp() {
        host.hub_ref().trace_anticheat_violation_like_cpp(
            "HandleMovementOpcode.InvalidPosition",
            opcode,
            "reject",
        );
        warn!(
            account = host.core().account_id,
            "Invalid movement position: {pos:?}"
        );
        return;
    }

    // C++ `HandleMovementOpcode` rejects movement generated before the
    // mover's spline has finalized. Keep this after GUID/position checks,
    // matching the source ordering and preserving sanitized admission.
    if !host
        .hub_ref()
        .mover_spline_finalized_like_cpp(mover_guid)
        .unwrap_or(false)
    {
        trace!(
            account = host.core().account_id,
            ?mover_guid,
            "Ignoring movement while mover MoveSpline is not finalized"
        );
        return;
    }

    if mover_is_player {
        host.hub_mut()
            .clear_player_emote_state_on_player_movement_like_cpp();
    }

    let current_mover_position = host.hub_ref().mover_position_like_cpp(mover_guid);
    let new_player_cell_like_cpp = mover_is_player.then(|| wow_map::cell_from_world(pos.x, pos.y));
    let old_player_cell_like_cpp = current_mover_position
        .filter(|_| mover_is_player)
        .map(|current| wow_map::cell_from_world(current.x, current.y));
    let load_player_active_grid_like_cpp = match (
        old_player_cell_like_cpp.as_ref(),
        new_player_cell_like_cpp.as_ref(),
    ) {
        (Some(old_cell), Some(new_cell)) => old_cell.diff_grid(new_cell),
        (None, Some(_)) => true,
        _ => false,
    };
    if let Some(transport) = &info.transport {
        if current_mover_position.is_some_and(|current| {
            pos.distance_2d(&current) > wow_core::Position::GRID_SIZE_LIKE_CPP
        }) {
            trace!(
                account = host.core().account_id,
                "Ignoring stale transport movement after large position delta"
            );
            return;
        }

        if transport.x.abs() > 75.0 || transport.y.abs() > 75.0 || transport.z.abs() > 75.0 {
            trace!(
                account = host.core().account_id,
                "Ignoring movement with invalid transport offset"
            );
            return;
        }

        if !wow_core::Position::new(
            pos.x + transport.x,
            pos.y + transport.y,
            pos.z + transport.z,
            pos.orientation + transport.o,
        )
        .is_valid_map_coord_like_cpp()
        {
            trace!(
                account = host.core().account_id,
                "Ignoring movement with invalid world transport coordinate"
            );
            return;
        }
    }

    if mover_is_player {
        let requested_transport_guid = info.transport.as_ref().map(|transport| transport.guid);
        let transport_membership = host
            .hub_ref()
            .reconcile_player_transport_membership_like_cpp(mover_guid, requested_transport_guid);
        if !matches!(
            transport_membership,
            MovementTransportMembershipLikeCpp::Attached(_)
        ) {
            // C++ calls MovementInfo::ResetTransport when no transport or
            // vehicle owns the mover after the add/remove attempt.
            info.transport = None;
        }
        host.hub_mut()
            .set_player_transport_info_like_cpp(info.transport.clone());
        // C++ handles fall/parachute/pet effects before assigning
        // `m_movementInfo`, but stand/fall/under-map/jump effects after
        // the vehicle early-return. Keep that phase boundary explicit.
        if host
            .hub_ref()
            .represented_current_vehicle_seat_allows_turning_like_cpp()
        {
            apply_movement_pre_position_side_effects_like_cpp(host, opcode, &info);
        } else {
            apply_movement_side_effects_like_cpp(host, opcode, &info);
        }
    } else if matches!(
        opcode,
        Some(ClientOpcodes::MoveSetFly) | Some(ClientOpcodes::MoveSetAdvFly)
    ) {
        // C++ removes the temporary pet from the active player, not from the
        // moved unit, even when a controlled unit is the mover.
        host.hub_mut().request_temporary_pet_unsummon_like_cpp();
    }
    info.guid = mover_guid;
    info.time = host
        .hub_ref()
        .adjust_client_movement_time_like_cpp(info.time);
    let adjusted_time = info.time;

    if mover_is_player {
        host.hub_mut().set_player_movement_time_like_cpp(info.time);
        host.hub_mut()
            .set_player_movement_flags_like_cpp(info.flags);
        host.hub_mut()
            .set_player_movement_jump_like_cpp(info.jump.clone());

        // TrinityCore returns before `UpdatePosition` and publication for
        // a passenger whose seat permits independent turning. Only the
        // facing changes; the vehicle remains authoritative for x/y/z.
        if host
            .hub_ref()
            .represented_current_vehicle_seat_allows_turning_like_cpp()
        {
            if host
                .hub_ref()
                .player_position_like_cpp()
                .is_some_and(|current| current.orientation != info.position.orientation)
            {
                host.hub_mut()
                    .set_player_orientation_like_cpp(info.position.orientation);
                host.remove_auras_with_interrupt_flags_like_cpp(
                    SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP,
                    0,
                );
            }
            return;
        }

        // Update server-side player position.
        host.hub_mut().set_player_position_like_cpp(info.position);
        let authoritative_grid_map_key = host
            .core()
            .current_canonical_player_map_key_like_cpp()
            .filter(|key| key.map_id == u32::from(host.core().player_map_id_like_cpp()));
        let grid_instance_id = authoritative_grid_map_key
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if load_player_active_grid_like_cpp {
            let outcome = player_grid_loader(
                host.core().player_map_id_like_cpp(),
                authoritative_grid_map_key.map(|key| key.instance_id),
                pos,
            );
            trace!(
                account = host.core().account_id,
                map_id = host.core().player_map_id_like_cpp(),
                instance_id = grid_instance_id,
                old_grid_x = old_player_cell_like_cpp.as_ref().map(|cell| cell.grid_x()),
                old_grid_y = old_player_cell_like_cpp.as_ref().map(|cell| cell.grid_y()),
                new_grid_x = new_player_cell_like_cpp.as_ref().map(|cell| cell.grid_x()),
                new_grid_y = new_player_cell_like_cpp.as_ref().map(|cell| cell.grid_y()),
                map_unavailable = outcome.map_unavailable,
                grid_loaded_now = outcome.grid_loaded_now,
                creature_records_added = outcome.creature_records_added,
                gameobject_records_added = outcome.gameobject_records_added,
                area_trigger_records_added = outcome.area_trigger_records_added,
                "C++ Map::PlayerRelocation active grid loaded before player visibility"
            );
            if (std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some()
                || std::env::var_os("RUSTYCORE_CREATURE_VIS_TRACE").is_some())
                && (outcome.map_unavailable
                    || outcome.map_created
                    || outcome.grid_loaded_now
                    || outcome.metadata_entries != 0
                    || outcome.skipped_already_loaded != 0
                    || outcome.skipped_should_not_spawn != 0
                    || outcome.skipped_difficulty_mismatch != 0
                    || outcome.stale_index_entries != 0
                    || outcome.creature_records_added != 0
                    || outcome.gameobject_records_added != 0
                    || outcome.area_trigger_records_added != 0
                    || outcome.pre_add_records_added != 0
                    || outcome.add_to_map_errors != 0
                    || outcome.load_record_missing != 0)
            {
                info!(
                    account = host.core().account_id,
                    map_id = host.core().player_map_id_like_cpp(),
                    instance_id = grid_instance_id,
                    x = pos.x,
                    y = pos.y,
                    z = pos.z,
                    old_grid_x = old_player_cell_like_cpp.as_ref().map(|cell| cell.grid_x()),
                    old_grid_y = old_player_cell_like_cpp.as_ref().map(|cell| cell.grid_y()),
                    new_grid_x = new_player_cell_like_cpp.as_ref().map(|cell| cell.grid_x()),
                    new_grid_y = new_player_cell_like_cpp.as_ref().map(|cell| cell.grid_y()),
                    map_unavailable = outcome.map_unavailable,
                    map_created = outcome.map_created,
                    grid_loaded_now = outcome.grid_loaded_now,
                    metadata_entries = outcome.metadata_entries,
                    skipped_already_loaded = outcome.skipped_already_loaded,
                    skipped_should_not_spawn = outcome.skipped_should_not_spawn,
                    skipped_difficulty_mismatch = outcome.skipped_difficulty_mismatch,
                    stale_index_entries = outcome.stale_index_entries,
                    creature_records_added = outcome.creature_records_added,
                    gameobject_records_added = outcome.gameobject_records_added,
                    area_trigger_records_added = outcome.area_trigger_records_added,
                    pre_add_records_added = outcome.pre_add_records_added,
                    add_to_map_errors = outcome.add_to_map_errors,
                    load_record_missing = outcome.load_record_missing,
                    creature_load_record_missing = outcome.creature_load_record_missing,
                    gameobject_load_record_missing = outcome.gameobject_load_record_missing,
                    area_trigger_load_record_missing = outcome.area_trigger_load_record_missing,
                    "RUST_CREATURE_VIS movement_grid_load"
                );
            }
        }
        let area_id = match zone_and_area_for_position_like_cpp(
            &host
                .hub_ref()
                .config
                .mmap_runtime_config_like_cpp()
                .data_dir,
            u32::from(host.core().player_map_id_like_cpp()),
            info.position.x,
            info.position.y,
            host.hub_ref()
                .catalogs
                .area_table_store()
                .map(|store| store.as_ref()),
            |map_id| {
                host.hub_ref()
                    .catalogs
                    .map_store()
                    .as_deref()
                    .map(|store| u32::from(store.area_table_id_like_cpp(map_id)))
                    .unwrap_or(0)
            },
        ) {
            Ok((zone_id, area_id)) => {
                if area_id != 0 {
                    host.update_zone_represented_like_cpp(zone_id, area_id);
                    area_id
                } else {
                    let Some((_, current_area_id)) = host.hub_ref().player_zone_area_like_cpp()
                    else {
                        return;
                    };
                    current_area_id
                }
            }
            Err(error) => {
                let Some((_, area_id)) = host.hub_ref().player_zone_area_like_cpp() else {
                    return;
                };
                warn!(
                    account = host.core().account_id,
                    map_id = host.core().player_map_id_like_cpp(),
                    x = info.position.x,
                    y = info.position.y,
                    %error,
                    "failed to resolve C++ terrain zone/area after movement; keeping existing zone/area"
                );
                area_id
            }
        };
        host.check_area_explore_and_outdoor_represented_with_catalogs_like_cpp(
            progression,
            area_id,
        )
        .await;
        // Keep the broadcast registry in sync so chat range checks are accurate.
        host.hub_ref().update_registry_position();
        trace!(
            account = host.core().account_id,
            x = pos.x,
            y = pos.y,
            z = pos.z,
            "Player moved"
        );

        // Dynamic visibility update: send new creatures/GOs that came into
        // range and remove those that left. Internally throttled to 50 yards.
        host.update_visibility_with_catalogs_like_cpp(creature_spawn_catalogs)
            .await;

        // Check area triggers at the new position
        host.check_area_triggers_with_catalogs_like_cpp(area_trigger_catalogs)
            .await;
    } else {
        let moved = host
            .core_mut()
            .mutate_world_creature(mover_guid, |creature| {
                creature.set_ai_position(info.position);
                creature.unit_mut().set_movement_time_like_cpp(info.time);
                creature.unit_mut().set_movement_flags_like_cpp(info.flags);
                creature.set_movement_flags_runtime_like_cpp(info.flags);
            })
            .is_some();
        trace!(
            account = host.core().account_id,
            mover = ?mover_guid,
            x = pos.x,
            y = pos.y,
            z = pos.z,
            represented = moved,
            "Controlled mover moved"
        );
    }

    // Aggro proximity (`CreatureAI::MoveInLineOfSight`) is the canonical
    // map's creature aggro phase, not a session check.

    // C++ `mover->SendMessageToSet(moveUpdate.Write(), _player)` uses
    // the mover visibility range and skips this player's session.
    // Candidate routing is cheap here; the receiver session applies the
    // final HaveAtClient gate through `SendIfVisibleLikeCpp`.
    if let Some(registry) = host.core().player_registry() {
        let move_update = MoveUpdate { info };
        let packet_bytes = move_update.to_bytes();
        let map_id = host.core().player_map_id_like_cpp();
        let instance_id = host
            .core()
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        for registration in registry.movement_recipients_within_range(
            player_guid,
            map_id,
            instance_id,
            pos,
            wow_entities::VISIBILITY_RADIUS_LIKE_CPP,
        ) {
            let _ = registry.try_send_current_command(
                registration,
                wow_world_core::session::mailbox::SessionCommand::SendIfVisibleLikeCpp(
                    wow_world_core::session::mailbox::SendIfVisibleLikeCppCommand {
                        queued_at: std::time::Instant::now(),
                        source_guid: mover_guid,
                        map_id,
                        instance_id,
                        packet_bytes: packet_bytes.clone(),
                    },
                ),
            );
        }
    }
    if std::env::var_os("RUSTYCORE_LOGIN_TRACE").is_some() {
        info!(
            account = host.core().account_id,
            ?opcode,
            adjusted_time,
            x = pos.x,
            y = pos.y,
            z = pos.z,
            "RUST_LOGIN_TRACE movement_applied"
        );
    }
}

pub fn apply_movement_side_effects_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    opcode: Option<ClientOpcodes>,
    info: &MovementInfo,
) {
    host.hub_mut()
        .clear_player_emote_state_on_player_movement_like_cpp();

    if matches!(opcode, Some(ClientOpcodes::MoveFallLand)) {
        handle_fall_like_cpp(host, info);
    }

    match opcode {
        Some(ClientOpcodes::MoveFallLand)
        | Some(ClientOpcodes::MoveStartSwim)
        | Some(ClientOpcodes::MoveSetFly) => {
            host.remove_auras_with_interrupt_flags_like_cpp(
                SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP,
                0,
            );
        }
        _ => {}
    }

    if matches!(
        opcode,
        Some(ClientOpcodes::MoveSetFly) | Some(ClientOpcodes::MoveSetAdvFly)
    ) {
        host.hub_mut().request_temporary_pet_unsummon_like_cpp();
    }

    if host.hub_ref().player_is_sit_state_like_cpp()
        && info
            .flags
            .intersects(MovementFlag::MASK_MOVING | MovementFlag::MASK_TURNING)
    {
        host.hub_mut()
            .set_player_stand_state_like_cpp(UnitStandStateType::Stand);
    }

    if matches!(opcode, Some(ClientOpcodes::MoveJump)) {
        host.remove_auras_with_interrupt_flags_like_cpp(
            0,
            SPELL_AURA_INTERRUPT_FLAG2_JUMP_LIKE_CPP,
        );
        host.request_jump_proc_like_cpp();
    }

    host.hub_mut().update_fall_information_if_needed_like_cpp(
        info,
        matches!(opcode, Some(ClientOpcodes::MoveFallLand)),
    );
    host.handle_under_map_like_cpp(info);
}

pub(crate) fn apply_movement_pre_position_side_effects_like_cpp<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    opcode: Option<ClientOpcodes>,
    info: &MovementInfo,
) {
    if matches!(opcode, Some(ClientOpcodes::MoveFallLand)) {
        handle_fall_like_cpp(host, info);
    }

    match opcode {
        Some(ClientOpcodes::MoveFallLand)
        | Some(ClientOpcodes::MoveStartSwim)
        | Some(ClientOpcodes::MoveSetFly) => {
            host.remove_auras_with_interrupt_flags_like_cpp(
                SPELL_AURA_INTERRUPT_FLAG_LANDING_OR_FLIGHT_LIKE_CPP,
                0,
            );
        }
        _ => {}
    }

    if matches!(
        opcode,
        Some(ClientOpcodes::MoveSetFly) | Some(ClientOpcodes::MoveSetAdvFly)
    ) {
        host.hub_mut().request_temporary_pet_unsummon_like_cpp();
    }
}

/// Handle C++ `HandleMovementAckMessage` opcodes.
pub async fn handle_movement_ack_message<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    opcode: ClientOpcodes,
    mut pkt: MovementAckMessage,
) {
    trace!(
        account = host.core().account_id,
        ?opcode,
        "MovementAckMessage"
    );
    record_validated_movement_ack_like_cpp(host, opcode, &mut pkt.ack, None);
}

/// Handle C++ `HandleMoveSplineDoneOpcode` bookkeeping until taxi runtime is complete.
pub async fn handle_move_spline_done<H: PlayerMovementHostLikeCpp>(
    host: &mut H,
    mut pkt: MoveSplineDone,
) {
    trace!(
        account = host.core().account_id,
        spline_id = pkt.spline_id,
        "MoveSplineDone"
    );
    handle_move_spline_done_taxi_like_cpp(host, &mut pkt.status, pkt.spline_id);
}
