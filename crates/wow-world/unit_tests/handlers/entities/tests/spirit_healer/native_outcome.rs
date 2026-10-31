use super::*;

const AT_LOGIN_RESURRECT: u16 = 0x0100;

pub(crate) fn remove_ghost_spell_metadata(session: &mut WorldSession) {
    session.set_spell_store(Arc::new(wow_data::SpellStore::new()));
}

pub(super) fn install_distinct_graveyards(
    session: &mut WorldSession,
    corpse_guid: ObjectGuid,
) -> Arc<wow_data::GraveyardStore> {
    let maps = crate::session::hub_ref(session)
        .catalogs
        .maps
        .store
        .clone()
        .expect("released ghost map store");
    let corpse_position = Position::new(1.0, 2.0, -600.0, 0.0);
    let manager = crate::session::hub_ref(session)
        .core
        .canonical_map_manager
        .clone()
        .expect("released ghost map manager");
    manager
        .lock()
        .unwrap()
        .find_map_mut(571, 0)
        .expect("released ghost map")
        .map_mut()
        .get_typed_corpse_mut(corpse_guid)
        .expect("released ghost corpse object")
        .world_mut()
        .relocate(corpse_position);
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.resurrection_state_mut_like_cpp().corpse_location = Some((571, corpse_position));
        })
        .expect("canonical Player corpse location");
    let corpse_graveyard = 5;
    let ghost_graveyard = wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP;
    let (safe_locs, _) = wow_data::WorldSafeLocStore::from_rows_like_cpp(
        [
            wow_data::WorldSafeLocRow {
                id: corpse_graveyard,
                map_id: 571,
                x: corpse_position.x,
                y: corpse_position.y,
                z: corpse_position.z,
                facing_degrees: 0.0,
            },
            wow_data::WorldSafeLocRow {
                id: ghost_graveyard,
                map_id: 571,
                x: 50.0,
                y: 60.0,
                z: 70.0,
                facing_degrees: 0.0,
            },
        ],
        &maps,
    );
    session.set_world_safe_loc_store_like_cpp(Arc::new(safe_locs));
    let (graveyards, _) = graveyard_store_with_links(0, [corpse_graveyard, ghost_graveyard], []);
    session.set_graveyard_store(graveyards.clone());
    graveyards
}

#[tokio::test]
async fn native_resurrection_failure_returns_error_and_keeps_player_dead() {
    let (mut session, _send_rx, _old_realm_rx, _port) = logged_in_released_ghost(43_901).await;
    let (realm_tx, realm_rx) = flume::bounded(8);
    session.install_realm_send_channel_for_test(realm_tx);
    let has_ghost = session
        .core
        .with_owned_player_like_cpp(|player| {
            let auras = &player.unit().subsystems().auras;
            auras
                .runtime_applications_like_cpp()
                .values()
                .any(|application| application.spell_id == wow_world_entities::SPELL_GHOST_LIKE_CPP)
        })
        .expect("canonical Player with released-ghost aura");
    assert!(has_ghost);
    remove_ghost_spell_metadata(&mut session);
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();

    let result = wow_world_entities::resurrect_player_like_cpp(
        &mut session,
        wow_world_entities::ResurrectionOperationCatalogs {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creature_spawns,
            graveyards: None,
        },
        0.5,
        false,
    )
    .await;

    assert!(result.is_err());
    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| player.unit().death_state()),
        Some(wow_constants::DeathState::Dead)
    );
    let release_opcodes = realm_rx
        .try_iter()
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
        .collect::<Vec<_>>();
    assert_eq!(
        release_opcodes,
        vec![ServerOpcodes::DeathReleaseLoc as u16],
        "the earlier source-ordered death-release packet is not rolled back"
    );
}

#[tokio::test]
async fn native_resurrection_success_returns_ok_for_canonical_player() {
    for (case, in_world, percent, expected) in
        [(0, true, 0.5, 50), (1, true, 2.0, 100), (2, false, 0.5, 50)]
    {
        let (mut session, _send_rx, realm_rx, _port) =
            logged_in_released_ghost(43_902 + case).await;
        let guid = session.player_guid().unwrap();
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_power_index(wow_constants::PowerType::Mana, Some(3));
                player
                    .unit_mut()
                    .set_max_power(wow_constants::PowerType::Mana, 100);
                player
                    .unit_mut()
                    .set_power(wow_constants::PowerType::Mana, 0);
                player
                    .unit_mut()
                    .set_power_index(wow_constants::PowerType::Rage, Some(1));
                player
                    .unit_mut()
                    .set_max_power(wow_constants::PowerType::Rage, 100);
                player
                    .unit_mut()
                    .set_power(wow_constants::PowerType::Rage, 0);
                player
                    .unit_mut()
                    .set_power_index(wow_constants::PowerType::Energy, None);
                player
                    .unit_mut()
                    .set_power_index(wow_constants::PowerType::Focus, None);
                player.unit_mut().clear_unit_data_changes();
                if !in_world {
                    player
                        .unit_mut()
                        .world_mut()
                        .object_mut()
                        .remove_from_world();
                }
            })
            .unwrap();
        let generators = session.id_generators_for_test_like_cpp();
        let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
        let result = wow_world_entities::resurrect_player_like_cpp(
            &mut session,
            wow_world_entities::ResurrectionOperationCatalogs {
                item_guid_generator: generators.item.as_ref(),
                creature_spawns: &creature_spawns,
                graveyards: None,
            },
            percent,
            false,
        )
        .await;
        if in_world {
            result.expect("native resurrection succeeds with the loaded Ghost/Wisp metadata");
        } else {
            assert_eq!(
                result,
                Err("canonical Player incarnation or residence changed after visibility update")
            );
        }
        assert!(session.player_is_alive_like_cpp());
        session
            .core
            .with_owned_player_like_cpp(|player| {
                assert_eq!(player.unit().world().object().is_in_world(), in_world);
                let max_health = player.unit().data().max_health;
                assert_eq!(
                    player.unit().data().health,
                    ((max_health as f32 * percent) as u64).min(max_health)
                );
                assert_eq!(player.get_power(wow_constants::PowerType::Mana), expected);
                assert_eq!(player.get_power(wow_constants::PowerType::Rage), 0);
                for bit in [116, 137 + 3, 137 + 1] {
                    assert!(player.unit().unit_data_changes_mask().is_set(bit));
                }
            })
            .unwrap();
        let realm = realm_rx.try_iter().collect::<Vec<_>>();
        assert_eq!(realm.len(), if in_world { 3 } else { 1 });
        assert_eq!(
            u16::from_le_bytes([realm[0][0], realm[0][1]]),
            ServerOpcodes::DeathReleaseLoc as u16
        );
        if in_world {
            for (packet, (power, value)) in realm[1..].iter().zip([
                (wow_constants::PowerType::Mana, expected),
                (wow_constants::PowerType::Rage, 0),
            ]) {
                assert_eq!(
                    *packet,
                    wow_packet::packets::combat::PowerUpdate {
                        guid,
                        powers: vec![(value, power as u8)],
                    }
                    .to_bytes()
                );
            }
        }
    }
}

#[tokio::test]
async fn reclaim_resurrection_failure_keeps_corpse_and_dead_state() {
    let (mut session, _send_rx, _realm_rx, port) =
        corpse::released_ghost_without_reclaim_delay_like_cpp(43_903).await;
    let (realm_tx, realm_rx) = flume::bounded(8);
    session.install_realm_send_channel_for_test(realm_tx);
    let player_guid = session.player_guid().expect("released ghost Player GUID");
    let corpse_guid =
        corpse::current_corpse_guid_like_cpp(&session, player_guid).expect("released ghost corpse");
    session.set_player_position_like_cpp(Position::new(10.0, 2.0, 3.0, 0.0));
    remove_ghost_spell_metadata(&mut session);
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();

    let reclaimed = wow_world_entities::reclaim_corpse_like_cpp(
        &mut session,
        wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creature_spawns,
            graveyards: &graveyards,
        },
    )
    .await;

    assert!(!reclaimed);
    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| player.unit().death_state()),
        Some(wow_constants::DeathState::Dead)
    );
    assert_eq!(
        corpse::current_corpse_guid_like_cpp(&session, player_guid),
        Some(corpse_guid)
    );
    assert!(port.delete_requests().is_empty());
    assert_eq!(
        realm_rx
            .try_iter()
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::DeathReleaseLoc as u16],
        "reclaim passed admission and reached the native resurrection failure"
    );
}

#[tokio::test]
async fn reclaim_resurrection_success_still_converts_corpse() {
    let (mut session, _send_rx, _realm_rx, port) = super::logged_in_released_ghost(43_904).await;
    let player_guid = session.player_guid().expect("released ghost Player GUID");
    let _corpse_guid =
        corpse::current_corpse_guid_like_cpp(&session, player_guid).expect("released ghost corpse");
    session.set_player_position_like_cpp(Position::new(10.0, 2.0, 3.0, 0.0));
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();

    let reclaimed = wow_world_entities::reclaim_corpse_like_cpp(
        &mut session,
        wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creature_spawns,
            graveyards: &graveyards,
        },
    )
    .await;

    assert!(reclaimed);
    assert!(session.player_is_alive_like_cpp());
    assert_eq!(
        corpse::current_corpse_guid_like_cpp(&session, player_guid),
        None,
        "successful reclaim replaces the corpse with bones"
    );
    assert!(!port.delete_requests().is_empty());
}

#[tokio::test]
async fn healer_resurrection_failure_skips_durability_bones_and_teleport() {
    let (mut session, send_rx, _old_realm_rx, port) = logged_in_released_ghost(43_905).await;
    let (realm_tx, realm_rx) = flume::bounded(8);
    session.install_realm_send_channel_for_test(realm_tx);
    let player_guid = session.player_guid().expect("released ghost Player GUID");
    let corpse_guid =
        corpse::current_corpse_guid_like_cpp(&session, player_guid).expect("released ghost corpse");
    session.set_player_inventory_slot_count_like_cpp(wow_entities::INVENTORY_DEFAULT_SIZE);
    let item_guid = ObjectGuid::create_item(1, 43_906);
    add_top_level_item(
        &mut session,
        player_guid,
        item_guid,
        43_907,
        EQUIPMENT_SLOT_CHEST,
        100,
        100,
        ItemUpdateState::Unchanged,
    );
    let graveyards = install_distinct_graveyards(&mut session, corpse_guid);
    let corpse_position = wow_world_entities::player_corpse_location_like_cpp(&session)
        .map(|(_, position)| position)
        .expect("released ghost corpse location");
    let ghost_position = session
        .player_position_like_cpp()
        .expect("released ghost position");
    assert_eq!(
        wow_world_entities::closest_graveyard_like_cpp(
            &mut session,
            graveyards.as_ref(),
            571,
            corpse_position,
        ),
        Some(5)
    );
    assert_eq!(
        wow_world_entities::closest_graveyard_like_cpp(
            &mut session,
            graveyards.as_ref(),
            571,
            ghost_position,
        ),
        Some(wow_data::ALLIANCE_GRAVEYARD_SAFE_LOC_ID_LIKE_CPP)
    );
    remove_ghost_spell_metadata(&mut session);
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();

    wow_world_entities::send_spirit_resurrect(
        &mut session,
        wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creature_spawns,
            graveyards: graveyards.as_ref(),
        },
    )
    .await;

    assert_eq!(item_durability(&session, item_guid), 100);
    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(
        session
            .core
            .with_owned_player_like_cpp(|player| player.unit().death_state()),
        Some(wow_constants::DeathState::Dead)
    );
    assert_eq!(
        corpse::current_corpse_guid_like_cpp(&session, player_guid),
        Some(corpse_guid)
    );
    assert!(port.delete_requests().is_empty());
    assert_eq!(session.pending_teleport_like_cpp(), None);
    assert!(
        drain_packets(&send_rx)
            .iter()
            .all(|bytes| packet_opcode(bytes) != ServerOpcodes::MoveTeleport as u16)
    );
    assert_eq!(
        realm_rx
            .try_iter()
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::DeathReleaseLoc as u16],
        "healer reached the native failure but performed no later teleport"
    );
}

#[tokio::test]
async fn healer_resurrection_success_still_applies_durability_and_bones() {
    let (mut session, _send_rx, _realm_rx, port) = logged_in_released_ghost(43_909).await;
    let player_guid = session.player_guid().expect("released ghost Player GUID");
    session.set_player_inventory_slot_count_like_cpp(wow_entities::INVENTORY_DEFAULT_SIZE);
    let item_guid = ObjectGuid::create_item(1, 43_910);
    add_top_level_item(
        &mut session,
        player_guid,
        item_guid,
        43_911,
        EQUIPMENT_SLOT_CHEST,
        100,
        100,
        ItemUpdateState::Unchanged,
    );
    let generators = session.id_generators_for_test_like_cpp();
    let creature_spawns = session.creature_spawn_catalogs_for_test_like_cpp();
    let graveyards = wow_data::GraveyardStore::default();

    wow_world_entities::send_spirit_resurrect(
        &mut session,
        wow_world_entities::CorpseLifecycleCatalogsLikeCpp {
            item_guid_generator: generators.item.as_ref(),
            creature_spawns: &creature_spawns,
            graveyards: &graveyards,
        },
    )
    .await;

    assert!(session.player_is_alive_like_cpp());
    assert_eq!(item_durability(&session, item_guid), 75);
    assert!(!port.delete_requests().is_empty());
}

#[tokio::test]
async fn login_resurrection_failure_returns_error_and_retains_at_login_flag() {
    let (mut session, _send_rx, _old_realm_rx, port) = logged_in_released_ghost(43_908).await;
    let (realm_tx, realm_rx) = flume::bounded(8);
    session.install_realm_send_channel_for_test(realm_tx);
    let player_guid = session.player_guid().expect("released ghost Player GUID");
    session
        .core
        .with_owned_player_mut_like_cpp(|player| {
            player.set_at_login_flags_like_cpp(AT_LOGIN_RESURRECT)
        })
        .expect("canonical Player login flags");
    remove_ghost_spell_metadata(&mut session);

    let result = super::super::corpse_login::restore_corpse(&mut session, None).await;

    assert!(result.is_err());
    assert!(!session.player_is_alive_like_cpp());
    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| {
            player.persistent_capability_state_like_cpp().at_login_flags & AT_LOGIN_RESURRECT
        }),
        Some(AT_LOGIN_RESURRECT)
    );
    assert!(
        !port.delete_requests().is_empty(),
        "C++ LoadCorpse spawns bones before attempting at-login resurrection"
    );
    assert!(
        corpse::current_corpse_guid_like_cpp(&session, player_guid).is_none(),
        "the completed bones phase is not inferred to have rolled back"
    );
    assert_eq!(
        realm_rx
            .try_iter()
            .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>(),
        vec![ServerOpcodes::DeathReleaseLoc as u16],
        "login reached the native resurrection failure after the earlier bones phase"
    );
}
