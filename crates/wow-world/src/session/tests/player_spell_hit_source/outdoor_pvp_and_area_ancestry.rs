use super::*;

fn configure_outdoor_pvp_tf_authority_fixture_like_cpp(session: &mut WorldSession) {
    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 3_519,
            continent_id: 530,
            parent_area_id: 0,
            area_bit: 1_143,
            exploration_level: 0,
            mount_flags: 2,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 3_697,
            continent_id: 530,
            parent_area_id: 3_519,
            area_bit: 1_321,
            exploration_level: 64,
            mount_flags: 2,
            flags: 0,
        },
    ])));
    session.set_player_zone_area_like_cpp(3_519, 3_697);
    session.set_player_zone_area_authority_complete_like_cpp(true);
}

#[test]
fn player_spell_hit_source_authority_audits_outdoor_pvp_tf_terokkar_buff_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    configure_outdoor_pvp_tf_authority_fixture_like_cpp(&mut session);
    assert!(
        session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "OutdoorPvPTF zone 3519 must admit spell 33377 only after both XP and outgoing-damage auras are proven hit-inert"
    );
}

#[test]
fn player_spell_hit_source_authority_keeps_outdoor_pvp_tf_dungeon_ids_fail_closed_like_cpp() {
    for zone_id in [3_791, 3_789, 3_792, 3_790] {
        let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
        configure_outdoor_pvp_tf_authority_fixture_like_cpp(&mut session);
        session.set_player_zone_area_like_cpp(zone_id, 3_697);
        assert!(
            !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
            "OutdoorPvPTF dungeon zone {zone_id} must remain fail-closed without C++ (Map*, zone) registration authority"
        );
    }
}

#[test]
fn player_spell_hit_source_authority_fails_closed_for_unproven_outdoor_pvp_tf_buff_like_cpp() {
    let mut missing_metadata = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    configure_outdoor_pvp_tf_authority_fixture_like_cpp(&mut missing_metadata);
    missing_metadata.set_spell_store(Arc::new(SpellStore::new()));
    assert!(
        !missing_metadata.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "missing effective spell 33377 metadata must fail closed"
    );

    let mut scripted = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    configure_outdoor_pvp_tf_authority_fixture_like_cpp(&mut scripted);
    scripted.set_spell_runtime_script_authority_like_cpp(
        Arc::new(BTreeSet::from([33_377])),
        Arc::new(BTreeSet::new()),
        Arc::new(BTreeSet::new()),
        Arc::new(BTreeSet::new()),
    );
    assert!(
        !scripted.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an exact runtime hook on spell 33377 must fail closed"
    );

    let mut hit_relevant = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    configure_outdoor_pvp_tf_authority_fixture_like_cpp(&mut hit_relevant);
    let mut spell_store = SpellStore::new();
    spell_store.insert(
        33_377,
        player_target_aura_spell_info_fixture_like_cpp(
            33_377,
            &[
                wow_data::spell::aura_types::SPELL_AURA_MOD_XP_PCT,
                wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACKER_MELEE_HIT_CHANCE,
            ],
        ),
    );
    hit_relevant.set_spell_store(Arc::new(spell_store));
    assert!(
        !hit_relevant.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a hit-relevant effective effect on spell 33377 must fail closed"
    );
}

#[test]
fn player_spell_hit_source_authority_gates_free_for_all_area_ancestry_like_cpp() {
    let mut session = complete_empty_player_spell_hit_authority_fixture_like_cpp();
    assert!(session.can_authorize_empty_player_spell_hit_aura_source_like_cpp());

    let area = |id, parent_area_id, flags| wow_data::AreaTableEntry {
        id,
        continent_id: 530,
        parent_area_id,
        area_bit: -1,
        exploration_level: 0,
        mount_flags: 0,
        flags,
    };
    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([
        area(12, 11, 0),
        area(11, 0, AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP),
    ])));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "an FFA ancestor makes C++ UpdateArea cast 208682 and 134735"
    );

    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([area(12, 11, 0)])));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a missing AreaTable ancestor must fail closed"
    );

    session.set_area_table_store(Arc::new(AreaTableStore::from_entries([
        area(12, 11, 0),
        area(11, 12, 0),
    ])));
    assert!(
        !session.can_authorize_empty_player_spell_hit_aura_source_like_cpp(),
        "a cyclic AreaTable hierarchy cannot prove coherent termination"
    );
}
