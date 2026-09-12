//! #771 — the canonical Player owns its queued spell-cast request.
//!
//! C++ `Player` holds `_pendingSpellCastRequest` (`Player.h:3154`) and performs
//! the transitions itself: `RequestSpellCast` (`Player.cpp:29078`), which
//! cancels the request it replaces (`:29082`), `CancelPendingCastRequest`
//! (`:29091`), which reads the cast id and spell id into `CastFailed` before
//! clearing the slot (`:29098`), and `ExecutePendingSpellCastRequest`
//! (`:29122`), which consumes the request it is holding.

use crate::{PendingSpellCastRequestLikeCpp, Player};
use wow_core::ObjectGuid;

fn caster() -> ObjectGuid {
    ObjectGuid::create_player(1, 77)
}

fn request(cast_id: i64, spell_id: i32) -> PendingSpellCastRequestLikeCpp {
    PendingSpellCastRequestLikeCpp {
        cast_id: ObjectGuid::new(6, cast_id),
        spell_id,
        casting_unit_guid: caster(),
        target_guid: caster(),
        target_data: Default::default(),
        spell_visual: Default::default(),
        metadata: Default::default(),
    }
}

fn player() -> Player {
    Player::new(Some(1), false)
}

#[test]
fn a_fresh_player_has_no_queued_cast_like_cpp() {
    let player = player();

    assert_eq!(player.pending_spell_cast_like_cpp(), None);
    assert_eq!(player.pending_spell_cast_snapshot_like_cpp(), None);
}

#[test]
fn the_first_request_replaces_nothing_like_cpp() {
    let mut player = player();

    assert_eq!(player.request_spell_cast_like_cpp(request(1, 133)), None);
    assert_eq!(
        player.pending_spell_cast_like_cpp().map(|r| r.spell_id),
        Some(133)
    );
}

#[test]
fn a_second_request_answers_the_one_it_replaces_like_cpp_request_spell_cast() {
    let mut player = player();
    player.request_spell_cast_like_cpp(request(1, 133));

    let replaced = player.request_spell_cast_like_cpp(request(2, 144));

    assert_eq!(replaced.map(|r| r.cast_id), Some(ObjectGuid::new(6, 1)));
    assert_eq!(
        player.pending_spell_cast_like_cpp().map(|r| r.cast_id),
        Some(ObjectGuid::new(6, 2))
    );
}

#[test]
fn cancelling_answers_the_waiting_request_and_clears_the_slot_like_cpp() {
    let mut player = player();
    player.request_spell_cast_like_cpp(request(1, 133));

    let cancelled = player.cancel_pending_spell_cast_like_cpp();

    assert_eq!(cancelled.map(|r| r.spell_id), Some(133));
    assert_eq!(player.pending_spell_cast_like_cpp(), None);
}

#[test]
fn cancelling_an_empty_slot_is_the_cpp_early_return() {
    let mut player = player();

    assert_eq!(player.cancel_pending_spell_cast_like_cpp(), None);
    assert_eq!(player.pending_spell_cast_like_cpp(), None);
}

#[test]
fn the_matching_take_consumes_the_request_it_planned_for_like_cpp() {
    let mut player = player();
    player.request_spell_cast_like_cpp(request(1, 133));

    let taken =
        player.take_matching_pending_spell_cast_like_cpp(ObjectGuid::new(6, 1), 133, caster());

    assert_eq!(taken.map(|r| r.cast_id), Some(ObjectGuid::new(6, 1)));
    assert_eq!(player.pending_spell_cast_like_cpp(), None);
}

#[test]
fn a_request_that_arrived_since_is_not_consumed_by_the_earlier_plan_like_cpp() {
    let mut player = player();
    player.request_spell_cast_like_cpp(request(1, 133));
    player.request_spell_cast_like_cpp(request(2, 133));

    let taken =
        player.take_matching_pending_spell_cast_like_cpp(ObjectGuid::new(6, 1), 133, caster());

    assert_eq!(taken, None);
    assert_eq!(
        player.pending_spell_cast_like_cpp().map(|r| r.cast_id),
        Some(ObjectGuid::new(6, 2))
    );
}

#[test]
fn each_part_of_the_cpp_identity_is_required_for_the_matching_take() {
    let other = ObjectGuid::create_player(1, 78);
    for (cast_id, spell_id, unit) in [
        (ObjectGuid::new(6, 9), 133, caster()),
        (ObjectGuid::new(6, 1), 144, caster()),
        (ObjectGuid::new(6, 1), 133, other),
    ] {
        let mut player = player();
        player.request_spell_cast_like_cpp(request(1, 133));

        assert_eq!(
            player.take_matching_pending_spell_cast_like_cpp(cast_id, spell_id, unit),
            None
        );
        assert!(player.pending_spell_cast_like_cpp().is_some());
    }
}

#[test]
fn the_matching_take_on_an_empty_slot_answers_nothing() {
    let mut player = player();

    assert_eq!(
        player.take_matching_pending_spell_cast_like_cpp(ObjectGuid::new(6, 1), 133, caster()),
        None
    );
}

#[test]
fn the_snapshot_is_a_copy_the_caller_owns() {
    let mut player = player();
    player.request_spell_cast_like_cpp(request(1, 133));

    let mut snapshot = player
        .pending_spell_cast_snapshot_like_cpp()
        .expect("queued");
    snapshot.spell_id = 999;

    assert_eq!(
        player.pending_spell_cast_like_cpp().map(|r| r.spell_id),
        Some(133)
    );
}
