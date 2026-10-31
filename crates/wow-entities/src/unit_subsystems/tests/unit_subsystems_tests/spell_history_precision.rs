use super::*;
use std::collections::VecDeque;

#[test]
fn precise_spell_history_update_preserves_equality_and_expires_held_month_deadline() {
    const DEADLINE_MS: u64 = 1_000;
    const DEADLINE_NS: u128 = DEADLINE_MS as u128 * 1_000_000;
    // C++ represents held cooldowns with +MONTH (`SpellHistory.cpp:35,424-428`).
    const HELD_DEADLINE_MS: u64 = 30 * 24 * 60 * 60 * 1_000;
    const HELD_DEADLINE_NS: u128 = HELD_DEADLINE_MS as u128 * 1_000_000;

    let mut history = SpellHistory::default();
    for (spell_id, category_id, cooldown_end_ms, on_hold) in
        [(1, 10, DEADLINE_MS, false), (2, 20, HELD_DEADLINE_MS, true)]
    {
        history.cooldowns.insert(
            spell_id,
            SpellCooldown {
                spell_id,
                item_id: 0,
                cooldown_end_ms,
                category_id,
                category_end_ms: cooldown_end_ms,
                on_hold,
            },
        );
        history.category_cooldowns.insert(category_id, spell_id);
    }
    history.charges.insert(
        30,
        VecDeque::from([SpellChargeState {
            recharge_start_ms: DEADLINE_MS - 1,
            recharge_end_ms: DEADLINE_MS,
        }]),
    );
    history.charges.insert(
        31,
        VecDeque::from([SpellChargeState {
            recharge_start_ms: DEADLINE_MS,
            recharge_end_ms: DEADLINE_MS + 1,
        }]),
    );

    history.update_at_unix_nanos(DEADLINE_NS);

    assert_eq!(history.cooldowns.len(), 2);
    assert_eq!(history.category_cooldowns.len(), 2);
    assert!(history.charges[&30].is_empty());
    assert_eq!(history.charges[&31].len(), 1);

    history.update_at_unix_nanos(DEADLINE_NS + 1);

    assert!(!history.cooldowns.contains_key(&1));
    assert!(!history.category_cooldowns.contains_key(&10));
    assert!(history.cooldowns[&2].on_hold);
    assert_eq!(history.cooldowns[&2].cooldown_end_ms, HELD_DEADLINE_MS);
    assert_eq!(history.category_cooldowns.get(&20), Some(&2));
    assert_eq!(history.charges[&31].len(), 1);

    history.update_at_unix_nanos(HELD_DEADLINE_NS);

    assert!(history.cooldowns.contains_key(&2));
    assert!(history.category_cooldowns.contains_key(&20));

    history.update_at_unix_nanos(HELD_DEADLINE_NS + 1);

    assert!(history.cooldowns.is_empty());
    assert!(history.category_cooldowns.is_empty());
}
