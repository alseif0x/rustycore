//! Ordered listener startup composition.

use crate::{create_session, stop_world_network_like_cpp};
use anyhow::Context;
use anyhow::bail;
use std::sync::Arc;

pub(super) struct NetworkListeners {
    pub(super) instance_network_abort_handle: tokio::task::AbortHandle,
    pub(super) realm_network_abort_handle: tokio::task::AbortHandle,
    pub(super) instance_handle: tokio::task::JoinHandle<anyhow::Result<()>>,
    pub(super) realm_handle: tokio::task::JoinHandle<anyhow::Result<()>>,
}

pub(super) async fn load(
    realm_addr: std::net::SocketAddr,
    instance_addr: std::net::SocketAddr,
    account_lookup: &Arc<dyn wow_network::world_socket::AccountLookup>,
    world_listener_policy: wow_network::WorldListenerPolicyLikeCpp,
    session_resources: &Arc<crate::session_resources::SessionResources>,
    session_mgr: &Arc<wow_network::session_mgr::SessionManager>,
    instances: &super::world_instance_startup::WorldInstanceManagers,
    world_spawns: &super::world_object_startup::WorldSpawnStartup,
    active_session_registry: &Arc<crate::ActiveWorldSessionRegistryLikeCpp>,
    world_runtime_state: &Arc<crate::WorldRuntimeStateLikeCpp>,
    battle_pet_account_registry: &Arc<wow_world::BattlePetAccountRegistryLikeCpp>,
    instance_port: u16,
    max_expansion: u8,
    mmap_runtime_config: &wow_world::MMapRuntimeConfigLikeCpp,
    mmap_pathfinder: &Option<Arc<wow_world::WorldMMapPathfinderWorkerLikeCpp>>,
    legacy_creature_aggro_config: &wow_world::session::LegacyCreatureAggroConfigLikeCpp,
) -> anyhow::Result<NetworkListeners> {
    let (realm_listener_ready_tx, realm_listener_ready_rx) = tokio::sync::oneshot::channel();
    let (instance_listener_ready_tx, instance_listener_ready_rx) = tokio::sync::oneshot::channel();

    // Spawn realm listener (existing world listener)
    let mut realm_handle = tokio::spawn({
        let lookup = Arc::clone(account_lookup);
        let listener_policy = world_listener_policy;
        let resources = Arc::clone(session_resources);
        let mgr = Arc::clone(session_mgr);
        let smap = Arc::clone(&instances.shared_map);
        let canonical_map = Arc::clone(&instances.canonical_map_manager);
        let spawn_metadata = Arc::clone(&world_spawns.canonical_spawn_metadata);
        let active_sessions = Arc::clone(active_session_registry);
        let runtime_state = Arc::clone(world_runtime_state);
        let battle_pet_accounts = Arc::clone(battle_pet_account_registry);
        let port = instance_port;
        let mmap_config = mmap_runtime_config.clone();
        let mmap_pathfinder = mmap_pathfinder.clone();
        let session_aggro_config = legacy_creature_aggro_config.clone();
        async move {
            wow_network::start_world_listener(
                realm_addr,
                lookup,
                listener_policy,
                move |account, pkt_rx, send_tx, send_write_fence_like_cpp, socket_timeouts| {
                    let resources = Arc::clone(&resources);
                    let mgr = Arc::clone(&mgr);
                    let smap = Arc::clone(&smap);
                    let canonical_map = Arc::clone(&canonical_map);
                    let spawn_metadata = Arc::clone(&spawn_metadata);
                    let active_sessions = Arc::clone(&active_sessions);
                    let runtime_state = Arc::clone(&runtime_state);
                    let mmap_pathfinder = mmap_pathfinder.clone();
                    let session_aggro_config = session_aggro_config.clone();
                    let battle_pet_accounts = Arc::clone(&battle_pet_accounts);
                    create_session(
                        account,
                        pkt_rx,
                        send_tx,
                        send_write_fence_like_cpp,
                        socket_timeouts,
                        resources,
                        mgr,
                        smap,
                        canonical_map,
                        spawn_metadata,
                        port,
                        max_expansion,
                        mmap_config.clone(),
                        mmap_pathfinder,
                        active_sessions,
                        session_aggro_config,
                        runtime_state,
                        battle_pet_accounts,
                    )
                },
                realm_listener_ready_tx,
            )
            .await
            .context("Realm listener error")
        }
    });

    // Spawn instance listener
    let mut instance_handle = tokio::spawn({
        let mgr = Arc::clone(session_mgr);
        async move {
            wow_network::start_instance_listener(instance_addr, mgr, instance_listener_ready_tx)
                .await
                .context("Instance listener error")
        }
    });
    let realm_network_abort_handle = realm_handle.abort_handle();
    let instance_network_abort_handle = instance_handle.abort_handle();

    let (realm_listener_ready, instance_listener_ready) =
        tokio::join!(realm_listener_ready_rx, instance_listener_ready_rx);
    let listener_start_error = match (realm_listener_ready, instance_listener_ready) {
        (Ok(Ok(())), Ok(Ok(()))) => None,
        (Ok(Err(error)), _) => Some(format!("realm listener bind failed: {error}")),
        (_, Ok(Err(error))) => Some(format!("instance listener bind failed: {error}")),
        (Err(error), _) => Some(format!("realm listener readiness task failed: {error}")),
        (_, Err(error)) => Some(format!("instance listener readiness task failed: {error}")),
    };
    if let Some(error) = listener_start_error {
        stop_world_network_like_cpp([
            ("realm", &realm_network_abort_handle),
            ("instance", &instance_network_abort_handle),
        ]);
        bail!(error);
    }
    Ok(NetworkListeners {
        realm_handle,
        instance_handle,
        realm_network_abort_handle,
        instance_network_abort_handle,
    })
}

pub(super) async fn publish_realm_online(
    login_db: &wow_database::LoginDatabase,
    realm_id: u16,
    listeners: &NetworkListeners,
) -> anyhow::Result<()> {
    // Match C++ startup: bind both world listeners successfully before
    // clearing the realm's offline flag, but still do so before DB writers and
    // map/runtime producers begin.
    if let Err(error) = crate::set_realm_online(login_db, realm_id).await {
        stop_world_network_like_cpp([
            ("realm", &listeners.realm_network_abort_handle),
            ("instance", &listeners.instance_network_abort_handle),
        ]);
        return Err(error);
    }
    Ok(())
}
