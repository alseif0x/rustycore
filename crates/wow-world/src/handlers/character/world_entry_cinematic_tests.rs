use crate::handlers::test_support::world::make_session;
use crate::test_fixtures::set_loaded_player_identity_like_cpp;
use std::sync::Arc;
use wow_constants::ServerOpcodes;
use wow_core::ObjectGuid;
use wow_data::character_progression::{
    ChrClassesEntry, ChrClassesStore, ChrRacesEntry, ChrRacesStore,
};
use wow_packet::WorldPacket;

fn chr_class_entry(id: u32, cinematic_sequence_id: u16) -> ChrClassesEntry {
    ChrClassesEntry {
        id,
        name: String::new(),
        filename: String::new(),
        name_male: String::new(),
        name_female: String::new(),
        pet_name_token: String::new(),
        create_screen_file_data_id: 0,
        select_screen_file_data_id: 0,
        icon_file_data_id: 0,
        low_res_screen_file_data_id: 0,
        flags: 0,
        starting_level: 1,
        armor_type_mask: 0,
        cinematic_sequence_id,
        default_spec: 0,
        has_strength_attack_bonus: 0,
        primary_stat_priority: 0,
        display_power: 0,
        ranged_attack_power_per_agility: 0,
        attack_power_per_agility: 0,
        attack_power_per_strength: 0,
        spell_class_set: 0,
        roles_mask: 0,
        damage_bonus_stat: 0,
        has_relic_slot: 0,
    }
}

fn chr_race_entry(id: u32, cinematic_sequence_id: i16) -> ChrRacesEntry {
    ChrRacesEntry {
        id,
        client_prefix: String::new(),
        client_file_string: String::new(),
        name: String::new(),
        flags: 0,
        male_display_id: 0,
        female_display_id: 0,
        high_res_male_display_id: 0,
        high_res_female_display_id: 0,
        res_sickness_spell_id: 0,
        splash_sound_id: 0,
        create_screen_file_data_id: 0,
        select_screen_file_data_id: 0,
        low_res_screen_file_data_id: 0,
        altered_form_start_visual_kit_id: [0; 3],
        altered_form_finish_visual_kit_id: [0; 3],
        heritage_armor_achievement_id: 0,
        starting_level: 1,
        ui_display_order: 0,
        playable_race_bit: 0,
        female_skeleton_file_data_id: 0,
        male_skeleton_file_data_id: 0,
        helmet_anim_scaling_race_id: 0,
        transmogrify_disabled_slot_mask: 0,
        faction_id: 0,
        cinematic_sequence_id,
        base_language: 0,
        creature_type: 0,
        alliance: 0,
        race_related: 0,
        unaltered_visual_race_id: 0,
        default_class_id: 0,
        neutral_race_id: 0,
    }
}

fn expected_trigger_cinematic(cinematic_id: u32) -> Vec<u8> {
    let mut expected = (ServerOpcodes::TriggerCinematic as u16)
        .to_le_bytes()
        .to_vec();
    expected.extend_from_slice(&cinematic_id.to_le_bytes());
    expected.extend_from_slice(&ObjectGuid::EMPTY.to_raw_bytes());
    expected
}

#[tokio::test]
async fn opening_cinematic_requires_zero_xp_and_prefers_class_like_cpp() {
    let (mut session, send_rx) = make_session();
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 8, 1, 0);
    session.set_player_xp_like_cpp(1);
    session.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([chr_class_entry(
        8, 111,
    )])));
    session.set_chr_races_store(Arc::new(ChrRacesStore::from_entries([chr_race_entry(
        1, 222,
    )])));

    session
        .handle_opening_cinematic(WorldPacket::new_empty())
        .await;
    assert!(send_rx.try_recv().is_err());

    session.set_player_xp_like_cpp(0);
    session
        .handle_opening_cinematic(WorldPacket::new_empty())
        .await;
    assert_eq!(send_rx.try_recv().unwrap(), expected_trigger_cinematic(111));

    let (mut fallback, fallback_rx) = make_session();
    set_loaded_player_identity_like_cpp(&mut fallback, 571, 1, 8, 1, 0);
    fallback.set_player_xp_like_cpp(0);
    fallback.set_chr_classes_store(Arc::new(ChrClassesStore::from_entries([chr_class_entry(
        8, 0,
    )])));
    fallback.set_chr_races_store(Arc::new(ChrRacesStore::from_entries([chr_race_entry(
        1, 222,
    )])));

    fallback
        .handle_opening_cinematic(WorldPacket::new_empty())
        .await;
    assert_eq!(
        fallback_rx.try_recv().unwrap(),
        expected_trigger_cinematic(222)
    );
}
