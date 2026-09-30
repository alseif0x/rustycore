//! CLI early exits and process configuration bootstrap.

use std::sync::Arc;
use tracing::info;
use crate::{WorldServerCliLikeCpp, WorldRuntimeStateLikeCpp, worldserver_cli_help_like_cpp, worldserver_full_version_like_cpp, load_world_config, log_startup_banner_like_cpp, create_pid_file_from_config_like_cpp, load_ip_location_from_config_like_cpp};

pub(super) struct ProcessStartup {
    pub(super) ip_location_store: Arc<wow_core::IpLocationStore>,
    pub(super) world_configs: wow_config::WorldConfigSet,
    pub(super) config_report: wow_config::LoadReport,
    pub(super) world_runtime_state: Arc<crate::WorldRuntimeStateLikeCpp>,
    pub(super) cli: crate::WorldServerCliLikeCpp,
}

pub(super) fn initialize(args: Vec<String>) -> anyhow::Result<Option<ProcessStartup>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    wow_logging::install_panic_hook_like_cpp();

    let cli = WorldServerCliLikeCpp::parse_from(args);
    if cli.show_help {
        print!("{}", worldserver_cli_help_like_cpp());
        return Ok(None);
    }
    if cli.show_version {
        println!("{}", worldserver_full_version_like_cpp());
        return Ok(None);
    }

    let world_runtime_state = Arc::new(WorldRuntimeStateLikeCpp::new());

    info!("RustyCore World Server starting...");

    let config_report = load_world_config(&cli)?;
    log_startup_banner_like_cpp(&config_report);
    let world_configs = wow_config::load_world_config_values();
    create_pid_file_from_config_like_cpp()?;
    let ip_location_store = Arc::new(load_ip_location_from_config_like_cpp());
    Ok(Some(ProcessStartup {
        cli,
        world_runtime_state,
        config_report,
        world_configs,
        ip_location_store,
    }))
}
