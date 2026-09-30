//! Pet scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn query_pet_name_missing_unit_sends_cpp_deny_shape() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 11, 901);

    session
        .handle_query_pet_name(QueryPetName { unit_guid: guid })
        .await;

    let bytes = send_rx.try_recv().expect("query pet name response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QueryPetNameResponse as u16
    );
    assert_eq!(&bytes[2..18], &guid.to_raw_bytes());
    assert_eq!(bytes[18], 0x00);
    assert_eq!(bytes.len(), 19);
}
#[tokio::test]
async fn query_pet_name_uses_canonical_owned_pet_name_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 904);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 21, 904);
    session.set_player_guid(Some(player_guid));

    let mut player = wow_entities::Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid);
    player.unit_mut().world_mut().set_map(571, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(10.0, 0.0, 0.0, 0.0));

    let mut pet = wow_entities::Pet::new(player_guid, wow_entities::PetType::Hunter);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(pet_guid);
    pet.creature_mut()
        .unit_mut()
        .world_mut()
        .set_map(571, 0)
        .unwrap();
    pet.creature_mut().unit_mut().world_mut().set_name("Misha");

    let mut manager = wow_map::MapManager::default();
    let map = manager.create_world_map(571, 0).map_mut();
    map.insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.insert_map_object_record(wow_entities::MapObjectRecord::new_pet(pet).unwrap())
        .unwrap();
    attach_map_manager(&mut session, manager);

    session
        .handle_query_pet_name(QueryPetName {
            unit_guid: pet_guid,
        })
        .await;

    let bytes = send_rx.try_recv().expect("query pet name response");
    assert_eq!(
        u16::from_le_bytes([bytes[0], bytes[1]]),
        wow_constants::ServerOpcodes::QueryPetNameResponse as u16
    );
    assert_eq!(&bytes[2..18], &pet_guid.to_raw_bytes());
    assert_eq!(bytes[18] & 0x80, 0x80);
    assert!(bytes.windows(5).any(|window| window == b"Misha"));
    assert!(bytes.windows(4).any(|window| window == 0_u32.to_le_bytes()));
}
#[test]
fn query_pet_name_handler_registration_matches_cpp() {
    let entry = inventory::iter::<PacketHandlerEntry>
        .into_iter()
        .find(|entry| entry.opcode == ClientOpcodes::QueryPetName)
        .expect("QueryPetName handler registration");

    assert_eq!(entry.status, SessionStatus::LoggedIn);
    assert_eq!(entry.processing, PacketProcessing::Inplace);
    assert_eq!(entry.handler_name, "handle_query_pet_name");
}
#[test]
fn enum_character_pet_data_stays_zero_for_ghost_non_pet_class_or_missing_template_like_cpp() {
    let store = enum_pet_template_store(416, 8);

    assert_eq!(
        enum_character_pet_data_like_cpp(
            PLAYER_FLAGS_GHOST_LIKE_CPP,
            0,
            CLASS_HUNTER_LIKE_CPP,
            416,
            1234,
            27,
            Some(&store),
        ),
        (0, 0, 0)
    );
    assert_eq!(
        enum_character_pet_data_like_cpp(0, 0, 1, 416, 1234, 27, Some(&store)),
        (0, 0, 0)
    );
    assert_eq!(
        enum_character_pet_data_like_cpp(0, 0, CLASS_HUNTER_LIKE_CPP, 999, 1234, 27, Some(&store)),
        (0, 0, 0)
    );
}
