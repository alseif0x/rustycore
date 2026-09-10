//! Spell packet regressions.
//!
//! Separated from spell.rs under #683.

use super::*;

fn spell_go_bytes(hit_targets: Vec<ObjectGuid>, miss_targets: Vec<SpellMissTarget>) -> Vec<u8> {
    SpellGoPkt {
        caster: ObjectGuid::EMPTY,
        cast_id: ObjectGuid::EMPTY,
        original_cast_id: ObjectGuid::EMPTY,
        spell_id: 12_345,
        visual: SpellCastVisual::default(),
        cast_flags: 0,
        cast_flags_ex: 0,
        cast_time_ms: 0,
        target: SpellTargetData::default(),
        cast_data: SpellCastData::default(),
        hit_targets,
        miss_targets,
    }
    .to_bytes()
}

fn read_spell_go_through_target(bytes: &[u8]) -> (WorldPacket, usize, usize, usize) {
    let mut pkt = WorldPacket::from_bytes(bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::SpellGo as u16
    );

    for _ in 0..4 {
        pkt.read_packed_guid().expect("cast guid");
    }
    pkt.read_int32().expect("spell id");
    SpellCastVisual::read(&mut pkt).expect("spell visual");
    pkt.read_uint32().expect("cast flags");
    pkt.read_uint32().expect("cast flags ex");
    pkt.read_uint32().expect("cast time");
    pkt.read_int32().expect("missile travel time");
    pkt.read_float().expect("missile pitch");
    pkt.read_uint8().expect("destination index");
    pkt.read_uint32().expect("immunity school");
    pkt.read_uint32().expect("immunity value");
    pkt.read_uint32().expect("predicted heal");
    pkt.read_uint8().expect("prediction type");
    pkt.read_packed_guid().expect("prediction beacon");

    let hit_count = pkt.read_bits(16).expect("HitTargets count") as usize;
    let miss_count = pkt.read_bits(16).expect("MissTargets count") as usize;
    let miss_status_count = pkt.read_bits(16).expect("MissStatus count") as usize;
    assert_eq!(pkt.read_bits(9).expect("RemainingPower count"), 0);
    assert!(!pkt.read_bit().expect("RemainingRunes presence"));
    assert_eq!(pkt.read_bits(16).expect("TargetPoints count"), 0);
    assert!(!pkt.read_bit().expect("AmmoDisplayID presence"));
    assert!(!pkt.read_bit().expect("AmmoInventoryType presence"));

    assert_eq!(
        SpellTargetData::read(&mut pkt).expect("target data"),
        SpellTargetData::default()
    );

    (pkt, hit_count, miss_count, miss_status_count)
}

fn write_minimal_spell_cast_request(
    pkt: &mut WorldPacket,
    cast_id: ObjectGuid,
    misc: [i32; 2],
    spell_id: i32,
) {
    pkt.write_packed_guid(&cast_id);
    pkt.write_int32(misc[0]);
    pkt.write_int32(misc[1]);
    pkt.write_int32(spell_id);
    SpellCastVisual::default().write(pkt);
    pkt.write_float(0.0);
    pkt.write_float(0.0);
    pkt.write_packed_guid(&ObjectGuid::EMPTY);
    pkt.write_uint32(0);
    pkt.write_uint32(0);
    pkt.write_uint32(0);
    pkt.write_bits(0, 5);
    pkt.write_bit(false);
    pkt.write_bits(0, 2);
    pkt.write_bit(false);
    pkt.flush_bits();
    SpellTargetData::default().write(pkt);
}

mod scenarios;
