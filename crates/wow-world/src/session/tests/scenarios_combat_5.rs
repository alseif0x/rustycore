//! Session scenarios exercising the represented melee absorb stage (#29).
//!
//! Split out of `scenarios_combat_4.rs` before that file reached the
//! 2,000-line test-file budget; the shared fixtures stay in the parent module.

use super::*;

/// C++ `Unit::CalcAbsorbResist`'s school-absorb loop
/// (`Unit.cpp:1812-1880`) for one physical melee hit.
///
/// The loop offers the damage to each shield in
/// `Trinity::AbsorbAuraOrderPred` order, clamps a negative (infinite) shield
/// amount to zero, clamps the consumed amount to the damage left and reports
/// the removal an amount-counting shield reaches at zero. Every case here is
/// the pure rule the map-owned swing and the session's aura transition share.
#[test]
fn represented_melee_absorb_matches_calc_absorb_resist_like_cpp() {
    use crate::session_rules::{
        RepresentedAbsorbShieldLikeCpp as Shield, represented_melee_absorb_like_cpp,
    };

    let shield = |slot: u8, spell_id: i32, amount: i32| Shield {
        slot,
        effect_index: 0,
        spell_id,
        category_id: 0,
        amount,
    };

    // No shield: the damage passes through untouched and nothing is consumed.
    let none = represented_melee_absorb_like_cpp(&[], 100);
    assert_eq!(none.absorbed, 0);
    assert_eq!(none.damage, 100);
    assert!(none.consumed.is_empty());

    // Zero damage returns before the loop (`if (!damageInfo.GetDamage()) return;`).
    let zero = represented_melee_absorb_like_cpp(&[shield(0, 91_200, 500)], 0);
    assert_eq!(zero.absorbed, 0);
    assert_eq!(zero.damage, 0);
    assert!(zero.consumed.is_empty());

    // A shield larger than the hit absorbs it whole and survives with the
    // remainder.
    let partial = represented_melee_absorb_like_cpp(&[shield(3, 91_200, 30)], 10);
    assert_eq!(partial.absorbed, 10);
    assert_eq!(partial.damage, 0);
    assert_eq!(partial.consumed.len(), 1);
    assert_eq!(partial.consumed[0].slot, 3);
    assert_eq!(partial.consumed[0].consumed, 10);
    assert_eq!(partial.consumed[0].remaining, 20);
    assert!(!partial.consumed[0].removed);

    // A shield exactly the size of the hit is spent and removed.
    let exact = represented_melee_absorb_like_cpp(&[shield(4, 91_200, 10)], 10);
    assert_eq!(exact.absorbed, 10);
    assert_eq!(exact.damage, 0);
    assert_eq!(exact.consumed[0].remaining, 0);
    assert!(exact.consumed[0].removed);

    // A small shield absorbs what it can and the rest lands; the spent shield
    // is removed and C++ carries on with the reduced damage.
    let spill = represented_melee_absorb_like_cpp(&[shield(5, 91_200, 4)], 10);
    assert_eq!(spill.absorbed, 4);
    assert_eq!(spill.damage, 6);
    assert_eq!(spill.consumed[0].consumed, 4);
    assert!(spill.consumed[0].removed);

    // A negative amount is an infinite-absorb script shield: without the
    // scripts C++ clamps it to zero, so it absorbs nothing and is never
    // removed here.
    let infinite = represented_melee_absorb_like_cpp(&[shield(6, 91_200, -1)], 10);
    assert_eq!(infinite.absorbed, 0);
    assert_eq!(infinite.damage, 10);
    assert_eq!(infinite.consumed[0].consumed, 0);
    assert_eq!(infinite.consumed[0].remaining, -1);
    assert!(!infinite.consumed[0].removed);

    // `AbsorbAuraOrderPred` (`SpellAuraEffects.h:365-407`): Fel Blossom, then
    // the Ice Barrier category, then Sacrifice, then any other shield, with
    // Cauterize and Spirit of Redemption always last. The first shield in that
    // order spends first, and once the damage is gone the remaining shields are
    // not visited.
    let ordered = represented_melee_absorb_like_cpp(
        &[
            shield(7, 91_201, 10), // plain shield, rank 3
            Shield {
                slot: 8,
                effect_index: 0,
                spell_id: 91_202,
                category_id: 471, // Ice Barrier, rank 1
                amount: 10,
            },
            shield(9, 86949, 10), // Cauterize, rank 4
        ],
        10,
    );
    assert_eq!(ordered.absorbed, 10);
    assert_eq!(ordered.damage, 0);
    assert_eq!(
        ordered.consumed.len(),
        1,
        "the loop stops once the damage is absorbed"
    );
    assert_eq!(
        ordered.consumed[0].slot, 8,
        "the Ice Barrier rank spends first"
    );
    assert!(ordered.consumed[0].removed);
}

/// `Trinity::AbsorbAuraOrderPred`'s named ranks, ranked for the stable sort the
/// represented loop uses (`SpellAuraEffects.h:365-407`).
#[test]
fn represented_absorb_priority_matches_absorb_aura_order_pred_like_cpp() {
    use crate::session_rules::{
        RepresentedAbsorbShieldLikeCpp as Shield, represented_absorb_priority_like_cpp,
    };

    let shield = |spell_id: i32, category_id: u32| Shield {
        slot: 0,
        effect_index: 0,
        spell_id,
        category_id,
        amount: 0,
    };
    let fel_blossom = represented_absorb_priority_like_cpp(&shield(28527, 0));
    let ice_barrier = represented_absorb_priority_like_cpp(&shield(11426, 471));
    let sacrifice = represented_absorb_priority_like_cpp(&shield(7812, 0));
    let plain = represented_absorb_priority_like_cpp(&shield(91_203, 0));
    let cauterize = represented_absorb_priority_like_cpp(&shield(86949, 0));
    let redemption = represented_absorb_priority_like_cpp(&shield(20711, 0));
    assert!(fel_blossom < ice_barrier);
    assert!(ice_barrier < sacrifice);
    assert!(sacrifice < plain);
    assert!(plain < cauterize);
    assert!(cauterize < redemption);
}

/// C++ `Unit::CalcAbsorbResist`'s mana-shield loop (`Unit.cpp:1886-1930`) for
/// one physical melee hit.
///
/// The shield's amount caps the damage it may take, the drain is that amount
/// scaled by `SpellEffectInfo::CalcValueMultiplier` (`Amplitude`), and the
/// absorbed damage scales down by the fraction of the drain the victim's mana
/// could pay.
#[test]
fn represented_melee_mana_absorb_matches_calc_absorb_resist_like_cpp() {
    use crate::session_rules::{
        RepresentedManaShieldLikeCpp as Shield, represented_melee_mana_absorb_like_cpp,
    };

    let shield = |slot: u8, amount: i32, mana_multiplier: f32| Shield {
        slot,
        effect_index: 0,
        spell_id: 91_520,
        amount,
        mana_multiplier,
    };

    // No shield and zero damage both return before the loop.
    let none = represented_melee_mana_absorb_like_cpp(&[], 10, 100);
    assert_eq!((none.absorbed, none.damage, none.mana_spent), (0, 10, 0));
    let zero = represented_melee_mana_absorb_like_cpp(&[shield(0, 30, 1.0)], 0, 100);
    assert_eq!((zero.absorbed, zero.damage, zero.mana_spent), (0, 0, 0));

    // Plenty of mana: the whole hit is absorbed, one point of mana per point of
    // damage, and the shield keeps the remainder.
    let full = represented_melee_mana_absorb_like_cpp(&[shield(1, 30, 1.0)], 10, 100);
    assert_eq!((full.absorbed, full.damage, full.mana_spent), (10, 0, 10));
    assert_eq!(full.consumed[0].remaining, 20);
    assert!(!full.consumed[0].removed);

    // The victim can only pay part of the drain, so only that fraction is
    // absorbed (`currentAbsorb * manaTaken / manaReduction`).
    let limited = represented_melee_mana_absorb_like_cpp(&[shield(1, 30, 1.0)], 10, 3);
    assert_eq!(
        (limited.absorbed, limited.damage, limited.mana_spent),
        (3, 7, 3)
    );
    assert_eq!(limited.consumed[0].remaining, 27);

    // `Amplitude` 2 drains two mana per absorbed point.
    let doubled = represented_melee_mana_absorb_like_cpp(&[shield(1, 30, 2.0)], 10, 100);
    assert_eq!(
        (doubled.absorbed, doubled.damage, doubled.mana_spent),
        (10, 0, 20)
    );

    // The shield's own amount caps the hit and a fully spent shield is removed.
    let capped = represented_melee_mana_absorb_like_cpp(&[shield(1, 4, 1.0)], 10, 100);
    assert_eq!(
        (capped.absorbed, capped.damage, capped.mana_spent),
        (4, 6, 4)
    );
    assert_eq!(capped.consumed[0].remaining, 0);
    assert!(capped.consumed[0].removed);

    // A negative amount is an infinite shield C++ clamps to zero for safety: it
    // absorbs nothing and is never removed by this loop.
    let negative = represented_melee_mana_absorb_like_cpp(&[shield(1, -1, 1.0)], 10, 100);
    assert_eq!(
        (negative.absorbed, negative.damage, negative.mana_spent),
        (0, 10, 0)
    );
    assert_eq!(negative.consumed[0].remaining, -1);
    assert!(!negative.consumed[0].removed);

    // No mana at all: nothing is absorbed and nothing is spent.
    let dry = represented_melee_mana_absorb_like_cpp(&[shield(1, 30, 1.0)], 10, 0);
    assert_eq!((dry.absorbed, dry.damage, dry.mana_spent), (0, 10, 0));
    assert_eq!(dry.consumed[0].remaining, 30);
    assert!(!dry.consumed[0].removed);
}
