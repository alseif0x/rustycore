use std::sync::Arc;

use wow_constants::PowerType;
use wow_core::ObjectGuid;

use crate::session::WorldSession;

fn make_session_with_send_capacity(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded(1);
    let (send_tx, send_rx) = flume::bounded(capacity);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54_261,
        vec![0; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_item_guid_generator_like_cpp(Arc::new(wow_core::ObjectGuidGenerator::new(
        wow_core::HighGuid::Item,
        1,
    )));
    (session, send_rx)
}

fn set_loaded_player_identity_like_cpp(
    session: &mut WorldSession,
    map_id: u16,
    race: u8,
    class: u8,
    level: u8,
    gender: u8,
) {
    session.set_loaded_player_identity_like_cpp(map_id, race, class, level, gender);
}

fn set_priest_level80_stats(session: &mut WorldSession, base_mana: u32, intellect: u16) {
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect,
            spirit: 30,
            base_mana,
        },
    )])));
    let mut class_entry = wow_data::character_progression::ChrClassesEntry::default();
    class_entry.id = 5;
    class_entry.starting_level = 1;
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([class_entry]),
    ));
    let mut regen_columns = [0.0; wow_data::RegenMpPerSptGameTableLikeCpp::VALUE_COLUMN_COUNT];
    regen_columns[4] = 0.003345;
    let regen_row = wow_data::RegenMpPerSptEntryLikeCpp::from_columns(regen_columns);
    let mut regen_rows = vec![wow_data::RegenMpPerSptEntryLikeCpp::default(); 79];
    regen_rows.push(regen_row);
    session.set_regen_game_tables(Arc::new(wow_data::RegenGameTablesLikeCpp::from_tables(
        wow_data::RegenMpPerSptGameTableLikeCpp::from_rows(regen_rows),
        wow_data::RegenHpPerSptGameTableLikeCpp::from_rows([]),
        wow_data::OctRegenHpGameTableLikeCpp::from_rows([]),
    )));
}

fn attach_stat_update_player_with_mana_and_health(
    session: &mut WorldSession,
    player_guid: ObjectGuid,
    current_mana: i32,
    max_mana: i32,
    current_health: u32,
    max_health: u32,
) {
    let identity = (
        session.player_map_id_like_cpp(),
        session.player_race_like_cpp(),
        session.player_class_like_cpp(),
        session.player_level_like_cpp(),
        session.player_gender_like_cpp(),
    );
    crate::canonical_player_access::install_canonical_player_owner_for_test(
        session,
        u32::from(identity.0),
        0,
    );
    session.set_loaded_player_identity_like_cpp(
        identity.0, identity.1, identity.2, identity.3, identity.4,
    );
    let manager = session
        .canonical_map_manager
        .as_ref()
        .expect("canonical map manager fixture");
    assert!(crate::canonical_player_access::configure_canonical_player_vitals_for_test(
        manager,
        player_guid,
        (
            current_health,
            max_health,
            PowerType::Mana,
            current_mana,
            max_mana,
            0,
        ),
    ));
}

fn total_stat_percentage_spell_store_like_cpp(
    spell_id: i32,
    is_ability: bool,
) -> wow_data::SpellStore {
    let mut store = wow_data::SpellStore::new();
    store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
                effect_base_points: 99,
                effect_die_sides: 1,
                effect_misc_value_2: 1 << 2,
                ..Default::default()
            }],
        },
    );
    let mut attributes = [0; 15];
    if is_ability {
        attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_IS_ABILITY;
    }
    store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    store
}

fn canonical_player_health_snapshot_for_test(
    session: &WorldSession,
) -> Option<(u32, u32)> {
    session.canonical_player_health_snapshot_like_cpp()
}

#[test]
fn login_passive_total_stat_aura_defers_values_update_until_create_like_cpp() {
    let (_packet_tx, packet_rx) = flume::bounded(4);
    let (send_tx, send_rx) = flume::unbounded();
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54_261,
        vec![0; 40],
        "esES".into(),
        packet_rx,
        send_tx,
    );
    let player_guid = ObjectGuid::create_player(1, 84);
    let spell_id = 90_085;
    session.set_player_guid(Some(player_guid));
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    session.set_player_stats(Arc::new(wow_data::PlayerStatsStore::from_entries([(
        (1, 5, 80),
        wow_data::PlayerLevelStats {
            strength: 10,
            agility: 10,
            stamina: 10,
            intellect: 40,
            spirit: 30,
            base_mana: 1_000,
        },
    )])));
    let mut chr_class = wow_data::character_progression::ChrClassesEntry::default();
    chr_class.id = 5;
    chr_class.starting_level = 1;
    session.set_chr_classes_store(Arc::new(
        wow_data::character_progression::ChrClassesStore::from_entries([chr_class]),
    ));
    let mut regen_columns =
        [0.0; wow_data::RegenMpPerSptGameTableLikeCpp::VALUE_COLUMN_COUNT];
    regen_columns[4] = 0.003345;
    let regen_row = wow_data::RegenMpPerSptEntryLikeCpp::from_columns(regen_columns);
    let mut regen_rows = vec![wow_data::RegenMpPerSptEntryLikeCpp::default(); 79];
    regen_rows.push(regen_row);
    session.set_regen_game_tables(Arc::new(wow_data::RegenGameTablesLikeCpp::from_tables(
        wow_data::RegenMpPerSptGameTableLikeCpp::from_rows(regen_rows),
        wow_data::RegenHpPerSptGameTableLikeCpp::from_rows([]),
        wow_data::OctRegenHpGameTableLikeCpp::from_rows([]),
    )));

    let player_guid = crate::canonical_player_access::install_canonical_player_owner_for_test(
        &mut session,
        571,
        0,
    );
    session.set_loaded_player_identity_like_cpp(571, 1, 5, 80, 0);
    let manager = session
        .canonical_map_manager
        .as_ref()
        .expect("canonical map manager fixture");
    assert!(crate::canonical_player_access::configure_canonical_player_vitals_for_test(
        manager,
        player_guid,
        (5, 10, PowerType::Mana, 777, 1_320, 0),
    ));

    session.set_known_spells_like_cpp(vec![spell_id]);
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: Some(
                wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
            ),
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE,
                effect_base_points: 99,
                effect_die_sides: 1,
                effect_misc_value_2: 1 << 2,
                ..Default::default()
            }],
        },
    );
    let mut attributes = [0; 15];
    attributes[0] = wow_data::spell::attributes::SPELL_ATTR0_PASSIVE;
    spell_store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    session.set_spell_store(Arc::new(spell_store));

    assert_eq!(session.state(), crate::session::SessionState::Authed);
    assert_eq!(session.apply_login_passive_known_spell_auras_like_cpp(), 1);
    assert_eq!(
        session.represented_total_stat_multipliers_like_cpp(),
        [1.0, 1.0, 2.0, 1.0, 1.0]
    );
    let opcodes = std::iter::from_fn(|| send_rx.try_recv().ok())
        .filter_map(|bytes| wow_packet::WorldPacket::from_bytes(&bytes).server_opcode())
        .collect::<Vec<_>>();
    assert!(
        !opcodes.contains(&wow_constants::ServerOpcodes::UpdateObject),
        "C++ folds login passive modifiers into UpdateAllStats/CreateObject instead of sending pre-create VALUES"
    );
}

#[test]
fn total_stat_percentage_ability_preserves_health_pct_on_apply_and_remove_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 82);
    let spell_id = 90_082;
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1_000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1_320, 5, 10);
    session.set_player_health_like_cpp(5, 10);
    session.set_spell_store(Arc::new(total_stat_percentage_spell_store_like_cpp(
        spell_id, true,
    )));
    session.set_state(crate::session::SessionState::LoggedIn);

    session
        .apply_aura(spell_id, player_guid, 30_000, 1)
        .expect("apply stamina ability");
    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((10, 20)),
        "C++ restores 50% health after the stamina ability raises max health"
    );

    let slot = session
        .visible_aura_slot_for_spell_like_cpp(spell_id)
        .expect("stamina ability aura slot");
    session.remove_aura(slot).expect("remove stamina ability");
    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((5, 10)),
        "C++ restores 50% health after the stamina ability lowers max health"
    );
}
