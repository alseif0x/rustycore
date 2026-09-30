// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Login identity projected into the canonical player and player registry.

use super::*;
use std::sync::Arc;
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator, Position};
use wow_data::character_progression::{ChrRacesEntry, ChrRacesStore};
use wow_packet::WorldPacket;

fn make_session_with_send_capacity(capacity: usize) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(HighGuid::Item, 1)));
    session.set_equipment_set_guid_generator_like_cpp(
        Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(1)),
    );
    (session, send_rx)
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

fn ensure_login_player_controller_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
    name: String,
    position: Position,
    map_id: u16,
    race: u8,
    class: u8,
    level: u8,
    gender: u8,
) -> bool {
    session.ensure_login_player_controller_like_cpp(
        guid, name, position, map_id, race, class, level, gender,
    )
}

#[test]
fn login_identity_hydrates_race_faction_into_registry_and_canonical_player_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let guid = ObjectGuid::create_player(1, 42_001);
    let canonical: crate::session::SharedCanonicalMapManager =
        Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    let registry = Arc::new(crate::session::directory::PlayerRegistry::default());
    let mut race_entry = chr_race_entry(1, 0);
    race_entry.faction_id = 1;

    session.set_chr_races_store(Arc::new(ChrRacesStore::from_entries([race_entry])));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_registry(Arc::clone(&registry));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));

    // Mirror the real LoadFromDB order: identity is loaded from the
    // character row before the controller/map/registry publication.
    session.set_player_guid(Some(guid));
    set_loaded_player_identity_like_cpp(&mut session, 571, 1, 1, 10, 0);
    assert!(ensure_login_player_controller_for_test(
        &mut session,
        guid,
        "FactionLogin".to_string(),
        Position::ZERO,
        571,
        1,
        1,
        10,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session.register_in_player_registry();

    assert_eq!(registry.legacy_aggro_candidates()[0].faction_template_id, 1);
    let manager = canonical.lock().unwrap();
    let player = manager
        .find_map(571, 0)
        .expect("login map")
        .map()
        .get_typed_player(guid)
        .expect("canonical login player");
    assert_eq!(player.unit().data().faction_template, 1);
}
