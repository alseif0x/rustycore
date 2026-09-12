//! Movement handlers regression scenarios.
//!
//! Separated from the movement.rs root under #654.

use super::*;
use crate::session::{
    AuraApplication, MMapRuntimeConfigLikeCpp, MoveSplineDoneTaxiActionLikeCpp,
    MoveTeleportAckActionLikeCpp, MovementSpeedAckActionLikeCpp, PlayerGridLoadOutcomeLikeCpp,
    RepresentedAuraEffectLikeCpp, RepresentedTaxiFlightNodeLikeCpp, SessionPlayerController,
    UnitMoveTypeLikeCpp,
};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use wow_constants::ServerOpcodes;
use wow_constants::movement::MovementFlag;
use wow_constants::unit::UnitFlags;
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_packet::packets::movement::TransportInfo;

fn make_session() -> WorldSession {
    make_session_with_send_rx().0
}

fn make_session_with_send_rx() -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded(8);
    let (send_tx, send_rx) = flume::bounded(8);
    let session = WorldSession::new(
        1,
        "MovementTest".into(),
        0,
        2,
        9,
        54261,
        vec![0; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    (session, send_rx)
}

fn movement_packet(opcode: ClientOpcodes, movement: &MovementInfo) -> wow_packet::WorldPacket {
    let mut inbound = wow_packet::WorldPacket::new_empty();
    inbound.write_uint16(opcode as u16);
    movement.write(&mut inbound);
    inbound.read_uint16().expect("movement opcode");
    inbound
}

fn unique_temp_data_dir(test_name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "rustycore-movement-{test_name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    fs::create_dir_all(dir.join("maps")).expect("create maps dir");
    dir
}

fn write_single_area_map_tile_like_cpp(
    data_dir: &std::path::Path,
    map_id: u32,
    x: f32,
    y: f32,
    area_id: u16,
) {
    const MAP_FILE_HEADER_SIZE_LIKE_CPP: usize = 44;
    const MAP_AREA_HEADER_SIZE_LIKE_CPP: usize = 8;
    const MAP_AREA_CELLS_PER_GRID_LIKE_CPP: usize = 16;

    let area_offset = MAP_FILE_HEADER_SIZE_LIKE_CPP as u32;
    let area_size = (MAP_AREA_HEADER_SIZE_LIKE_CPP
        + MAP_AREA_CELLS_PER_GRID_LIKE_CPP
            * MAP_AREA_CELLS_PER_GRID_LIKE_CPP
            * std::mem::size_of::<u16>()) as u32;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"MAPS");
    bytes.extend_from_slice(&10_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&area_offset.to_le_bytes());
    bytes.extend_from_slice(&area_size.to_le_bytes());
    for _ in 0..6 {
        bytes.extend_from_slice(&0_u32.to_le_bytes());
    }
    assert_eq!(bytes.len(), MAP_FILE_HEADER_SIZE_LIKE_CPP);
    bytes.extend_from_slice(b"AREA");
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&area_id.to_le_bytes());
    for _ in 0..(MAP_AREA_CELLS_PER_GRID_LIKE_CPP * MAP_AREA_CELLS_PER_GRID_LIKE_CPP) {
        bytes.extend_from_slice(&area_id.to_le_bytes());
    }

    let (gx, gy) = crate::map_manager::terrain_grid_coords_for_wow_position_like_cpp(x, y);
    fs::write(
        data_dir
            .join("maps")
            .join(format!("{map_id:04}_{gx:02}_{gy:02}.map")),
        bytes,
    )
    .expect("write movement area map");
}

fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        if let Some(opcode) = wow_packet::WorldPacket::from_bytes(&bytes).server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}

fn visible_aura(slot: u8, flags: u32, flags2: u32) -> AuraApplication {
    AuraApplication {
        spell_id: 1000 + i32::from(slot),
        difficulty_id: 0,
        caster_guid: ObjectGuid::EMPTY,
        slot,
        duration_total: 30_000,
        duration_remaining: 30_000,
        stack_count: 1,
        aura_flags: 0x1,
        effect_mask: 0x1,
        aura_interrupt_flags: flags,
        aura_interrupt_flags2: flags2,
        represented_effect: None,
        represented_amount: 0,
        represented_effect_amounts: Vec::new(),
        represented_misc_value: None,
        represented_multiplier: 1.0,
        applied_at: std::time::Instant::now(),
    }
}

fn fall_aura(
    slot: u8,
    effect: RepresentedAuraEffectLikeCpp,
    amount: i32,
    multiplier: f32,
) -> AuraApplication {
    AuraApplication {
        represented_effect: Some(effect),
        represented_amount: amount,
        represented_multiplier: multiplier,
        ..visible_aura(slot, 0, 0)
    }
}

fn broadcast_info(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
) -> crate::session::directory::PlayerSessionRegistrationLikeCpp {
    let (command_tx, _command_rx) = flume::bounded(1);
    broadcast_info_with_command(guid, send_tx, command_tx)
}

fn broadcast_info_with_command(
    guid: ObjectGuid,
    send_tx: flume::Sender<Vec<u8>>,
    command_tx: flume::Sender<crate::session::mailbox::SessionCommand>,
) -> crate::session::directory::PlayerSessionRegistrationLikeCpp {
    crate::session::directory::PlayerSessionRegistrationLikeCpp {
        identity: crate::session::directory::PlayerDirectoryIdentityLikeCpp {
            player_name: format!("Player{}", guid.counter()),
            account_id: guid.counter() as u32,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            active_expansion: 2,
        },
        placement: crate::session::directory::PlayerDirectoryPlacementLikeCpp {
            map_id: 0,
            instance_id: 0,
            position: wow_core::Position::ZERO,
            is_in_world: true,
            level: 1,
            is_alive: true,
        },
        active_loot_rolls: Vec::new(),
        realm_send_tx: send_tx.clone(),
        send_tx,
        command_tx,
        session_phase_tx: crate::session::directory::detached_session_phase_rail_like_cpp(),
        durable_creature_runtime_commands_like_cpp: Default::default(),
        client_visible_guids_like_cpp: Default::default(),
        advanced_combat_logging_enabled_like_cpp: Default::default(),
        visibility_refresh_pending_like_cpp: Default::default(),
    }
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
