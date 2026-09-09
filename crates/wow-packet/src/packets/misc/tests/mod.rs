//! Miscellaneous packet regressions.
//!
//! Separated from the tests.rs root under #640.

// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Misc packet tests for [`super`].
//!
//! Extracted from the inline module by issue #227.

#![cfg(test)]

use super::*;

fn write_minimal_toy_spell_cast(
    pkt: &mut WorldPacket,
    cast_id: ObjectGuid,
    item_id: i32,
    spell_id: i32,
) {
    use crate::packets::spell::{SpellCastVisual, SpellTargetData};

    pkt.write_packed_guid(&cast_id);
    pkt.write_int32(item_id);
    pkt.write_int32(0);
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

fn sample_battle_pet_journal_pet_like_cpp(
    pet_guid: ObjectGuid,
    owner_guid: ObjectGuid,
) -> BattlePetJournalPet {
    BattlePetJournalPet {
        guid: pet_guid,
        species: 11,
        creature_id: 22,
        display_id: 33,
        breed: 44,
        level: 55,
        exp: 66,
        flags: 77,
        power: 88,
        health: 99,
        max_health: 111,
        speed: 222,
        quality: 3,
        owner_info: Some(BattlePetJournalPetOwnerInfo {
            guid: owner_guid,
            player_virtual_realm: 123,
            player_native_realm: 456,
        }),
        name: "Misha".to_string(),
    }
}

fn assert_sample_battle_pet_journal_pet_like_cpp(
    body: &mut WorldPacket,
    pet_guid: ObjectGuid,
    owner_guid: ObjectGuid,
) {
    assert_eq!(body.read_packed_guid().unwrap(), pet_guid);
    assert_eq!(body.read_uint32().unwrap(), 11);
    assert_eq!(body.read_uint32().unwrap(), 22);
    assert_eq!(body.read_uint32().unwrap(), 33);
    assert_eq!(body.read_uint16().unwrap(), 44);
    assert_eq!(body.read_uint16().unwrap(), 55);
    assert_eq!(body.read_uint16().unwrap(), 66);
    assert_eq!(body.read_uint16().unwrap(), 77);
    assert_eq!(body.read_uint32().unwrap(), 88);
    assert_eq!(body.read_uint32().unwrap(), 99);
    assert_eq!(body.read_uint32().unwrap(), 111);
    assert_eq!(body.read_uint32().unwrap(), 222);
    assert_eq!(body.read_uint8().unwrap(), 3);
    assert_eq!(body.read_bits(7).unwrap(), 5);
    assert!(body.read_bit().unwrap());
    assert!(!body.read_bit().unwrap());
    assert_eq!(body.read_string(5).unwrap(), "Misha");
    assert_eq!(body.read_packed_guid().unwrap(), owner_guid);
    assert_eq!(body.read_uint32().unwrap(), 123);
    assert_eq!(body.read_uint32().unwrap(), 456);
}

mod scenarios_1;
mod scenarios_2;
mod scenarios_3;
mod scenarios_4;
mod scenarios_5;
mod scenarios_6;
