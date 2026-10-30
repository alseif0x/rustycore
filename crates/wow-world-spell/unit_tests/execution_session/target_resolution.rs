use super::*;

use wow_core::ObjectGuid;
use wow_data::spell::spell_effect_types as effect_types;

fn spell_info(effects: Vec<wow_data::SpellEffectInfo>) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id: 1,
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
        effects,
    }
}

fn effect(effect: u32, target_1: u32, target_2: u32) -> wow_data::SpellEffectInfo {
    wow_data::SpellEffectInfo {
        effect,
        implicit_target_1: target_1,
        implicit_target_2: target_2,
        ..Default::default()
    }
}

#[test]
fn selected_full_server_aura_adds_only_the_implicit_caster_unit_flag() {
    let selected_spell_info = spell_info(vec![effect(effect_types::SPELL_EFFECT_APPLY_AURA, 0, 1)]);
    let original = SpellTargetData {
        flags: 0x0000_0040,
        ..Default::default()
    };
    let mut target = original.clone();

    add_implicit_unit_caster_flag_for_selected_full_server_aura_like_cpp(
        &mut target,
        &selected_spell_info,
        true,
    );

    assert_eq!(
        target.flags,
        original.flags | SPELL_TARGET_FLAG_UNIT_LIKE_CPP
    );
    assert_eq!(target.unit, ObjectGuid::EMPTY);
    assert_eq!(target.item, original.item);
    assert_eq!(target.src_location, original.src_location);
    assert_eq!(target.dst_location, original.dst_location);
    assert_eq!(target.orientation, original.orientation);
    assert_eq!(target.map_id, original.map_id);
    assert_eq!(target.name, original.name);

    let mut unselected_target = original.clone();
    add_implicit_unit_caster_flag_for_selected_full_server_aura_like_cpp(
        &mut unselected_target,
        &selected_spell_info,
        false,
    );
    assert_eq!(unselected_target, original);

    let inactive_effect = spell_info(vec![effect(0, 1, 0)]);
    add_implicit_unit_caster_flag_for_selected_full_server_aura_like_cpp(
        &mut unselected_target,
        &inactive_effect,
        true,
    );
    assert_eq!(unselected_target, original);
}

#[test]
fn spell_go_hit_list_is_empty_only_for_caster_only_self_resurrection_and_stuck() {
    let target_guid = ObjectGuid::create_player(1, 77);
    let caster_only_effects = spell_info(vec![
        effect(effect_types::SPELL_EFFECT_SELF_RESURRECT, 0, 0),
        effect(effect_types::SPELL_EFFECT_STUCK, 0, 0),
        effect(0, 0, 0),
    ]);
    assert!(
        spell_go_hit_targets_like_cpp(
            &caster_only_effects,
            &SpellTargetData {
                flags: 0x0000_0040,
                ..Default::default()
            },
            target_guid,
        )
        .is_empty()
    );

    let mixed_effects = spell_info(vec![
        effect(effect_types::SPELL_EFFECT_SELF_RESURRECT, 0, 0),
        effect(effect_types::SPELL_EFFECT_HEAL, 0, 0),
    ]);
    let implicit_target_effect = spell_info(vec![effect(
        effect_types::SPELL_EFFECT_SELF_RESURRECT,
        1,
        0,
    )]);
    let implicit_target_2_effect = spell_info(vec![effect(effect_types::SPELL_EFFECT_STUCK, 0, 1)]);
    let inactive_effects = spell_info(vec![effect(0, 0, 0)]);
    let no_effects = spell_info(Vec::new());

    for (info, target_data) in [
        (&mixed_effects, SpellTargetData::default()),
        (&implicit_target_effect, SpellTargetData::default()),
        (&implicit_target_2_effect, SpellTargetData::default()),
        (
            &caster_only_effects,
            SpellTargetData {
                flags: 0x0000_0080, // TARGET_FLAG_UNIT_ENEMY.
                ..Default::default()
            },
        ),
        (
            &caster_only_effects,
            SpellTargetData {
                unit: target_guid,
                ..Default::default()
            },
        ),
        (&inactive_effects, SpellTargetData::default()),
        (&no_effects, SpellTargetData::default()),
    ] {
        assert_eq!(
            spell_go_hit_targets_like_cpp(info, &target_data, target_guid),
            vec![target_guid],
            "non-matching metadata retains the existing hit list"
        );
    }
}
