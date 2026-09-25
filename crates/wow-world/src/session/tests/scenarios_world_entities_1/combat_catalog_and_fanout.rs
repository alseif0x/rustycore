use super::*;

#[test]
fn chr_classes_store_drives_creature_display_power_else_fallback_like_cpp() {
    use wow_data::character_progression::{ChrClassesEntry, ChrClassesStore};
    let (mut session, _, _) = make_session();

    // [M0.1/#14] Without the ChrClasses store, class display power uses the
    // hardcoded fallback (class 8 = Mage → Mana).
    assert_eq!(
        session.creature_display_power_for_class_like_cpp(8),
        PowerType::Mana as u8
    );

    // With the store loaded, the DB2 `display_power` wins over the fallback.
    // Give class 8 a non-default power (Rage) to prove the store is consulted.
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([ChrClassesEntry {
        id: 8,
        display_power: PowerType::Rage as u8,
        ..Default::default()
    }])));
    assert_eq!(
        session.creature_display_power_for_class_like_cpp(8),
        PowerType::Rage as u8
    );
    // A class with no DB2 row still falls back (class 4 = Rogue → Energy).
    assert_eq!(
        session.creature_display_power_for_class_like_cpp(4),
        PowerType::Energy as u8
    );
}

#[test]
fn spell_threat_entry_prefers_exact_spell_like_cpp() {
    let (mut session, _, _) = make_session();
    let mut entries = HashMap::new();
    entries.insert(11, test_spell_threat_entry_like_cpp(11));
    entries.insert(42, test_spell_threat_entry_like_cpp(42));
    session.set_spell_threat_store(Arc::new(wow_data::SpellThreatStoreLikeCpp {
        entries_by_spell_id: entries,
    }));
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 42,
                supercedes_spell_id: 11,
            }],
            |_| true,
        ),
    ));

    let entry = session
        .spell_threat_entry_like_cpp(42)
        .expect("exact threat entry");

    assert_eq!(entry.flat_mod, 42);
}

#[test]
fn spell_threat_entry_falls_back_to_first_rank_like_cpp() {
    let (mut session, _, _) = make_session();
    let mut entries = HashMap::new();
    entries.insert(11, test_spell_threat_entry_like_cpp(11));
    session.set_spell_threat_store(Arc::new(wow_data::SpellThreatStoreLikeCpp {
        entries_by_spell_id: entries,
    }));
    session.set_spell_chain_store(Arc::new(
        wow_data::SpellChainStoreLikeCpp::from_skill_line_ability_supercedes_like_cpp(
            [wow_data::SpellRankEdgeLikeCpp {
                spell_id: 42,
                supercedes_spell_id: 11,
            }],
            |_| true,
        ),
    ));

    let entry = session
        .spell_threat_entry_like_cpp(42)
        .expect("first-rank threat entry");

    assert_eq!(entry.flat_mod, 11);
}

#[test]
fn creature_realm_fanout_uses_validated_position_without_canonical_mirror() {
    let (mut source, _, _) = make_session();
    let source_player = ObjectGuid::create_player(1, 42);
    let observer = ObjectGuid::create_player(1, 43);
    let creature = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 1014);
    let source_position = Position::new(10.0, 20.0, 30.0, 0.0);
    let packet_bytes = vec![0x44, 0x55, 0x68];
    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (observer_send_tx, _observer_send_rx) = flume::bounded(1);
    let (observer_command_tx, observer_command_rx) = flume::bounded(1);
    let mut observer_info =
        broadcast_info_with_command(observer, observer_send_tx, observer_command_tx);
    observer_info.placement.map_id = 571;
    observer_info.placement.instance_id = 0;
    observer_info.placement.position = source_position;
    registry.register_or_replace(observer, observer_info, Default::default());

    source.set_player_guid(Some(source_player));
    source.set_player_map_position_like_cpp(571, Position::ZERO);
    source.set_player_registry(registry);
    source.broadcast_creature_packet_from_position_to_visible_set_realm_like_cpp(
        creature,
        source_position,
        packet_bytes.clone(),
    );

    let command = observer_command_rx.try_recv().expect("observer fanout");
    let SessionCommand::SendRealmIfVisibleFromLegacySourceLikeCpp(command) = command else {
        panic!("creature visual must use the Realm-visible command");
    };
    assert_eq!(command.source_guid, creature);
    assert_eq!(command.map_id, 571);
    assert_eq!(command.instance_id, 0);
    assert_eq!(command.packet_bytes, packet_bytes);
}
