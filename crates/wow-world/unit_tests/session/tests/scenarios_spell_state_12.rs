//! Session scenarios exercising spell-state and effect responsibilities.
//!
//! Split out of `session_tests.rs` under #626. Focused children group player
//! power effects, extra attacks, inebriation, reputation, and creature powers.

use super::*;

#[path = "scenarios_spell_state_12/creature_power_effects.rs"]
mod creature_power_effects;
#[path = "scenarios_spell_state_12/extra_attacks.rs"]
mod extra_attacks;
#[path = "scenarios_spell_state_12/inebriate.rs"]
mod inebriate;
#[path = "scenarios_spell_state_12/player_power_effects.rs"]
mod player_power_effects;
#[path = "scenarios_spell_state_12/reputation.rs"]
mod reputation;

#[test]
fn represented_spell_positivity_covers_common_cpp_buffs_and_debuffs() {
    let mut periodic_heal = threat_spell_info_like_cpp(
        18_146,
        wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        10,
    );
    periodic_heal.effects[0].effect_aura = wow_data::spell::aura_types::SPELL_AURA_PERIODIC_HEAL;
    periodic_heal.effects[0].implicit_target_1 = 21; // TARGET_UNIT_TARGET_ALLY
    assert!(wow_data::represented_spell_is_positive_like_cpp(
        &periodic_heal
    ));

    let mut absorb = periodic_heal.clone();
    absorb.effects[0].effect_aura = wow_data::spell::aura_types::SPELL_AURA_SCHOOL_ABSORB;
    assert!(wow_data::represented_spell_is_positive_like_cpp(&absorb));

    let mut stat_buff = periodic_heal.clone();
    stat_buff.effects[0].effect_aura = wow_data::spell::aura_types::SPELL_AURA_MOD_STAT;
    assert!(wow_data::represented_spell_is_positive_like_cpp(&stat_buff));
    stat_buff.effects[0].effect_base_points = -10;
    assert!(!wow_data::represented_spell_is_positive_like_cpp(
        &stat_buff
    ));

    let mut enemy_periodic_damage = periodic_heal;
    enemy_periodic_damage.effects[0].effect_aura =
        wow_data::spell::aura_types::SPELL_AURA_PERIODIC_DAMAGE;
    enemy_periodic_damage.effects[0].implicit_target_1 = 6; // TARGET_UNIT_TARGET_ENEMY
    assert!(!wow_data::represented_spell_is_positive_like_cpp(
        &enemy_periodic_damage
    ));
}
