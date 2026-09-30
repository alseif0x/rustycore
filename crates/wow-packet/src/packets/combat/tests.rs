use super::*;
use wow_constants::ServerOpcodes;
use wow_constants::creature::AiReaction;
use wow_core::ObjectGuid;

use crate::ServerPacket;
use crate::world_packet::WorldPacket;

#[test]
fn attacker_state_update_writes_the_block_fields_like_cpp() {
    let attacker = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        0,
        0,
        0,
        125,
        0x1236,
    );
    let victim = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        0,
        0,
        0,
        126,
        0x1237,
    );
    let bytes = AttackerStateUpdate {
        attacker,
        victim,
        hit_info: HIT_INFO_AFFECTS_VICTIM | HIT_INFO_BLOCK,
        damage: 70,
        original_damage: 100,
        over_damage: -1,
        blocked: 30,
        absorbed: 0,
        victim_state: VICTIM_STATE_HIT,
        school_mask: 1,
        target_level: 80,
        expansion: 2,
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::AttackerStateUpdate as u16
    );
    let _ = pkt.read_bit().expect("has_log_data");
    let attack_round_info_size = pkt.read_uint32().expect("attackRoundInfo size") as usize;
    let attack_round_info = pkt
        .read_bytes(attack_round_info_size)
        .expect("attackRoundInfo bytes");
    let mut info = WorldPacket::from_bytes(&attack_round_info);
    assert_eq!(
        info.read_uint32().expect("hitInfo"),
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_BLOCK
    );
    assert_eq!(info.read_packed_guid().expect("attacker"), attacker);
    assert_eq!(info.read_packed_guid().expect("victim"), victim);
    assert_eq!(info.read_int32().expect("damage"), 70);
    assert_eq!(info.read_int32().expect("original damage"), 100);
    assert_eq!(info.read_int32().expect("over damage"), -1);
    // `CombatLogPackets.cpp:355-365`: the presence byte, then the sub-damage
    // struct. This swing carries no absorb/resist bit, so neither amount is
    // serialized.
    assert_eq!(info.read_uint8().expect("sub damage present"), 1);
    assert_eq!(info.read_int32().expect("sub damage school mask"), 1);
    assert_eq!(info.read_float().expect("sub damage float"), 70.0);
    assert_eq!(info.read_int32().expect("sub damage"), 70);
    assert_eq!(info.read_uint8().expect("victim state"), VICTIM_STATE_HIT);
    assert_eq!(info.read_uint32().expect("attacker state"), 0);
    assert_eq!(info.read_uint32().expect("melee spell id"), 0);
    // `CombatLogPackets.cpp:373-397`: the blocked amount, then the trailing
    // `float Unk` the same condition writes.
    assert_eq!(info.read_int32().expect("blocked"), 30);
    assert_eq!(info.read_float().expect("unk"), 0.0);
    assert_eq!(info.read_uint8().expect("content tuning type"), 0);
    assert_eq!(info.read_uint8().expect("target level"), 80);
}

#[test]
fn hit_info_and_victim_state_match_cpp_3_4_3_like_cpp() {
    // `UnitDefines.h:440-465` and `Unit.h:45-55` in the 3.4.3 target.
    assert_eq!(HIT_INFO_AFFECTS_VICTIM, 0x0000_0002);
    assert_eq!(HIT_INFO_OFFHAND, 0x0000_0004);
    assert_eq!(HIT_INFO_MISS, 0x0000_0010);
    assert_eq!(HIT_INFO_CRITICAL_HIT, 0x0000_0200);
    assert_eq!(HIT_INFO_CRUSHING, 0x0002_0000);
    assert_eq!(HIT_INFO_FULL_ABSORB, 0x0000_0020);
    assert_eq!(HIT_INFO_PARTIAL_ABSORB, 0x0000_0040);
    assert_eq!(HIT_INFO_GLANCING, 0x0001_0000);
    assert_eq!(HIT_INFO_FAKE_DAMAGE, 0x0100_0000);
    assert_eq!(VICTIM_STATE_INTACT, 0);
    assert_eq!(VICTIM_STATE_HIT, 1);
    assert_eq!(VICTIM_STATE_DODGE, 2);
    assert_eq!(VICTIM_STATE_PARRY, 3);
}

#[test]
fn attacker_state_update_writes_custom_hit_info_like_cpp() {
    let attacker = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        0,
        0,
        0,
        123,
        0x1234,
    );
    let victim = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        0,
        0,
        0,
        124,
        0x1235,
    );
    let bytes = AttackerStateUpdate {
        attacker,
        victim,
        hit_info: HIT_INFO_AFFECTS_VICTIM | HIT_INFO_FAKE_DAMAGE,
        damage: 0,
        original_damage: 0,
        over_damage: -1,
        blocked: 0,
        absorbed: 0,
        victim_state: VICTIM_STATE_HIT,
        school_mask: 1,
        target_level: 80,
        expansion: 2,
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::AttackerStateUpdate as u16
    );
    assert!(
        !pkt.read_bit().expect("has_log_data"),
        "CombatLogServerPacket writes the log-data bit before attackRoundInfo size"
    );
    let attack_round_info_size = pkt.read_uint32().expect("attackRoundInfo size") as usize;
    let attack_round_info = pkt
        .read_bytes(attack_round_info_size)
        .expect("attackRoundInfo bytes");
    let mut info = WorldPacket::from_bytes(&attack_round_info);
    assert_eq!(
        info.read_uint32().expect("hitInfo"),
        HIT_INFO_AFFECTS_VICTIM | HIT_INFO_FAKE_DAMAGE
    );
    assert_eq!(info.read_packed_guid().expect("attacker"), attacker);
    assert_eq!(info.read_packed_guid().expect("victim"), victim);
    assert_eq!(info.read_int32().expect("damage"), 0);
    assert_eq!(info.read_int32().expect("original damage"), 0);
    assert_eq!(info.read_int32().expect("over damage"), -1);
    assert_eq!(info.read_uint8().expect("sub damage present"), 1);
    assert_eq!(info.read_int32().expect("sub damage school mask"), 1);
    assert_eq!(info.read_float().expect("sub damage float"), 0.0);
    assert_eq!(info.read_int32().expect("sub damage"), 0);
    assert_eq!(info.read_uint8().expect("victim state"), VICTIM_STATE_HIT);
    assert_eq!(info.read_uint32().expect("attacker state"), 0);
    assert_eq!(info.read_uint32().expect("melee spell id"), 0);
    assert_eq!(info.read_uint8().expect("content tuning type"), 0);
    assert_eq!(info.read_uint8().expect("target level"), 80);
    assert_eq!(info.read_uint8().expect("expansion"), 2);
    assert_eq!(info.read_int16().expect("player level delta"), 0);
    assert_eq!(info.read_int8().expect("target scaling level delta"), 0);
    assert_eq!(info.read_float().expect("player item level"), 0.0);
    assert_eq!(info.read_float().expect("target item level"), 0.0);
    assert_eq!(info.read_uint32().expect("scaling curve"), 0);
    assert_eq!(info.read_uint32().expect("content tuning flags"), 0);
    assert_eq!(info.read_int32().expect("player content tuning id"), 0);
    assert_eq!(info.read_int32().expect("target content tuning id"), 0);
    assert!(
        info.is_empty(),
        "attackRoundInfo must not contain the combat-log bit"
    );
}

/// C++ `AttackerStateUpdate::Write` (`CombatLogPackets.cpp:355-365`): a
/// school absorb sets `HITINFO_PARTIAL_ABSORB`/`HITINFO_FULL_ABSORB` and the
/// packet then carries `SubDmg.Absorbed` after the sub-damage integers.
#[test]
fn attacker_state_update_writes_absorbed_sub_damage_like_cpp() {
    let guid = |entry: u32, low: i64| {
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 0, 0, 0, entry, low)
    };
    for (hit_info, damage, absorbed) in [
        (HIT_INFO_AFFECTS_VICTIM | HIT_INFO_PARTIAL_ABSORB, 70, 30),
        (HIT_INFO_AFFECTS_VICTIM | HIT_INFO_FULL_ABSORB, 0, 100),
    ] {
        let bytes = AttackerStateUpdate {
            attacker: guid(123, 0x1234),
            victim: guid(124, 0x1235),
            hit_info,
            damage,
            original_damage: 100,
            over_damage: -1,
            blocked: 0,
            absorbed,
            victim_state: VICTIM_STATE_HIT,
            school_mask: 1,
            target_level: 80,
            expansion: 2,
        }
        .to_bytes();

        let mut pkt = WorldPacket::from_bytes(&bytes);
        assert_eq!(
            pkt.read_uint16().expect("opcode"),
            ServerOpcodes::AttackerStateUpdate as u16
        );
        let _ = pkt.read_bit().expect("has_log_data");
        let size = pkt.read_uint32().expect("attackRoundInfo size") as usize;
        let round_info = pkt.read_bytes(size).expect("attackRoundInfo bytes");
        let mut info = WorldPacket::from_bytes(&round_info);
        assert_eq!(info.read_uint32().expect("hitInfo"), hit_info);
        let _ = info.read_packed_guid().expect("attacker");
        let _ = info.read_packed_guid().expect("victim");
        assert_eq!(info.read_int32().expect("damage"), damage);
        assert_eq!(info.read_int32().expect("original damage"), 100);
        assert_eq!(info.read_int32().expect("over damage"), -1);
        assert_eq!(info.read_uint8().expect("sub damage present"), 1);
        assert_eq!(info.read_int32().expect("sub damage school mask"), 1);
        assert_eq!(info.read_float().expect("sub damage float"), damage as f32);
        assert_eq!(info.read_int32().expect("sub damage"), damage);
        assert_eq!(info.read_int32().expect("absorbed"), absorbed);
        assert_eq!(info.read_uint8().expect("victim state"), VICTIM_STATE_HIT);
    }
}

#[test]
fn ai_reaction_serializes_guid_then_reaction_like_cpp() {
    let guid = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        0,
        0,
        0,
        123,
        0x1234,
    );
    let bytes = AIReaction {
        unit_guid: guid,
        reaction: AiReaction::Alert,
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::AiReaction as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("unit guid"), guid);
    assert_eq!(
        pkt.read_uint32().expect("reaction"),
        AiReaction::Alert as u32
    );
    assert!(pkt.is_empty());
}

#[test]
fn health_update_writes_packed_guid_and_i64_health_like_cpp() {
    let guid = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let bytes = HealthUpdate { guid, health: 83 }.to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::HealthUpdate as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("guid"), guid);
    assert_eq!(pkt.read_int64().expect("health"), 83);
    assert!(pkt.is_empty());
}

#[test]
fn power_update_writes_guid_count_and_power_type_pairs_like_cpp() {
    let guid = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let bytes = PowerUpdate {
        guid,
        powers: vec![(4321, 0)],
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::PowerUpdate as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("guid"), guid);
    assert_eq!(pkt.read_uint32().expect("count"), 1);
    assert_eq!(pkt.read_int32().expect("power"), 4321);
    assert_eq!(pkt.read_uint8().expect("power type"), 0);
    assert!(pkt.is_empty());
}

#[test]
fn spell_absorb_log_writes_cpp_field_order_like_cpp() {
    let attacker = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        0,
        0,
        0,
        123,
        0x1234,
    );
    let victim = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let caster = ObjectGuid::create_player(1, 0x1112_1314_1516_1718);
    let bytes = SpellAbsorbLog {
        attacker,
        victim,
        absorbed_spell_id: 0,
        absorb_spell_id: 17_262,
        caster,
        absorbed: 10,
        original_damage: 10,
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::SpellAbsorbLog as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("attacker"), attacker);
    assert_eq!(pkt.read_packed_guid().expect("victim"), victim);
    assert_eq!(pkt.read_int32().expect("absorbed spell id"), 0);
    assert_eq!(pkt.read_int32().expect("absorb spell id"), 17_262);
    assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
    assert_eq!(pkt.read_int32().expect("absorbed"), 10);
    assert_eq!(pkt.read_int32().expect("original damage"), 10);
    assert_eq!(pkt.read_uint32().expect("supporters"), 0);
    // `WriteBit(Unk)` then `WriteLogDataBit()`: the basic packet carries two
    // false bits and no log data.
    assert!(!pkt.has_bit().expect("unk"));
    assert!(!pkt.has_bit().expect("has log data"));
    assert!(pkt.is_empty());
}

#[test]
fn spell_non_melee_damage_log_writes_cpp_field_order_like_cpp() {
    let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let target = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        0,
        0,
        0,
        123,
        0x1234,
    );
    let cast_id =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Cast, 0, 1, 0, 0, 456, 0x5678);
    let bytes = SpellNonMeleeDamageLog {
        target,
        caster,
        cast_id,
        spell_id: 116,
        visual_id: 42,
        damage: 83,
        original_damage: 100,
        overkill: -1,
        school_mask: 4,
        absorbed: 10,
        resisted: 7,
        shield_block: 0,
        periodic: false,
        flags: 0,
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::SpellNonMeleeDamageLog as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("me"), target);
    assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
    assert_eq!(pkt.read_packed_guid().expect("cast id"), cast_id);
    assert_eq!(pkt.read_int32().expect("spell id"), 116);
    assert_eq!(pkt.read_int32().expect("visual"), 42);
    assert_eq!(pkt.read_int32().expect("damage"), 83);
    assert_eq!(pkt.read_int32().expect("original damage"), 100);
    assert_eq!(pkt.read_int32().expect("overkill"), -1);
    assert_eq!(pkt.read_uint8().expect("school mask"), 4);
    assert_eq!(pkt.read_int32().expect("absorbed"), 10);
    assert_eq!(pkt.read_int32().expect("resisted"), 7);
    assert_eq!(pkt.read_int32().expect("shield block"), 0);
    assert_eq!(pkt.read_uint32().expect("world text viewers"), 0);
    assert_eq!(pkt.read_uint32().expect("supporters"), 0);
    // Bits: `Periodic`, seven `Flags`, debug info, log data and content
    // tuning, all flushed together.
    assert!(!pkt.has_bit().expect("periodic"));
    assert_eq!(pkt.read_bits(7).expect("flags"), 0);
    assert!(!pkt.has_bit().expect("debug info"));
    assert!(!pkt.has_bit().expect("has log data"));
    assert!(!pkt.has_bit().expect("content tuning"));
    assert!(pkt.is_empty());
}

#[test]
fn spell_heal_log_writes_cpp_field_order_like_cpp() {
    let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let target = ObjectGuid::create_player(1, 0x1112_1314_1516_1718);
    let bytes = SpellHealLog {
        target,
        caster,
        spell_id: 2_066_001,
        health: 300,
        original_heal: 300,
        over_heal: 40,
        absorbed: 0,
        crit: false,
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::SpellHealLog as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("target"), target);
    assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
    assert_eq!(pkt.read_int32().expect("spell id"), 2_066_001);
    assert_eq!(pkt.read_int32().expect("health"), 300);
    assert_eq!(pkt.read_int32().expect("original heal"), 300);
    assert_eq!(pkt.read_int32().expect("over heal"), 40);
    assert_eq!(pkt.read_int32().expect("absorbed"), 0);
    assert_eq!(pkt.read_uint32().expect("supporters"), 0);
    assert!(!pkt.has_bit().expect("crit"));
    assert!(!pkt.has_bit().expect("crit roll made"));
    assert!(!pkt.has_bit().expect("crit roll needed"));
    assert!(!pkt.has_bit().expect("has log data"));
    assert!(!pkt.has_bit().expect("content tuning"));
    assert!(pkt.is_empty());
}

#[test]
fn spell_heal_absorb_log_writes_cpp_field_order_like_cpp() {
    let target = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let absorb_caster = ObjectGuid::create_player(1, 0x1112_1314_1516_1718);
    let healer = ObjectGuid::create_player(1, 0x2122_2324_2526_2728);
    let bytes = SpellHealAbsorbLog {
        target,
        absorb_caster,
        healer,
        absorb_spell_id: 17_262,
        absorbed_spell_id: 2_066_001,
        absorbed: 120,
        original_heal: 300,
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::SpellHealAbsorbLog as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("target"), target);
    assert_eq!(
        pkt.read_packed_guid().expect("absorb caster"),
        absorb_caster
    );
    assert_eq!(pkt.read_packed_guid().expect("healer"), healer);
    assert_eq!(pkt.read_int32().expect("absorb spell id"), 17_262);
    assert_eq!(pkt.read_int32().expect("absorbed spell id"), 2_066_001);
    assert_eq!(pkt.read_int32().expect("absorbed"), 120);
    assert_eq!(pkt.read_int32().expect("original heal"), 300);
    assert!(!pkt.has_bit().expect("content tuning"));
    assert!(pkt.is_empty());
}

#[test]
fn spell_energize_log_writes_cpp_field_order_like_cpp() {
    let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let target = ObjectGuid::create_player(1, 0x1112_1314_1516_1718);
    let bytes = SpellEnergizeLog {
        target,
        caster,
        spell_id: 793,
        power_type: 0,
        amount: 50,
        over_energize: 10,
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::SpellEnergizeLog as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("target"), target);
    assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
    assert_eq!(pkt.read_int32().expect("spell id"), 793);
    assert_eq!(pkt.read_int32().expect("power type"), 0);
    assert_eq!(pkt.read_int32().expect("amount"), 50);
    assert_eq!(pkt.read_int32().expect("over energize"), 10);
    assert!(!pkt.has_bit().expect("has log data"));
    assert!(pkt.is_empty());
}

#[test]
fn spell_execute_log_writes_cpp_effect_lists() {
    let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let victim =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 9_001, 44);
    let bytes = SpellExecuteLog {
        caster,
        spell_id: 2_971,
        effects: vec![
            SpellLogEffect {
                effect: 122, // SPELL_EFFECT_POWER_DRAIN
                power_drain_targets: vec![SpellLogEffectPowerDrainParams {
                    victim,
                    points: 40,
                    power_type: 0,
                    amplitude: 0.5,
                }],
                ..Default::default()
            },
            SpellLogEffect {
                effect: 16, // SPELL_EFFECT_ADD_EXTRA_ATTACKS
                extra_attacks_targets: vec![SpellLogEffectExtraAttacksParams {
                    victim,
                    num_attacks: 3,
                }],
                ..Default::default()
            },
        ],
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::SpellExecuteLog as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
    assert_eq!(pkt.read_int32().expect("spell id"), 2_971);
    assert_eq!(pkt.read_uint32().expect("effect count"), 2);
    // C++ writes each effect's id, the six list counts, then the rows.
    assert_eq!(pkt.read_int32().expect("effect"), 122);
    assert_eq!(pkt.read_uint32().expect("power drain count"), 1);
    for list in 0..5 {
        assert_eq!(
            pkt.read_uint32().expect("empty list count"),
            0,
            "list {list} has no producer in this represented cast"
        );
    }
    assert_eq!(pkt.read_packed_guid().expect("drain victim"), victim);
    assert_eq!(pkt.read_uint32().expect("points"), 40);
    assert_eq!(pkt.read_uint32().expect("power type"), 0);
    assert_eq!(pkt.read_float().expect("amplitude"), 0.5);
    assert_eq!(pkt.read_int32().expect("effect"), 16);
    assert_eq!(pkt.read_uint32().expect("power drain count"), 0);
    assert_eq!(pkt.read_uint32().expect("extra attacks count"), 1);
    for list in 0..4 {
        assert_eq!(
            pkt.read_uint32().expect("empty list count"),
            0,
            "list {list} has no producer in this represented cast"
        );
    }
    assert_eq!(pkt.read_packed_guid().expect("extra victim"), victim);
    assert_eq!(pkt.read_uint32().expect("num attacks"), 3);
    assert!(!pkt.has_bit().expect("has log data"));
    assert!(pkt.is_empty());
}

#[test]
fn spell_execute_log_writes_the_durability_generic_trade_and_feed_lists() {
    let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let victim =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 9_001, 45);
    let bytes = SpellExecuteLog {
        caster,
        spell_id: 4_036,
        effects: vec![SpellLogEffect {
            effect: 25, // SPELL_EFFECT_DURABILITY_DAMAGE
            durability_damage_targets: vec![SpellLogEffectDurabilityDamageParams {
                victim,
                item_id: 300,
                amount: 15,
            }],
            generic_victim_targets: vec![SpellLogEffectGenericVictimParams { victim }],
            trade_skill_targets: vec![SpellLogEffectTradeSkillItemParams { item_id: 1_234 }],
            feed_pet_targets: vec![SpellLogEffectFeedPetParams { item_id: 5_678 }],
            ..Default::default()
        }],
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::SpellExecuteLog as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
    assert_eq!(pkt.read_int32().expect("spell id"), 4_036);
    assert_eq!(pkt.read_uint32().expect("effect count"), 1);
    assert_eq!(pkt.read_int32().expect("effect"), 25);
    assert_eq!(pkt.read_uint32().expect("power drain count"), 0);
    assert_eq!(pkt.read_uint32().expect("extra attacks count"), 0);
    assert_eq!(pkt.read_uint32().expect("durability count"), 1);
    assert_eq!(pkt.read_uint32().expect("generic victim count"), 1);
    assert_eq!(pkt.read_uint32().expect("trade skill count"), 1);
    assert_eq!(pkt.read_uint32().expect("feed pet count"), 1);
    // C++ writes the rows in list order after all six counts
    // (`CombatLogPackets.cpp:120-153`).
    assert_eq!(pkt.read_packed_guid().expect("durability victim"), victim);
    assert_eq!(pkt.read_int32().expect("item id"), 300);
    assert_eq!(pkt.read_int32().expect("amount"), 15);
    assert_eq!(pkt.read_packed_guid().expect("generic victim"), victim);
    assert_eq!(pkt.read_int32().expect("trade skill item"), 1_234);
    assert_eq!(pkt.read_int32().expect("feed pet item"), 5_678);
    assert!(!pkt.has_bit().expect("has log data"));
    assert!(pkt.is_empty());
}

#[test]
fn interrupt_power_regen_writes_only_the_power_type_like_cpp() {
    let bytes = InterruptPowerRegen { power_type: 3 }.to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::InterruptPowerRegen as u16
    );
    assert_eq!(
        pkt.read_int32().expect("power type"),
        3,
        "C++ `InterruptPowerRegen::Write` emits `int32(PowerType)` only"
    );
    assert!(pkt.is_empty());
}

#[test]
fn environmental_damage_log_writes_cpp_shape_without_log_data() {
    let victim = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let bytes = EnvironmentalDamageLog {
        victim,
        damage_type: 2,
        amount: 117,
        resisted: 0,
        absorbed: 0,
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::EnvironmentalDamageLog as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("victim"), victim);
    assert_eq!(pkt.read_uint8().expect("type"), 2);
    assert_eq!(pkt.read_int32().expect("amount"), 117);
    assert_eq!(pkt.read_int32().expect("resisted"), 0);
    assert_eq!(pkt.read_int32().expect("absorbed"), 0);
    assert!(!pkt.has_bit().expect("has log data"));
    assert!(pkt.is_empty());
}

#[test]
fn pvp_credit_writes_cpp_field_order_like_cpp() {
    let target = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let bytes = PvpCredit {
        original_honor: 42,
        honor: 40,
        target,
        rank: 7,
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::PvpCredit as u16
    );
    assert_eq!(pkt.read_int32().expect("OriginalHonor"), 42);
    assert_eq!(pkt.read_int32().expect("Honor"), 40);
    assert_eq!(pkt.read_packed_guid().expect("Target"), target);
    assert_eq!(pkt.read_int32().expect("Rank"), 7);
    assert!(pkt.is_empty());
}

#[test]
fn break_target_writes_unit_guid_like_cpp() {
    let unit_guid = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let bytes = BreakTarget { unit_guid }.to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::BreakTarget as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("UnitGUID"), unit_guid);
    assert!(pkt.is_empty());
}

#[test]
fn cancel_combat_writes_empty_cpp_payload_like_cpp() {
    let bytes = CancelCombat.to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::CancelCombat as u16
    );
    assert!(pkt.is_empty());
}

#[test]
fn spell_instakill_log_writes_target_caster_and_spell_like_cpp() {
    let target =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Creature, 0, 1, 0, 0, 9_001, 44);
    let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let bytes = SpellInstakillLog {
        target,
        caster,
        spell_id: 5_333,
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::SpellInstakillLog as u16
    );
    assert_eq!(pkt.read_packed_guid().expect("target"), target);
    assert_eq!(pkt.read_packed_guid().expect("caster"), caster);
    assert_eq!(pkt.read_int32().expect("spell id"), 5_333);
    assert!(pkt.is_empty());
}

#[test]
fn spell_miss_log_writes_cpp_field_order_like_cpp() {
    let caster = ObjectGuid::create_player(1, 0x0102_0304_0506_0708);
    let victim = ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::Creature,
        0,
        0,
        0,
        0,
        123,
        0x1234,
    );
    let bytes = SpellMissLog {
        spell_id: 91_364,
        caster,
        entries: vec![SpellMissLogEntry {
            victim,
            miss_reason: 7,
        }],
    }
    .to_bytes();

    let mut pkt = WorldPacket::from_bytes(&bytes);
    assert_eq!(
        pkt.read_uint16().expect("opcode"),
        ServerOpcodes::SpellMissLog as u16
    );
    assert_eq!(pkt.read_int32().expect("spell id"), 91_364);
    assert_eq!(pkt.read_guid().expect("caster"), caster);
    assert_eq!(pkt.read_uint32().expect("entry count"), 1);
    assert_eq!(pkt.read_guid().expect("victim"), victim);
    assert_eq!(pkt.read_uint8().expect("miss reason"), 7);
    assert!(!pkt.has_bit().expect("debug absent"));
    assert!(pkt.is_empty());
}
