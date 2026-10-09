//! #1263 F6-8D1 regressions: the production phase path runs on canonical
//! ownership.
//!
//! D1 ports the complete per-creature phase operations onto canonical
//! `Creature`/`CreatureRuntimeLikeCpp` ownership. The legacy `WorldCreature`
//! keeps delegating entry points only, so the production tick's *decisions and
//! mutations* land on the canonical entity while the packet projection on the
//! bridge stays a projection.
//!
//! The test drives the real production melee tick with **one** creature in two
//! halves. First the canonical store is configured but holds no incarnation:
//! F6-8C's ownership gate refuses the selection and the canonical entity is
//! untouched. Then the canonical owner holds that incarnation and the same
//! operation selects and executes, consuming the canonical swing timer. Nothing
//! else differs, so the canonical ownership is the only variable.

use super::*;

/// Give the canonical owner the incarnation for one legacy representation.
///
/// The fixture inserts the canonical record directly, exactly as the loot
/// fixtures do: the shared application root only synchronizes an **existing**
/// incarnation (a missing one is `Refused`, not created), and the production
/// path that creates one is the session's own registration/publication.
fn adopt_canonical_incarnation_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) {
    let representation = manager
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .find_creature(0, 0, guid)
        .expect("the fixture registered the legacy representation")
        .creature
        .clone();
    let mut guard = canonical
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let map = guard
        .find_map_mut(0, 0)
        .expect("the fixture attached the canonical map instance");
    map.map_mut()
        .insert_map_object_record(
            wow_entities::MapObjectRecord::new_creature(representation)
                .expect("the registered creature is a valid canonical record"),
        )
        .expect("the fixture inserts the canonical incarnation");
}

/// The canonical swing timer the ported `record_swing` must rearm from
/// `UNIT_FIELD_BASEATTACKTIME`.
const D1_REARMED_SWING_TIMER_LIKE_CPP: u64 = 2_000;

#[test]
fn ported_melee_execution_consumes_the_canonical_swing_timer_in_the_production_tick_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 94_101);
    add_canonical_test_player_on_map(
        &canonical,
        player,
        Position::new(10.0, 10.0, 0.0, 0.0),
        0,
        0,
    );
    {
        let mut guard = canonical
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let victim = guard
            .find_map_mut(0, 0)
            .expect("the fixture attached the canonical map")
            .map_mut()
            .get_typed_player_mut(player)
            .expect("the fixture attached the canonical player");
        victim.unit_mut().set_max_health(100);
        victim.unit_mut().set_health(100);
    }
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(94_102);
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.seed_runtime_rng_like_cpp(0x94_102);
        })
        .unwrap();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    let canonical_swing_timer_like_cpp = |manager: &crate::map_manager::SharedMapManager| {
        manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_creature(0, 0, creature_guid)
            .expect("the fixture registered the legacy representation")
            .creature
            .ai_ownership()
            .swing_timer_ms
    };
    let projection_before = manager
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .find_creature(0, 0, creature_guid)
        .expect("the fixture registered the legacy representation")
        .create_data
        .clone();

    let refused = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );
    assert_eq!(
        refused.canonical_incarnation_rejections, 1,
        "a legacy representation with no canonical incarnation selects no swing"
    );
    assert_eq!(refused.swings_ready, 0);
    assert_eq!(
        canonical_swing_timer_like_cpp(&manager),
        0,
        "the refusal consumes no canonical swing timer"
    );

    adopt_canonical_incarnation_like_cpp(&manager, &canonical, creature_guid);

    let decided = run_legacy_creature_melee_tick_once_like_cpp(
        &manager,
        Some(&canonical),
        &Default::default(),
    );
    assert_eq!(decided.canonical_incarnation_rejections, 0);
    assert_eq!(decided.swings_ready, 1, "{decided:?}");
    assert_eq!(
        canonical_swing_timer_like_cpp(&manager),
        D1_REARMED_SWING_TIMER_LIKE_CPP,
        "the ported swing record rearms the canonical swing timer"
    );
    let after = manager
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .find_creature(0, 0, creature_guid)
        .expect("the fixture registered the legacy representation")
        .create_data
        .clone();
    assert_eq!(
        after.movement_flags, projection_before.movement_flags,
        "the phase operations keep no movement state on the packet projection"
    );
    assert_eq!(
        after.health, projection_before.health,
        "the phase operations keep no health state on the packet projection"
    );
}
