//! Session scenarios exercising the represented pets responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn battle_pet_grant_level_caps_at_max_resets_xp_and_sends_update_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x18f);
    install_represented_battle_pet_stat_stores_like_cpp(&mut session);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 7,
            level: 23,
            exp: 5,
            flags: 6,
            power: 10,
            health: 50,
            max_health: 100,
            speed: 20,
            quality: 3,
            owner_info: None,
            name: String::new(),
            name_timestamp: 0,
            declined_names: None,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        },
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_grant_battle_pet_level_represented_like_cpp(pet_guid, 5),
        RepresentedBattlePetGrantLevelOutcomeLikeCpp::Changed
    );

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("leveled pet");
    assert_eq!(pet.level, MAX_BATTLE_PET_LEVEL_LIKE_CPP);
    assert_eq!(pet.exp, 0);
    assert_eq!(pet.health, 1225);
    assert_eq!(pet.max_health, 1225);
    assert_eq!(pet.power, 131);
    assert_eq!(pet.speed, 84);
    assert_eq!(pet.save_info, RepresentedBattlePetSaveInfoLikeCpp::Changed);
    assert_eq!(
        session.represented_battle_pet_level_criteria_like_cpp(),
        &[
            RepresentedBattlePetLevelCriteriaLikeCpp {
                species: 11,
                level: 24
            },
            RepresentedBattlePetLevelCriteriaLikeCpp {
                species: 11,
                level: 25
            }
        ]
    );

    let packets = drain_server_packet_bytes(&send_rx);
    assert_eq!(packets.len(), 1);
    let mut packet = wow_packet::WorldPacket::from_bytes(&packets[0]);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::BattlePetUpdates as u16
    );
    assert_eq!(packet.read_uint32().expect("pet count"), 1);
    assert!(!packet.read_bit().expect("pet added"));
    assert_eq!(packet.read_packed_guid().expect("pet guid"), pet_guid);
    assert_eq!(packet.read_uint32().expect("species"), 11);
    assert_eq!(packet.read_uint32().expect("creature"), 22);
    assert_eq!(packet.read_uint32().expect("display"), 33);
    assert_eq!(packet.read_uint16().expect("breed"), 7);
    assert_eq!(packet.read_uint16().expect("level"), 25);
    assert_eq!(packet.read_uint16().expect("exp"), 0);
    assert_eq!(packet.read_uint16().expect("flags"), 6);
    assert_eq!(packet.read_uint32().expect("power"), 131);
    assert_eq!(packet.read_uint32().expect("health"), 1225);
    assert_eq!(packet.read_uint32().expect("max health"), 1225);
    assert_eq!(packet.read_uint32().expect("speed"), 84);
    assert_eq!(packet.read_uint8().expect("quality"), 3);
}
#[tokio::test]
async fn battle_pet_grant_level_does_not_abort_when_calculate_stats_returns_early_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x190);
    install_represented_battle_pet_stat_stores_like_cpp(&mut session);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            breed: 99,
            level: 23,
            exp: 5,
            power: 10,
            health: 50,
            max_health: 100,
            speed: 20,
            quality: 3,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_grant_battle_pet_level_represented_like_cpp(pet_guid, 1),
        RepresentedBattlePetGrantLevelOutcomeLikeCpp::Changed
    );

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("leveled pet");
    assert_eq!(pet.level, 24);
    assert_eq!(pet.exp, 5);
    assert_eq!(pet.max_health, 100);
    assert_eq!(pet.power, 10);
    assert_eq!(pet.speed, 20);
    assert_eq!(pet.health, 100);
    assert_eq!(
        session.represented_battle_pet_level_criteria_like_cpp(),
        &[RepresentedBattlePetLevelCriteriaLikeCpp {
            species: 11,
            level: 24
        }]
    );
    assert_eq!(pet.save_info, RepresentedBattlePetSaveInfoLikeCpp::Changed);
}
#[tokio::test]
async fn battle_pet_grant_experience_applies_cpp_gates_without_side_effects() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x191);
    let max_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x192);
    let unknown_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x193);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            breed: 7,
            level: 23,
            exp: 5,
            health: 50,
            max_health: 100,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.add_represented_battle_pet_packet_info_like_cpp(
        max_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 12,
            breed: 7,
            level: MAX_BATTLE_PET_LEVEL_LIKE_CPP,
            exp: 5,
            health: 50,
            max_health: 100,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );

    assert_eq!(
        session.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            pet_guid,
            1,
            RepresentedBattlePetXpSourceLikeCpp::SpellEffect,
            1.0,
        ),
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::NoJournalLock
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            unknown_guid,
            1,
            RepresentedBattlePetXpSourceLikeCpp::SpellEffect,
            1.0,
        ),
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::UnknownPet
    );
    assert_eq!(
        session.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            pet_guid,
            0,
            RepresentedBattlePetXpSourceLikeCpp::SpellEffect,
            1.0,
        ),
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::InvalidXpOrSource
    );
    assert_eq!(
        session.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            pet_guid,
            1,
            RepresentedBattlePetXpSourceLikeCpp::Invalid,
            1.0,
        ),
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::InvalidXpOrSource
    );
    install_represented_battle_pet_species_flags_like_cpp(
        &mut session,
        11,
        wow_data::BATTLE_PET_SPECIES_FLAG_CANT_BATTLE_LIKE_CPP,
    );
    assert_eq!(
        session.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            pet_guid,
            1,
            RepresentedBattlePetXpSourceLikeCpp::SpellEffect,
            1.0,
        ),
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::CantBattle
    );
    install_represented_battle_pet_species_flags_like_cpp(&mut session, 11, 0);
    assert_eq!(
        session.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            max_guid,
            1,
            RepresentedBattlePetXpSourceLikeCpp::SpellEffect,
            1.0,
        ),
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::AlreadyMaxLevel
    );
    assert_eq!(
        session.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            pet_guid,
            1,
            RepresentedBattlePetXpSourceLikeCpp::SpellEffect,
            1.0,
        ),
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::MissingXpRow
    );

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("unchanged pet");
    assert_eq!(pet.level, 23);
    assert_eq!(pet.exp, 5);
    assert_eq!(pet.health, 50);
    assert_eq!(pet.max_health, 100);
    assert!(
        session
            .represented_battle_pet_level_criteria_like_cpp()
            .is_empty()
    );
    assert!(
        session
            .represented_battle_pet_active_level_criteria_like_cpp()
            .is_empty()
    );
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}
#[tokio::test]
async fn battle_pet_grant_experience_prefers_real_game_table_over_represented_projection_like_cpp()
{
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x197);
    install_represented_battle_pet_stat_stores_like_cpp(&mut session);
    session.set_represented_battle_pet_xp_per_level_like_cpp(23, 999);
    session.set_represented_battle_pet_xp_per_level_like_cpp(24, 999);

    let mut rows = vec![wow_data::BattlePetXpEntryLikeCpp::default(); 24];
    rows[22] = wow_data::BattlePetXpEntryLikeCpp {
        wins: 2.0,
        xp: 50.0,
    };
    rows[23] = wow_data::BattlePetXpEntryLikeCpp {
        wins: 4.0,
        xp: 25.0,
    };
    session.set_battle_pet_xp_game_table(Arc::new(
        wow_data::BattlePetXpGameTableLikeCpp::from_rows(rows),
    ));

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            breed: 7,
            level: 23,
            exp: 20,
            power: 10,
            health: 50,
            max_health: 100,
            speed: 20,
            quality: 3,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            pet_guid,
            150,
            RepresentedBattlePetXpSourceLikeCpp::SpellEffect,
            1.0,
        ),
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::Changed
    );

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("experienced pet");
    assert_eq!(pet.level, 24);
    assert_eq!(pet.exp, 70);
}
#[tokio::test]
async fn battle_pet_grant_experience_pet_battle_applies_multiplier_and_active_criteria_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x195);
    install_represented_battle_pet_stat_stores_like_cpp(&mut session);
    session.set_represented_battle_pet_xp_per_level_like_cpp(23, 100);
    session.set_represented_battle_pet_xp_per_level_like_cpp(24, 100);
    session.set_represented_battle_pet_xp_per_level_like_cpp(25, 100);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            breed: 7,
            level: 23,
            exp: 0,
            power: 10,
            health: 50,
            max_health: 100,
            speed: 20,
            quality: 3,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            pet_guid,
            200,
            RepresentedBattlePetXpSourceLikeCpp::PetBattle,
            1.5,
        ),
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::Changed
    );

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("experienced pet");
    assert_eq!(pet.level, MAX_BATTLE_PET_LEVEL_LIKE_CPP);
    assert_eq!(pet.exp, 0);
    assert_eq!(pet.health, 1225);
    assert_eq!(pet.max_health, 1225);
    assert_eq!(pet.power, 131);
    assert_eq!(pet.speed, 84);
    let expected = [
        RepresentedBattlePetLevelCriteriaLikeCpp {
            species: 11,
            level: 24,
        },
        RepresentedBattlePetLevelCriteriaLikeCpp {
            species: 11,
            level: 25,
        },
    ];
    assert_eq!(
        session.represented_battle_pet_level_criteria_like_cpp(),
        &expected
    );
    assert_eq!(
        session.represented_battle_pet_active_level_criteria_like_cpp(),
        &expected
    );
}
#[tokio::test]
async fn battle_pet_grant_experience_missing_later_xp_row_keeps_prior_criteria_but_not_pet_mutation_like_cpp()
 {
    let (mut session, _, send_rx) = make_session();
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x196);
    session.set_represented_battle_pet_xp_per_level_like_cpp(23, 100);
    session.set_represented_battle_pet_xp_per_level_like_cpp(24, 100);

    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            breed: 7,
            level: 23,
            exp: 0,
            power: 10,
            health: 50,
            max_health: 100,
            speed: 20,
            quality: 3,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    assert_eq!(
        session.battle_pet_grant_battle_pet_experience_represented_like_cpp(
            pet_guid,
            250,
            RepresentedBattlePetXpSourceLikeCpp::PetBattle,
            1.0,
        ),
        RepresentedBattlePetGrantExperienceOutcomeLikeCpp::MissingXpRow
    );

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("unchanged pet");
    assert_eq!(pet.level, 23);
    assert_eq!(pet.exp, 0);
    assert_eq!(pet.health, 50);
    assert_eq!(pet.max_health, 100);
    assert_eq!(
        session.represented_battle_pet_level_criteria_like_cpp(),
        &[RepresentedBattlePetLevelCriteriaLikeCpp {
            species: 11,
            level: 24
        }]
    );
    assert_eq!(
        session.represented_battle_pet_active_level_criteria_like_cpp(),
        &[RepresentedBattlePetLevelCriteriaLikeCpp {
            species: 11,
            level: 24
        }]
    );
    assert!(drain_server_packet_bytes(&send_rx).is_empty());
}
#[test]
fn battle_pet_summon_toggles_known_pet_and_ignores_unknown_like_cpp() {
    let (mut session, _, _) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x135);
    let other_guid = ObjectGuid::new(0, 0x136);
    let unknown_guid = ObjectGuid::new(0, 0x137);

    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );
    session.add_represented_battle_pet_like_cpp(
        other_guid,
        0,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );

    assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));
    assert_eq!(
        session.represented_summoned_battle_pet_guid_like_cpp(),
        Some(pet_guid)
    );

    assert!(session.battle_pet_summon_toggle_like_cpp(other_guid));
    assert_eq!(
        session.represented_summoned_battle_pet_guid_like_cpp(),
        Some(other_guid)
    );

    assert!(session.battle_pet_summon_toggle_like_cpp(other_guid));
    assert_eq!(
        session.represented_summoned_battle_pet_guid_like_cpp(),
        None
    );

    assert!(!session.battle_pet_summon_toggle_like_cpp(unknown_guid));
    assert_eq!(
        session.represented_summoned_battle_pet_guid_like_cpp(),
        None
    );
}
#[test]
fn battle_pet_update_notify_requires_known_active_pet_like_cpp() {
    let (mut session, _, _) = make_session();
    let pet_guid = ObjectGuid::new(0, 0x138);
    let other_guid = ObjectGuid::new(0, 0x139);
    let unknown_guid = ObjectGuid::new(0, 0x13a);

    session.add_represented_battle_pet_like_cpp(
        pet_guid,
        0,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );
    session.add_represented_battle_pet_like_cpp(
        other_guid,
        0,
        RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    );

    assert!(!session.battle_pet_update_notify_like_cpp(pet_guid));
    assert_eq!(session.represented_battle_pet_data_updates_like_cpp(), &[]);

    assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));
    assert!(!session.battle_pet_update_notify_like_cpp(other_guid));
    assert!(!session.battle_pet_update_notify_like_cpp(unknown_guid));
    assert_eq!(session.represented_battle_pet_data_updates_like_cpp(), &[]);

    assert!(session.battle_pet_update_notify_like_cpp(pet_guid));
    assert_eq!(
        session.represented_battle_pet_data_updates_like_cpp(),
        &[pet_guid]
    );
}
#[test]
fn battle_pet_update_notify_sets_canonical_player_pet_data_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::new(60_000, 1)));
    let player_guid = ObjectGuid::create_player(1, 42);
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x13b);
    session.set_canonical_map_manager(Arc::clone(&canonical));
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
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "PetOwner".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical player map");
    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            creature_id: 22,
            display_id: 33,
            breed: 44,
            level: 17,
            exp: 0,
            flags: 0,
            power: 0,
            health: 0,
            max_health: 0,
            speed: 0,
            quality: 3,
            owner_info: None,
            name: String::new(),
            name_timestamp: 0,
            declined_names: None,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
        },
    );
    assert!(session.battle_pet_summon_toggle_like_cpp(pet_guid));

    assert!(session.battle_pet_update_notify_like_cpp(pet_guid));

    let (summoned_guid, quality, level, player_mask, active_mask, unit_mask) = session
        .mutate_canonical_player_like_cpp(|player| {
            (
                player.active_data().summoned_battle_pet_guid,
                player.data().current_battle_pet_breed_quality,
                player.unit().data().wild_battle_pet_level,
                player.player_data_changes_mask().clone(),
                player.active_player_data_changes_mask().clone(),
                player.unit().unit_data_changes_mask().clone(),
            )
        })
        .expect("canonical player");
    assert_eq!(summoned_guid, pet_guid);
    assert_eq!(quality, 3);
    assert_eq!(level, 17);
    assert!(player_mask.is_set(wow_entities::PLAYER_DATA_CURRENT_BATTLE_PET_BREED_QUALITY_BIT));
    assert!(active_mask.is_set(wow_entities::ACTIVE_PLAYER_DATA_SUMMONED_BATTLE_PET_GUID_BIT));
    assert!(unit_mask.is_set(wow_entities::UNIT_DATA_WILD_BATTLE_PET_LEVEL_BIT));
}
#[test]
fn battle_pet_query_companion_reads_canonical_map_unitdata_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let unit_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 777, 42);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let battle_pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 43);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    add_canonical_test_creature_indexed_on_map_with_level(
        &canonical,
        unit_guid,
        777,
        Position::default(),
        571,
        0,
        1,
    );
    {
        let mut manager = canonical.lock().unwrap();
        let creature = manager
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(unit_guid)
            .unwrap();
        creature.set_summon_like_cpp(true);
        creature
            .unit_mut()
            .subsystems_mut()
            .control
            .set_owner_guid(Some(owner_guid));
        creature
            .unit_mut()
            .set_battle_pet_companion_guid_like_cpp(Some(battle_pet_guid));
        creature
            .unit_mut()
            .set_battle_pet_companion_name_timestamp_like_cpp(1234);
    }

    assert_eq!(
        session.represented_battle_pet_query_companion_like_cpp(unit_guid),
        Some(RepresentedBattlePetQueryCompanionLikeCpp {
            creature_id: 777,
            name_timestamp: 1234,
            is_summon: true,
            owner_is_player: true,
            battle_pet_companion_guid: Some(battle_pet_guid),
        })
    );
}
