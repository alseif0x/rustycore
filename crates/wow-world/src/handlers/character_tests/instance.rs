//! Instance scenarios for [`super`].
//!
//! Split out of character_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn typed_map_corpse_base_failure_publishes_nothing_like_cpp() {
    let (session, manager, _) =
        map_corpse_session_with_port_like_cpp(PersistedMapCorpseLoadOutcomeLikeCpp::Failed {
            reason: "base query failed".to_owned(),
        });

    let outcome = session.load_map_corpse_data_like_cpp(571, 9).await;

    assert_eq!(outcome, MapCorpseLoadOutcomeLikeCpp::default());
    assert!(
        !manager
            .lock()
            .unwrap()
            .find_map(571, 9)
            .unwrap()
            .map()
            .corpse_data_loaded_like_cpp()
    );
}
#[tokio::test]
async fn typed_map_corpse_auxiliary_failures_are_independent_and_non_fatal_like_cpp() {
    let cases = [
        (
            MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "phase query failed".to_owned(),
            },
            MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
        ),
        (
            MapCorpseAuxiliaryLoadOutcomeLikeCpp::Loaded(Vec::new()),
            MapCorpseAuxiliaryLoadOutcomeLikeCpp::Failed {
                reason: "customization query failed".to_owned(),
            },
        ),
    ];

    for (phases, customizations) in cases {
        let (session, manager, _) =
            map_corpse_session_with_port_like_cpp(PersistedMapCorpseLoadOutcomeLikeCpp::Loaded {
                corpses: vec![invalid_map_corpse_load_row_like_cpp()],
                phases,
                customizations,
            });

        let outcome = session.load_map_corpse_data_like_cpp(571, 9).await;

        assert_eq!(outcome.invalid_type_rows, 1);
        assert_eq!(outcome.corpses_added, 0);
        assert!(
            manager
                .lock()
                .unwrap()
                .find_map(571, 9)
                .unwrap()
                .map()
                .corpse_data_loaded_like_cpp()
        );
    }
}
#[test]
fn init_world_states_builder_orders_realm_then_map_and_filters_area_like_cpp() {
    let area_store = wow_data::AreaTableStore::from_entries([
        wow_data::AreaTableEntry {
            id: 4395,
            continent_id: 571,
            parent_area_id: 0,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
        wow_data::AreaTableEntry {
            id: 4613,
            continent_id: 571,
            parent_area_id: 4395,
            area_bit: -1,
            exploration_level: 0,
            mount_flags: 0,
            flags: 0,
        },
    ]);
    let templates = [
        LoginWorldStateTemplateLikeCpp {
            id: 10,
            default_value: 1,
            map_ids: BTreeSet::new(),
            area_ids: BTreeSet::new(),
        },
        LoginWorldStateTemplateLikeCpp {
            id: 20,
            default_value: 2,
            map_ids: BTreeSet::from([571]),
            area_ids: BTreeSet::new(),
        },
        LoginWorldStateTemplateLikeCpp {
            id: 30,
            default_value: 3,
            map_ids: BTreeSet::from([571]),
            area_ids: BTreeSet::from([4395]),
        },
        LoginWorldStateTemplateLikeCpp {
            id: 40,
            default_value: 4,
            map_ids: BTreeSet::from([571]),
            area_ids: BTreeSet::from([9999]),
        },
        LoginWorldStateTemplateLikeCpp {
            id: 50,
            default_value: 5,
            map_ids: BTreeSet::from([WORLDSTATE_ANY_MAP_LIKE_CPP]),
            area_ids: BTreeSet::new(),
        },
    ];

    assert_eq!(
        build_initial_world_states_like_cpp(
            templates,
            [(20, 22), (999, 999)],
            571,
            4613,
            Some(&area_store),
        ),
        vec![(10, 1), (50, 5), (20, 22), (30, 3)]
    );
}
#[tokio::test]
async fn binder_activate_rejects_instanceable_map_like_cpp() {
    let (mut session, send_rx, canonical) = make_bank_slot_session(2);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 31);
    insert_banker_creature(&canonical, innkeeper, NPCFlags1::INNKEEPER.bits());
    let player_guid = session.player_guid().expect("player guid");
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
        .relocate(Position::new(0.0, 0.0, 0.0, 0.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    player
        .unit_mut()
        .add_unit_state(wow_constants::unit::UnitState::DIED.bits());
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    const FEIGN_DEATH_SLOT: u8 = 7;
    session.visible_auras.insert(
        FEIGN_DEATH_SLOT,
        AuraApplication {
            spell_id: 5384,
            difficulty_id: 0,
            caster_guid: player_guid,
            slot: FEIGN_DEATH_SLOT,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::FeignDeath),
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: std::time::Instant::now(),
        },
    );
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_INSTANCE,
            expansion_id: 2,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));

    session
        .handle_binder_activate(Hello { unit: innkeeper })
        .await;

    assert!(session.represented_homebind_like_cpp().is_none());
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::AuraUpdate],
        "C++ removes feign death before SendBindPoint rejects an instanceable map"
    );
    assert!(!session.visible_auras.contains_key(&FEIGN_DEATH_SLOT));
    assert_eq!(
        session
            .mutate_canonical_player_like_cpp(|player| player
                .unit()
                .has_unit_state(wow_constants::unit::UnitState::DIED.bits()))
            .expect("canonical player"),
        false
    );
}
#[test]
fn vendor_buy_destination_maps_player_container_like_cpp() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let buy = BuyItem {
        vendor_guid: ObjectGuid::EMPTY,
        container_guid: player_guid,
        quantity: 1,
        muid: 1,
        slot: 35,
        item_type: 0,
        item_id: 700,
    };

    assert_eq!(
        vendor_buy_direct_inventory_destination(player_guid, &buy),
        Some((INVENTORY_SLOT_BAG_0, 35))
    );
}
#[test]
fn enum_character_flags_do_not_map_resting_like_cpp() {
    let flags = enum_character_flags_like_cpp(0x20, 0, 0, None, false);

    assert_eq!(flags.flags, 0);
}
#[test]
fn enum_character_flags_map_ghost_rename_billing_and_declined_like_cpp() {
    let flags = enum_character_flags_like_cpp(
        PLAYER_FLAGS_GHOST_LIKE_CPP,
        AT_LOGIN_RENAME_LIKE_CPP,
        42,
        Some("Genitive"),
        true,
    );

    assert_eq!(
        flags.flags,
        CHARACTER_FLAG_GHOST_LIKE_CPP
            | CHARACTER_FLAG_RENAME_LIKE_CPP
            | CHARACTER_FLAG_LOCKED_BY_BILLING_LIKE_CPP
            | CHARACTER_FLAG_DECLINED_LIKE_CPP
    );
    assert_eq!(flags.flags2, 0);
    assert!(!flags.first_login);
}
