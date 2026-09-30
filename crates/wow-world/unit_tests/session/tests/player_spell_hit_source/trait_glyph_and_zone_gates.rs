use super::*;

#[test]
fn player_spell_hit_source_authority_requires_empty_traits_and_active_glyphs_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    session.begin_represented_trait_config_authority_load_like_cpp();
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "trait config and entry queries must both be authoritative"
    );

    assert!(
        session.complete_represented_trait_config_authority_load_like_cpp(
            [(1, 1, 71, 1), (2, 1, 72, 1)],
            true,
        )
    );
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a missing C++ specialization config can be created with granted entries"
    );

    assert!(
        session.complete_represented_trait_config_authority_load_like_cpp(
            [(1, 1, 71, 1), (2, 1, 72, 1), (3, 1, 73, 1)],
            false,
        )
    );
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a nonempty persisted entry query is not the narrow empty-config proof"
    );

    assert!(
        session.complete_represented_trait_config_authority_load_like_cpp(
            [(1, 1, 71, 1), (2, 1, 72, 1), (3, 1, 73, 1)],
            true,
        )
    );
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    assert!(
        session.set_complete_represented_spell_trait_definition_ids_like_cpp([(90_010, 70_010,)])
    );
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ active traits can cast an unrepresented trait spell"
    );

    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.load_represented_glyph_row_like_cpp(&glyph_catalog::catalog(123), 0, 0, 123));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ _LoadGlyphAuras casts every nonzero active-group glyph"
    );

    session.set_represented_active_talent_group_like_cpp(1);
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an inactive-group glyph is not cast by C++ _LoadGlyphAuras"
    );
    session.set_represented_active_talent_group_like_cpp(0);
    assert!(!session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());
}

#[test]
fn player_spell_hit_source_authority_gates_update_zone_aura_producers_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert_eq!(
        session.player_world_local_state_like_cpp(),
        Some(
            wow_entities::PlayerWorldLocalState::from_represented_parts_like_cpp(
                1, 12, true, false, None, 0, None,
            )
        )
    );
    assert_eq!(session.represented_player_flags_value_like_cpp(), Some(0));
    assert!(!session.represented_player_has_flag_like_cpp(PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP));
    assert_eq!(
        (
            session.represented_spell_area_autocast_source_is_empty_like_cpp(),
            session.represented_war_mode_update_zone_aura_source_is_empty_like_cpp(),
            session.represented_update_area_pvp_rule_aura_source_is_empty_like_cpp(),
            session.represented_update_zone_script_aura_source_is_hit_inert_like_cpp(),
        ),
        (true, true, true, true),
    );
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    session.set_loaded_player_flags_like_cpp(PLAYER_FLAGS_WAR_MODE_DESIRED_LIKE_CPP);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "C++ UpdateWarModeAuras casts 282559 or 269083 when War Mode is desired"
    );

    session.set_loaded_player_flags_like_cpp(0);
    session.set_player_zone_area_like_cpp(4_197, 4_197);
    session.set_player_zone_area_authority_complete_like_cpp(true);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "BattlefieldWG adds control phase-shift auras on zone entry"
    );

    session.set_player_zone_area_like_cpp(3_518, 3_697);
    session.set_player_zone_area_authority_complete_like_cpp(true);
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "OutdoorPvPNA spell 33795 is audited as incoming SpellHitResult-inert aura type 79"
    );

    session.set_player_zone_area_authority_complete_like_cpp(false);
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an unverified zone cannot prove OutdoorPvP/Battlefield sources hit-inert"
    );
}
