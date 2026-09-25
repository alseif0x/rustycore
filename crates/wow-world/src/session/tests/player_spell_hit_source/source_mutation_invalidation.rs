use super::*;

#[test]
fn player_spell_hit_source_sync_sets_and_mutations_invalidate_canonical_auras_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 70_013);
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AuthorityCanonical".into(),
        Position::ZERO,
        571,
        1,
        1,
        1,
        0,
    ));
    assert!(session.install_detached_canonical_player_for_test_like_cpp());
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    session.set_player_aura_authority_complete_like_cpp(true);
    session.complete_player_equipment_inventory_authority_load_like_cpp();
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp(std::iter::empty::<
            RepresentedPlayerSpellLikeCpp,
        >(),)
    );
    complete_empty_player_dynamic_spell_hit_aura_sources_like_cpp(&mut session);

    assert_eq!(
        session.sync_player_spell_hit_aura_authority_to_canonical_like_cpp(),
        Some(true)
    );
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()),
        Some(true)
    );

    assert!(session.battle_pet_unlock_slot_like_cpp(0));
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()),
        Some(false),
        "battle-pet slot mutations must revoke the canonical proof immediately"
    );
    assert!(!session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());
    assert!(
        session.complete_represented_battle_pet_slot_authority_load_like_cpp([
            (0, None, true),
            (1, None, true),
            (2, None, true),
        ])
    );
    assert_eq!(
        session.sync_player_spell_hit_aura_authority_to_canonical_like_cpp(),
        Some(true)
    );

    session.learn_known_spell_like_cpp(90_003);
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()),
        Some(false),
        "known-spell mutation must revoke the canonical proof immediately"
    );

    session.set_known_spells_like_cpp(Vec::new());
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp(std::iter::empty::<
            RepresentedPlayerSpellLikeCpp,
        >(),)
    );
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert_eq!(
        session.sync_player_spell_hit_aura_authority_to_canonical_like_cpp(),
        Some(true)
    );
    session.begin_player_equipment_inventory_authority_load_like_cpp();
    assert_eq!(
        session.canonical_player_snapshot_like_cpp(|player| player
            .unit()
            .subsystems()
            .auras
            .has_complete_spell_hit_inert_aura_authority_like_cpp()),
        Some(false),
        "starting a new equipment query must revoke the canonical proof"
    );
}
