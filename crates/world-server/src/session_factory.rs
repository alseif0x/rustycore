//! Authenticated connection to configured `WorldSession` composition.

use super::*;

pub(super) fn load_realm_info_from_snapshot_like_cpp(
    realm_list: &SharedRealmListLikeCpp,
    realm_id: u16,
) -> Result<RealmListEntryLikeCpp> {
    let realm_list = realm_list.lock().expect("realm list mutex poisoned");
    realm_list
        .get_realm_by_id_like_cpp(u32::from(realm_id))
        .cloned()
        .with_context(|| format!("Realm {realm_id} not found in initialized RealmList snapshot"))
}

pub(super) fn realm_name_records_from_snapshot_like_cpp(
    realm_list: &SharedRealmListLikeCpp,
) -> Arc<Vec<(u32, String, String)>> {
    let realm_list = realm_list.lock().expect("realm list mutex poisoned");
    Arc::new(
        realm_list
            .realms
            .values()
            .map(|realm| {
                (
                    realm.id.address_like_cpp(),
                    realm.name.clone(),
                    realm.normalized_name.clone(),
                )
            })
            .collect(),
    )
}

/// Load the build-specific Win64AuthSeed from `build_info`.
pub(super) async fn load_realm_win64_auth_seed_like_cpp(
    login_db: &LoginDatabase,
    build: u32,
) -> Result<[u8; 16]> {
    let seed_result = login_db
        .direct_query(&format!(
            "SELECT win64AuthSeed FROM build_info WHERE build = {build}"
        ))
        .await
        .context("Failed to query build_info")?;

    let seed_hex: String = if seed_result.is_empty() {
        anyhow::bail!("No build_info entry for build {build}");
    } else {
        seed_result.try_read(0).unwrap_or_default()
    };

    if seed_hex.len() != 32 {
        anyhow::bail!(
            "Invalid Win64AuthSeed for build {build}: expected 32 hex chars, got {}",
            seed_hex.len()
        );
    }

    // Parse hex string into 16 bytes
    let mut seed = [0u8; 16];
    for (i, byte) in seed.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&seed_hex[i * 2..i * 2 + 2], 16)
            .with_context(|| format!("Invalid hex in auth seed at position {i}"))?;
    }

    Ok(seed)
}

/// Resolve a realm endpoint string into the IPv4 address stored by C++ `Realm::Addresses`.
///
/// TrinityCore resolves `realmlist.address` / `localAddress` while building the
/// realm list, then `Realm::GetAddressForClient` selects one of those resolved
/// addresses for both JoinRealm and `SMSG_CONNECT_TO`. Hostnames must therefore
/// be resolved here too; falling back to 127.0.0.1 makes remote clients fail the
/// instance handoff and forces a non-C++ login path.
pub(super) async fn resolve_realm_endpoint_address_like_cpp(
    field_name: &str,
    hostname: &str,
    realm_name: &str,
    realm_id: u32,
) -> Result<[u8; 4]> {
    let endpoints = tokio::net::lookup_host((hostname, 0))
        .await
        .with_context(|| {
            format!(
                "Could not resolve {field_name} {hostname} for realm \"{realm_name}\" id {realm_id}"
            )
        })?;
    let address = first_ipv4_address_like_cpp(endpoints).with_context(|| {
        format!(
            "Could not resolve {field_name} {hostname} for realm \"{realm_name}\" id {realm_id} to an IPv4 address"
        )
    })?;

    tracing::info!(
        field_name,
        hostname,
        %address,
        realm_name,
        realm_id,
        "Resolved realm endpoint address like C++"
    );
    Ok(address.octets())
}

pub(super) fn first_ipv4_address_like_cpp(
    endpoints: impl IntoIterator<Item = SocketAddr>,
) -> Option<Ipv4Addr> {
    endpoints.into_iter().find_map(|endpoint| match endpoint {
        SocketAddr::V4(v4) => Some(*v4.ip()),
        SocketAddr::V6(_) => None,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WorldSessionRunOutcomeLikeCpp {
    Finished,
    ForceCancelled,
}

pub(super) async fn run_world_session_shutdown_finalize_step_like_cpp<F>(
    world_runtime_state: &WorldRuntimeStateLikeCpp,
    step_timeout: Duration,
    step: F,
) -> bool
where
    F: std::future::Future<Output = ()>,
{
    if tokio::time::timeout(step_timeout, step).await.is_ok() {
        true
    } else {
        // A bounded shutdown must not look successful after abandoning player
        // persistence or shared-runtime cleanup work.
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        false
    }
}

pub(super) async fn run_world_session_until_disconnect_like_cpp(
    session: &mut WorldSession,
    account_id: u32,
    active_session_registry: &ActiveWorldSessionRegistryLikeCpp,
    cancellation: &ActiveWorldSessionCancellationLikeCpp,
) -> WorldSessionRunOutcomeLikeCpp {
    tokio::select! {
        _ = cancellation.cancelled_like_cpp() => {
            return WorldSessionRunOutcomeLikeCpp::ForceCancelled;
        }
        _ = session.load_global_account_data_like_cpp() => {}
    }
    tokio::select! {
        _ = cancellation.cancelled_like_cpp() => {
            return WorldSessionRunOutcomeLikeCpp::ForceCancelled;
        }
        _ = session.load_tutorials_data_like_cpp() => {}
    }
    session.send_session_init_packets();

    info!("Session ready for account {account_id}");

    let mut last_session_update = Instant::now();
    loop {
        if active_session_registry.should_stop_sessions_like_cpp() {
            info!(
                account_id,
                "World session observed shutdown gate; disconnecting cooperatively"
            );
            return WorldSessionRunOutcomeLikeCpp::Finished;
        }

        let update = warn_about_sync_queries_scope_like_cpp(async {
            let now = Instant::now();
            let diff_ms = now
                .saturating_duration_since(last_session_update)
                .as_millis()
                .min(u128::from(u32::MAX)) as u32;
            last_session_update = now;

            let count = session.update(diff_ms);
            session.process_pending().await;
            (count, session.is_disconnecting())
        });
        let (count, disconnecting) = tokio::select! {
            _ = cancellation.cancelled_like_cpp() => {
                return WorldSessionRunOutcomeLikeCpp::ForceCancelled;
            }
            result = update => result,
        };

        if disconnecting {
            info!("Session for account {account_id} disconnecting");
            return WorldSessionRunOutcomeLikeCpp::Finished;
        }

        if count == 0 {
            tokio::select! {
                _ = cancellation.cancelled_like_cpp() => {
                    return WorldSessionRunOutcomeLikeCpp::ForceCancelled;
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(50)) => {}
            }
        }
    }
}

/// Create and run a WorldSession for an authenticated connection.
///
/// This is called by the accept loop after auth completes.
/// Runs the session update loop until the packet channel is closed.
pub(super) async fn create_session(
    account: AccountInfo,
    pkt_rx: flume::Receiver<wow_packet::WorldPacket>,
    send_tx: flume::Sender<Vec<u8>>,
    send_write_fence_like_cpp: wow_network::SocketWriteFenceLikeCpp,
    socket_timeouts: SocketTimeoutsLikeCpp,
    resources: Arc<SessionResources>,
    session_mgr: Arc<SessionManager>,
    shared_map: SharedMapManager,
    canonical_map_manager: SharedCanonicalMapManager,
    canonical_spawn_metadata: SharedCanonicalSpawnMetadataLikeCpp,
    loaded_grid_creature_respawn_caches: LoadedGridCreatureRespawnCachesLikeCpp,
    object_accessor: wow_world::SharedObjectAccessor,
    instance_port: u16,
    max_expansion: u8,
    mmap_runtime_config: MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: Option<Arc<WorldMMapPathfinderWorkerLikeCpp>>,
    active_session_registry: Arc<ActiveWorldSessionRegistryLikeCpp>,
    legacy_creature_aggro_config: wow_world::session::LegacyCreatureAggroConfigLikeCpp,
    world_runtime_state: Arc<WorldRuntimeStateLikeCpp>,
    battle_pet_account_registry: Arc<BattlePetAccountRegistryLikeCpp>,
) {
    info!(
        "Creating session for account {} (bnet_id={})",
        account.id, account.battlenet_account_id
    );

    // Use the DERIVED 40-byte session key from realm auth handshake.
    // C# writes this to the DB (UPD_ACCOUNT_INFO_CONTINUED_SESSION) and the
    // instance socket reads it back. We skip the DB roundtrip by passing it directly.
    // NOTE: This is NOT the raw BNet key (64 bytes) from the DB. It's the
    // HMAC-SHA256 derived key used for AuthContinuedSession validation.
    let session_key_raw = account.derived_session_key.clone();

    // C# caps only ActiveExpansionLevel to the server's max expansion,
    // but sends AccountExpansionLevel as the raw DB value (e.g. 9=Dragonflight).
    // The client uses AccountExpansionLevel to unlock classes in the char list.
    let active_expansion = account.expansion.min(max_expansion);
    let account_expansion = account.expansion; // raw from DB, NOT capped

    let mut session = WorldSession::new(
        account.id,
        String::new(), // account_name
        account.security,
        active_expansion,
        account_expansion, // AccountExpansionLevel: raw from DB, like C#
        54261,             // build
        session_key_raw,
        account.locale.clone(),
        pkt_rx,
        send_tx,
    );
    session.set_send_write_fence_like_cpp(send_write_fence_like_cpp);
    let Some((active_session_id, session_cancellation)) =
        active_session_registry.try_register(account.id, session.session_command_tx())
    else {
        info!(
            account_id = account.id,
            "Rejecting authenticated world session because shutdown registration gate is closed"
        );
        return;
    };
    let active_session_registration = ActiveWorldSessionRegistrationGuardLikeCpp {
        registry: Arc::clone(&active_session_registry),
        id: active_session_id,
    };
    let account_id = account.id;
    // Configure session with resources
    if let Some(ref db) = resources.char_db {
        session.set_char_db(Arc::clone(db));
    }
    if let Some(ref db) = resources.login_db {
        session.set_login_db(Arc::clone(db));
    }
    if let Some(ref port) = resources.player_lifecycle_port {
        session.set_player_lifecycle_port_like_cpp(Arc::clone(port));
    }
    if let Some(ref port) = resources.session_account_state_port {
        session.set_session_account_state_port_like_cpp(Arc::clone(port));
    }
    if let Some(ref port) = resources.packet_spoof_ban_persistence_port {
        session.set_packet_spoof_ban_persistence_port_like_cpp(Arc::clone(port));
    }
    if let Some(ref port) = resources.void_storage_persistence_port {
        session.set_void_storage_persistence_port_like_cpp(Arc::clone(port));
    }
    if let Some(ref port) = resources.social_persistence_port {
        session.set_social_persistence_port_like_cpp(Arc::clone(port));
    }
    if let Some(ref port) = resources.map_corpse_persistence_port {
        session.set_map_corpse_persistence_port_like_cpp(Arc::clone(port));
    }
    if let Some(ref port) = resources.quest_poi_persistence_port {
        session.set_quest_poi_persistence_port_like_cpp(Arc::clone(port));
    }
    if let Some(ref port) = resources.stored_item_money_persistence_port {
        session.set_stored_item_money_persistence_port_like_cpp(Arc::clone(port));
    }
    if let Some(ref port) = resources.group_loot_money_persistence_port {
        session.set_group_loot_money_persistence_port_like_cpp(Arc::clone(port));
    }
    if let Some(ref port) = resources.support_bug_report_persistence_port {
        session.set_support_bug_report_persistence_port_like_cpp(Arc::clone(port));
    }
    if let Some(ref port) = resources.next_mail_time_persistence_port {
        session.set_next_mail_time_persistence_port_like_cpp(Arc::clone(port));
    }
    session.set_remote_address_like_cpp(account.client_address.map(|addr| addr.to_string()));
    session.set_battlenet_account_id(account.battlenet_account_id);
    session.set_recruiter_id_like_cpp(account.recruiter);
    session.set_is_a_recruiter_like_cpp(account.is_a_recruiter);
    session.set_mute_time_like_cpp(account.mute_time);
    if let Some(ref generator) = resources.guid_generator {
        session.set_guid_generator(Arc::clone(generator));
    }
    if let Some(ref generator) = resources.item_guid_generator {
        session.set_item_guid_generator_like_cpp(Arc::clone(generator));
    }
    if let Some(ref generator) = resources.equipment_set_guid_generator {
        session.set_equipment_set_guid_generator_like_cpp(Arc::clone(generator));
    }
    if let Some(ref generator) = resources.void_storage_item_id_generator {
        session.set_void_storage_item_id_generator_like_cpp(Arc::clone(generator));
    }
    if let Some(ref mgr) = resources.instance_lock_mgr {
        session.set_instance_lock_mgr(Arc::clone(mgr));
    }
    if let Some(ref db) = resources.world_db {
        session.set_world_db(Arc::clone(db));
    }
    if let Some(ref store) = resources.trainer_store {
        session.set_trainer_store_like_cpp(Arc::clone(store));
    }
    if let Some(ref store) = resources.bank_bag_slot_prices_store {
        session.set_bank_bag_slot_prices_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.currency_types_store {
        session.set_currency_types_store(Arc::clone(store));
    }
    if let Some(ref stores) = resources.import_price_stores {
        session.set_import_price_stores(Arc::clone(stores));
    }
    if let Some(ref store) = resources.emotes_store {
        session.set_emotes_store_like_cpp(Arc::clone(store));
    }
    if let Some(ref store) = resources.emotes_text_store {
        session.set_emotes_text_store_like_cpp(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_class_store {
        session.set_item_class_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_currency_cost_store {
        session.set_item_currency_cost_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_extended_cost_store {
        session.set_item_extended_cost_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_store {
        session.set_item_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_child_equipment_store {
        session.set_item_child_equipment_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_appearance_store {
        session.set_item_appearance_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_modified_appearance_store {
        session.set_item_modified_appearance_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_search_name_store {
        session.set_item_search_name_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.trinity_string_store {
        session.set_trinity_string_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.heirloom_store {
        session.set_heirloom_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.toy_store {
        session.set_toy_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.battle_pet_breed_quality_store {
        session.set_battle_pet_breed_quality_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.battle_pet_breed_state_store {
        session.set_battle_pet_breed_state_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.battle_pet_species_store {
        session.set_battle_pet_species_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.battle_pet_selection_store {
        session.set_battle_pet_selection_store_like_cpp(Arc::clone(store));
    }
    // Issue #161: the recoverable purchase saga builds its production
    // Character DB store from the session's own character database handle.
    session.install_battle_pet_purchase_store_from_char_db_like_cpp();
    if let Some(ref store) = resources.battle_pet_species_state_store {
        session.set_battle_pet_species_state_store(Arc::clone(store));
    }
    if let Some(ref table) = resources.battle_pet_xp_game_table {
        session.set_battle_pet_xp_game_table(Arc::clone(table));
    }
    if let Some(ref table) = resources.combat_ratings_game_table {
        session.set_combat_ratings_game_table(Arc::clone(table));
    }
    if let Some(ref table) = resources.shield_block_regular_game_table {
        session.set_shield_block_regular_game_table(Arc::clone(table));
    }
    if let Some(ref store) = resources.transmog_set_item_store {
        session.set_transmog_set_item_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_price_base_store {
        session.set_item_price_base_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_limit_category_store {
        session.set_item_limit_category_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_limit_category_condition_store {
        session.set_item_limit_category_condition_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.player_create_info_store {
        session.set_player_create_info_store_like_cpp(Arc::clone(store));
    }
    if let Some(ref store) = resources.player_create_cast_spell_store {
        session.set_player_create_cast_spell_store_like_cpp(Arc::clone(store));
    }
    if let Some(ref store) = resources.player_create_custom_spell_store {
        session.set_player_create_custom_spell_store_like_cpp(Arc::clone(store));
    }
    if let Some(ref store) = resources.player_stats {
        session.set_player_stats(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_bonus_db2_store {
        session.set_item_bonus_db2_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.pvp_item_store {
        session.set_pvp_item_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_set_store {
        session.set_item_set_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_set_spell_store {
        session.set_item_set_spell_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_stats_store {
        session.set_item_stats_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.durability_costs_store {
        session.set_durability_costs_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.durability_quality_store {
        session.set_durability_quality_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_effect_store {
        session.set_item_effect_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_random_suffix_store {
        session.set_item_random_suffix_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_random_properties_store {
        session.set_item_random_properties_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_spec_override_store {
        session.set_item_spec_override_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.rand_prop_points_store {
        session.set_rand_prop_points_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_random_enchantment_template_store {
        session.set_item_random_enchantment_template_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.item_disenchant_loot_store {
        session.set_item_disenchant_loot_store(Arc::clone(store));
    }
    if let Some(ref stores) = resources.loot_stores {
        session.set_loot_stores(Arc::clone(stores));
    }
    if let Some(ref store) = resources.condition_store {
        session.set_condition_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.player_condition_store {
        session.set_player_condition_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.adventure_map_poi_store {
        session.set_adventure_map_poi_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.content_tuning_store {
        session.set_content_tuning_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.curve_store {
        session.set_curve_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.curve_point_store {
        session.set_curve_point_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.scaling_stat_distribution_store {
        session.set_scaling_stat_distribution_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.scaling_stat_values_store {
        session.set_scaling_stat_values_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.disable_mgr {
        session.set_disable_mgr(Arc::clone(store));
    }
    if let Some(ref store) = resources.difficulty_store {
        session.set_difficulty_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.lock_store {
        session.set_lock_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_item_enchantment_store {
        session.set_spell_item_enchantment_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_item_enchantment_condition_store {
        session.set_spell_item_enchantment_condition_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.gem_properties_store {
        session.set_gem_properties_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_enchant_proc_store {
        session.set_spell_enchant_proc_store(Arc::clone(store));
    }
    if let Some(ref cache) = resources.hotfix_blob_cache {
        session.set_hotfix_blob_cache(Arc::clone(cache));
    }
    if let Some(ref store) = resources.tact_key_store {
        session.set_tact_key_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.skill_store {
        session.set_skill_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.trait_definition_store {
        session.set_trait_definition_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.trait_node_entry_store {
        session.set_trait_node_entry_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.skill_line_store {
        session.set_skill_line_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.skill_tiers_store {
        session.set_skill_tiers_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.talent_store {
        session.set_talent_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.talent_tab_store {
        session.set_talent_tab_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.num_talents_at_level_store {
        session.set_num_talents_at_level_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.glyph_properties_store {
        session.set_glyph_properties_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.chr_races_store {
        session.set_chr_races_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.chr_classes_store {
        session.set_chr_classes_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.power_type_store {
        session.set_power_type_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_store {
        session.set_spell_store(Arc::clone(store));
    }
    if let Some(ref catalog) = resources.spell_acquisition_catalog {
        session.set_spell_acquisition_catalog(Arc::clone(catalog));
    }
    if let (Some(casts), Some(crafts)) = (
        resources.spell_acquisition_safe_cast_spell_ids.as_ref(),
        resources.spell_acquisition_valid_craft_spell_ids.as_ref(),
    ) {
        session.set_spell_acquisition_static_authority_like_cpp(
            casts.iter().copied(),
            crafts.iter().copied(),
        );
    }
    if let (Some(exact), Some(all_ranks), Some(legacy), Some(rejected_linked_triggers)) = (
        resources.spell_script_exact_spell_ids.as_ref(),
        resources.spell_script_all_rank_root_spell_ids.as_ref(),
        resources.legacy_spell_script_spell_ids.as_ref(),
        resources.spell_linked_rejected_trigger_spell_ids.as_ref(),
    ) {
        session.set_spell_runtime_script_authority_like_cpp(
            Arc::clone(exact),
            Arc::clone(all_ranks),
            Arc::clone(legacy),
            Arc::clone(rejected_linked_triggers),
        );
    }
    if let Some(ref store) = resources.spell_levels_store {
        session.set_spell_levels_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_chain_store {
        session.set_spell_chain_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_category_store {
        session.set_spell_category_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.npc_spell_click_store {
        session.set_npc_spell_click_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_aura_options_store {
        session.set_spell_aura_options_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_aura_restrictions_store {
        session.set_spell_aura_restrictions_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_target_restrictions_store {
        session.set_spell_target_restrictions_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_equipped_items_store {
        session.set_spell_equipped_items_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_misc_store {
        session.set_spell_misc_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_group_store {
        session.set_spell_group_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_group_stack_rule_store {
        session.set_spell_group_stack_rule_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_linked_store {
        session.set_spell_linked_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_pet_aura_store {
        session.set_spell_pet_aura_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_area_store {
        session.set_spell_area_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_custom_attribute_store {
        session.set_spell_custom_attribute_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.serverside_spell_store {
        session.set_serverside_spell_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_learn_skill_store {
        session.set_spell_learn_skill_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_learn_spell_store {
        session.set_spell_learn_spell_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.pet_levelup_spell_store {
        session.set_pet_levelup_spell_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.pet_default_spell_store {
        session.set_pet_default_spell_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.pet_family_spell_store {
        session.set_pet_family_spell_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_proc_store {
        session.set_spell_proc_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_required_store {
        session.set_spell_required_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_threat_store {
        session.set_spell_threat_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_duration_store {
        session.set_spell_duration_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_radius_store {
        session.set_spell_radius_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_range_store {
        session.set_spell_range_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_target_position_store {
        session.set_spell_target_position_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_totem_model_store {
        session.set_spell_totem_model_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.movie_store {
        session.set_movie_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.script_name_interner {
        session.set_script_name_interner(Arc::clone(store));
    }
    if let Some(ref store) = resources.gameobject_template_lifecycle_store {
        session.set_gameobject_template_lifecycle_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.area_table_store {
        session.set_area_table_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.fishing_base_skill_store {
        session.set_fishing_base_skill_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.area_trigger_db2_store {
        session.set_area_trigger_db2_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.area_trigger_store {
        session.set_area_trigger_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.area_trigger_script_store {
        session.set_area_trigger_script_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.tavern_area_trigger_store {
        session.set_tavern_area_trigger_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.graveyard_store {
        session.set_graveyard_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.chr_specialization_store {
        session.set_chr_specialization_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.dungeon_encounter_store {
        session.set_dungeon_encounter_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.map_store {
        session.set_map_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.world_safe_loc_store {
        session.set_world_safe_loc_store_like_cpp(Arc::clone(store));
    }
    if let Some(ref store) = resources.map_difficulty_store {
        session.set_map_difficulty_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.map_difficulty_x_condition_store {
        session.set_map_difficulty_x_condition_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.access_requirement_store {
        session.set_access_requirement_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.lfg_dungeons_store {
        session.set_lfg_dungeons_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.lfg_dungeon_store_like_cpp {
        session.set_lfg_dungeon_store_like_cpp(Arc::clone(store));
    }
    if let Some(ref store) = resources.battlemaster_list_store {
        session.set_battlemaster_list_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_template_lifecycle_store {
        session.set_creature_template_lifecycle_store_like_cpp(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_template_mount_store {
        session.set_creature_template_mount_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_equipment_store {
        session.set_creature_equipment_store_like_cpp(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_display_info_store {
        session.set_creature_display_info_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_display_info_extra_store {
        session.set_creature_display_info_extra_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.gameobject_display_info_store {
        session.set_gameobject_display_info_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_model_info_store {
        session.set_creature_model_info_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_addon_store {
        session.set_creature_addon_store_like_cpp(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_difficulty_store {
        session.set_creature_difficulty_store_like_cpp(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_base_stats_store {
        session.set_creature_base_stats_store_like_cpp(Arc::clone(store));
    }
    session.set_creature_health_rates_like_cpp(resources.creature_health_rates);
    if let Some(ref store) = resources.creature_model_data_store {
        session.set_creature_model_data_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.mount_store {
        session.set_mount_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.mount_definition_store {
        session.set_mount_definition_store_like_cpp(Arc::clone(store));
    }
    if let Some(ref store) = resources.mount_capability_store {
        session.set_mount_capability_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.mount_type_x_capability_store {
        session.set_mount_type_x_capability_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.mount_x_display_store {
        session.set_mount_x_display_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.spell_shapeshift_form_store {
        session.set_spell_shapeshift_form_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.vehicle_store {
        session.set_vehicle_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.vehicle_seat_store {
        session.set_vehicle_seat_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.vehicle_template_store {
        session.set_vehicle_template_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.vehicle_accessory_store {
        session.set_vehicle_accessory_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.terrain_swap_store {
        session.set_terrain_swap_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.phase_store {
        session.set_phase_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.phase_group_store {
        session.set_phase_group_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.quest_store {
        session.set_quest_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.quest_xp_store {
        session.set_quest_xp_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.quest_money_reward_store {
        session.set_quest_money_reward_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.quest_v2_store {
        session.set_quest_v2_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.quest_info_store {
        session.set_quest_info_store(Arc::clone(store));
    }
    session.set_quest_low_level_hide_diff_like_cpp(resources.quest_low_level_hide_diff);
    session.set_quest_high_level_hide_diff_like_cpp(resources.quest_high_level_hide_diff);
    if let Some(ref store) = resources.quest_package_item_store {
        session.set_quest_package_item_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.quest_faction_reward_store {
        session.set_quest_faction_reward_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.progression_faction_store {
        session.set_faction_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.faction_template_store {
        session.set_faction_template_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.friendship_rep_reaction_store {
        session.set_friendship_rep_reaction_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.paragon_reputation_store {
        session.set_paragon_reputation_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.reputation_reward_rate_store {
        session.set_reputation_reward_rate_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.creature_onkill_reputation_store {
        session.set_creature_onkill_reputation_store(Arc::clone(store));
    }
    if let Some(ref store) = resources.reputation_spillover_template_store {
        session.set_reputation_spillover_template_store(Arc::clone(store));
    }
    if let Some(ref table) = resources.player_xp_table {
        session.set_player_xp_table(Arc::clone(table));
    }
    if let Some(ref store) = resources.exploration_base_xp_store {
        session.set_exploration_base_xp_store_like_cpp(Arc::clone(store));
    }
    session.set_exploration_xp_rate_like_cpp(resources.exploration_xp_rate);
    session.set_rested_xp_config_like_cpp(
        resources.max_player_level_config,
        resources.rest_offline_wilderness_rate,
        resources.rest_offline_tavern_or_city_rate,
        resources.rest_ingame_rate,
    );
    session.set_max_primary_trade_skills_like_cpp(resources.max_primary_trade_skills);
    session.set_pvp_realm_like_cpp(resources.is_pvp_realm);
    session.set_ffa_pvp_realm_like_cpp(resources.is_ffa_pvp_realm);
    session.set_recruit_a_friend_xp_config_like_cpp(
        resources.max_recruit_a_friend_bonus_player_level,
        resources.max_recruit_a_friend_bonus_player_level_difference,
    );
    session.set_min_quest_scaled_xp_ratio_like_cpp(resources.min_quest_scaled_xp_ratio);
    session.set_min_discovered_scaled_xp_ratio_like_cpp(resources.min_discovered_scaled_xp_ratio);
    if let Some(ref modules) = resources.module_registry {
        session.set_module_registry_like_cpp(Arc::clone(modules));
    }
    if let Some(ref registry) = resources.player_registry {
        session.set_player_registry(Arc::clone(registry));
    }
    if let Some(sender) = resources.game_event_quest_complete_tx.as_ref() {
        session.set_game_event_quest_complete_sender_like_cpp(sender.clone());
    }
    session.set_loot_drop_rates_like_cpp(resources.loot_drop_rates);
    session.set_reputation_rates_like_cpp(resources.reputation_rates);
    session.set_repair_cost_rate_like_cpp(resources.repair_cost_rate);
    session.set_reset_schedule_like_cpp(resources.reset_schedule);
    session.set_no_reset_talent_cost_like_cpp(resources.no_reset_talent_cost);
    session.set_offhand_check_at_spell_unlearn_like_cpp(resources.offhand_check_at_spell_unlearn);
    session.set_vmap_indoor_check_like_cpp(resources.vmap_indoor_check);
    session.set_start_all_explored_like_cpp(resources.start_all_explored);
    session.set_start_all_reputation_like_cpp(resources.start_all_reputation);
    session.set_start_all_spells_like_cpp(resources.start_all_spells);
    session.set_represented_support_enabled_like_cpp(resources.support_enabled);
    session.set_represented_support_tickets_enabled_like_cpp(resources.support_tickets_enabled);
    session.set_represented_support_bugs_enabled_like_cpp(resources.support_bugs_enabled);
    session
        .set_represented_support_complaints_enabled_like_cpp(resources.support_complaints_enabled);
    session.set_represented_support_suggestions_enabled_like_cpp(
        resources.support_suggestions_enabled,
    );
    session.set_enable_ae_loot_like_cpp(resources.enable_ae_loot);
    session.set_addon_channel_like_cpp(resources.addon_channel);
    session.set_server_expansion_like_cpp(resources.server_expansion);
    session.set_characters_per_realm_like_cpp(resources.characters_per_realm);
    session.set_declined_names_used_like_cpp(resources.declined_names_used);
    session.set_feature_system_bpay_store_enabled_like_cpp(
        resources.feature_system_bpay_store_enabled,
    );
    session.set_feature_system_character_undelete_enabled_like_cpp(
        resources.feature_system_character_undelete_enabled,
    );
    session.set_instance_ignore_raid_like_cpp(resources.instance_ignore_raid);
    session.set_instance_ignore_level_like_cpp(resources.instance_ignore_level);
    session.set_max_instances_per_hour_like_cpp(resources.max_instances_per_hour);
    session.set_chat_fake_message_preventing_like_cpp(resources.chat_fake_message_preventing);
    session.set_party_raid_warnings_like_cpp(resources.party_raid_warnings);
    session.set_allow_gm_group_like_cpp(resources.allow_gm_group);
    session
        .set_allow_two_side_interaction_group_like_cpp(resources.allow_two_side_interaction_group);
    session.set_party_level_req_like_cpp(resources.party_level_req);
    session.set_chat_strict_link_checking_kick_like_cpp(resources.chat_strict_link_checking_kick);
    session.set_chat_level_requirements_like_cpp(resources.chat_level_requirements);
    session.set_chat_listen_ranges_like_cpp(resources.chat_listen_ranges);
    session.set_chat_flood_config_like_cpp(resources.chat_flood_config);
    session.set_socket_timeouts_like_cpp(socket_timeouts);
    session.set_packet_spoof_config_like_cpp(resources.packet_spoof_config);
    session.set_player_save_interval_ms_like_cpp(resources.player_save_interval_ms);
    session.set_legacy_creature_aggro_config_like_cpp(legacy_creature_aggro_config.clone());
    session.set_mmap_runtime_config_like_cpp(mmap_runtime_config.clone());
    if let Some(ref pathfinder) = mmap_pathfinder {
        session.set_mmap_pathfinder_like_cpp(Arc::clone(pathfinder));
    }
    let waypoint_spawn_metadata = Arc::clone(&canonical_spawn_metadata);
    session.set_waypoint_path_resolver_like_cpp(Arc::new(move |path_id| {
        waypoint_spawn_metadata
            .lock()
            .ok()
            .and_then(|metadata| metadata.waypoint_paths_like_cpp().get(path_id).cloned())
    }));
    let grid_canonical_map_manager = Arc::clone(&canonical_map_manager);
    let grid_legacy_manager = Arc::clone(&shared_map);
    let grid_spawn_metadata = Arc::clone(&canonical_spawn_metadata);
    let grid_loaded_caches = loaded_grid_creature_respawn_caches.clone();
    let grid_map_store = resources.map_store.as_ref().map(Arc::clone);
    let grid_area_trigger_template_store = resources
        .area_trigger_template_store
        .as_ref()
        .map(Arc::clone)
        .expect("world-server SessionResources must provide AreaTriggerTemplateStore");
    session.set_player_grid_load_resolver_like_cpp(Arc::new(
        move |map_id, instance_id, position| {
            ensure_login_player_grid_loaded_like_cpp(
                &grid_canonical_map_manager,
                &grid_legacy_manager,
                &grid_spawn_metadata,
                &grid_loaded_caches,
                grid_area_trigger_template_store.as_ref(),
                grid_map_store.as_deref(),
                map_id,
                instance_id,
                position,
            )
        },
    ));
    session.set_object_accessor(Arc::clone(&object_accessor));
    if let (Some(greg), Some(pinv)) = (&resources.group_registry, &resources.pending_invites) {
        session.set_group_registry(Arc::clone(greg), Arc::clone(pinv));
    }
    session.set_realm_handle_like_cpp(
        resources.realm_region,
        resources.realm_battlegroup,
        resources.realm_id,
    );
    session.set_realm_names_like_cpp(resources.realm_names.iter().cloned());
    match battle_pet_account_registry
        .attach_like_cpp(account.battlenet_account_id)
        .await
    {
        Ok(attachment) => session.set_battle_pet_account_attachment_like_cpp(attachment),
        Err(error) => {
            tracing::error!(
                account_id = account.id,
                battlenet_account_id = account.battlenet_account_id,
                %error,
                "Rejecting world session because its canonical battle-pet journal could not load"
            );
            return;
        }
    }
    session.set_map_manager(Arc::clone(&shared_map));
    session.set_canonical_map_manager(Arc::clone(&canonical_map_manager));

    // Select the correct realm IP for ConnectTo based on client address.
    // C++ delegates to Trinity::Net::SelectAddressForClient after scanning
    // local interfaces. Rust scans IPv4 interfaces on demand and falls back to
    // the old /24 approximation only if no usable local network is found.
    let connect_ip = get_address_for_client(
        account.client_address,
        resources.realm_external_address,
        resources.realm_local_address,
    );

    // Configure C++ `SMSG_CONNECT_TO` flow — real clients enter the world on
    // the instance socket after `AuthContinuedSession`.
    session.set_session_mgr(Arc::clone(&session_mgr));
    session.set_instance_endpoint(connect_ip, instance_port);

    if run_world_session_until_disconnect_like_cpp(
        &mut session,
        account_id,
        active_session_registry.as_ref(),
        session_cancellation.as_ref(),
    )
    .await
        == WorldSessionRunOutcomeLikeCpp::ForceCancelled
    {
        tracing::error!(
            account_id,
            "Force-cancelled world session after shutdown grace period"
        );
    }
    // Cancellation only discards initialization/gameplay work. During server
    // shutdown, disconnect persistence and cleanup each get an independent
    // bounded attempt. Normal disconnects preserve the prior unbounded save
    // contract and are never truncated by shutdown policy.
    if active_session_registry.is_shutting_down_like_cpp() {
        if !run_world_session_shutdown_finalize_step_like_cpp(
            world_runtime_state.as_ref(),
            WORLD_SESSION_FINALIZE_STEP_TIMEOUT_LIKE_CPP,
            session.save_disconnect_player_to_db_like_cpp(),
        )
        .await
        {
            tracing::error!(
                account_id,
                timeout_ms = WORLD_SESSION_FINALIZE_STEP_TIMEOUT_LIKE_CPP.as_millis(),
                "Timed out saving disconnected world session during shutdown finalization"
            );
        }
        if !run_world_session_shutdown_finalize_step_like_cpp(
            world_runtime_state.as_ref(),
            WORLD_SESSION_FINALIZE_STEP_TIMEOUT_LIKE_CPP,
            session.cleanup_shared_runtime_state_on_disconnect_like_cpp(),
        )
        .await
        {
            tracing::error!(
                account_id,
                timeout_ms = WORLD_SESSION_FINALIZE_STEP_TIMEOUT_LIKE_CPP.as_millis(),
                "Timed out cleaning shared runtime state during world-session finalization"
            );
        }
    } else {
        session.save_disconnect_player_to_db_like_cpp().await;
        session
            .cleanup_shared_runtime_state_on_disconnect_like_cpp()
            .await;
    }
    drop(active_session_registration);
}

/// Select the correct realm IP for a client, matching C++ `Realm::GetAddressForClient`.
///
/// This uses the shared SelectAddressForClient-like priority rules. The local
/// network source is scanned from host interfaces, with a /24 fallback when no
/// usable IPv4 interface is reported.
pub(super) fn get_address_for_client(
    client_ip: Option<std::net::IpAddr>,
    external: [u8; 4],
    local: [u8; 4],
) -> [u8; 4] {
    let scanned_networks = scan_local_ipv4_networks_like_cpp();
    get_address_for_client_with_local_networks(client_ip, external, local, &scanned_networks)
}

pub(super) fn get_address_for_client_with_local_networks(
    client_ip: Option<std::net::IpAddr>,
    external: [u8; 4],
    local: [u8; 4],
    scanned_networks: &[Ipv4NetworkLikeCpp],
) -> [u8; 4] {
    let external_v4 = std::net::Ipv4Addr::from(external);
    let local_v4 = std::net::Ipv4Addr::from(local);
    let client_v4 = match client_ip {
        Some(std::net::IpAddr::V4(v4)) => Some(v4),
        _ => None,
    };
    let fallback_networks = [Ipv4NetworkLikeCpp::new(local_v4, 24)];
    let local_networks = if scanned_networks.is_empty() {
        fallback_networks.as_slice()
    } else {
        scanned_networks
    };
    wow_core::realm_ipv4_address_for_client_like_cpp(
        client_v4,
        external_v4,
        local_v4,
        local_networks,
    )
    .octets()
}

/// Format an IPv4 address for display.
pub(super) fn format_ipv4(ip: [u8; 4]) -> String {
    format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
}

/// Decode a hex string into raw bytes.
pub(super) fn hex_to_bytes(hex: &str) -> Vec<u8> {
    let hex = hex.trim();
    if hex.is_empty() {
        return Vec::new();
    }
    (0..hex.len())
        .step_by(2)
        .filter_map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Map DBC.Locale config value to the folder name.
///
/// The config can be a numeric ID (C# style) or already a locale name.
/// WoW locale IDs: 0=enUS, 1=koKR, 2=frFR, 3=deDE, 4=zhCN, 5=zhTW,
/// 6=esES, 7=esMX, 8=ruRU, 9=jaJP, 10=ptBR, 11=itIT.
pub(super) fn locale_id_to_name(raw: &str) -> String {
    match raw.trim() {
        "0" => "enUS".into(),
        "1" => "koKR".into(),
        "2" => "frFR".into(),
        "3" => "deDE".into(),
        "4" => "zhCN".into(),
        "5" => "zhTW".into(),
        "6" => "esES".into(),
        "7" => "esMX".into(),
        "8" => "ruRU".into(),
        "9" => "jaJP".into(),
        "10" => "ptBR".into(),
        "11" => "itIT".into(),
        other => other.into(), // already a name like "esES"
    }
}
