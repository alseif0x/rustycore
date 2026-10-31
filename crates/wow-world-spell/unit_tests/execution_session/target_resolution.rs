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

fn explicit_target_data() -> SpellTargetData {
    SpellTargetData {
        flags: 0x0000_0002,
        unit: ObjectGuid::create_player(1, 77),
        item: ObjectGuid::new(6, 78),
        src_location: Some(wow_packet::packets::spell::TargetLocation {
            transport: ObjectGuid::new(6, 79),
            position: wow_core::Position::new(1.0, 2.0, 3.0, 0.5),
        }),
        dst_location: Some(wow_packet::packets::spell::TargetLocation {
            transport: ObjectGuid::new(6, 80),
            position: wow_core::Position::new(4.0, 5.0, 6.0, 1.0),
        }),
        orientation: Some(1.5),
        map_id: Some(571),
        name: "explicit".to_owned(),
    }
}

#[test]
fn caster_only_stuck_clears_every_explicit_target_field_for_start_and_go() {
    let stuck = spell_info(vec![
        effect(effect_types::SPELL_EFFECT_STUCK, 0, 0),
        effect(0, 1, 1),
    ]);
    let original = explicit_target_data();

    let start_target = normalize_caster_only_stuck_target_data_like_cpp(&stuck, original.clone());
    let go_target = normalize_caster_only_stuck_target_data_like_cpp(&stuck, original);

    assert_eq!(start_target, SpellTargetData::default());
    assert_eq!(go_target, SpellTargetData::default());
}

#[test]
fn caster_only_stuck_normalization_preserves_mixed_and_other_spell_targets() {
    let original = explicit_target_data();
    let spells = [
        spell_info(vec![effect(
            effect_types::SPELL_EFFECT_SELF_RESURRECT,
            0,
            0,
        )]),
        spell_info(vec![
            effect(effect_types::SPELL_EFFECT_STUCK, 0, 0),
            effect(effect_types::SPELL_EFFECT_HEAL, 0, 0),
        ]),
        spell_info(vec![effect(effect_types::SPELL_EFFECT_STUCK, 1, 0)]),
        spell_info(vec![effect(effect_types::SPELL_EFFECT_STUCK, 0, 1)]),
    ];

    for spell in spells {
        assert_eq!(
            normalize_caster_only_stuck_target_data_like_cpp(&spell, original.clone()),
            original
        );
    }
}

fn represented_hearthstone_spell_info() -> wow_data::SpellInfo {
    let mut spell = spell_info(vec![
        wow_data::SpellEffectInfo {
            effect_index: 0,
            effect: effect_types::SPELL_EFFECT_TELEPORT_UNITS_OLD,
            ..Default::default()
        },
        wow_data::SpellEffectInfo {
            effect_index: 1,
            effect: 77,
            ..Default::default()
        },
    ]);
    spell.spell_id = 8690;
    spell
}

#[test]
fn hearthstone_start_defaults_target_and_go_clears_only_explicit_unit() {
    let spell = represented_hearthstone_spell_info();
    let original = SpellTargetData {
        flags: 0x0000_0042,
        ..explicit_target_data()
    };

    assert_eq!(
        normalize_hearthstone_spell_start_target_data_like_cpp(&spell, original.clone()),
        SpellTargetData::default()
    );
    assert_eq!(
        normalize_hearthstone_spell_go_target_data_like_cpp(&spell, original.clone()),
        SpellTargetData {
            unit: ObjectGuid::EMPTY,
            ..original
        }
    );
}

#[test]
fn hearthstone_target_normalization_rejects_other_spells_and_effect_rows() {
    let original = explicit_target_data();
    let mut other_spell = represented_hearthstone_spell_info();
    other_spell.spell_id = 8691;
    let mut wrong_effect = represented_hearthstone_spell_info();
    wrong_effect.effects[1].effect = effect_types::SPELL_EFFECT_HEAL;
    let mut extra_effect = represented_hearthstone_spell_info();
    extra_effect.effects.push(wow_data::SpellEffectInfo {
        effect_index: 2,
        effect: effect_types::SPELL_EFFECT_HEAL,
        ..Default::default()
    });
    let mut wrong_effect_index = represented_hearthstone_spell_info();
    wrong_effect_index.effects[1].effect_index = 2;

    for spell in [other_spell, wrong_effect, extra_effect, wrong_effect_index] {
        assert_eq!(
            normalize_hearthstone_spell_start_target_data_like_cpp(&spell, original.clone()),
            original
        );
        assert_eq!(
            normalize_hearthstone_spell_go_target_data_like_cpp(&spell, original.clone()),
            original
        );
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
