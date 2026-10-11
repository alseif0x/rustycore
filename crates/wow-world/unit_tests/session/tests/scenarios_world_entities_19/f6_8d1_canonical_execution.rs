//! #1263 F6-8D1 regressions: the production phase path runs on canonical
//! ownership.
//!
//! D1 ports the complete per-creature phase operations onto canonical
//! `Creature`/`CreatureRuntimeLikeCpp` ownership, so the production phase's
//! *decisions and mutations* land on the canonical entity. The test drives the
//! creature melee phase with **one** creature and asserts the swing consumes
//! and rearms the canonical swing timer. (#1263 F6-8E deleted the F6-8C
//! ownership-refusal half: no legacy copy without an incarnation can exist.)

use super::*;

/// The canonical swing timer the ported `record_swing` must rearm from
/// `UNIT_FIELD_BASEATTACKTIME`.
const D1_REARMED_SWING_TIMER_LIKE_CPP: u64 = 2_000;

#[test]
fn ported_melee_execution_consumes_the_canonical_swing_timer_in_the_production_tick_like_cpp() {
    let manager = shared_map_manager();
    let canonical = Arc::clone(&manager);
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
    let manager = session
        .core
        .canonical_map_manager
        .clone()
        .unwrap_or_else(|| manager.clone());
    register_test_creature(&mut session, manager.clone(), creature_guid, 25);
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature.enter_combat(player);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            creature.seed_runtime_rng_like_cpp(0x94_102);
        })
        .unwrap();

    let canonical_swing_timer_like_cpp = |manager: &SharedCanonicalMapManager| {
        manager
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_creature(0, 0, creature_guid)
            .expect("the fixture registered the incarnation")
            .creature
            .ai_ownership()
            .swing_timer_ms
    };
    let decided =
        run_canonical_creature_melee_phase_for_test_like_cpp(&canonical, &Default::default());
    assert_eq!(decided.swings_ready, 1, "{decided:?}");
    assert_eq!(
        canonical_swing_timer_like_cpp(&manager),
        D1_REARMED_SWING_TIMER_LIKE_CPP,
        "the ported swing record rearms the canonical swing timer"
    );
}
