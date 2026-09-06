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
    handler_catalogs: &wow_world::session::SessionHandlerCatalogsLikeCpp,
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
    session.send_session_init_packets_with_policy_like_cpp(
        handler_catalogs.support_feature_policy.as_ref(),
        handler_catalogs.hotfixes.as_ref(),
    );

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

            let count = session.update_with_catalogs_like_cpp(diff_ms, handler_catalogs);
            session
                .process_pending_with_catalogs_like_cpp(handler_catalogs)
                .await;
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
    resources.core.install_into_session_like_cpp(&mut session);
    session.set_remote_address_like_cpp(account.client_address.map(|addr| addr.to_string()));
    session.set_battlenet_account_id(account.battlenet_account_id);
    session.set_recruiter_id_like_cpp(account.recruiter);
    session.set_is_a_recruiter_like_cpp(account.is_a_recruiter);
    session.set_mute_time_like_cpp(account.mute_time);
    resources
        .inventory
        .install_into_session_like_cpp(&mut session);
    resources.player.install_into_session_like_cpp(&mut session);
    resources.spells.install_into_session_like_cpp(&mut session);
    resources.world.install_into_session_like_cpp(&mut session);
    resources
        .progression
        .install_into_session_like_cpp(&mut session);
    resources
        .runtime
        .install_into_session_like_cpp(&mut session, socket_timeouts);
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
    resources.realm.install_into_session_like_cpp(&mut session);
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
        resources.realm.realm_external_address,
        resources.realm.realm_local_address,
    );

    // Configure C++ `SMSG_CONNECT_TO` flow — real clients enter the world on
    // the instance socket after `AuthContinuedSession`.
    session.set_session_mgr(Arc::clone(&session_mgr));
    session.set_instance_endpoint(connect_ip, instance_port);

    if run_world_session_until_disconnect_like_cpp(
        &mut session,
        resources.core.handler_catalogs.as_ref(),
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
    // Retired reads cannot start writes. Join writes already submitted by ready
    // callbacks before saving/discarding this Session. A timeout is fatal, not an
    // acknowledgement that a transaction rolled back or a worker stopped.
    let rename_drain = async {
        if !session.finish_character_rename_callbacks_like_cpp().await {
            world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        }
    };
    if active_session_registry.is_shutting_down_like_cpp() {
        if !run_world_session_shutdown_finalize_step_like_cpp(
            world_runtime_state.as_ref(),
            WORLD_SESSION_FINALIZE_STEP_TIMEOUT_LIKE_CPP,
            rename_drain,
        )
        .await
        {
            tracing::error!(
                account_id,
                "Timed out draining character rename commits; completion unproven"
            );
        }
    } else {
        rename_drain.await;
    }
    // Complete retained native transfer work before save/cleanup; never treat
    // an unavailable incarnation or unfinished operation as a clean disconnect.
    if !session.finish_worldport_native_before_disconnect_like_cpp() {
        world_runtime_state.stop_now_like_cpp(ERROR_EXIT_CODE_LIKE_CPP);
        tracing::error!(
            account_id,
            "Worldport native completion unavailable; refusing normal save and cleanup"
        );
        return;
    }
    // During server
    // shutdown, disconnect persistence and cleanup each get an independent
    // bounded attempt. Normal disconnects preserve the prior unbounded save
    // contract and are never truncated by shutdown policy.
    if active_session_registry.is_shutting_down_like_cpp() {
        if !run_world_session_shutdown_finalize_step_like_cpp(
            world_runtime_state.as_ref(),
            WORLD_SESSION_FINALIZE_STEP_TIMEOUT_LIKE_CPP,
            session.save_disconnect_player_to_db_with_generator_like_cpp(
                resources.core.handler_catalogs.id_generators.item.as_ref(),
            ),
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
            session.cleanup_shared_runtime_state_on_disconnect_with_generator_like_cpp(
                resources.core.handler_catalogs.id_generators.item.as_ref(),
            ),
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
        session
            .save_disconnect_player_to_db_with_generator_like_cpp(
                resources.core.handler_catalogs.id_generators.item.as_ref(),
            )
            .await;
        session
            .cleanup_shared_runtime_state_on_disconnect_with_generator_like_cpp(
                resources.core.handler_catalogs.id_generators.item.as_ref(),
            )
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
