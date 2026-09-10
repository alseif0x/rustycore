//! Object GUID regressions.
//!
//! Moved out of guid.rs under #685; every test is unchanged.

use super::*;

#[test]
fn test_empty_guid() {
    let guid = ObjectGuid::EMPTY;
    assert!(guid.is_empty());
    assert_eq!(guid.high_type(), HighGuid::Null);
    assert_eq!(guid.counter(), 0);
}

#[test]
fn test_create_player() {
    let guid = ObjectGuid::create_player(1, 42);
    assert!(guid.is_player());
    assert!(!guid.is_empty());
    assert_eq!(guid.high_type(), HighGuid::Player);
    assert_eq!(guid.realm_id(), 1);
    assert_eq!(guid.counter(), 42);
    assert_eq!(guid.type_id(), TypeId::Player);
}

#[test]
fn test_create_creature() {
    let guid = ObjectGuid::create_creature_like_cpp(
        7,    // active realm_id
        530,  // map_id (Outland)
        1234, // entry
        5678, // counter
    );
    assert!(guid.is_creature());
    assert_eq!(guid.high_type(), HighGuid::Creature);
    assert_eq!(guid.realm_id(), 7);
    assert_eq!(guid.server_id(), 0);
    assert_eq!(guid.map_id(), 530);
    assert_eq!(guid.entry(), 1234);
    assert_eq!(guid.counter(), 5678);
    assert_eq!(guid.type_id(), TypeId::Unit);
}

#[test]
fn test_create_world_object_helpers_like_cpp() {
    let vehicle = ObjectGuid::create_vehicle_like_cpp(7, 571, 29929, 0x6A);
    assert_eq!(vehicle.high_type(), HighGuid::Vehicle);
    assert_eq!(vehicle.realm_id(), 7);
    assert_eq!(vehicle.server_id(), 0);
    assert_eq!(vehicle.map_id(), 571);
    assert_eq!(vehicle.entry(), 29929);
    assert_eq!(vehicle.counter(), 0x6A);

    let gameobject = ObjectGuid::create_gameobject_like_cpp(571, 9001, 22);
    assert_eq!(gameobject.high_type(), HighGuid::GameObject);
    assert_eq!(gameobject.realm_id(), 0);
    assert_eq!(gameobject.server_id(), 0);
    assert_eq!(gameobject.map_id(), 571);
    assert_eq!(gameobject.entry(), 9001);
    assert_eq!(gameobject.counter(), 22);

    let area_trigger = ObjectGuid::create_area_trigger_like_cpp(571, 2001, 99);
    assert_eq!(area_trigger.high_type(), HighGuid::AreaTrigger);
    assert_eq!(area_trigger.realm_id(), 0);
    assert_eq!(area_trigger.server_id(), 0);
    assert_eq!(area_trigger.map_id(), 571);
    assert_eq!(area_trigger.entry(), 2001);
    assert_eq!(area_trigger.counter(), 99);
}

#[test]
fn test_has_entry_matches_cpp_world_object_format_types() {
    let with_entry = [
        HighGuid::WorldTransaction,
        HighGuid::Conversation,
        HighGuid::Creature,
        HighGuid::Vehicle,
        HighGuid::Pet,
        HighGuid::GameObject,
        HighGuid::DynamicObject,
        HighGuid::AreaTrigger,
        HighGuid::Corpse,
        HighGuid::LootObject,
        HighGuid::SceneObject,
        HighGuid::Scenario,
        HighGuid::AIGroup,
        HighGuid::DynamicDoor,
        HighGuid::Vignette,
        HighGuid::CallForHelp,
        HighGuid::AIResource,
        HighGuid::AILock,
        HighGuid::AILockTicket,
        HighGuid::Cast,
    ];

    for high in with_entry {
        let guid = ObjectGuid::create_world_object(high, 0, 0, 571, 0, 1234, 55);
        assert!(
            guid.has_entry(),
            "{high:?} should expose entry like C++ WorldObject"
        );
        assert_eq!(guid.entry(), 1234);
    }
}

#[test]
fn test_has_entry_rejects_cpp_non_world_object_format_types() {
    let without_entry = [
        HighGuid::Null,
        HighGuid::Uniq,
        HighGuid::Player,
        HighGuid::Item,
        HighGuid::StaticDoor,
        HighGuid::Transport,
        HighGuid::ClientActor,
        HighGuid::ChatChannel,
        HighGuid::Party,
        HighGuid::Guild,
        HighGuid::WowAccount,
        HighGuid::BNetAccount,
        HighGuid::GMTask,
        HighGuid::MobileSession,
        HighGuid::RaidGroup,
        HighGuid::Spell,
        HighGuid::Mail,
        HighGuid::WebObj,
        HighGuid::LFGObject,
        HighGuid::LFGList,
        HighGuid::UserRouter,
        HighGuid::PVPQueueGroup,
        HighGuid::UserClient,
        HighGuid::PetBattle,
        HighGuid::UniqUserClient,
        HighGuid::BattlePet,
        HighGuid::CommerceObj,
        HighGuid::ClientSession,
        HighGuid::ClientConnection,
        HighGuid::ClubFinder,
        HighGuid::ToolsClient,
        HighGuid::WorldLayer,
        HighGuid::ArenaTeam,
        HighGuid::LMMParty,
        HighGuid::LMMLobby,
    ];

    for high in without_entry {
        let guid = ObjectGuid::new(
            ((high as i64) << 58) | ((0x12_3456i64 & 0x7F_FFFF) << 6),
            0xABCD,
        );
        assert!(
            !guid.has_entry(),
            "{high:?} should not expose entry in C++ non-WorldObject format"
        );
    }
}

#[test]
fn test_has_entry_only_controls_display_not_packed_bits() {
    let raw = ObjectGuid::new(
        ((HighGuid::Player as i64) << 58) | ((0x12_3456i64 & 0x7F_FFFF) << 6),
        42,
    );
    let restored = ObjectGuid::from_raw_bytes(&raw.to_raw_bytes());

    assert_eq!(raw, restored);
    assert_eq!(restored.entry(), 0x12_3456);
    assert!(!restored.has_entry());
    assert!(!format!("{restored}").contains("Entry:"));

    let creature = ObjectGuid::create_creature_like_cpp(1, 571, 9999, 123456);
    assert!(creature.has_entry());
    assert!(format!("{creature}").contains("Entry: 9999"));
}

#[test]
fn test_create_item() {
    let guid = ObjectGuid::create_item(1, 99999);
    assert!(guid.is_item());
    assert_eq!(guid.counter(), 99999);
    assert_eq!(guid.type_id(), TypeId::Item);
}

#[test]
fn test_create_uniq() {
    let guid = ObjectGuid::create_uniq(10);
    assert_eq!(guid.high_type(), HighGuid::Uniq);
    assert_eq!(guid.counter(), 10);
}

#[test]
fn test_create_transport() {
    let guid = ObjectGuid::create_transport(HighGuid::Transport, 100);
    assert!(guid.is_mo_transport());
    assert!(guid.is_any_type_game_object());
    assert_eq!(guid.counter(), 100);
}

#[test]
fn test_create_global() {
    let guid = ObjectGuid::create_global(HighGuid::Party, 0, 12345);
    assert!(guid.is_party());
    assert_eq!(guid.counter(), 12345);
}

#[test]
fn test_create_guild() {
    let guid = ObjectGuid::create_guild(HighGuid::Guild, 1, 42);
    assert!(guid.is_guild());
    assert_eq!(guid.realm_id(), 1);
    assert_eq!(guid.counter(), 42);
}

#[test]
fn test_raw_bytes_roundtrip() {
    let original = ObjectGuid::create_creature_like_cpp(1, 571, 9999, 123456);
    let bytes = original.to_raw_bytes();
    let restored = ObjectGuid::from_raw_bytes(&bytes);
    assert_eq!(original, restored);
}

#[test]
fn test_guid_ordering() {
    let a = ObjectGuid::create_player(1, 1);
    let b = ObjectGuid::create_player(1, 2);
    assert!(a < b);
}

#[test]
fn test_guid_generator() {
    let generator = ObjectGuidGenerator::new(HighGuid::Creature, 1);
    assert_eq!(generator.generate(), 1);
    assert_eq!(generator.generate(), 2);
    assert_eq!(generator.generate(), 3);
    assert_eq!(generator.next_after_max_used(), 4);
}

#[test]
fn equipment_set_guid_generator_allocates_one_process_wide_sequence() {
    let generator = std::sync::Arc::new(EquipmentSetGuidGeneratorLikeCpp::new(41));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let generator = std::sync::Arc::clone(&generator);
            std::thread::spawn(move || (0..128).map(|_| generator.generate()).collect::<Vec<_>>())
        })
        .collect();
    let mut generated: Vec<_> = handles
        .into_iter()
        .flat_map(|handle| handle.join().expect("allocator worker"))
        .collect();
    generated.sort_unstable();

    assert_eq!(generated, (41..41 + 8 * 128).collect::<Vec<_>>());
    assert_eq!(generator.next_after_max_used(), 41 + 8 * 128);
}

#[test]
fn equipment_set_guid_generator_stops_at_cpp_limit() {
    let generator = EquipmentSetGuidGeneratorLikeCpp::new(EQUIPMENT_SET_GUID_LIMIT_LIKE_CPP - 1);
    assert_eq!(
        generator.try_generate(),
        Some(EQUIPMENT_SET_GUID_LIMIT_LIKE_CPP - 1)
    );
    assert_eq!(
        generator.next_after_max_used(),
        EQUIPMENT_SET_GUID_LIMIT_LIKE_CPP
    );
    assert_eq!(generator.try_generate(), None);
    assert_eq!(
        generator.next_after_max_used(),
        EQUIPMENT_SET_GUID_LIMIT_LIKE_CPP
    );
}

#[test]
fn void_storage_item_id_generator_is_process_wide_and_concurrent() {
    let generator = std::sync::Arc::new(VoidStorageItemIdGeneratorLikeCpp::new(700));
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let generator = std::sync::Arc::clone(&generator);
            std::thread::spawn(move || (0..64).map(|_| generator.generate()).collect::<Vec<_>>())
        })
        .collect();
    let mut generated: Vec<_> = handles
        .into_iter()
        .flat_map(|handle| handle.join().expect("allocator worker"))
        .collect();
    generated.sort_unstable();

    assert_eq!(generated, (700..700 + 8 * 64).collect::<Vec<_>>());
    assert_eq!(generator.next_after_max_used(), 700 + 8 * 64);
}

#[test]
fn void_storage_item_id_generator_stops_at_packet_guid_limit() {
    assert_eq!(
        VOID_STORAGE_ITEM_ID_LIMIT_LIKE_PACKET_GUID,
        ObjectGuid::max_counter(HighGuid::Item) as u64 + 1
    );
    let generator =
        VoidStorageItemIdGeneratorLikeCpp::new(VOID_STORAGE_ITEM_ID_LIMIT_LIKE_PACKET_GUID - 1);
    assert_eq!(
        generator.try_generate(),
        Some(VOID_STORAGE_ITEM_ID_LIMIT_LIKE_PACKET_GUID - 1)
    );
    assert_eq!(generator.try_generate(), None);
}

#[test]
fn test_typed_guid_player() {
    let guid = ObjectGuid::create_player(1, 42);
    let player = guid.as_player();
    assert!(player.is_some());
    assert_eq!(player.unwrap().counter(), 42);

    let creature_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 0, 0, 1, 1);
    assert!(creature_guid.as_player().is_none());
}

#[test]
fn test_typed_guid_creature() {
    let guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 0, 0, 100, 50);
    let creature = guid.as_creature();
    assert!(creature.is_some());
    assert_eq!(creature.unwrap().entry(), 100);

    let player_guid = ObjectGuid::create_player(1, 42);
    assert!(player_guid.as_creature().is_none());
}

#[test]
fn test_map_specific() {
    assert!(ObjectGuid::is_map_specific(HighGuid::Creature));
    assert!(ObjectGuid::is_map_specific(HighGuid::GameObject));
    assert!(!ObjectGuid::is_map_specific(HighGuid::Player));
    assert!(!ObjectGuid::is_map_specific(HighGuid::Item));
}

#[test]
fn test_realm_specific() {
    assert!(ObjectGuid::is_realm_specific(HighGuid::Player));
    assert!(ObjectGuid::is_realm_specific(HighGuid::Item));
    assert!(!ObjectGuid::is_realm_specific(HighGuid::Creature));
}

#[test]
fn test_is_global() {
    assert!(ObjectGuid::is_global(HighGuid::Party));
    assert!(ObjectGuid::is_global(HighGuid::BattlePet));
    assert!(!ObjectGuid::is_global(HighGuid::Player));
}
