//! Original creature-melee APP contracts: catalog, runtime, packet and CAS boundaries.
use rand::{Rng, SeedableRng, rngs::StdRng};
use std::sync::Arc;
use wow_constants::{UnitFlags, UnitState};
use wow_core::{ObjectGuid, Position, guid::HighGuid};
use wow_packet::ServerPacket;
use wow_packet::WorldPacket;
use wow_packet::opcodes::ServerOpcodes;
use wow_world::player_directory::PlayerRegistry;
use wow_world::session::creature_melee_fixtures::CreatureMeleePlayerController as SessionPlayerController;
use wow_world::session::creature_melee_fixtures::*;
use wow_world::session::mailbox::{ApplyCreatureMeleeDamageLikeCppCommand, SessionCommand};
use wow_world::session::{
    SessionState, SharedCanonicalMapManager, WorldSession,
    run_legacy_creature_melee_tick_once_like_cpp,
};

mod support;
use support::*;

fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        if let Some(opcode) = wow_packet::WorldPacket::from_bytes(&bytes).server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}

#[path = "creature_melee_tests/case_1.rs"]
mod case_1;

#[path = "creature_melee_tests/case_2.rs"]
mod case_2;

#[path = "creature_melee_tests/case_3.rs"]
mod case_3;

#[path = "creature_melee_tests/case_4.rs"]
mod case_4;

#[path = "creature_melee_tests/case_5.rs"]
mod case_5;

#[path = "creature_melee_tests/case_6.rs"]
mod case_6;

#[path = "creature_melee_tests/case_7.rs"]
mod case_7;

#[path = "creature_melee_tests/case_8.rs"]
mod case_8;

#[path = "creature_melee_tests/case_9.rs"]
mod case_9;

#[path = "creature_melee_tests/case_10.rs"]
mod case_10;

#[path = "creature_melee_tests/case_11.rs"]
mod case_11;

#[path = "creature_melee_tests/case_12.rs"]
mod case_12;

#[path = "creature_melee_tests/case_13.rs"]
mod case_13;

#[path = "creature_melee_tests/case_14.rs"]
mod case_14;

#[path = "creature_melee_tests/case_15.rs"]
mod case_15;

#[path = "creature_melee_tests/case_16.rs"]
mod case_16;

#[path = "creature_melee_tests/case_17.rs"]
mod case_17;

#[path = "creature_melee_tests/case_18.rs"]
mod case_18;

#[path = "creature_melee_tests/case_19.rs"]
mod case_19;

#[path = "creature_melee_tests/case_20.rs"]
mod case_20;

#[path = "creature_melee_tests/case_21.rs"]
mod case_21;

#[path = "creature_melee_tests/case_22.rs"]
mod case_22;

#[path = "creature_melee_tests/case_23.rs"]
mod case_23;

#[path = "creature_melee_tests/case_24.rs"]
mod case_24;

#[path = "creature_melee_tests/case_25.rs"]
mod case_25;

#[path = "creature_melee_tests/case_26.rs"]
mod case_26;

#[path = "creature_melee_tests/case_27.rs"]
mod case_27;

#[path = "creature_melee_tests/case_29.rs"]
mod case_29;

#[path = "creature_melee_tests/case_30.rs"]
mod case_30;

#[path = "creature_melee_tests/case_31.rs"]
mod case_31;

#[path = "creature_melee_tests/case_32.rs"]
mod case_32;

#[path = "creature_melee_tests/case_33.rs"]
mod case_33;

#[path = "creature_melee_tests/case_34.rs"]
mod case_34;

#[path = "creature_melee_tests/case_35.rs"]
mod case_35;

#[path = "creature_melee_tests/case_36.rs"]
mod case_36;

#[path = "creature_melee_tests/case_37.rs"]
mod case_37;

#[path = "creature_melee_tests/case_38.rs"]
mod case_38;

#[path = "creature_melee_tests/case_39.rs"]
mod case_39;

#[path = "creature_melee_tests/case_40.rs"]
mod case_40;

#[path = "creature_melee_tests/case_41.rs"]
mod case_41;

#[path = "creature_melee_tests/case_42.rs"]
mod case_42;

#[path = "creature_melee_tests/case_43.rs"]
mod case_43;

#[path = "creature_melee_tests/case_44.rs"]
mod case_44;

#[path = "creature_melee_tests/case_45.rs"]
mod case_45;

#[path = "creature_melee_tests/case_46.rs"]
mod case_46;

#[path = "creature_melee_tests/case_47.rs"]
mod case_47;

#[path = "creature_melee_tests/case_48.rs"]
mod case_48;
