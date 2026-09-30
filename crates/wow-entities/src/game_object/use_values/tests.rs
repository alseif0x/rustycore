use super::*;
use std::panic::{AssertUnwindSafe, catch_unwind};

// Independent test-owned inputs for the borrowed engine, not a Session fixture.
#[derive(Debug, PartialEq, Eq)]
struct Values {
    loot_state: Option<LootState>,
    loot_state_unit_guid: ObjectGuid,
    go_state: Option<GoState>,
    prev_go_state: Option<GoState>,
    flags: u32,
    cooldown: Option<Instant>,
    go_type: Option<u8>,
    trap_source: Option<TrapUseSource>,
    slots: Vec<Option<ObjectGuid>>,
}

impl Values {
    fn empty() -> Self {
        Self {
            loot_state: None,
            loot_state_unit_guid: ObjectGuid::EMPTY,
            go_state: None,
            prev_go_state: None,
            flags: 0,
            cooldown: None,
            go_type: None,
            trap_source: None,
            slots: Vec::new(),
        }
    }

    fn borrow(&mut self) -> GameObjectUseValues<'_> {
        GameObjectUseValues::borrow(
            &mut self.loot_state,
            &mut self.loot_state_unit_guid,
            &mut self.go_state,
            &mut self.prev_go_state,
            &mut self.flags,
            &mut self.cooldown,
            &mut self.go_type,
            &mut self.trap_source,
            &mut self.slots,
        )
    }

    fn armed_door(loot_state: Option<LootState>, deadline: Instant) -> Self {
        let mut values = Self::empty();
        values.loot_state = loot_state;
        values.loot_state_unit_guid = ObjectGuid::new(2, 10);
        values.go_state = Some(GoState::Active);
        values.prev_go_state = Some(GoState::Destroyed);
        values.flags = GO_FLAG_IN_USE | 0x80;
        values.cooldown = Some(deadline);
        values
    }
}

fn trap_source(spell_id: u32, cooldown_secs: u32, charges: u32) -> TrapUseSource {
    TrapUseSource {
        radius: 3,
        spell_id,
        charges,
        cooldown_secs,
        start_delay_secs: 7,
        ignore_totems: true,
        check_all_units: false,
    }
}

fn chair_source(slots: u32, height: u32, event: u32) -> ChairUseSource {
    ChairUseSource {
        chair_slots: slots,
        chair_height: height,
        triggered_event_id: event,
    }
}

#[test]
fn zero_cooldown_does_not_prepare_clock_or_entry() {
    let mut preparations = 0;
    let outcome = GameObjectUseValues::apply_cooldown(0, || {
        preparations += 1;
        panic!("zero cooldown must not reach clock or entry preparation")
    });
    assert_eq!(outcome, CooldownOutcome::NoCooldown);
    assert_eq!(preparations, 0);
}

#[test]
fn cooldown_prepares_once_and_accepts_exact_deadline() {
    let now = Instant::now();
    let deadline = now + Duration::from_secs(1);
    let mut cooldown = Some(deadline);
    let mut preparations = 0;
    let rejected = GameObjectUseValues::apply_cooldown(5, || {
        preparations += 1;
        (now, &mut cooldown)
    });
    assert_eq!(rejected, CooldownOutcome::Rejected);
    assert_eq!(cooldown, Some(deadline));
    assert_eq!(preparations, 1);
    let started = GameObjectUseValues::apply_cooldown(5, || {
        preparations += 1;
        (deadline, &mut cooldown)
    });
    assert_eq!(started, CooldownOutcome::Started { cooldown_secs: 5 });
    assert_eq!(cooldown, Some(deadline + Duration::from_secs(5)));
    assert_eq!(preparations, 2);
}

#[test]
fn door_none_defaults_and_ready_toggle_keep_supplied_restore_time() {
    let now = Instant::now();
    let user = ObjectGuid::new(2, 20);
    let mut values = Values::empty();
    values.flags = 0x80;
    assert_eq!(
        values.borrow().use_door_or_button(user, 1234, now),
        Some(GoState::Active)
    );
    assert_eq!(values.prev_go_state, Some(GoState::Ready));
    assert_eq!(values.go_state, Some(GoState::Active));
    assert_eq!(values.loot_state, Some(LootState::Activated));
    assert_eq!(values.loot_state_unit_guid, user);
    assert_eq!(values.flags, 0x80 | GO_FLAG_IN_USE);
    assert_eq!(values.cooldown, Some(now + Duration::from_millis(1234)));

    values.loot_state = Some(LootState::Ready);
    assert_eq!(
        values.borrow().use_door_or_button(user, 0, now),
        Some(GoState::Ready)
    );
    assert_eq!(values.prev_go_state, Some(GoState::Active));
    assert_eq!(values.cooldown, None);
}

#[test]
fn door_nonready_rejection_leaves_all_borrowed_values_unchanged() {
    let now = Instant::now();
    for loot_state in [
        LootState::NotReady,
        LootState::Activated,
        LootState::JustDeactivated,
    ] {
        let mut values = Values::armed_door(Some(loot_state), now);
        assert_eq!(
            values
                .borrow()
                .use_door_or_button(ObjectGuid::EMPTY, 1, now),
            None
        );
        assert_eq!(values, Values::armed_door(Some(loot_state), now));
    }
}

#[test]
fn reset_none_restores_default_without_changing_other_values() {
    let now = Instant::now();
    let mut values = Values::armed_door(None, now);
    values.prev_go_state = None;
    assert_eq!(values.borrow().reset_door_or_button(), Some(GoState::Ready));
    assert_eq!(values.go_state, Some(GoState::Ready));
    assert_eq!(values.loot_state, Some(LootState::JustDeactivated));
    assert_eq!(values.cooldown, None);
    assert_eq!(values.flags, 0x80);
    assert_eq!(values.loot_state_unit_guid, ObjectGuid::new(2, 10));
    assert_eq!(values.prev_go_state, None);
}

#[test]
fn reset_ready_and_just_deactivated_are_complete_noops() {
    let now = Instant::now();
    for loot_state in [LootState::Ready, LootState::JustDeactivated] {
        let mut values = Values::armed_door(Some(loot_state), now);
        assert_eq!(values.borrow().reset_door_or_button(), None);
        assert_eq!(values, Values::armed_door(Some(loot_state), now));
    }
}

#[test]
fn standalone_tick_expires_at_equality_without_batch_type_gate() {
    let now = Instant::now();
    let deadline = now + Duration::from_millis(10);
    let mut values = Values::armed_door(None, deadline);
    assert_eq!(values.borrow().tick_door_or_button(now), None);
    assert_eq!(values, Values::armed_door(None, deadline));
    assert_eq!(
        values.borrow().tick_door_or_button(deadline),
        Some(GoState::Destroyed)
    );
    assert_eq!(values.go_state, Some(GoState::Destroyed));
    assert_eq!(values.loot_state, Some(LootState::JustDeactivated));
    assert_eq!(values.cooldown, None);
    assert_eq!(values.go_type, None);
    assert_eq!(values.borrow().tick_door_or_button(deadline), None);
}

#[test]
fn batch_door_reset_requires_type_activation_and_due_deadline() {
    let now = Instant::now();
    let activated = Some(LootState::Activated);
    for go_type in [GAMEOBJECT_TYPE_DOOR, GAMEOBJECT_TYPE_BUTTON] {
        assert!(GameObjectUseValues::door_reset_due(
            Some(go_type as u8),
            activated,
            Some(now),
            now
        ));
    }
    for (go_type, loot_state, deadline) in [
        (None, activated, Some(now)),
        (Some(GAMEOBJECT_TYPE_TRAP as u8), activated, Some(now)),
        (Some(GAMEOBJECT_TYPE_DOOR as u8), None, Some(now)),
        (
            Some(GAMEOBJECT_TYPE_DOOR as u8),
            Some(LootState::Ready),
            Some(now),
        ),
        (Some(GAMEOBJECT_TYPE_DOOR as u8), activated, None),
        (
            Some(GAMEOBJECT_TYPE_DOOR as u8),
            activated,
            Some(now + Duration::from_secs(1)),
        ),
    ] {
        assert!(!GameObjectUseValues::door_reset_due(
            go_type, loot_state, deadline, now
        ));
    }
}

#[test]
fn trap_emits_spell_then_cooldown_and_deactivates_one_charge() {
    let now = Instant::now();
    let source = trap_source(100, 2, 1);
    let mut values = Values::empty();
    let mut effects = Vec::new();
    assert!(
        values
            .borrow()
            .use_trap(source, now, |effect| effects.push(effect))
    );
    assert_eq!(
        effects,
        vec![
            TrapUseEffect::CastSpell { spell_id: 100 },
            TrapUseEffect::CooldownStarted { cooldown_secs: 2 },
        ]
    );
    assert_eq!(values.go_type, Some(GAMEOBJECT_TYPE_TRAP as u8));
    assert_eq!(values.trap_source, Some(source));
    assert_eq!(values.cooldown, Some(now + Duration::from_secs(2)));
    assert_eq!(values.loot_state, Some(LootState::JustDeactivated));
}

#[test]
fn trap_future_deadline_rejects_before_source_and_type_writes() {
    let now = Instant::now();
    let deadline = now + Duration::from_secs(1);
    let mut values = Values::armed_door(Some(LootState::Activated), deadline);
    let mut effects = Vec::new();
    assert!(
        !values
            .borrow()
            .use_trap(trap_source(100, 2, 1), now, |effect| effects.push(effect))
    );
    assert_eq!(effects, vec![TrapUseEffect::CooldownRejected]);
    assert_eq!(
        values,
        Values::armed_door(Some(LootState::Activated), deadline)
    );
}

#[test]
fn trap_spell_emission_interruption_precedes_deadline_write() {
    let now = Instant::now();
    let source = trap_source(100, 2, 1);
    let mut values = Values::empty();
    let mut effects = Vec::new();
    let result = catch_unwind(AssertUnwindSafe(|| {
        values.borrow().use_trap(source, now, |effect| {
            effects.push(effect);
            panic!("interrupt spell emission");
        });
    }));
    assert!(result.is_err());
    assert_eq!(effects, vec![TrapUseEffect::CastSpell { spell_id: 100 }]);
    assert_eq!(values.go_type, Some(GAMEOBJECT_TYPE_TRAP as u8));
    assert_eq!(values.trap_source, Some(source));
    assert_eq!(values.cooldown, None);
    assert_eq!(values.loot_state, None);
}

#[test]
fn trap_cooldown_emission_interruption_precedes_deactivation() {
    let now = Instant::now();
    let mut values = Values::empty();
    let mut effects = Vec::new();
    let result = catch_unwind(AssertUnwindSafe(|| {
        values
            .borrow()
            .use_trap(trap_source(100, 2, 1), now, |effect| {
                let interrupt = matches!(effect, TrapUseEffect::CooldownStarted { .. });
                effects.push(effect);
                if interrupt {
                    panic!("interrupt cooldown emission");
                }
            });
    }));
    assert!(result.is_err());
    assert_eq!(
        effects,
        vec![
            TrapUseEffect::CastSpell { spell_id: 100 },
            TrapUseEffect::CooldownStarted { cooldown_secs: 2 },
        ]
    );
    assert_eq!(values.cooldown, Some(now + Duration::from_secs(2)));
    assert_eq!(values.loot_state, None);
}

#[test]
fn trap_zero_spell_uses_default_cooldown_and_keeps_multiple_charges_active() {
    let now = Instant::now();
    let mut values = Values::empty();
    values.cooldown = Some(now);
    values.loot_state = Some(LootState::Activated);
    let mut effects = Vec::new();
    assert!(
        values
            .borrow()
            .use_trap(trap_source(0, 0, 2), now, |effect| effects.push(effect))
    );
    assert_eq!(
        effects,
        vec![TrapUseEffect::CooldownStarted { cooldown_secs: 4 }]
    );
    assert_eq!(values.cooldown, Some(now + Duration::from_secs(4)));
    assert_eq!(values.loot_state, Some(LootState::Activated));
}

#[test]
fn chair_ties_choose_last_free_slot_and_occupied_slots_are_retained() {
    let mut values = Values::empty();
    let first = ObjectGuid::new(2, 30);
    let second = ObjectGuid::new(2, 31);
    let source = chair_source(3, 2, 77);
    let placement =
        GameObjectUseValues::use_chair(first, Position::ZERO, Position::ZERO, 0.0, source, || {
            values.borrow()
        })
        .unwrap();
    assert_eq!(placement.slot(), 2);
    assert_eq!(placement.teleport_position(), Position::ZERO);
    assert_eq!(placement.raw_stand_state(), 6);
    assert_eq!(placement.trigger_event(), Some(77));
    assert_eq!(values.slots, vec![None, None, Some(first)]);
    let placement =
        GameObjectUseValues::use_chair(second, Position::ZERO, Position::ZERO, 0.0, source, || {
            values.borrow()
        })
        .unwrap();
    assert_eq!(placement.slot(), 1);
    assert_eq!(values.slots, vec![None, Some(second), Some(first)]);
    values.slots[0] = Some(first);
    assert!(
        GameObjectUseValues::use_chair(second, Position::ZERO, Position::ZERO, 0.0, source, || {
            values.borrow()
        },)
        .is_none()
    );
    assert_eq!(values.slots, vec![Some(first), Some(second), Some(first)]);
}

#[test]
fn chair_clamps_initial_count_but_does_not_resize_existing_slots() {
    let player = ObjectGuid::new(2, 40);
    for (requested, expected_count) in [(0, 1), (99, 5)] {
        let mut values = Values::empty();
        let placement = GameObjectUseValues::use_chair(
            player,
            Position::ZERO,
            Position::ZERO,
            0.0,
            chair_source(requested, u32::MAX, 0),
            || values.borrow(),
        )
        .unwrap();
        assert_eq!(values.slots.len(), expected_count);
        assert_eq!(placement.slot(), (expected_count - 1) as u32);
        assert_eq!(placement.raw_stand_state(), u32::MAX);
        assert_eq!(placement.trigger_event(), None);
    }
    let mut values = Values::empty();
    values.slots = vec![None; 6];
    let placement = GameObjectUseValues::use_chair(
        player,
        Position::ZERO,
        Position::ZERO,
        0.0,
        chair_source(1, 0, 0),
        || values.borrow(),
    )
    .unwrap();
    assert_eq!(placement.slot(), 5);
    assert_eq!(values.slots.len(), 6);
    assert_eq!(values.slots[5], Some(player));
}

#[test]
fn chair_nan_distances_do_not_select_or_clear_slots() {
    let occupant = ObjectGuid::new(2, 50);
    let player = ObjectGuid::new(2, 51);
    for (player_position, object_position, size) in [
        (Position::new(f32::NAN, 0.0, 0.0, 0.0), Position::ZERO, 1.0),
        (Position::ZERO, Position::new(0.0, 0.0, 0.0, f32::NAN), 1.0),
        (Position::ZERO, Position::ZERO, f32::NAN),
    ] {
        let mut values = Values::empty();
        values.slots = vec![Some(occupant), None];
        assert!(
            GameObjectUseValues::use_chair(
                player,
                player_position,
                object_position,
                size,
                chair_source(2, 0, 0),
                || values.borrow(),
            )
            .is_none()
        );
        assert_eq!(values.slots, vec![Some(occupant), None]);
    }
}

#[test]
fn separate_borrowed_states_do_not_share_timers_or_chair_occupants() {
    let now = Instant::now();
    let first_player = ObjectGuid::new(2, 60);
    let second_player = ObjectGuid::new(2, 61);
    let mut first = Values::empty();
    let mut second = Values::empty();
    let first_view = first.borrow();
    let second_view = second.borrow();
    assert_eq!(
        first_view.use_door_or_button(first_player, 1000, now),
        Some(GoState::Active)
    );
    assert_eq!(second_view.reset_door_or_button(), Some(GoState::Ready));
    assert_eq!(first.cooldown, Some(now + Duration::from_secs(1)));
    assert_eq!(second.cooldown, None);
    assert_eq!(first.loot_state, Some(LootState::Activated));
    assert_eq!(second.loot_state, Some(LootState::JustDeactivated));
    let source = chair_source(2, 0, 0);
    assert_eq!(
        GameObjectUseValues::use_chair(
            first_player,
            Position::ZERO,
            Position::ZERO,
            0.0,
            source,
            || first.borrow(),
        )
        .unwrap()
        .slot(),
        1
    );
    assert!(second.slots.is_empty());
    assert_eq!(
        GameObjectUseValues::use_chair(
            second_player,
            Position::ZERO,
            Position::ZERO,
            0.0,
            source,
            || second.borrow(),
        )
        .unwrap()
        .slot(),
        1
    );
    assert_eq!(first.slots, vec![None, Some(first_player)]);
    assert_eq!(second.slots, vec![None, Some(second_player)]);
}
