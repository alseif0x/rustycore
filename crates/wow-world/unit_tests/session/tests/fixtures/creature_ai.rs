//! Creature spell, wire, and melee fixtures.
//!
//! These builders retain the original session-test behavior and are
//! visible only within the parent `session::tests` subtree.

use super::*;

pub(in crate::session::tests) fn aggro_hostile_config_with_rate_like_cpp(
    creature_aggro_rate: f32,
) -> CreatureAggroConfigLikeCpp {
    CreatureAggroConfigLikeCpp {
        creature_aggro_rate,
        ..aggro_hostile_config_like_cpp()
    }
}

pub(in crate::session::tests) fn creature_spell_bound_hook_tick_fixture_like_cpp(
    ai_name: &str,
    passive: bool,
    spell_id: i32,
    creature_counter: i64,
    victim_counter: i64,
) -> (
    SharedCanonicalMapManager,
    SharedCanonicalMapManager,
    CreatureAggroConfigLikeCpp,
    ObjectGuid,
) {
    let manager = shared_map_manager();
    let canonical = Arc::clone(&manager);
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(creature_counter);
    let victim_guid = ObjectGuid::create_player(1, victim_counter);
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.set_ai_identity_names_runtime_like_cpp(ai_name, String::new());
            creature.set_spell(0, u32::try_from(spell_id).unwrap());
            creature.enter_combat(victim_guid);
            creature.ai_ownership_mut().last_swing_ms = 0;
            creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let mut config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 6, 0),
        passive,
        30.0,
    );
    config.spell_script_exact_spell_ids_like_cpp =
        Some(Arc::new(BTreeSet::from([u32::try_from(spell_id).unwrap()])));
    (manager, canonical, config, creature_guid)
}

pub(in crate::session::tests) fn decode_atomic_creature_spell_wire_pair_like_cpp(
    event: &RuntimeEvent,
) -> (
    CreatureSpellWireHeaderLikeCpp,
    CreatureSpellWireHeaderLikeCpp,
) {
    let basic_go_packet_bytes = match &event.recipients {
        crate::runtime_plan::RecipientRule::NearbyVisibleDurableSpellCast {
            basic_go_packet_bytes,
            ..
        } => basic_go_packet_bytes,
        _ => panic!("creature spell START/GO must use the atomic cast recipient rule"),
    };
    (
        decode_creature_spell_wire_header_like_cpp(&event.packet_bytes),
        decode_creature_spell_wire_header_like_cpp(basic_go_packet_bytes),
    )
}
