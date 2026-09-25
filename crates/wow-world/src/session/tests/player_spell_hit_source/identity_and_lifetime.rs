use super::*;

#[test]
fn rejected_spell_linked_trigger_blocks_hookless_inert_candidate_like_cpp() {
    const CANDIDATE_SPELL_ID: u32 = 33_795;

    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID));
    assert!(session.player_target_spell_is_hit_inert_like_cpp(CANDIDATE_SPELL_ID, 0));

    session.spell_linked_rejected_trigger_spell_ids_like_cpp = None;
    assert!(
        !session.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID),
        "missing rejected-row authority must fail closed"
    );

    session.set_spell_runtime_script_authority_like_cpp(
        Arc::new(BTreeSet::new()),
        Arc::new(BTreeSet::new()),
        Arc::new(BTreeSet::new()),
        Arc::new(BTreeSet::from([CANDIDATE_SPELL_ID])),
    );
    assert!(
        !session.spell_has_no_unrepresented_runtime_hooks_like_cpp(CANDIDATE_SPELL_ID),
        "a rejected spell_linked_spell row still represents a possible C++ hook"
    );
    assert!(!session.player_target_spell_is_hit_inert_like_cpp(CANDIDATE_SPELL_ID, 0));
}

#[test]
fn player_spell_hit_source_authority_requires_one_complete_identity_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 70_010);
    let next_player_guid = ObjectGuid::create_player(1, 70_011);

    session.set_player_guid(Some(player_guid));
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp(std::iter::empty::<
            RepresentedPlayerSpellLikeCpp,
        >(),)
    );
    session.set_player_aura_authority_complete_like_cpp(true);
    session.complete_player_equipment_inventory_authority_load_like_cpp();
    complete_empty_player_dynamic_spell_hit_aura_sources_like_cpp(&mut session);
    assert!(session.player_aura_authority_complete_like_cpp());
    assert!(session.player_equipment_inventory_authority_complete_like_cpp());
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a GUID without its matching Player controller is incomplete identity proof"
    );

    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AuthorityOwner".into(),
        Position::ZERO,
        571,
        1,
        1,
        1,
        0,
    ));
    assert!(session.player_aura_authority_complete_like_cpp());
    assert!(session.player_equipment_inventory_authority_complete_like_cpp());
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    assert!(!session.ensure_login_player_controller_like_cpp(
        player_guid,
        "AuthorityOwner".into(),
        Position::ZERO,
        571,
        1,
        1,
        1,
        0,
    ));
    assert!(
        session.player_aura_authority_complete_like_cpp(),
        "reattaching the same C++ Player identity must retain its aura-source proof"
    );
    assert!(session.player_equipment_inventory_authority_complete_like_cpp());

    session.set_player_guid(Some(next_player_guid));
    assert!(!session.player_aura_authority_complete_like_cpp());
    assert!(!session.player_equipment_inventory_authority_complete_like_cpp());
    assert!(!session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());
}

#[test]
fn player_spell_hit_source_tombstone_does_not_cross_player_lifetimes_like_cpp() {
    let (mut session, _, _) = make_session();
    let first = ObjectGuid::create_player(1, 70_030);
    let second = ObjectGuid::create_player(1, 70_031);

    session.set_player_guid(Some(first));
    session.tombstone_player_spell_hit_aura_authority_like_cpp();
    assert!(session.player_spell_hit_aura_authority_tombstoned_like_cpp);

    session.set_player_guid(Some(second));
    assert!(
        !session.player_spell_hit_aura_authority_tombstoned_like_cpp,
        "FIRST-login loss belongs to the old C++ Player, not the surviving WorldSession"
    );
}

#[test]
fn player_spell_hit_source_authority_is_fail_closed_for_sources_and_passives_like_cpp() {
    let (mut session, _, _) = make_session();
    let player_guid = ObjectGuid::create_player(1, 70_012);
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "AuthoritySources".into(),
        Position::ZERO,
        571,
        1,
        1,
        1,
        0,
    ));
    session.set_player_aura_authority_complete_like_cpp(true);

    assert!(!session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());
    session.complete_player_equipment_inventory_authority_load_like_cpp();
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "empty known_spells is not evidence until its represented DB rows are complete"
    );
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp(std::iter::empty::<
            RepresentedPlayerSpellLikeCpp,
        >(),)
    );
    complete_empty_player_dynamic_spell_hit_aura_sources_like_cpp(&mut session);
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    const KNOWN_SPELL: i32 = 90_001;
    session.set_known_spells_like_cpp(vec![KNOWN_SPELL]);
    assert!(
        session.set_complete_represented_player_spell_rows_like_cpp([
            RepresentedPlayerSpellLikeCpp {
                spell_id: KNOWN_SPELL,
                active: true,
                disabled: false,
                dependent: false,
                favorite: false,
                state: RepresentedPlayerSpellStateLikeCpp::Unchanged,
            },
        ])
    );
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(2),
        "missing SpellMisc metadata must not classify a known spell as non-passive"
    );

    let mut passive_store = SpellStore::new();
    let mut passive_attributes = [0; 15];
    passive_attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    passive_store.insert_spell_misc_attributes_for_difficulty_like_cpp(
        KNOWN_SPELL,
        2,
        passive_attributes,
    );
    session.set_spell_store(Arc::new(passive_store));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(2),
        "the selected difficulty's passive bit must fail closed"
    );

    let mut non_passive_store = SpellStore::new();
    non_passive_store.insert_spell_misc_attributes_for_difficulty_like_cpp(KNOWN_SPELL, 2, [0; 15]);
    session.set_spell_store(Arc::new(non_passive_store));
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(2));

    session.insert_inventory_item_like_cpp(
        INVENTORY_SLOT_ITEM_START,
        InventoryItem {
            guid: ObjectGuid::create_item(1, 90_002),
            entry_id: 25,
            db_guid: 90_002,
            inventory_type: None,
        },
    );
    assert!(!session.can_authorize_empty_player_spell_hit_aura_source_for_difficulty_like_cpp(2));
}
