//! Creature runtime scenarios of the represented world entities responsibility.
//!
//! #1363 slice 3b moved these session-free tests verbatim out of the `wow-world`
//! session scenario module `scenarios_world_entities_24`; the tests there that
//! build a `WorldSession` stay in `wow-world`.

use super::*;

#[test]
fn creature_spell_canonical_targetability_honors_cpp_untargetable_override() {
    let attributes = [0_u32; 15];
    for flag in [
        UnitFlags::NON_ATTACKABLE,
        UnitFlags::UNINTERACTIBLE,
        UnitFlags::ON_TAXI,
        UnitFlags::NOT_ATTACKABLE_1,
        UnitFlags::IMMUNE_TO_NPC,
        UnitFlags::NON_ATTACKABLE_2,
    ] {
        assert!(!creature_spell_target_accepts_npc_attack_like_cpp(
            flag,
            &attributes
        ));
    }

    let mut can_target_untargetable = attributes;
    can_target_untargetable[6] = 0x0100_0000; // SPELL_ATTR6_CAN_TARGET_UNTARGETABLE
    assert!(creature_spell_target_accepts_npc_attack_like_cpp(
        UnitFlags::NON_ATTACKABLE_2,
        &can_target_untargetable,
    ));
    assert!(!creature_spell_target_accepts_npc_attack_like_cpp(
        UnitFlags::NON_ATTACKABLE | UnitFlags::NON_ATTACKABLE_2,
        &can_target_untargetable,
    ));
}

#[test]
fn creature_spell_canonical_targetability_rechecks_full_cpp_state() {
    let canonical = shared_canonical_map_manager();
    let creature_guid = test_creature_guid(91_334);
    let victim_guid = ObjectGuid::create_player(1, 91_335);
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    let config = aggro_hostile_config_like_cpp();
    let attributes = [0_u32; 15];

    let target_is_valid = |canonical: &SharedCanonicalMapManager| {
        let manager = canonical.lock().unwrap();
        let map = manager.find_map(0, 0).unwrap().map();
        map.with_creature_like_cpp(creature_guid, |creature| {
            creature_spell_target_is_valid_attack_target_like_cpp(
                creature,
                map.get_typed_player(victim_guid).unwrap(),
                &attributes,
                &config,
            )
        })
        .unwrap()
    };
    assert!(target_is_valid(&canonical));

    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(victim_guid)
        .unwrap()
        .set_game_master_like_cpp(true);
    assert!(
        !target_is_valid(&canonical),
        "a player entering GM mode after aggro must fail the cast-time C++ target check"
    );

    let mut manager = canonical.lock().unwrap();
    let victim = manager
        .find_map_mut(0, 0)
        .unwrap()
        .map_mut()
        .get_typed_player_mut(victim_guid)
        .unwrap();
    victim.set_game_master_like_cpp(false);
    victim
        .unit_mut()
        .add_unit_state(UnitState::IN_FLIGHT.bits());
    drop(manager);
    assert!(
        !target_is_valid(&canonical),
        "UNIT_STATE_UNATTACKABLE changes must also be observed at cast time"
    );
}
