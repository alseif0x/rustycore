//! Creature-victim melee absorption scenarios under #29.

use super::*;

/// C++ `Unit::CalcAbsorbResist`'s school-absorb loop (`Unit.cpp:1789-1880`)
/// must spend a creature victim's canonical `AuraEffect` amount in the map
/// phase. The next swings consume the remainder instead of rebuilding the
/// shield from its spell base value; exhaustion removes the aura and publishes
/// the absorb log before the attacker-state packet.
#[test]
fn legacy_creature_melee_tick_once_absorbs_creature_victim_damage_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;
    use wow_constants::ServerOpcodes;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    canonical.lock().unwrap().create_world_map(0, 0);
    let attacker_guid = test_creature_guid(93_001);
    let victim_guid = test_creature_guid(93_002);
    let shield_spell_id = 93_100_i32;
    let hit_spell_id = 93_101_i32;
    let damage_immunity_spell_id = 93_102_i32;

    let (mut session, _, _) = make_session();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    register_test_creature(&mut session, manager.clone(), attacker_guid, 100);
    register_test_creature(&mut session, manager.clone(), victim_guid, 100);
    session
        .mutate_world_creature(attacker_guid, |creature| {
            creature.creature.unit_mut().set_level(80);
            creature.creature.ai_ownership_mut().min_damage = 10;
            creature.creature.ai_ownership_mut().max_damage = 10;
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();
    session
        .mutate_world_creature(victim_guid, |creature| {
            creature.creature.unit_mut().set_level(80);
            creature.creature.set_flags_extra_runtime_like_cpp(
                wow_constants::CreatureFlagsExtra::NO_CRIT.bits(),
            );
        })
        .unwrap();

    let mut spell_store = wow_data::SpellStore::new();
    let spell_info = |spell_id, amount, aura_type, misc_value| wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_base_points: amount,
        effect_bonus_coefficient: 0.0,
        aura_type: Some(aura_type),
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_type,
            effect_misc_value_1: misc_value,
            effect_base_points: amount,
            ..Default::default()
        }],
    };
    spell_store.insert(
        hit_spell_id,
        spell_info(
            hit_spell_id,
            5,
            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            0,
        ),
    );
    spell_store.insert(
        shield_spell_id,
        spell_info(
            shield_spell_id,
            30,
            wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB,
            0x01,
        ),
    );
    spell_store.insert(
        damage_immunity_spell_id,
        spell_info(
            damage_immunity_spell_id,
            0,
            wow_data::spell::aura_types::SPELL_AURA_DAMAGE_IMMUNITY,
            0x01,
        ),
    );
    let spell_store = Arc::new(spell_store);
    session.set_spell_store(Arc::clone(&spell_store));

    // The +5 hit aura cancels C++'s flat 5% miss band. Its amount is registered
    // on the canonical creature so the production creature projection reads it.
    session
        .mutate_world_creature(attacker_guid, |creature| {
            let aura = wow_entities::AppliedAuraRef::new(hit_spell_id as u32, attacker_guid, 0, 1);
            creature
                .creature
                .unit_mut()
                .subsystems_mut()
                .auras
                .register_applied_aura_effect_like_cpp(
                    aura,
                    wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
                    5,
                    0,
                );
        })
        .unwrap();

    let shield = wow_entities::AppliedAuraRef::new(shield_spell_id as u32, attacker_guid, 0, 1);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .with_creature_mut_like_cpp(victim_guid, |victim| {
            victim
                .unit_mut()
                .subsystems_mut()
                .auras
                .register_applied_aura_effect_like_cpp(
                    shield,
                    wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB,
                    30,
                    0x01,
                );
        })
        .unwrap();

    let config = crate::session::LegacyCreatureAggroConfigLikeCpp {
        spell_store: Some(spell_store),
        ..Default::default()
    };
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let victim_health = || {
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(victim_guid, |victim| victim.unit().data().health)
            .unwrap()
    };
    let shield_amount = || {
        canonical
            .lock()
            .unwrap()
            .find_map(0, 0)
            .unwrap()
            .map()
            .with_creature_like_cpp(victim_guid, |victim| {
                victim
                    .unit()
                    .subsystems()
                    .auras
                    .applied_aura_amounts
                    .get(&shield)
                    .copied()
            })
            .unwrap()
    };
    let event_opcodes = |outcome: &crate::session::LegacyCreatureMeleeTickOutcomeLikeCpp| {
        outcome
            .plan
            .events
            .iter()
            .filter_map(|event| {
                (event.packet_bytes.len() >= 2)
                    .then(|| u16::from_le_bytes([event.packet_bytes[0], event.packet_bytes[1]]))
            })
            .collect::<Vec<_>>()
    };
    let event_victim_state = |outcome: &crate::session::LegacyCreatureMeleeTickOutcomeLikeCpp| {
        let event = outcome
            .plan
            .events
            .iter()
            .find(|event| {
                event.packet_bytes.len() > 2
                    && u16::from_le_bytes([event.packet_bytes[0], event.packet_bytes[1]])
                        == ServerOpcodes::AttackerStateUpdate as u16
            })
            .expect("attacker state update event");
        let mut packet = wow_packet::world_packet::WorldPacket::from_bytes(&event.packet_bytes);
        packet.read_uint16().expect("opcode");
        packet.read_bit().expect("has log data");
        let info_len = packet.read_uint32().expect("attackRoundInfo size") as usize;
        let info_bytes = packet.read_bytes(info_len).expect("attackRoundInfo bytes");
        let mut info = wow_packet::world_packet::WorldPacket::from_bytes(&info_bytes);
        let hit_info = info.read_uint32().expect("hitInfo");
        info.read_packed_guid().expect("attacker");
        info.read_packed_guid().expect("victim");
        info.read_int32().expect("damage");
        info.read_int32().expect("original damage");
        info.read_int32().expect("over damage");
        if info.read_uint8().expect("sub-damage present") != 0 {
            info.read_int32().expect("sub-damage school");
            info.read_float().expect("sub-damage float");
            info.read_int32().expect("sub-damage amount");
            if hit_info
                & (wow_packet::packets::combat::HIT_INFO_FULL_ABSORB
                    | wow_packet::packets::combat::HIT_INFO_PARTIAL_ABSORB)
                != 0
            {
                info.read_int32().expect("absorbed");
            }
        }
        info.read_uint8().expect("victim state")
    };
    let tick = |session: &mut WorldSession| {
        session
            .mutate_world_creature(attacker_guid, |creature| {
                creature.creature.ai_ownership_mut().last_swing_ms = 0;
                creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            })
            .unwrap();
        run_legacy_creature_melee_tick_once_like_cpp(&manager, Some(&canonical), &config)
    };

    let first = tick(&mut session);
    assert_eq!(first.canonical_creature_hits, 1);
    assert_eq!(victim_health(), 100, "the first 10 points are absorbed");
    assert_eq!(shield_amount(), Some(20), "the canonical pool is spent");
    let first_opcodes = event_opcodes(&first);
    assert!(first_opcodes.contains(&(ServerOpcodes::SpellAbsorbLog as u16)));
    assert!(first_opcodes.contains(&(ServerOpcodes::AttackerStateUpdate as u16)));

    let second = tick(&mut session);
    assert_eq!(victim_health(), 100);
    assert_eq!(shield_amount(), Some(10));
    assert!(event_opcodes(&second).contains(&(ServerOpcodes::SpellAbsorbLog as u16)));

    let third = tick(&mut session);
    assert_eq!(victim_health(), 100);
    assert_eq!(
        shield_amount(),
        None,
        "exhaustion removes the canonical aura"
    );
    let third_opcodes = event_opcodes(&third);
    let absorb = third_opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::SpellAbsorbLog as u16)
        .expect("C++ publishes the creature victim absorb log");
    let removal = third_opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::AuraUpdate as u16)
        .expect("exhaustion publishes the creature aura removal");
    let attack = third_opcodes
        .iter()
        .position(|opcode| *opcode == ServerOpcodes::AttackerStateUpdate as u16)
        .expect("the melee result remains published");
    assert!(absorb < removal && removal < attack);

    let fourth = tick(&mut session);
    assert_eq!(victim_health(), 90, "the next swing lands after removal");
    assert!(!event_opcodes(&fourth).contains(&(ServerOpcodes::SpellAbsorbLog as u16)));

    // C++ `Unit::IsImmunedToDamage` also consults the damage-immunity mask
    // (`Unit.cpp:7318-7336`), independently of school immunity.
    let damage_immunity =
        wow_entities::AppliedAuraRef::new(damage_immunity_spell_id as u32, attacker_guid, 1, 1);
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .with_creature_mut_like_cpp(victim_guid, |victim| {
            victim
                .unit_mut()
                .subsystems_mut()
                .auras
                .register_applied_aura_effect_like_cpp(
                    damage_immunity,
                    wow_data::spell::aura_types::SPELL_AURA_DAMAGE_IMMUNITY,
                    0,
                    0x01,
                );
        })
        .unwrap();
    let immune = tick(&mut session);
    assert_eq!(victim_health(), 90, "damage immunity prevents the swing");
    assert_eq!(immune.canonical_creature_hits, 0);
    assert_eq!(
        event_victim_state(&immune),
        wow_packet::packets::combat::VICTIM_STATE_IS_IMMUNE
    );
}
