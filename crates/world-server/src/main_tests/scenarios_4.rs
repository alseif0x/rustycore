//! Scenarios for [`super`], part 4.
//!
//! Split out of main_tests.rs under #628; assertions and registrations are
//! unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn world_listener_captures_application_resources_outside_transport_boundary() {
    let source = fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/app.rs"))
        .expect("world-server composition source should be readable");
    let listener_start = source
        .find("wow_network::start_world_listener(")
        .expect("world listener call must exist");
    let listener_end = listener_start
        + source[listener_start..]
            .find("realm_listener_ready_tx,")
            .expect("world listener readiness argument must exist");
    let listener_call = &source[listener_start..listener_end];

    assert!(source[..listener_start].contains("let resources = Arc::clone(&session_resources);"));
    assert!(
        listener_call.contains(
            "move |account, pkt_rx, send_tx, send_write_fence_like_cpp, socket_timeouts|"
        )
    );
    assert!(listener_call.contains("let resources = Arc::clone(&resources);"));
    assert!(listener_call.contains("create_session("));
    assert!(
        !listener_call.contains("session_resources"),
        "the application aggregate must be captured outside the listener call"
    );
}
#[test]
fn primary_profession_capacity_config_and_session_resource_wiring_are_pinned() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");

    for (configured, expected) in [
        (None, 2),
        (Some("0"), 0),
        (Some("1"), 1),
        (Some("2"), 2),
        (Some("11"), 11),
        (Some("-1"), 2),
        (Some("12"), 2),
    ] {
        let source = configured
            .map(|value| format!("MaxPrimaryTradeSkill = {value}\n"))
            .unwrap_or_default();
        wow_config::load_config_from_str(&source).expect("config should load");
        let configs = wow_config::load_world_config_values();
        assert_eq!(
            max_primary_trade_skills_like_cpp(&configs),
            expected,
            "configured value {configured:?}"
        );
    }

    let composition_source =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/app.rs"))
            .expect("world-server composition source should be readable");
    let resources_source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/session_resources.rs"),
    )
    .expect("world-server session resources source should be readable");
    let materialization_needle = [
        "max_primary_trade_skills:",
        " max_primary_trade_skills_like_cpp(&world_configs),",
    ]
    .concat();
    let propagation_needle = [
        "session.set_max_primary_trade_skills_like_cpp(",
        "self.max_primary_trade_skills);",
    ]
    .concat();
    assert!(
        composition_source.contains(&materialization_needle),
        "SessionResources must materialize the validated configuration"
    );
    assert!(
        resources_source.contains(&propagation_needle),
        "the progression capability must propagate its validated policy atomically"
    );
}
#[test]
fn production_persistence_capabilities_are_required_and_installed_atomically() {
    let composition_source =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/app.rs"))
            .expect("world-server composition source should be readable");
    let resources_source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/session_resources.rs"),
    )
    .expect("SessionResources source should be readable");
    let session_factory_source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/session_factory.rs"),
    )
    .expect("world-server session factory source should be readable");

    for constructor in [
        "SessionAdmissionPersistenceLikeCpp::required_like_cpp(",
        "PlayerPersistenceCapabilitiesLikeCpp::required_like_cpp(",
        "WorldPersistenceCapabilitiesLikeCpp::required_like_cpp(",
        "CatalogPersistenceCapabilitiesLikeCpp::required_like_cpp(",
    ] {
        assert!(
            composition_source.contains(constructor),
            "the composition root must construct required capability {constructor}"
        );
    }
    assert!(
        resources_source.contains("core: SessionCoreCapabilitiesLikeCpp"),
        "SessionResources must require the core capability bundle"
    );
    assert!(
        resources_source
            .contains("persistence: wow_world::session::SessionPersistencePortsLikeCpp"),
        "the core capability bundle must carry one complete persistence graph"
    );
    assert!(
        session_factory_source
            .contains("resources.core.install_into_session_like_cpp(&mut session)"),
        "create_session must install the complete core graph atomically"
    );
    assert!(
        resources_source.contains(
            "session.set_required_persistence_capabilities_like_cpp(self.persistence.clone())"
        ),
        "the atomic core installer must publish the complete persistence graph"
    );
    assert!(
        !session_factory_source.contains("if let Some(ref port) = resources."),
        "production session construction must not silently omit persistence capabilities"
    );
}
#[test]
fn session_resources_requires_named_capability_bundles() {
    let composition_source =
        fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/app.rs"))
            .expect("world-server composition source should be readable");
    let resources_source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/session_resources.rs"),
    )
    .expect("SessionResources source should be readable");
    let session_factory_source = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/session_factory.rs"),
    )
    .expect("session factory source should be readable");
    let session_resources = resources_source
        .split("pub(super) struct SessionResources {")
        .nth(1)
        .and_then(|tail| tail.split_once("\n}").map(|(body, _)| body))
        .expect("SessionResources declaration should be present");

    for required_bundle in [
        "core: SessionCoreCapabilitiesLikeCpp",
        "inventory: SessionInventoryCapabilitiesLikeCpp",
        "player: SessionPlayerCatalogCapabilitiesLikeCpp",
        "spells: SessionSpellCatalogCapabilitiesLikeCpp",
        "world: SessionWorldCatalogCapabilitiesLikeCpp",
        "progression: SessionProgressionCapabilitiesLikeCpp",
        "runtime: SessionRuntimePolicyCapabilitiesLikeCpp",
        "realm: SessionRealmCapabilitiesLikeCpp",
    ] {
        assert!(
            session_resources.contains(required_bundle),
            "missing required capability bundle {required_bundle}"
        );
    }
    assert_eq!(
        session_resources.matches("pub(super)").count(),
        8,
        "the outer construction contract must not regress into a field-by-field service locator"
    );
    assert!(
        !session_resources.contains("Option<"),
        "named capability bundles must be mandatory at production construction"
    );
    assert!(
        !resources_source.contains("Option<"),
        "every inner production capability must also be required by its Rust type"
    );
    for retired_test_only_catalog in [
        "adventure_map_poi_store",
        "addon_channel",
        "allow_gm_group",
        "allow_two_side_interaction_group",
        "area_trigger_db2_store",
        "area_trigger_script_store",
        "area_trigger_store",
        "bank_bag_slot_prices_store",
        "battlemaster_list_store",
        "characters_per_realm",
        "chat_fake_message_preventing",
        "chat_flood_config",
        "chat_level_requirements",
        "chat_listen_ranges",
        "chat_strict_link_checking_kick",
        "emotes_store",
        "emotes_text_store",
        "feature_system_bpay_store_enabled",
        "feature_system_character_undelete_enabled",
        "graveyard_store",
        "import_price_stores",
        "item_class_store",
        "item_currency_cost_store",
        "item_disenchant_loot_store",
        "item_price_base_store",
        "lfg_dungeon_store_like_cpp",
        "module_registry",
        "pet_default_spell_store",
        "pet_family_spell_store",
        "pet_levelup_spell_store",
        "player_create_cast_spell_store",
        "player_create_custom_spell_store",
        "player_create_info_store",
        "party_level_req",
        "party_raid_warnings",
        "serverside_spell_store",
        "spell_enchant_proc_store",
        "spell_totem_model_store",
        "tact_key_store",
        "tavern_area_trigger_store",
        "support_bugs_enabled",
        "support_complaints_enabled",
        "support_enabled",
        "support_suggestions_enabled",
        "support_tickets_enabled",
        "vehicle_template_store",
    ] {
        assert!(
            !resources_source.contains(retired_test_only_catalog),
            "test-only catalog {retired_test_only_catalog} must not be projected into production sessions"
        );
    }
    for retired_session_generator in [
        "pub(super) guid_generator:",
        "pub(super) item_guid_generator:",
        "pub(super) equipment_set_guid_generator:",
        "pub(super) void_storage_item_id_generator:",
    ] {
        assert!(
            !resources_source.contains(retired_session_generator),
            "process-owned generator {retired_session_generator} must be borrowed by the handler instead of installed into WorldSession"
        );
    }
    assert!(
        composition_source
            .contains("id_generators: Arc::new(wow_world::session::SessionIdGeneratorsLikeCpp"),
        "the composition root must group process-owned generators in the borrowed handler capabilities"
    );
    assert!(
        composition_source
            .contains("item_valuation: Arc::new(wow_world::session::ItemValuationCatalogsLikeCpp"),
        "the composition root must group process-owned item valuation stores in borrowed handler capabilities"
    );
    assert!(
        !resources_source.contains("session.set_item_disenchant_loot_store("),
        "item valuation catalogs must be borrowed by loot handlers instead of installed into WorldSession"
    );
    assert!(
        composition_source.contains(
            "player_bootstrap: Arc::new(wow_world::session::PlayerBootstrapCatalogsLikeCpp"
        ),
        "the composition root must group ObjectMgr PlayerInfo data in borrowed login capabilities"
    );
    assert!(
        !resources_source.contains("session.set_player_create_info_store_like_cpp("),
        "PlayerInfo creation data must be borrowed during login instead of installed into WorldSession"
    );
    for retired_player_start_copy in [
        "session.set_start_all_explored_like_cpp(",
        "session.set_start_all_reputation_like_cpp(",
        "session.set_start_all_spells_like_cpp(",
    ] {
        assert!(
            !resources_source.contains(retired_player_start_copy),
            "C++ World player-start policy {retired_player_start_copy} must be borrowed during Player bootstrap"
        );
    }
    assert!(
        composition_source.contains(
            "player_rest_rates: Arc::new(wow_world::session::PlayerRestRatePolicyLikeCpp"
        ),
        "the composition root must group C++ World rest rates in one borrowed Player policy"
    );
    assert!(
        !resources_source.contains("rest_offline_wilderness_rate: f32")
            && !resources_source.contains("rest_offline_tavern_or_city_rate: f32")
            && !resources_source.contains("rest_ingame_rate: f32"),
        "C++ World rest rates must not remain in the SessionResources installation graph"
    );
    assert!(
        composition_source
            .contains("creature_spawns: Arc::new(wow_world::session::CreatureSpawnCatalogsLikeCpp"),
        "the composition root must group ObjectMgr/World creature materialization catalogs"
    );
    for retired_creature_catalog_copy in [
        "session.set_creature_difficulty_store_like_cpp(",
        "session.set_creature_base_stats_store_like_cpp(",
        "session.set_creature_health_rates_like_cpp(",
        "session.set_creature_addon_store_like_cpp(",
        "session.set_creature_equipment_store_like_cpp(",
    ] {
        assert!(
            !resources_source.contains(retired_creature_catalog_copy),
            "process-owned creature catalog {retired_creature_catalog_copy} must be borrowed during materialization"
        );
    }
    assert!(
        composition_source
            .contains("chat_policy: Arc::new(wow_world::session::ChatPolicyCatalogsLikeCpp"),
        "the composition root must group process-owned C++ World chat policy for dispatch"
    );
    assert!(
        !resources_source.contains("session.set_chat_flood_config_like_cpp("),
        "C++ World chat policy must be borrowed by handlers instead of copied into WorldSession"
    );
    assert!(
        composition_source
            .contains("group_invite_policy: Arc::new(wow_world::session::GroupInvitePolicyLikeCpp"),
        "the composition root must group process-owned C++ World party-invite policy"
    );
    assert!(
        !resources_source.contains("session.set_party_level_req_like_cpp("),
        "C++ World party policy must be borrowed by handlers instead of copied into WorldSession"
    );
    assert!(
        composition_source.contains(
            "support_feature_policy: Arc::new(wow_world::session::SupportFeaturePolicyLikeCpp"
        ),
        "the composition root must group C++ SupportMgr and feature-system process policy"
    );
    assert!(
        !resources_source.contains("session.set_represented_support_enabled_like_cpp("),
        "C++ support policy must be borrowed by handlers instead of copied into WorldSession"
    );
    assert!(
        resources_source
            .contains("handler_catalogs: Arc<wow_world::session::SessionHandlerCatalogsLikeCpp>"),
        "the composition owner must retain the required immutable dispatch catalogs"
    );
    assert!(
        !resources_source.contains("session.set_object_mgr_catalogs_like_cpp("),
        "ObjectMgr query catalogs must not be projected into production WorldSession state"
    );
    assert!(
        session_factory_source.contains("resources.core.handler_catalogs.as_ref()"),
        "the outer driver must borrow the process-owned catalogs for dispatch"
    );
    // #787 moved the phases themselves into the task that owns the session:
    // the driver no longer calls the update/process pair on its own clock, it
    // runs the phase the canonical producer asks for. The invariant this
    // guarded is unchanged — the catalogs are borrowed at the call, never
    // installed into the session.
    assert!(
        session_factory_source
            .contains(".run_requested_session_phase_like_cpp(request, handler_catalogs)"),
        "the driver must pass immutable catalogs explicitly instead of installing a session locator"
    );
    assert!(
        !session_factory_source.contains("set_session_handler_catalogs_like_cpp("),
        "the session pass must borrow runtime catalogs instead of retaining them"
    );
    assert_eq!(
        session_factory_source
            .matches("install_into_session_like_cpp(&mut session")
            .count(),
        8,
        "the factory must install exactly the eight named capability bundles"
    );
    for forbidden_projection in [
        "resources.core.persistence",
        "resources.inventory.item_store",
        "resources.player.condition_store",
        "resources.spells.spell_store",
        "resources.progression.quest_store",
        "resources.runtime.module_registry",
        "resources.realm.realm_names",
    ] {
        assert!(
            !session_factory_source.contains(forbidden_projection),
            "the factory must not project bundle member {forbidden_projection} into WorldSession"
        );
    }

    let construction = composition_source
        .find("let session_resources = SessionResources {")
        .expect("SessionResources construction should exist");
    let publication = composition_source
        .find("let session_resources = Arc::new(session_resources);")
        .expect("fully constructed resources should be published through Arc");
    let listener = composition_source
        .find("wow_network::start_world_listener(")
        .expect("world listener should exist");
    assert!(construction < publication && publication < listener);
}
#[test]
fn realm_id_config_is_required_and_non_zero_like_cpp() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("").expect("config should load");
    let missing = realm_id_like_cpp().expect_err("missing RealmID must fail");
    assert!(
        missing
            .to_string()
            .contains("Realm ID not defined in configuration file")
    );

    wow_config::load_config_from_str("RealmID = 0\n").expect("config should load");
    let zero = realm_id_like_cpp().expect_err("RealmID 0 must fail");
    assert!(
        zero.to_string()
            .contains("Realm ID not defined in configuration file")
    );

    wow_config::load_config_from_str("RealmID = 3\n").expect("config should load");
    assert_eq!(realm_id_like_cpp().expect("valid RealmID"), 3);
}
#[test]
fn db_keepalive_config_and_pool_scope_match_cpp() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");

    wow_config::load_config_from_str("").expect("config should load");
    let configs = wow_config::load_world_config_values();
    assert_eq!(db_keepalive_interval_minutes_like_cpp(&configs), 30);

    wow_config::load_config_from_str("MaxPingTime = 7\n").expect("config should load");
    let configs = wow_config::load_world_config_values();
    assert_eq!(db_keepalive_interval_minutes_like_cpp(&configs), 7);
    assert_eq!(
        db_keepalive_database_names_like_cpp(),
        ["Character", "Login", "World"]
    );
    assert_eq!(db_keepalive_sql_like_cpp(), "SELECT 1");
}
#[test]
fn world_db_version_sentinel_accepts_only_current_tdb_like_cpp() {
    assert_eq!(REQUIRED_TDB_VERSION_LIKE_CPP, "TDB 343.24081");
    assert_eq!(REQUIRED_TDB_CACHE_ID_LIKE_CPP, 24081);

    let current = WorldDbVersionLikeCpp {
        db_version: REQUIRED_TDB_VERSION_LIKE_CPP.to_string(),
        cache_id: REQUIRED_TDB_CACHE_ID_LIKE_CPP,
    };
    assert!(world_db_version_matches_required_like_cpp(&current));

    let wrong_version = WorldDbVersionLikeCpp {
        db_version: "TDB 343.24080".to_string(),
        cache_id: REQUIRED_TDB_CACHE_ID_LIKE_CPP,
    };
    assert!(!world_db_version_matches_required_like_cpp(&wrong_version));

    let wrong_cache = WorldDbVersionLikeCpp {
        db_version: REQUIRED_TDB_VERSION_LIKE_CPP.to_string(),
        cache_id: 24080,
    };
    assert!(!world_db_version_matches_required_like_cpp(&wrong_cache));
}
#[test]
fn world_db_version_mismatch_reports_expected_and_found_like_cpp() {
    let mismatch = WorldDbVersionLikeCpp {
        db_version: "TDB 343.00000".to_string(),
        cache_id: 0,
    };
    let message = world_db_version_mismatch_message_like_cpp(Some(&mismatch));

    assert!(message.contains("World database version mismatch"));
    assert!(message.contains("expected TDB 343.24081 / cache_id 24081"));
    assert!(message.contains("found TDB 343.00000 / cache_id 0"));

    let missing = world_db_version_mismatch_message_like_cpp(None);
    assert!(missing.contains("Unknown world database."));
}
#[test]
fn database_pool_size_uses_cpp_worker_and_synch_thread_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");

    wow_config::load_config_from_str("").expect("config should load");
    assert_eq!(database_pool_size_like_cpp("Login"), 2);
    assert_eq!(database_pool_size_like_cpp("Character"), 2);

    wow_config::load_config_from_str(
        r#"
LoginDatabase.WorkerThreads = 3
LoginDatabase.SynchThreads = 5
CharacterDatabase.WorkerThreads = 1
CharacterDatabase.SynchThreads = 2
WorldDatabase.WorkerThreads = 0
WorldDatabase.SynchThreads = 33
"#,
    )
    .expect("config should load");

    assert_eq!(database_pool_size_like_cpp("Login"), 8);
    assert_eq!(database_pool_size_like_cpp("Character"), 3);
    assert_eq!(database_pool_size_like_cpp("World"), 2);
}
/// The sessionless tap index must answer what the session answered.
///
/// `WorldSession::current_group_member_guids_for_tap_like_cpp` reads the
/// session's own `group_guid` mirror plus the registry. The tick owner has no
/// session, so it asks the membership authority directly — and the two must
/// agree, member for member, or relocating the melee phase would silently
/// change who is tapped in (#28).
#[test]
fn sessionless_tap_group_index_matches_the_session_answer_like_cpp() {
    use wow_social::group::{GroupInfo, GroupRegistry};

    let leader = ObjectGuid::create_player(1, 6_001);
    let second = ObjectGuid::create_player(1, 6_002);
    let third = ObjectGuid::create_player(1, 6_003);
    let ungrouped = ObjectGuid::create_player(1, 6_004);

    let registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(leader);
    assert!(group.add_member(second));
    assert!(group.add_member(third));
    let group_guid = group.group_guid;
    registry.register_group_like_cpp(group_guid, group);

    let index = build_tap_group_index_like_cpp(Some(&registry));

    // Every member sees the others and never itself — the exact contract of
    // `current_group_member_guids_for_tap_like_cpp`.
    for (member, expected) in [
        (leader, vec![second, third]),
        (second, vec![leader, third]),
        (third, vec![leader, second]),
    ] {
        let mut actual = index.get(&member).cloned().unwrap_or_default();
        actual.sort();
        let mut expected = expected;
        expected.sort();
        assert_eq!(actual, expected, "tap group for {member:?}");
        assert!(!actual.contains(&member), "a member never taps itself in");
    }

    assert!(
        index.get(&ungrouped).is_none(),
        "an ungrouped player has no tap group, as the session returns an empty vec"
    );
    assert!(
        build_tap_group_index_like_cpp(None).is_empty(),
        "no registry means no tap groups, not a panic"
    );
}
/// The tick owner is decided once, before the loop that reads it starts.
///
/// A flip after `spawn_legacy_creature_runtime_update_loop_like_cpp` is the only
/// remaining window in which the loop and a session can both tick the same
/// creature, so the single production call site is asserted rather than left to
/// convention (#28).
#[test]
fn set_tick_owner_has_exactly_one_production_call_site_before_the_loop_spawns() {
    let app = include_str!("../app.rs");
    let calls: Vec<_> = app.match_indices("set_tick_owner(").collect();
    assert_eq!(
        calls.len(),
        1,
        "production must decide the tick owner exactly once, found {}",
        calls.len()
    );
    let spawn = app
        .find("spawn_legacy_creature_runtime_update_loop_like_cpp(")
        .expect("the global legacy creature loop is spawned in app.rs");
    assert!(
        calls[0].0 < spawn,
        "the owner must be set before the loop that reads it is spawned"
    );

    for source in [
        include_str!("../lib.rs"),
        include_str!("../runtime/delivery.rs"),
        include_str!("../runtime/map.rs"),
    ] {
        assert!(
            !source.contains("set_tick_owner("),
            "only app.rs may decide the tick owner"
        );
    }
}
#[test]
fn startup_has_no_schema_mutation_and_validates_before_runtime_writes() {
    let app = include_str!("../app.rs");
    assert!(!app.contains(&["populate_typed_", "database_like_cpp"].concat()));
    assert!(!app.contains(&["update_typed_", "database_like_cpp"].concat()));
    assert!(!app.contains("Updates.SourcePath"));
    assert!(!app.contains(&["update-databases", "-only"].concat()));
    assert!(!app.contains("auto_create"));
    assert_eq!(app.matches("validate_runtime_schema(").count(), 1);
    let validation = app.find("validate_runtime_schema(").unwrap();
    let first_runtime_write = app.find("clear_online_accounts_like_cpp(").unwrap();
    assert!(validation < first_runtime_write);
}
#[test]
fn legacy_creature_global_runtime_config_defaults_to_cpp_map_owned_runtime() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");

    wow_config::load_config_from_str("").expect("config should load");
    assert!(legacy_creature_global_runtime_enabled_from_config_like_cpp());

    wow_config::load_config_from_str("RustyCore.LegacyCreatureGlobalRuntime = 0\n")
        .expect("config should load");
    assert!(!legacy_creature_global_runtime_enabled_from_config_like_cpp());

    wow_config::load_config_from_str("RustyCore.LegacyCreatureGlobalRuntime = 1\n")
        .expect("config should load");
    assert!(legacy_creature_global_runtime_enabled_from_config_like_cpp());
}
#[test]
fn legacy_creature_aggro_config_uses_cpp_no_gray_aggro_keys_like_cpp() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");

    wow_config::load_config_from_str(
        r#"
MaxPlayerLevel = 70
NoGrayAggro.Above = 80
NoGrayAggro.Below = 90
Rate.Creature.Aggro = 2
Visibility.Distance.Continents = 20
Visibility.Distance.Instances = 9999
Visibility.Distance.BG = 140
Visibility.Distance.Arenas = 150
CreatureFamilyAssistanceRadius = 22
CreatureFamilyAssistanceDelay = 3456
"#,
    )
    .expect("config should load");
    let configs = wow_config::load_world_config_values();
    let config = legacy_creature_aggro_config_like_cpp(&configs);

    // C++ first clamps NoGrayAggro values to MaxPlayerLevel, then clamps
    // Below down to Above when Above > 0 && Above < Below.
    assert_eq!(config.no_gray_aggro_above, 70);
    assert_eq!(config.no_gray_aggro_below, 70);
    assert_eq!(config.creature_aggro_rate, 2.0);
    assert_eq!(config.max_player_level_config, 70);
    assert_eq!(config.visibility_distance_continents, 90.0);
    assert_eq!(
        config.visibility_distance_instances,
        wow_entities::MAX_VISIBILITY_DISTANCE
    );
    assert_eq!(config.visibility_distance_battlegrounds, 140.0);
    assert_eq!(config.visibility_distance_arenas, 150.0);
    assert_eq!(config.family_assistance_radius, 22.0);
    assert_eq!(config.family_assistance_delay_ms, 3_456);
}
#[test]
fn loot_drop_rates_use_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        r#"
Rate.Drop.Item.Poor = 0.5
Rate.Drop.Item.Rare = 3
Rate.Drop.Item.Referenced = 4
Rate.Drop.Item.ReferencedAmount = 2
Rate.Drop.Money = 6
Rate.Corpse.Decay.Looted = 0.25
"#,
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    let rates = loot_drop_rates_like_cpp(&configs);
    assert_eq!(rates.item_poor, 0.5);
    assert_eq!(rates.item_normal, 1.0);
    assert_eq!(rates.item_rare, 3.0);
    assert_eq!(rates.item_referenced, 4.0);
    assert_eq!(rates.item_referenced_amount, 2.0);
    assert_eq!(rates.money, 6.0);
    assert_eq!(rates.corpse_decay_looted, 0.25);
}
#[test]
fn reputation_rates_use_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        r#"
Rate.Reputation.Gain = 2
Rate.Reputation.LowLevel.Kill = 0.25
Rate.Reputation.LowLevel.Quest = 0.5
Rate.Reputation.RecruitAFriendBonus = 0.2
MaxRecruitAFriendBonusDistance = 45
"#,
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    let rates = reputation_rates_like_cpp(&configs);
    assert_eq!(rates.gain, 2.0);
    assert_eq!(rates.low_level_kill, 0.25);
    assert_eq!(rates.low_level_quest, 0.5);
    assert_eq!(rates.recruit_a_friend_bonus, 0.2);
    assert_eq!(rates.recruit_a_friend_distance, 45.0);
}
#[test]
fn repair_cost_rate_uses_cpp_world_config_key_and_clamps_negative_like_cpp() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("Rate.RepairCost = 2.5\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(repair_cost_rate_like_cpp(&configs), 2.5);

    wow_config::load_config_from_str("Rate.RepairCost = -1\n").expect("config should load");
    let configs = wow_config::load_world_config_values();
    assert_eq!(repair_cost_rate_like_cpp(&configs), 0.0);
}
#[test]
fn reset_schedule_uses_cpp_world_config_defaults_and_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        reset_schedule_like_cpp(&configs),
        ResetSchedule {
            hour: 8,
            week_day: 2,
        }
    );

    wow_config::load_config_from_str(
        r#"
ResetSchedule.Hour = 6
ResetSchedule.WeekDay = 5
"#,
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        reset_schedule_like_cpp(&configs),
        ResetSchedule {
            hour: 6,
            week_day: 5,
        }
    );
}
#[test]
fn enable_ae_loot_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("EnableAELoot = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(&configs, "CONFIG_ENABLE_AE_LOOT", false));
}
#[test]
fn addon_channel_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("AddonChannel = 0\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(!world_config_bool(&configs, "CONFIG_ADDON_CHANNEL", true));
}
#[test]
fn no_reset_talent_cost_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("NoResetTalentsCost = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_NO_RESET_TALENT_COST",
        false
    ));
}
#[test]
fn offhand_check_at_spell_unlearn_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("OffhandCheckAtSpellUnlearn = 0\n")
        .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(!world_config_bool(
        &configs,
        "CONFIG_OFFHAND_CHECK_AT_SPELL_UNLEARN",
        true
    ));
}
#[test]
fn vmap_indoor_check_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("vmap.enableIndoorCheck = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_VMAP_INDOOR_CHECK",
        false
    ));
}
#[test]
fn player_start_explored_and_reputation_use_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "PlayerStart.MapsExplored = 1\nPlayerStart.AllReputation = 1\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_START_ALL_EXPLORED",
        false
    ));
    assert!(world_config_bool(&configs, "CONFIG_START_ALL_REP", false));
}
#[test]
fn instance_ignore_raid_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("Instance.IgnoreRaid = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_INSTANCE_IGNORE_RAID",
        false
    ));
}
#[test]
fn instance_ignore_level_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("Instance.IgnoreLevel = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_INSTANCE_IGNORE_LEVEL",
        false
    ));
}
#[test]
fn account_instances_per_hour_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("AccountInstancesPerHour = 7\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert_eq!(
        world_config_u32(&configs, "CONFIG_MAX_INSTANCES_PER_HOUR", 5),
        7
    );
}
#[test]
fn chat_fake_message_preventing_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("ChatFakeMessagePreventing = 1\n")
        .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_CHAT_FAKE_MESSAGE_PREVENTING",
        false
    ));
}
#[test]
fn party_raid_warnings_uses_cpp_world_config_key() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str("PartyRaidWarnings = 1\n").expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(
        &configs,
        "CONFIG_CHAT_PARTY_RAID_WARNINGS",
        false
    ));
}
#[test]
fn party_invite_configs_use_cpp_world_config_keys() {
    let _guard = TEST_LOCK.lock().expect("test lock poisoned");
    wow_config::load_config_from_str(
        "GM.AllowInvite = 1\n\
         AllowTwoSide.Interaction.Group = 1\n\
         PartyLevelReq = 12\n",
    )
    .expect("config should load");

    let configs = wow_config::load_world_config_values();
    assert!(world_config_bool(&configs, "CONFIG_ALLOW_GM_GROUP", false));
    assert!(world_config_bool(
        &configs,
        "CONFIG_ALLOW_TWO_SIDE_INTERACTION_GROUP",
        false
    ));
    assert_eq!(world_config_u32(&configs, "CONFIG_PARTY_LEVEL_REQ", 1), 12);
}
