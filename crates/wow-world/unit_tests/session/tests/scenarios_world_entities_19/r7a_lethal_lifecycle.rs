//! F6-7 R7a regressions: the lethal operation through the gated mutation root.
//!
//! The reviewer's named silent break for R7: a lethal operation must reach the
//! canonical incarnation and fire its AI/combat cleanup, loot lifecycle,
//! corpse/respawn timers and respawn-save intent exactly once — not zero times
//! because the gate refused an owned representation, and not twice because the
//! legacy mutation and its canonical synchronization both published. A stale
//! replay must fire nothing at all.

use super::r7a_canonical_mutation::*;
use super::*;

/// The reviewer's named silent break: a lethal operation must fire its AI/combat
/// cleanup, loot lifecycle, corpse/respawn timers and respawn-save intent
/// exactly once, and a stale replay of it must fire nothing.
#[test]
fn lethal_kill_operation_fires_each_lifecycle_fact_exactly_once_like_cpp() {
    let guid = test_creature_guid(91_608);
    let attacker = ObjectGuid::create_player(1, 91_609);
    let (mut session, manager, canonical) = mirrored_session_like_cpp(guid, 100);
    let loot_lifecycle_before = legacy_creature_like_cpp(&manager, guid)
        .expect("legacy representation")
        .loot_lifecycle_revision_like_cpp();

    // The production lethal closure (`damage_and_combat_application.rs:358`,
    // `creature_melee_tick.rs:80`): apply the lethal damage through the gated
    // root, then run the production kill-hook completion.
    let mut callback_runs = 0_usize;
    let died = session
        .mutate_world_creature(guid, |creature| {
            callback_runs += 1;
            creature.enter_combat(attacker);
            creature.take_damage_before_death_state_like_cpp(1_000)
        })
        .expect("the lethal mutation is admitted against the live incarnation");
    assert!(died, "the lethal damage kills the creature");
    assert_eq!(
        callback_runs, 1,
        "the lethal mutation executes exactly once"
    );

    let completed = session
        .complete_represented_creature_death_state_after_kill_hooks_like_cpp(attacker, guid)
        .is_some();
    assert!(
        completed,
        "the canonical incarnation applies the lethal completion"
    );

    // AI/combat cleanup, loot lifecycle and the kill hooks: each recorded fact
    // once, not zero times and not twice.
    assert_eq!(
        lethal_event_counts_like_cpp(&session, guid),
        [1, 1, 1, 1, 1, 1],
        "one lethal operation fires each kill fact exactly once"
    );

    let legacy = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    let canonical_creature = canonical_creature_like_cpp(&canonical, guid).expect("canonical");
    assert!(legacy.unit().is_dead());
    assert_eq!(
        observables_like_cpp(&legacy),
        observables_like_cpp(&canonical_creature),
        "the lethal result is canonical, not legacy-only"
    );
    assert!(
        legacy
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(canonical_creature.loot_authority_like_cpp()),
        "the lethal operation keeps one loot authority"
    );
    assert!(
        legacy
            .unit()
            .shares_health_state_revision_authority_like_cpp(
                &canonical_creature
                    .unit()
                    .health_state_revision_authority_like_cpp()
            ),
        "the lethal operation keeps one health timeline"
    );
    assert!(legacy.ai_ownership().combat_target.is_none());
    assert!(legacy.ai_ownership().move_target.is_none());
    assert!(legacy.unit().attacking().is_none());
    assert_eq!(
        legacy.loot_lifecycle_revision_like_cpp(),
        loot_lifecycle_before + 1,
        "the loot lifecycle advances exactly once"
    );
    let death_time_ms = legacy
        .ai_ownership()
        .death_time_ms
        .expect("the death time is stamped once");
    let corpse_despawn_at_ms = legacy
        .ai_ownership()
        .corpse_despawn_at_ms
        .expect("the corpse timer is stamped once");
    assert_eq!(
        corpse_despawn_at_ms,
        death_time_ms + u64::from(legacy.corpse_delay()) * 1_000,
        "the corpse timer is one consistent stamp, not two"
    );
    assert_eq!(
        legacy.respawn_delay(),
        u32::try_from(legacy.ai_ownership().respawn_time_secs).unwrap_or(u32::MAX),
        "the respawn delay is stamped once"
    );
    assert!(
        legacy.runtime_state().save_respawn_requested,
        "the respawn-save intent is raised"
    );
    assert!(
        legacy.respawn_time() > 0,
        "the respawn schedule is recorded once"
    );

    // Stale replay: the canonical incarnation advances without the
    // representation, so the replay is refused and fires nothing.
    let counts_before_replay = lethal_event_counts_like_cpp(&session, guid);
    let legacy_before_replay = observables_like_cpp(
        &legacy_creature_like_cpp(&manager, guid).expect("legacy representation"),
    );
    advance_canonical_max_health_like_cpp(
        &canonical,
        guid,
        canonical_creature.unit().data().max_health + 10,
    );
    let canonical_before_replay =
        observables_like_cpp(&canonical_creature_like_cpp(&canonical, guid).expect("canonical"));
    let replay_runs = {
        let mut replay_runs = 0_usize;
        let replayed = session
            .mutate_world_creature(guid, |creature| {
                replay_runs += 1;
                creature.complete_death_state_after_kill_hooks_like_cpp();
            })
            .is_some();
        assert!(!replayed, "a stale lethal replay is refused");
        replay_runs
    };
    assert_eq!(replay_runs, 0, "the refused replay is never invoked");
    assert!(
        session
            .complete_represented_creature_death_state_after_kill_hooks_like_cpp(attacker, guid)
            .is_none(),
        "the stale kill-hook completion publishes nothing"
    );
    assert_eq!(
        lethal_event_counts_like_cpp(&session, guid),
        counts_before_replay,
        "a stale replay fires no kill fact again"
    );
    assert_eq!(
        observables_like_cpp(
            &legacy_creature_like_cpp(&manager, guid).expect("legacy representation")
        ),
        legacy_before_replay,
        "a stale replay leaves the representation's corpse and timers untouched"
    );
    assert_eq!(
        observables_like_cpp(&canonical_creature_like_cpp(&canonical, guid).expect("canonical")),
        canonical_before_replay,
        "a stale replay leaves the canonical incarnation untouched"
    );
}
