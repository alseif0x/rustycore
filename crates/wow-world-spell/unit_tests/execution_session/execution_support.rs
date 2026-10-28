use super::*;

fn effect(effect: u32, effect_index: u32, effect_aura: i32) -> wow_data::SpellEffectInfo {
    wow_data::SpellEffectInfo {
        effect,
        effect_index,
        effect_aura,
        ..Default::default()
    }
}

fn spell_info(effects: Vec<wow_data::SpellEffectInfo>, effect_type: u32) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id: 1,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects,
    }
}

fn context(
    player_guid: wow_core::ObjectGuid,
    target_guid: wow_core::ObjectGuid,
    selected_aura_constructor: bool,
) -> SpellEffectRouteContextLikeCpp {
    SpellEffectRouteContextLikeCpp {
        player_guid,
        target_guid,
        selected_aura_constructor,
    }
}

#[test]
fn effect_route_separates_source_noops_direct_post_go_and_unavailable_mutations() {
    use wow_data::spell::spell_effect_types as effect_types;

    let info = spell_info(Vec::new(), effect_types::SPELL_EFFECT_HEAL);
    let context = context(
        wow_core::ObjectGuid::EMPTY,
        wow_core::ObjectGuid::EMPTY,
        false,
    );
    let classify =
        |effect_type| classify_spell_effect_route_like_cpp(&info, effect_type, None, context);

    assert_eq!(
        classify(effect_types::SPELL_EFFECT_NONE),
        SpellEffectExecutionRouteLikeCpp::SourceInert
    );
    // Source SpellEffects.cpp dispatches type 24 to EffectCreateItem, which
    // mutates inventory through DoCreateItem; this executor has no such route.
    assert_eq!(classify(24), SpellEffectExecutionRouteLikeCpp::Unavailable);
    assert_eq!(
        classify(effect_types::SPELL_EFFECT_DUMMY),
        SpellEffectExecutionRouteLikeCpp::Unavailable
    );
    assert_eq!(
        classify(u32::MAX),
        SpellEffectExecutionRouteLikeCpp::Unavailable
    );
    assert_eq!(
        classify(effect_types::SPELL_EFFECT_HEAL),
        SpellEffectExecutionRouteLikeCpp::Direct
    );
    for post_go in [
        effect_types::SPELL_EFFECT_ADD_FARSIGHT,
        effect_types::SPELL_EFFECT_TELEPORT_UNITS,
        effect_types::SPELL_EFFECT_BIND,
        effect_types::SPELL_EFFECT_SUMMON_OBJECT_WILD,
        effect_types::SPELL_EFFECT_SUMMON_OBJECT_SLOT1,
    ] {
        assert_eq!(classify(post_go), SpellEffectExecutionRouteLikeCpp::PostGo);
    }
}

#[test]
fn aura_route_keeps_supported_groups_and_rejects_skipped_generic_player_groups() {
    use wow_data::spell::spell_effect_types as effect_types;

    let player_guid = wow_core::ObjectGuid::create_player(1, 41);
    let creature_guid = wow_core::ObjectGuid::create_creature_like_cpp(1, 571, 42, 1);
    let generic_multi = spell_info(
        vec![
            effect(
                effect_types::SPELL_EFFECT_APPLY_AURA,
                0,
                wow_data::spell::aura_types::SPELL_AURA_MOD_STAT,
            ),
            effect(
                effect_types::SPELL_EFFECT_APPLY_AURA,
                1,
                wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_HEALTH,
            ),
        ],
        effect_types::SPELL_EFFECT_APPLY_AURA,
    );

    assert_eq!(
        classify_spell_effect_route_like_cpp(
            &generic_multi,
            effect_types::SPELL_EFFECT_APPLY_AURA,
            Some(0),
            context(player_guid, player_guid, false),
        ),
        SpellEffectExecutionRouteLikeCpp::Unavailable
    );
    assert_eq!(
        classify_spell_effect_route_like_cpp(
            &generic_multi,
            effect_types::SPELL_EFFECT_APPLY_AURA,
            Some(0),
            context(player_guid, creature_guid, false),
        ),
        SpellEffectExecutionRouteLikeCpp::ApplyAura
    );
    assert_eq!(
        classify_spell_effect_route_like_cpp(
            &generic_multi,
            effect_types::SPELL_EFFECT_APPLY_AURA,
            Some(0),
            context(player_guid, player_guid, true),
        ),
        SpellEffectExecutionRouteLikeCpp::ApplyAura
    );

    let generic_single = spell_info(
        vec![effect(
            effect_types::SPELL_EFFECT_APPLY_AURA,
            0,
            wow_data::spell::aura_types::SPELL_AURA_MOD_STAT,
        )],
        effect_types::SPELL_EFFECT_APPLY_AURA,
    );
    assert_eq!(
        classify_spell_effect_route_like_cpp(
            &generic_single,
            effect_types::SPELL_EFFECT_APPLY_AURA,
            Some(0),
            context(player_guid, player_guid, false),
        ),
        SpellEffectExecutionRouteLikeCpp::ApplyAura
    );
}

#[test]
fn effect_route_preflight_checks_all_rows_and_primary_fallback_before_prepare() {
    use wow_data::spell::spell_effect_types as effect_types;

    let guid = wow_core::ObjectGuid::EMPTY;
    let inert = spell_info(
        vec![effect(effect_types::SPELL_EFFECT_NONE, 0, 0)],
        effect_types::SPELL_EFFECT_NONE,
    );
    assert_eq!(
        validate_spell_effect_routes_like_cpp(&inert, inert.effect_type, guid, guid, false),
        Ok(())
    );

    let create_item = spell_info(vec![effect(24, 0, 0)], 24);
    assert_eq!(
        validate_spell_effect_routes_like_cpp(&create_item, 24, guid, guid, false),
        Err("normal cast contains an unimplemented spell effect")
    );

    let fallback_unknown = spell_info(
        vec![effect(effect_types::SPELL_EFFECT_HEAL, 0, 0)],
        u32::MAX,
    );
    assert_eq!(
        validate_spell_effect_routes_like_cpp(
            &fallback_unknown,
            fallback_unknown.effect_type,
            guid,
            guid,
            false,
        ),
        Err("normal cast contains an unimplemented primary spell effect")
    );
}
