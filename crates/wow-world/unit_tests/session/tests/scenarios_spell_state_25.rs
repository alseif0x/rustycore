//! Session scenarios exercising the represented unit aura state and the spell
//! damage percentage terms that read it (`Unit::HasAuraState`,
//! `Unit::SpellDamagePctDone`).
//!
//! Split out of scenarios_spell_state_11.rs when that file reached the
//! physical test-file budget; assertions and registrations are unchanged and
//! the shared fixtures stay in the parent module.

use super::*;

fn represented_direct_damage_spell_like_cpp(
    spell_id: i32,
    base_damage: i32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE,
            effect_base_points: base_damage,
            ..Default::default()
        }],
    }
}

fn represented_aura_spell_like_cpp(
    spell_id: i32,
    aura_type: i32,
    misc_value: i32,
    amount: i32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_base_points: amount,
        effect_bonus_coefficient: 0.0,
        aura_type: Some(aura_type),
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_aura: aura_type,
            effect_misc_value_1: misc_value,
            effect_base_points: amount,
            ..Default::default()
        }],
    }
}

fn represented_frost_spell_misc_like_cpp(spell_ids: &[i32]) -> wow_data::SpellMiscStore {
    wow_data::SpellMiscStore::from_entries(spell_ids.iter().map(|spell_id| {
        wow_data::SpellMiscEntry {
            id: *spell_id as u32,
            spell_id: *spell_id as u32,
            school_mask: 1 << 1,
            ..Default::default()
        }
    }))
}

fn represented_direct_heal_spell_like_cpp(
    spell_id: i32,
    base_heal: i32,
    coefficient: f32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: coefficient,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL,
            effect_base_points: base_heal,
            ..Default::default()
        }],
    }
}

fn represented_ap_scaled_damage_spell_like_cpp(
    spell_id: i32,
    base_damage: i32,
    coefficient_from_ap: f32,
) -> wow_data::SpellInfo {
    let mut spell = represented_direct_damage_spell_like_cpp(spell_id, base_damage);
    if let Some(effect) = spell.effects.first_mut() {
        effect.effect_bonus_coefficient_from_ap = coefficient_from_ap;
    }
    spell
}

/// Shared fixture for the C++ `Player::UpdateEquipSpellsAtFormChange` scenarios:
/// a cat-form shapeshift aura spell plus the form store that gives form 1 a
/// `CombatRoundTime`.
fn represented_cat_form_fixture_like_cpp(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    mut spell_store: wow_data::SpellStore,
) -> (i32, u32) {
    let form_id = 1_u32;
    let shapeshift_spell_id = 90_996_i32;
    spell_store.insert(
        shapeshift_spell_id,
        represented_aura_spell_like_cpp(
            shapeshift_spell_id,
            wow_data::spell::aura_types::SPELL_AURA_MOD_SHAPESHIFT,
            form_id as i32,
            0,
        ),
    );
    session.set_spell_store(Arc::new(spell_store));
    session.set_spell_shapeshift_form_store(Arc::new(
        wow_data::SpellShapeshiftFormStore::from_entries([wow_data::SpellShapeshiftFormEntry {
            id: form_id,
            name: "Cat Form".to_string(),
            creature_type: 0,
            flags: 0,
            attack_icon_file_id: 0,
            bonus_action_bar: 0,
            combat_round_time: 1_000,
            damage_variance: 0.0,
            mount_type_id: 0,
            creature_display_id: [0; 4],
            preset_spell_id: [0; wow_data::MAX_SHAPESHIFT_SPELLS],
        }]),
    ));
    let _ = player_guid;
    (shapeshift_spell_id, form_id)
}

#[path = "scenarios_spell_state_25/attack_speed_and_form_timing.rs"]
mod attack_speed_and_form_timing;
#[path = "scenarios_spell_state_25/health_derived_aurastate.rs"]
mod health_derived_aurastate;
#[path = "scenarios_spell_state_25/shapeshift_forms_and_display_power.rs"]
mod shapeshift_forms_and_display_power;
#[path = "scenarios_spell_state_25/spell_damage_and_healing.rs"]
mod spell_damage_and_healing;
#[path = "scenarios_spell_state_25/spell_power_coefficients.rs"]
mod spell_power_coefficients;
