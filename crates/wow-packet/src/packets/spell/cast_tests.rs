// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Focused byte tests for the optional `SpellCastData` payload.

use wow_constants::ServerOpcodes;
use wow_core::{ObjectGuid, Position};

use crate::ServerPacket;
use crate::world_packet::WorldPacket;

use super::{
    CreatureImmunities, MissileTrajectoryResult, RuneData, SpellCastData, SpellCastVisual,
    SpellGoPkt, SpellHealPrediction, SpellMissReason, SpellMissTarget, SpellPowerData,
    SpellTargetData, TargetLocation,
};

fn payload_packet() -> SpellGoPkt {
    let caster = ObjectGuid::create_player(1, 0x101);
    let unit = ObjectGuid::create_player(1, 0x202);
    let hit_target =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 0x303, 0);
    let miss_target =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 0x404, 0);
    let target_point_transport = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Transport,
        0,
        1,
        571,
        0,
        0x505,
        0,
    );
    let beacon = ObjectGuid::create_player(1, 0x606);
    let cast_id =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Cast, 0, 1, 571, 0, 0x707, 1);

    SpellGoPkt {
        caster,
        cast_id,
        original_cast_id: ObjectGuid::create_player(1, 0x808),
        spell_id: 0x1122_3344,
        visual: SpellCastVisual {
            spell_visual_id: 0x5566_7788,
            script_visual_id: 0,
        },
        cast_flags: 0x0102_0304,
        cast_flags_ex: 0x0506_0708,
        cast_time_ms: 0x090a_0b0c,
        target: SpellTargetData::default(),
        cast_data: SpellCastData {
            caster_unit: Some(unit),
            remaining_power: vec![SpellPowerData {
                amount: -0x0102_0304,
                power_type: -3,
            }],
            remaining_runes: Some(RuneData {
                start: 7,
                count: 2,
                cooldowns: vec![0x11, 0x22],
            }),
            missile_trajectory: MissileTrajectoryResult {
                travel_time: 0x1213_1415,
                pitch: 1.25,
            },
            ammo_display_id: Some(-1234),
            ammo_inventory_type: Some(4567),
            dest_loc_spell_cast_index: 9,
            target_points: vec![TargetLocation {
                transport: target_point_transport,
                position: Position::xyz(10.25, -20.5, 30.75),
            }],
            immunities: CreatureImmunities {
                school: 0xa1b2_c3d4,
                value: 0x0102_0304,
            },
            heal_prediction: SpellHealPrediction {
                beacon_guid: beacon,
                points: 0x1020_3040,
                prediction_type: 6,
            },
        },
        hit_targets: vec![hit_target],
        miss_targets: vec![SpellMissTarget::reflected(
            miss_target,
            SpellMissReason::Resist,
        )],
    }
}

#[test]
fn spell_cast_data_writes_nonempty_fields_in_cpp_order() {
    let bytes = payload_packet().to_bytes();
    assert_eq!(
        &bytes[..2],
        &(ServerOpcodes::SpellGo as u16).to_le_bytes(),
        "the packet must retain the SMSG_SPELL_GO opcode"
    );

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    let caster = pkt.read_packed_guid().expect("CasterGUID");
    let caster_unit = pkt.read_packed_guid().expect("CasterUnit");
    assert_eq!(caster, ObjectGuid::create_player(1, 0x101));
    assert_eq!(caster_unit, ObjectGuid::create_player(1, 0x202));
    assert_eq!(
        pkt.read_packed_guid().expect("CastID"),
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Cast, 0, 1, 571, 0, 0x707, 1,)
    );
    assert_eq!(
        pkt.read_packed_guid().expect("OriginalCastID"),
        ObjectGuid::create_player(1, 0x808)
    );
    assert_eq!(pkt.read_int32().expect("SpellID"), 0x1122_3344);
    assert_eq!(
        SpellCastVisual::read(&mut pkt)
            .expect("SpellVisual")
            .spell_visual_id,
        0x5566_7788
    );
    assert_eq!(pkt.read_uint32().expect("CastFlags"), 0x0102_0304);
    assert_eq!(pkt.read_uint32().expect("CastFlagsEx"), 0x0506_0708);
    assert_eq!(pkt.read_uint32().expect("CastTime"), 0x090a_0b0c);

    // SpellCastData fixed fields precede all bit counts.
    assert_eq!(pkt.read_uint32().expect("TravelTime"), 0x1213_1415);
    assert_eq!(pkt.read_float().expect("Pitch"), 1.25);
    assert_eq!(pkt.read_uint8().expect("DestLocSpellCastIndex"), 9);
    assert_eq!(pkt.read_uint32().expect("Immunities.School"), 0xa1b2_c3d4);
    assert_eq!(pkt.read_uint32().expect("Immunities.Value"), 0x0102_0304);
    assert_eq!(pkt.read_uint32().expect("Predict.Points"), 0x1020_3040);
    assert_eq!(pkt.read_uint8().expect("Predict.Type"), 6);
    assert_eq!(
        pkt.read_packed_guid().expect("Predict.BeaconGUID"),
        ObjectGuid::create_player(1, 0x606)
    );

    assert_eq!(pkt.read_bits(16).expect("HitTargets count"), 1);
    assert_eq!(pkt.read_bits(16).expect("MissTargets count"), 1);
    assert_eq!(pkt.read_bits(16).expect("MissStatus count"), 1);
    assert_eq!(pkt.read_bits(9).expect("RemainingPower count"), 1);
    assert!(pkt.read_bit().expect("RemainingRunes presence"));
    assert_eq!(pkt.read_bits(16).expect("TargetPoints count"), 1);
    assert!(pkt.read_bit().expect("AmmoDisplayID presence"));
    assert!(pkt.read_bit().expect("AmmoInventoryType presence"));

    // The remaining sections are checked against explicit values in wire
    // order; this is intentionally not a struct roundtrip test.
    assert_eq!(
        SpellTargetData::read(&mut pkt).expect("Target"),
        SpellTargetData::default()
    );
    assert_eq!(
        pkt.read_packed_guid().expect("HitTargets[0]"),
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 0x303, 0,)
    );
    assert_eq!(
        pkt.read_packed_guid().expect("MissTargets[0]"),
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 571, 0, 0x404, 0,)
    );
    assert_eq!(
        pkt.read_uint8().expect("MissStatus reason"),
        SpellMissReason::Reflect as u8
    );
    assert_eq!(
        pkt.read_uint8().expect("MissStatus reflect status"),
        SpellMissReason::Resist as u8
    );
    assert_eq!(
        pkt.read_int32().expect("RemainingPower amount"),
        -0x0102_0304
    );
    assert_eq!(pkt.read_int8().expect("RemainingPower type"), -3);
    assert_eq!(pkt.read_uint8().expect("RuneData.Start"), 7);
    assert_eq!(pkt.read_uint8().expect("RuneData.Count"), 2);
    assert_eq!(pkt.read_uint32().expect("RuneData cooldown count"), 2);
    assert_eq!(
        pkt.read_bytes(2).expect("RuneData cooldowns"),
        vec![0x11, 0x22]
    );
    assert_eq!(
        pkt.read_packed_guid().expect("TargetPoints[0].Transport"),
        ObjectGuid::create_world_object(
            wow_core::guid::HighGuid::Transport,
            0,
            1,
            571,
            0,
            0x505,
            0,
        )
    );
    assert_eq!(pkt.read_float().expect("TargetPoints[0].X"), 10.25);
    assert_eq!(pkt.read_float().expect("TargetPoints[0].Y"), -20.5);
    assert_eq!(pkt.read_float().expect("TargetPoints[0].Z"), 30.75);
    assert_eq!(pkt.read_int32().expect("AmmoDisplayID"), -1234);
    assert_eq!(pkt.read_int32().expect("AmmoInventoryType"), 4567);

    // C++ `SpellGo::Write` appends `WriteLogDataBit` and `FlushBits` after the
    // shared payload, so a packet without advanced combat-log data still ends
    // in one flushed byte carrying a clear bit.
    pkt.reset_bits();
    assert!(
        !pkt.has_bit().expect("combat-log data bit"),
        "SMSG_SPELL_GO without log data must clear the combat-log bit"
    );
    assert!(
        pkt.is_empty(),
        "all C++ SpellCastData bytes must be consumed"
    );
}

#[test]
fn spell_cast_data_rejects_counts_that_do_not_fit_wire_fields() {
    let mut too_many_power_rows = payload_packet();
    too_many_power_rows.cast_data.remaining_power = vec![SpellPowerData::default(); 0x200];
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = too_many_power_rows.to_bytes();
        }))
        .is_err(),
        "9-bit RemainingPower count must fail instead of truncating"
    );

    let mut too_many_target_points = payload_packet();
    too_many_target_points.cast_data.target_points = vec![TargetLocation::default(); 0x1_0000];
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = too_many_target_points.to_bytes();
        }))
        .is_err(),
        "16-bit TargetPoints count must fail instead of truncating"
    );

    let mut too_many_hits = payload_packet();
    too_many_hits.hit_targets = vec![ObjectGuid::EMPTY; 0x1_0000];
    assert!(
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = too_many_hits.to_bytes();
        }))
        .is_err(),
        "16-bit HitTargets count must fail instead of truncating"
    );
}

#[test]
fn default_cast_data_preserves_legacy_unit_caster_bytes() {
    let caster = ObjectGuid::create_player(1, 0x909);
    let bytes = SpellGoPkt {
        caster,
        cast_id: ObjectGuid::EMPTY,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: 12_345,
        visual: SpellCastVisual::default(),
        cast_flags: 0,
        cast_flags_ex: 0,
        cast_time_ms: 0,
        target: SpellTargetData::default(),
        cast_data: SpellCastData::default(),
        hit_targets: Vec::new(),
        miss_targets: Vec::new(),
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes[2..]);
    assert_eq!(pkt.read_packed_guid().expect("CasterGUID"), caster);
    assert_eq!(
        pkt.read_packed_guid().expect("legacy CasterUnit"),
        caster,
        "default caster_unit must retain the previous duplicate-caster wire shape"
    );
}
