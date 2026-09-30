//! Spell scenarios for [`super`].
//!
//! Split out of map_manager_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn health_aura_state_like_cpp_matches_cpp_modify_aura_state() {
    // Regression for the world-entry ERROR #132 client crash: every creature
    // CREATE block must carry UNIT_FIELD_AURASTATE matching C++ Unit::Update ->
    // ModifyAuraState (Unit.cpp:469-476). A full-HP alive creature yields
    // 0x00D00000 (bits 20|22|23 = WOUND_HEALTH_20_80 | HEALTHY_75 | WOUND_HEALTH_35_80).
    // The client tests bit 0x100000 of this field on a per-frame tick; 0 crashed it.
    assert_eq!(
        WorldCreature::health_aura_state_like_cpp(100, 100, true),
        0x00D0_0000,
        "full-HP alive creature must match C++ 0x00D00000"
    );
    // Dead unit / zero max: no aura state (C++ only runs ModifyAuraState if IsAlive).
    assert_eq!(WorldCreature::health_aura_state_like_cpp(0, 100, false), 0);
    assert_eq!(WorldCreature::health_aura_state_like_cpp(50, 0, true), 0);
    // Low health (<=20%): WOUNDED_20/25/35 + WOUND_HEALTH_20_80 + WOUND_HEALTH_35_80
    // bits set, HEALTHY_75 clear. Must include the crash bit 0x100000.
    let low = WorldCreature::health_aura_state_like_cpp(10, 100, true);
    assert_ne!(
        low & 0x0010_0000,
        0,
        "WOUND_HEALTH_20_80 (0x100000) set at low HP"
    );
    assert_eq!(low & 0x0040_0000, 0, "HEALTHY_75 clear at low HP");
    // Mid health (50%): none of the threshold states (not <35, not >75, not <20/>80).
    assert_eq!(WorldCreature::health_aura_state_like_cpp(50, 100, true), 0);
}
#[test]
fn reversed_damage_bounds_reject_and_tombstone_only_spell_rng_authority() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 70009);
    let mut creature = test_creature(guid);
    creature.creature.ai_ownership_mut().min_damage = 10;
    creature.creature.ai_ownership_mut().max_damage = 5;

    assert_eq!(creature.roll_damage(), None);
    assert!(!creature.runtime_rng_authority_complete_like_cpp());
    assert_eq!(creature.random_creature_spell_hit_roll_like_cpp(), None);
    assert!(
        creature
            .pick_random_destination_from_current_position_like_cpp(12.0)
            .is_some(),
        "the conservative spell-RNG tombstone must not freeze legacy movement"
    );
}
/// Smoke: `RecipientRule::MapBroadcastVisible` stores map_id and instance_id.
#[test]
fn recipient_rule_map_broadcast_visible_stores_fields() {
    let rule = RecipientRule::MapBroadcastVisible {
        map_id: 0,
        instance_id: 5,
    };

    if let RecipientRule::MapBroadcastVisible {
        map_id,
        instance_id,
    } = rule
    {
        assert_eq!(map_id, 0);
        assert_eq!(instance_id, 5);
    } else {
        panic!("expected MapBroadcastVisible");
    }
}
