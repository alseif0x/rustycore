use crate::handlers::test_support::world::make_session;
use std::sync::Arc;
use wow_constants::PowerType;
use wow_core::ObjectGuid;

fn set_loaded_player_identity_like_cpp(
    session: &mut crate::session::WorldSession,
    map_id: u16,
    race: u8,
    class: u8,
    level: u8,
    gender: u8,
) {
    session.set_loaded_player_identity_like_cpp(map_id, race, class, level, gender);
}

fn set_priest_level80_stats(
    session: &mut crate::session::WorldSession,
    base_mana: u32,
    intellect: u16,
) {
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

fn attach_stat_update_player_with_mana(
    session: &mut crate::session::WorldSession,
    player_guid: ObjectGuid,
    current_mana: i32,
    max_mana: i32,
) {
    attach_stat_update_player_with_mana_and_health(
        session,
        player_guid,
        current_mana,
        max_mana,
        100,
        100,
    );
}

fn attach_stat_update_player_with_mana_and_health(
    session: &mut crate::session::WorldSession,
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
    assert!(
        crate::canonical_player_access::configure_canonical_player_vitals_for_test(
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
        )
    );
}

fn canonical_player_power_snapshot_for_test(
    session: &crate::session::WorldSession,
    power_type: PowerType,
) -> Option<(i32, i32)> {
    session.canonical_player_power_snapshot_like_cpp(power_type)
}

fn canonical_player_health_snapshot_for_test(
    session: &crate::session::WorldSession,
) -> Option<(u32, u32)> {
    session.canonical_player_health_snapshot_like_cpp()
}

#[test]
fn stat_update_preserves_current_mana_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana(&mut session, player_guid, 777, 1320);

    let (_, changes) = session
        .player_stat_changes_like_cpp()
        .expect("stat changes");

    assert_eq!(changes.power0, 777);
    assert_eq!(changes.max_power0, 1320);
    assert_eq!(changes.base_mana, 1000);
    assert_eq!(
        canonical_player_power_snapshot_for_test(&session, PowerType::Mana),
        Some((777, 1320))
    );
}

#[test]
fn stat_update_clamps_current_mana_to_new_max_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 78);
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana(&mut session, player_guid, 2_000, 2_500);

    let (_, changes) = session
        .player_stat_changes_like_cpp()
        .expect("stat changes");

    assert_eq!(changes.power0, 1320);
    assert_eq!(changes.max_power0, 1320);
    assert_eq!(changes.base_mana, 1000);
    assert_eq!(
        canonical_player_power_snapshot_for_test(&session, PowerType::Mana),
        Some((1320, 1320))
    );
}

#[test]
fn stat_update_preserves_current_health_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 79);
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 7, 500);

    let (_, changes) = session
        .player_stat_changes_like_cpp()
        .expect("stat changes");

    assert_eq!(changes.health, 7);
    assert_eq!(changes.max_health, 10);
    assert_eq!(session.player_health_like_cpp(), 7);
    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((7, 10))
    );
}

#[test]
fn stat_update_clamps_current_health_to_new_max_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 80);
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 500, 500);

    let (_, changes) = session
        .player_stat_changes_like_cpp()
        .expect("stat changes");

    assert_eq!(changes.health, 10);
    assert_eq!(changes.max_health, 10);
    assert_eq!(session.player_health_like_cpp(), 10);
    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((10, 10))
    );
}

#[test]
fn total_stat_percentage_non_ability_keeps_current_health_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 83);
    let spell_id = 90_083;
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1_000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1_320, 5, 10);
    session.set_player_health_like_cpp(5, 10);
    session.set_spell_store(Arc::new(total_stat_percentage_spell_store_like_cpp(
        spell_id, false,
    )));
    session.set_state(crate::session::SessionState::LoggedIn);

    session
        .apply_aura(spell_id, player_guid, 30_000, 1)
        .expect("apply non-ability stamina aura");

    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((5, 20)),
        "C++ only preserves health percentage for SPELL_ATTR0_IS_ABILITY"
    );
}

#[test]
fn level_up_stat_update_refills_health_and_mana_like_cpp() {
    let (mut session, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 76);
    session.set_player_guid(Some(player_guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 5, 80, 0);
    set_priest_level80_stats(&mut session, 1000, 40);
    attach_stat_update_player_with_mana_and_health(&mut session, player_guid, 777, 1320, 3, 10);

    session.send_level_up_stat_update_like_cpp();

    assert_eq!(
        canonical_player_health_snapshot_for_test(&session),
        Some((10, 10))
    );
    assert_eq!(
        canonical_player_power_snapshot_for_test(&session, PowerType::Mana),
        Some((1320, 1320))
    );
    assert_eq!(session.player_health_like_cpp(), 10);
}
