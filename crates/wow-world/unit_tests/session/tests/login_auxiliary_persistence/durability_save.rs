//! A populated production inventory load must preserve known durability at normal save.

use super::*;
use wow_constants::ItemUpdateState;
use wow_persistence::{PlayerEquipmentInventoryLoadRowLikeCpp, PlayerInventoryItemLoadRowLikeCpp};

#[tokio::test]
async fn populated_login_loss_map_drain_and_partial_save_preserve_unrepresented_item_fields() {
    let operations = [
        wow_entities::PlayerResurrectionOperation::ReclaimCorpse {
            restore_percent: 0.5,
        },
        wow_entities::PlayerResurrectionOperation::RequestData(
            wow_entities::PlayerResurrectionRequestLikeCpp {
                resurrecter: ObjectGuid::create_player(1, 77),
                map_id: 571,
                position: wow_core::Position::ZERO,
                health: 100,
                mana: 0,
                aura: 0,
            },
        ),
    ];
    for (operation, outcome, concurrent) in operations.into_iter().flat_map(|operation| {
        [
            (PersistenceOutcomeLikeCpp::Applied { rows: 1 }, false),
            (PersistenceOutcomeLikeCpp::Applied { rows: 1 }, true),
            (
                PersistenceOutcomeLikeCpp::Failed {
                    reason: "controlled rollback".into(),
                },
                true,
            ),
        ]
        .map(|(outcome, concurrent)| (operation, outcome, concurrent))
    }) {
        let (mut session, _, output) = make_session();
        let owner = install_canonical_player_owner_for_test(&mut session, 571, 0);
        session.set_item_store(Arc::new(wow_data::ItemStore::from_records([
            wow_data::ItemRecord {
                id: 2121,
                class_id: 4,
                subclass_id: 1,
                material: 0,
                inventory_type: 5,
                sheathe_type: 0,
                random_select: 0,
                random_suffix_group_id: 0,
                scaling_stat_distribution_id: 0,
                scaling_stat_value: 0,
            },
        ])));
        session.set_item_stats_store(Arc::new(
            wow_data::ItemStatsStore::from_sparse_and_random_property_templates(
                [(
                    2121,
                    wow_data::ItemSparseTemplateEntry {
                        flags: [0; 4],
                        bag_family: 0,
                        start_quest_id: 0,
                        stackable: 1,
                        max_count: 0,
                        lock_id: 0,
                        required_reputation_rank: 0,
                        sell_price: 0,
                        buy_price: 0,
                        vendor_stack_count: 1,
                        price_variance: 1.0,
                        price_random_value: 0.0,
                        max_durability: 40,
                        other_faction_item_id: 0,
                        content_tuning_id: 0,
                        player_level_to_item_level_curve_id: 0,
                        limit_category: 0,
                        instance_bound: 0,
                        zone_bound: [0; 2],
                        required_reputation_faction: 0,
                        allowable_class: 0,
                        required_expansion: 0,
                        bonding: 0,
                        container_slots: 0,
                        inventory_type: 5,
                    },
                )],
                [(
                    2121,
                    wow_data::ItemRandomPropertyTemplateEntry {
                        item_level: 5,
                        quality: 1,
                        inventory_type: 5,
                    },
                )],
            ),
        ));
        let loaded = |id| PlayerInventoryItemLoadRowLikeCpp {
            item_entry: 2121,
            item_db_guid: id,
            count: 1,
            durability: 40,
            context: 0,
            flags: 0,
            played_time: 0,
            enchantments: String::new(),
            random_properties_id: 0,
            random_properties_seed: 0,
            gems: std::array::from_fn(|_| (0, String::new(), 0)),
            paid_money: None,
            paid_extended_cost: None,
            expiration: 0,
            spell_charges: String::new(),
        };
        let load_port: Arc<dyn PlayerLifecyclePortLikeCpp> = AuxiliaryLoadPortLikeCpp::new([
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentInventory(vec![
                    PlayerEquipmentInventoryLoadRowLikeCpp {
                        slot: wow_entities::EQUIPMENT_SLOT_CHEST,
                        item: loaded(9121),
                    },
                    PlayerEquipmentInventoryLoadRowLikeCpp {
                        slot: wow_entities::INVENTORY_SLOT_ITEM_START,
                        item: loaded(9122),
                    },
                ]),
            ),
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                PlayerLoginAuxiliaryLoadedLikeCpp::BagInventory(vec![]),
            ),
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                PlayerLoginAuxiliaryLoadedLikeCpp::EquipmentSets(vec![]),
            ),
            PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                PlayerLoginAuxiliaryLoadedLikeCpp::TransmogOutfits(vec![]),
            ),
        ]);
        wow_world_character::load_inventory_for_login_like_cpp(&mut session, &load_port, owner, 1)
            .await
            .expect("populated production inventory load");
        session.set_item_effect_store(Arc::new(wow_data::ItemEffectStore::from_entries([])));
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                assert!(
                    player
                        .inventory_runtime_like_cpp()
                        .carried_inventory_loaded_like_cpp()
                );
                player.set_inventory_slot_count(16);
                assert_eq!(
                    player.get_item_by_pos(
                        wow_entities::INVENTORY_SLOT_BAG_0,
                        wow_entities::INVENTORY_SLOT_ITEM_START
                    ),
                    Some(ObjectGuid::create_item(1, 9122))
                );
            })
            .unwrap();
        let generators = session.id_generators_for_test_like_cpp();
        let creatures = session.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_spell::cast_all_obtain_spells_like_cpp(
            &mut session,
            generators.item.as_ref(),
            &creatures,
        )
        .await
        .expect("loaded carried 2121 has no obtain effects without full Item save authority");
        let item_guid = ObjectGuid::create_item(1, 9121);
        let backpack_guid = ObjectGuid::create_item(1, 9122);
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                assert!(
                    !player
                        .inventory_runtime_like_cpp()
                        .equipment_inventory_authority_complete_like_cpp()
                );
                for guid in [item_guid, backpack_guid] {
                    let item = &player.inventory_runtime_like_cpp().item_objects()[&guid];
                    assert_eq!(item.update_state(), ItemUpdateState::Unchanged);
                    assert_eq!(item.owner_guid(), owner);
                    assert_eq!(item.data().max_durability, 60);
                }
                player.set_player_aura_authority_complete_like_cpp(true);
                player.clear_data_changes();
            })
            .unwrap();
        assert_eq!(
            session.apply_represented_durability_loss_all_like_cpp(0.1, false),
            1
        );
        let canonical = Arc::clone(session.core.canonical_map_manager.as_ref().unwrap());
        canonical.lock().unwrap().update(1);
        {
            let (visibility, mut hub) = crate::session::split_visibility_mut(&mut session);
            assert_eq!(visibility.send_represented_player_unit_values_updates_from_last_map_send_object_updates_like_cpp(&mut hub), 1);
        }
        assert_eq!(
            drain_server_opcodes(&output),
            vec![ServerOpcodes::UpdateObject]
        );

        let save_port = crate::session::RecordingPlayerLifecyclePort::new(outcome.clone());
        session.set_player_lifecycle_port_like_cpp(save_port.clone());
        crate::player_inventory_test_fixture::complete_known_empty_player_currencies_like_cpp(
            &mut session,
        );
        session
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .resurrection_state_mut_like_cpp()
                    .delayed_after_teleport = Some(wow_entities::PlayerResurrectionContinuation {
                    operation,
                    phase: wow_entities::PlayerResurrectionPhase::SavingPlayer,
                });
            })
            .unwrap();
        session
            .core
            .owned_inventory_access_like_cpp()
            .with_inventory_runtime_mut_like_cpp(|runtime| {
                runtime
                    .item_objects_mut()
                    .get_mut(&item_guid)
                    .unwrap()
                    .set_count(7);
            })
            .unwrap();
        save_port.hold_character_save();
        let mut save = Box::pin(session.save_current_player_to_db_like_cpp());
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(std::future::Future::poll(save.as_mut(), &mut context).is_pending());
        let requests = save_port.character_save_requests();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0].item_durability,
            Some(vec![wow_persistence::PlayerItemDurabilitySave {
                item_guid: 9121,
                durability: 34
            }])
        );
        assert_eq!(requests[0].inventory_items, None);
        assert!(!requests[0].committed_groups_like_cpp().inventory_items);
        // The save future owns Session; mutate only through the established Map owner.
        if concurrent {
            let mut manager = canonical.lock().unwrap();
            let player = manager
                .find_map_mut(571, 0)
                .unwrap()
                .map_mut()
                .get_typed_player_mut(owner)
                .unwrap();
            let item = player
                .inventory_runtime_mut_like_cpp()
                .item_objects_mut()
                .get_mut(&item_guid)
                .unwrap();
            item.set_durability(19);
            item.set_count(8);
        }
        save_port.release_character_save();
        save.await;
        session
            .core
            .with_owned_player_like_cpp(|player| {
                assert!(
                    !player
                        .inventory_runtime_like_cpp()
                        .equipment_inventory_authority_complete_like_cpp()
                );
                assert_eq!(
                    player
                        .resurrection_state_like_cpp()
                        .delayed_after_teleport
                        .unwrap()
                        .phase,
                    if matches!(outcome, PersistenceOutcomeLikeCpp::Applied { .. }) {
                        wow_entities::PlayerResurrectionPhase::PlayerSaveApplied
                    } else {
                        wow_entities::PlayerResurrectionPhase::SavingPlayer
                    }
                );
                let item = &player.inventory_runtime_like_cpp().item_objects()[&item_guid];
                assert_eq!(item.data().durability, if concurrent { 19 } else { 34 });
                assert_eq!(item.count(), if concurrent { 8 } else { 7 });
                assert_eq!(item.update_state(), ItemUpdateState::Changed);
                assert!(
                    item.item_data_changes_mask()
                        .is_set(wow_entities::ITEM_DATA_STACK_COUNT_BIT)
                );
                assert_eq!(
                    player.inventory_runtime_like_cpp().item_objects()[&backpack_guid]
                        .data()
                        .durability,
                    40
                );
            })
            .unwrap();
        if matches!(outcome, PersistenceOutcomeLikeCpp::Applied { .. }) {
            session
                .core
                .with_owned_player_mut_like_cpp(|player| {
                    player
                        .resurrection_state_mut_like_cpp()
                        .delayed_after_teleport = None;
                })
                .unwrap();
            session.save_current_player_to_db_like_cpp().await;
            assert_eq!(
                save_port.character_save_requests()[1].item_durability,
                Some(vec![wow_persistence::PlayerItemDurabilitySave {
                    item_guid: 9121,
                    durability: if concurrent { 19 } else { 34 }
                }])
            );
        }
    }
}
