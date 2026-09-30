use super::*;
use rand::{Rng, RngCore, SeedableRng, rngs::StdRng};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn represented_disenchant_loot_template_row_guards_match_cpp_shape() {
    let valid = LootStoreItem {
        item_id: 10940,
        reference: 0,
        chance: 100.0,
        needs_quest: false,
        loot_mode: DEFAULT_LOOT_MODE,
        group_id: 0,
        min_count: 1,
        max_count: 2,
    };
    assert!(plain_row_can_roll(&valid, true));

    let mut missing_item = valid;
    missing_item.item_id = 0;
    assert!(!plain_row_can_roll(&missing_item, true));

    let mut bad_count = valid;
    bad_count.max_count = 0;
    assert!(!plain_row_can_roll(&bad_count, true));

    let reference = LootStoreItem {
        item_id: 0,
        reference: 700,
        chance: 100.0,
        needs_quest: false,
        loot_mode: DEFAULT_LOOT_MODE,
        group_id: 0,
        min_count: 1,
        max_count: 1,
    };
    assert!(reference_row_can_roll(&reference));
}
#[test]
fn represented_disenchant_loot_template_frame_splits_group_rows_like_cpp() {
    let rows = vec![
        LootStoreItem {
            item_id: 10940,
            reference: 0,
            chance: 100.0,
            needs_quest: false,
            loot_mode: DEFAULT_LOOT_MODE,
            group_id: 0,
            min_count: 1,
            max_count: 1,
        },
        LootStoreItem {
            item_id: 10978,
            reference: 0,
            chance: 0.0,
            needs_quest: false,
            loot_mode: DEFAULT_LOOT_MODE,
            group_id: 2,
            min_count: 1,
            max_count: 1,
        },
        LootStoreItem {
            item_id: 0,
            reference: 700,
            chance: 100.0,
            needs_quest: false,
            loot_mode: DEFAULT_LOOT_MODE,
            group_id: 2,
            min_count: 1,
            max_count: 1,
        },
    ];

    let frame = disenchant_frame(rows, 0);

    assert_eq!(frame.template.entries().len(), 2);
    assert_eq!(frame.template.groups().len(), 2);
    assert_eq!(frame.template.groups()[1].equal_chanced().len(), 1);
    assert_eq!(frame.template.entries()[1].reference, 700);
    assert_eq!(frame.template.entries()[1].group_id, 2);
}
#[test]
fn represented_disenchant_group_roll_uses_caller_rng_like_cpp_count() {
    let rows = vec![LootStoreItem {
        item_id: 10940,
        reference: 0,
        chance: 100.0,
        needs_quest: false,
        loot_mode: DEFAULT_LOOT_MODE,
        group_id: 1,
        min_count: 2,
        max_count: 7,
    }];
    let frame = disenchant_frame(rows, 1);
    let group = &frame.template.groups()[0];

    let mut expected_rng = StdRng::seed_from_u64(0xD15E);
    let _expected_group = group.roll_like_cpp(DEFAULT_LOOT_MODE, &mut expected_rng, |_| true);
    let expected_count = expected_rng.gen_range(2..=7);

    let mut rng = StdRng::seed_from_u64(0xD15E);
    let row = group
        .roll_like_cpp(DEFAULT_LOOT_MODE, &mut rng, |_| true)
        .expect("group should roll the guaranteed disenchant row");
    let count = rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));

    assert_eq!(row.item_id, 10940);
    assert_eq!(count, expected_count);
}

struct TracedRng {
    inner: StdRng,
    events: Rc<RefCell<Vec<&'static str>>>,
}

impl RngCore for TracedRng {
    fn next_u32(&mut self) -> u32 {
        self.events.borrow_mut().push("draw");
        self.inner.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.events.borrow_mut().push("draw");
        self.inner.next_u64()
    }

    fn fill_bytes(&mut self, bytes: &mut [u8]) {
        self.events.borrow_mut().push("draw");
        self.inner.fill_bytes(bytes);
    }

    fn try_fill_bytes(&mut self, bytes: &mut [u8]) -> Result<(), rand::Error> {
        self.events.borrow_mut().push("draw");
        self.inner.try_fill_bytes(bytes)
    }
}

fn plain(item_id: u32, min_count: u8, max_count: u8) -> LootStoreItem {
    LootStoreItem {
        item_id,
        reference: 0,
        chance: 100.0,
        needs_quest: false,
        loot_mode: DEFAULT_LOOT_MODE,
        group_id: 0,
        min_count,
        max_count,
    }
}

fn reference(id: u32, group_id: u8, max_count: u8) -> LootStoreItem {
    LootStoreItem {
        item_id: 0,
        reference: id,
        chance: 100.0,
        needs_quest: false,
        loot_mode: DEFAULT_LOOT_MODE,
        group_id,
        min_count: 1,
        max_count,
    }
}

#[test]
fn pending_reference_keeps_the_parent_and_does_not_query_or_draw_again() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut rng = TracedRng {
        inner: StdRng::seed_from_u64(1),
        events: events.clone(),
    };
    let mut builder = DisenchantLootBuilder::new(vec![reference(700, 2, 3), plain(20, 1, 1)]);
    for _ in 0..2 {
        assert_eq!(
            builder.next_reference(
                &mut rng,
                |_| panic!("pending reference must not query an item"),
                |_| panic!("reference must not query an item rate"),
                || panic!("100 percent reference must not query its rate"),
            ),
            Some(700),
        );
        assert!(events.borrow().is_empty());
        assert_eq!(builder.frames.len(), 1);
        assert_eq!(builder.frames[0].entry_index, 1);
        assert_eq!(builder.processed_frames, 0);
        assert_eq!(builder.pending_row.unwrap().group_id, 2);
        assert_eq!(builder.pending_row.unwrap().max_count, 3);
    }
    assert!(builder.resume_reference(Vec::new(), 0.0));
    assert!(builder.pending_row.is_none());
    assert_eq!(builder.processed_frames, 1);
    assert_eq!(builder.frames.len(), 1);
    assert_eq!(
        builder.next_reference(
            &mut rng,
            |_| Some(20),
            |_| panic!("guaranteed item rate"),
            || { panic!("no further reference") }
        ),
        None,
    );
    let entries = builder.into_entries(ObjectGuid::EMPTY);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].item_id, 20);
    assert_eq!(entries[0].allowed_looters, vec![ObjectGuid::EMPTY]);
    assert_eq!(entries[0].roll_winner, ObjectGuid::EMPTY);
}

#[test]
fn nested_references_repeat_load_requests_and_resume_parents_in_lifo_order() {
    let mut rng = StdRng::seed_from_u64(2);
    let mut builder =
        DisenchantLootBuilder::new(vec![plain(10, 1, 1), reference(700, 0, 2), plain(20, 1, 1)]);
    assert_eq!(
        builder.next_reference(&mut rng, |_| Some(20), |_| 1.0, || 1.0),
        Some(700)
    );
    assert!(builder.resume_reference(
        vec![plain(30, 1, 1), reference(800, 0, 1), plain(40, 1, 1),],
        1.0
    ));
    assert_eq!(builder.frames.len(), 3);
    let mut requested = Vec::new();
    while let Some(id) = builder.next_reference(&mut rng, |_| Some(20), |_| 1.0, || 1.0) {
        requested.push(id);
        assert_eq!(id, 800);
        assert!(builder.resume_reference(vec![plain(50, 1, 1)], 1.0));
    }
    assert_eq!(requested, vec![800, 800]);
    assert_eq!(builder.processed_frames, 3);
    let entries = builder.into_entries(ObjectGuid::EMPTY);
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.item_id)
            .collect::<Vec<_>>(),
        vec![10, 30, 50, 40, 30, 50, 40, 20],
    );
}

#[test]
fn requested_group_excludes_plain_rows_and_other_groups() {
    let mut rng = StdRng::seed_from_u64(3);
    let mut builder = DisenchantLootBuilder::new(vec![reference(700, 2, 1)]);
    assert_eq!(
        builder.next_reference(&mut rng, |_| Some(20), |_| 1.0, || 1.0),
        Some(700)
    );
    let mut group_one = plain(101, 1, 1);
    group_one.group_id = 1;
    let mut group_two = plain(202, 2, 7);
    group_two.group_id = 2;
    assert!(builder.resume_reference(vec![plain(99, 1, 1), group_one, group_two], 1.0));
    let mut queries = Vec::new();
    assert_eq!(
        builder.next_reference(
            &mut rng,
            |id| {
                queries.push(id);
                Some(20)
            },
            |_| panic!("group rates are not sampled"),
            || panic!("no nested reference")
        ),
        None
    );
    assert_eq!(queries, vec![202, 202]);
    let entries = builder.into_entries(ObjectGuid::EMPTY);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].item_id, 202);
    assert!((2..=7).contains(&entries[0].quantity));
}

#[test]
fn missing_requested_group_finishes_without_metadata_or_rng_calls() {
    let events = Rc::new(RefCell::new(Vec::new()));
    let mut rng = TracedRng {
        inner: StdRng::seed_from_u64(4),
        events: events.clone(),
    };
    let mut builder = DisenchantLootBuilder::new(vec![reference(700, 7, 1)]);
    assert_eq!(
        builder.next_reference(&mut rng, |_| Some(1), |_| 1.0, || 1.0),
        Some(700)
    );
    let mut only_group = plain(10, 2, 7);
    only_group.group_id = 1;
    assert!(builder.resume_reference(vec![only_group], 1.0));
    assert_eq!(
        builder.next_reference(
            &mut rng,
            |_| panic!("group does not exist"),
            |_| panic!("no item rate"),
            || panic!("no reference rate"),
        ),
        None
    );
    assert!(events.borrow().is_empty());
    assert!(builder.into_entries(ObjectGuid::EMPTY).is_empty());
}

#[test]
fn full_template_processes_plain_entries_before_ordered_groups_with_the_same_rng() {
    let mut group_one = plain(101, 2, 7);
    group_one.group_id = 1;
    let mut group_two = plain(202, 3, 8);
    group_two.group_id = 2;
    let mut builder = DisenchantLootBuilder::new(vec![group_two, plain(99, 1, 2), group_one]);
    let mut expected_rng = StdRng::seed_from_u64(5);
    let mut expected_counts = vec![expected_rng.gen_range(1u32..=2)];
    for group in builder.frames[0].template.groups() {
        let row = group
            .roll_like_cpp(DEFAULT_LOOT_MODE, &mut expected_rng, |_| true)
            .unwrap();
        expected_counts
            .push(expected_rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count)));
    }
    let mut rng = StdRng::seed_from_u64(5);
    assert_eq!(
        builder.next_reference(
            &mut rng,
            |_| Some(20),
            |_| { panic!("all items and groups are guaranteed") },
            || panic!("no reference")
        ),
        None
    );
    let entries = builder.into_entries(ObjectGuid::EMPTY);
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.item_id)
            .collect::<Vec<_>>(),
        vec![99, 101, 202]
    );
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.quantity)
            .collect::<Vec<_>>(),
        expected_counts
    );
    assert_eq!(rng.next_u64(), expected_rng.next_u64());
}

#[test]
fn guards_and_guaranteed_rows_keep_rate_lookups_lazy() {
    let mut rolled_item = plain(12, 1, 1);
    rolled_item.chance = 50.0;
    let mut missing_item = plain(13, 1, 1);
    missing_item.chance = 50.0;
    let mut bad_count = plain(14, 1, 0);
    bad_count.chance = 50.0;
    let mut wrong_mode = plain(15, 1, 1);
    wrong_mode.loot_mode = 2;
    wrong_mode.chance = 50.0;
    let mut bad_reference = reference(601, 0, 1);
    bad_reference.min_count = 0;
    let mut wrong_reference_mode = reference(602, 0, 1);
    wrong_reference_mode.loot_mode = 2;
    let mut rolled_reference = reference(603, 0, 1);
    rolled_reference.chance = 50.0;
    let mut builder = DisenchantLootBuilder::new(vec![
        plain(11, 2, 7),
        rolled_item,
        missing_item,
        bad_count,
        wrong_mode,
        bad_reference,
        wrong_reference_mode,
        rolled_reference,
        reference(700, 0, 1),
    ]);
    let mut templates = Vec::new();
    let mut item_rates = Vec::new();
    let mut reference_rates = 0;
    let mut rng = StdRng::seed_from_u64(6);
    assert_eq!(
        builder.next_reference(
            &mut rng,
            |id| {
                templates.push(id);
                (id != 13).then_some(20)
            },
            |id| {
                item_rates.push(id);
                0.0
            },
            || {
                reference_rates += 1;
                0.0
            }
        ),
        Some(700)
    );
    assert_eq!(templates, vec![11, 11, 12, 13, 14, 15]);
    assert_eq!(item_rates, vec![12]);
    assert_eq!(reference_rates, 1);
    assert_eq!(builder.loot_items.len(), 1);
    assert_eq!(builder.loot_items[0].item_id, 11);
    let mut expected_rng = StdRng::seed_from_u64(6);
    let expected_count = expected_rng.gen_range(2u32..=7);
    let _item_chance = expected_rng.gen_range(0.0f32..100.0f32);
    let _reference_chance = expected_rng.gen_range(0.0f32..100.0f32);
    assert_eq!(builder.loot_items[0].quantity, expected_count);
    assert_eq!(rng.next_u64(), expected_rng.next_u64());
}

#[test]
fn count_draw_precedes_second_template_lookup_with_missing_or_zero_max_stack() {
    for second_lookup in [None, Some(0)] {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut rng = TracedRng {
            inner: StdRng::seed_from_u64(7),
            events: events.clone(),
        };
        let mut builder = DisenchantLootBuilder::new(vec![plain(10, 2, 7)]);
        let mut lookups = 0;
        assert_eq!(
            builder.next_reference(
                &mut rng,
                |_| {
                    events.borrow_mut().push("template");
                    lookups += 1;
                    if lookups == 1 {
                        Some(20)
                    } else {
                        second_lookup
                    }
                },
                |_| panic!("guaranteed item"),
                || panic!("no reference")
            ),
            None
        );
        let events = events.borrow();
        assert_eq!(events.first(), Some(&"template"));
        assert_eq!(events.last(), Some(&"template"));
        assert!(events.len() >= 3);
        assert!(
            events[1..events.len() - 1]
                .iter()
                .all(|event| *event == "draw")
        );
        assert_eq!(lookups, 2);
        let entries = builder.into_entries(ObjectGuid::EMPTY);
        assert!((2..=7).contains(&entries.len()));
        assert!(entries.iter().all(|entry| entry.quantity == 1));
    }
}

#[test]
fn sub_hundred_base_chance_still_draws_with_zero_negative_nan_or_large_rates() {
    for (rate, wins) in [
        (0.0, false),
        (-1.0, false),
        (f32::NAN, false),
        (2.0, true),
        (f32::INFINITY, true),
    ] {
        let mut rng = StdRng::seed_from_u64(8);
        let mut expected_rng = StdRng::seed_from_u64(8);
        let _draw = expected_rng.gen_range(0.0f32..100.0f32);
        assert_eq!(roll_chance_with_rate(60.0, rate, &mut rng), wins);
        assert_eq!(rng.next_u64(), expected_rng.next_u64());
    }
    let mut rng = StdRng::seed_from_u64(8);
    let mut expected_rng = StdRng::seed_from_u64(8);
    assert!(roll_chance_with_rate(100.0, f32::NAN, &mut rng));
    assert_eq!(rng.next_u64(), expected_rng.next_u64());
}

#[test]
fn reference_amount_keeps_fractional_truncation_and_float_cast_extremes() {
    assert_eq!(reference_max_count(3, 0.5), 1);
    assert_eq!(reference_max_count(3, 2.0), 6);
    assert_eq!(reference_max_count(3, 0.0), 0);
    assert_eq!(reference_max_count(3, -1.0), 0);
    assert_eq!(reference_max_count(3, f32::NAN), 0);
    assert_eq!(reference_max_count(3, f32::INFINITY), u32::MAX);
    let mut builder = DisenchantLootBuilder::new(vec![reference(700, 0, 3), plain(20, 1, 1)]);
    let mut rng = StdRng::seed_from_u64(9);
    assert_eq!(
        builder.next_reference(&mut rng, |_| Some(20), |_| 1.0, || 1.0),
        Some(700)
    );
    assert!(builder.resume_reference(vec![plain(10, 1, 1)], 0.5));
    assert_eq!(builder.processed_frames, 1);
    assert_eq!(builder.frames.len(), 2);
    assert_eq!(
        builder.next_reference(&mut rng, |_| Some(20), |_| 1.0, || 1.0),
        None
    );
    assert_eq!(
        builder
            .into_entries(ObjectGuid::EMPTY)
            .iter()
            .map(|entry| entry.item_id)
            .collect::<Vec<_>>(),
        vec![10, 20]
    );
}

#[test]
fn cap_stops_only_after_sixty_fifth_reference_pushes_and_keeps_partial_loot() {
    let mut builder = DisenchantLootBuilder::new(vec![plain(10, 1, 1), reference(700, 0, 1)]);
    let mut rng = StdRng::seed_from_u64(10);
    for count in 1..=64 {
        assert_eq!(
            builder.next_reference(&mut rng, |_| Some(20), |_| 1.0, || 1.0),
            Some(700)
        );
        let before = builder.frames.len();
        assert!(builder.resume_reference(vec![reference(700, 0, 1)], 1.0));
        assert_eq!(builder.processed_frames, count);
        assert_eq!(builder.frames.len(), before + 1);
    }
    assert_eq!(
        builder.next_reference(&mut rng, |_| Some(20), |_| 1.0, || 1.0),
        Some(700)
    );
    let before = builder.frames.len();
    assert!(!builder.resume_reference(vec![plain(99, 1, 1)], 2.0));
    assert_eq!(builder.processed_frames, 65);
    assert_eq!(builder.frames.len(), before + 2);
    assert!(builder.pending_row.is_none());
    let entries = builder.into_entries(ObjectGuid::EMPTY);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].item_id, 10);
}

#[test]
fn full_stack_limit_does_not_skip_later_chance_count_queries_or_references() {
    let mut later = plain(20, 2, 7);
    later.chance = 50.0;
    let mut builder =
        DisenchantLootBuilder::new(vec![plain(10, 19, 19), later, reference(700, 0, 1)]);
    let mut rng = StdRng::seed_from_u64(11);
    let queries = RefCell::new(Vec::new());
    let lookup = |id| {
        queries.borrow_mut().push(id);
        Some(1)
    };
    assert_eq!(
        builder.next_reference(&mut rng, lookup, |_| 2.0, || 1.0),
        Some(700)
    );
    assert_eq!(*queries.borrow(), vec![10, 10, 20, 20]);
    assert_eq!(builder.loot_items.len(), 18);
    assert!(builder.resume_reference(vec![plain(30, 2, 7)], 1.0));
    assert_eq!(
        builder.next_reference(
            &mut rng,
            |id| {
                queries.borrow_mut().push(id);
                Some(1)
            },
            |_| panic!("guaranteed referenced item"),
            || panic!("no nested reference")
        ),
        None
    );
    assert_eq!(*queries.borrow(), vec![10, 10, 20, 20, 30, 30]);
    let mut expected_rng = StdRng::seed_from_u64(11);
    let _first_count = expected_rng.gen_range(19u32..=19);
    let _chance = expected_rng.gen_range(0.0f32..100.0f32);
    let _later_count = expected_rng.gen_range(2u32..=7);
    let _referenced_count = expected_rng.gen_range(2u32..=7);
    assert_eq!(rng.next_u64(), expected_rng.next_u64());
    let entries = builder.into_entries(ObjectGuid::EMPTY);
    assert_eq!(entries.len(), 18);
    assert!(
        entries
            .iter()
            .all(|entry| entry.item_id == 10 && entry.quantity == 1)
    );
}

#[test]
fn stacks_preserve_defaults_then_assign_list_ids_and_winner_without_merging() {
    let mut builder = DisenchantLootBuilder::new(vec![plain(10, 7, 7), plain(10, 1, 1)]);
    let mut rng = StdRng::seed_from_u64(12);
    assert_eq!(
        builder.next_reference(&mut rng, |_| Some(3), |_| 1.0, || 1.0),
        None
    );
    assert!(builder.loot_items.iter().all(|entry| {
        entry.allowed_looters.is_empty() && entry.roll_winner == ObjectGuid::EMPTY
    }));
    let winner = ObjectGuid::create_player(1, 42);
    let entries = builder.into_entries(winner);
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.quantity)
            .collect::<Vec<_>>(),
        vec![3, 3, 1, 1]
    );
    for (index, entry) in entries.iter().enumerate() {
        assert_eq!(entry.loot_list_id, index as u8);
        assert_eq!(entry.item_id, 10);
        assert_eq!(entry.allowed_looters, vec![winner]);
        assert_eq!(entry.roll_winner, winner);
        assert_eq!(entry.random_properties_id, 0);
        assert_eq!(entry.random_properties_seed, 0);
        assert_eq!(entry.item_context, 0);
        assert_eq!(
            entry.flags,
            LootEntryFlags {
                follow_loot_rules: true,
                ..Default::default()
            }
        );
        assert!(entry.ffa_looted_by.is_empty());
        assert!(!entry.taken);
    }
}
