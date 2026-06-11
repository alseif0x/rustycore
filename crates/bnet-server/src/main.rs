//! BNet Authentication Server — entry point.
//!
//! Handles Battle.net account login via REST (HTTPS) and BNet RPC (TLS).
//! This is a drop-in replacement for the C# BNetServer.

mod ip_location;
mod legacy_password;
mod realm;
mod rest;
mod rpc;
mod secret_mgr;
mod state;

use anyhow::{Context, Result};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use wow_config::{DatabaseInfo, LoadReport};
use wow_database::{LoginDatabase, LoginStatements, build_connection_string_with_ssl_like_cpp};

use crate::ip_location::IpLocationStore;
use crate::state::AppState;

const BNET_CONFIG_CANDIDATES: &[&str] = &[
    "bnetserver.conf",
    "bnetserver.conf.dist",
    "BNetServer.conf",
    "BNetServer.conf.dist",
];
const BNET_CONFIG_DIR: &str = "bnetserver.conf.d";

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = BnetCliLikeCpp::parse_from(std::env::args().skip(1));
    if cli.show_help {
        print!("{}", bnet_cli_help_like_cpp());
        return Ok(());
    }
    if cli.show_version {
        println!("{}", bnet_full_version_like_cpp());
        return Ok(());
    }

    let config_report = load_bnet_config(&cli)?;
    log_startup_banner_like_cpp(&config_report);
    log_thread_config_like_cpp();
    create_pid_file_from_config_like_cpp()?;

    // Database connection
    let login_info = wow_config::get_database_info_default(
        "Login",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "auth"),
    );
    log_database_target_like_cpp("login", &login_info);

    let conn_str = build_connection_string_with_ssl_like_cpp(
        &login_info.host,
        &login_info.port_or_socket,
        &login_info.username,
        &login_info.password,
        &login_info.database,
        login_info.ssl,
    );
    let login_db = LoginDatabase::open(&conn_str)
        .await
        .context("Failed to connect to login database")?;

    tracing::info!("Connected to login database");

    run_database_updates_like_cpp(&login_db, &login_info).await;
    if cli.update_databases_only {
        login_db.close().await;
        tracing::info!("Database update-only mode complete.");
        return Ok(());
    }

    let legacy_password_report =
        legacy_password::migrate_legacy_password_hashes_like_cpp(&login_db)
            .await
            .context("Failed to migrate legacy Battle.net password hashes")?;
    if !legacy_password_report.column_present {
        tracing::debug!("Legacy Battle.net sha_pass_hash column not present; skipping migration");
    }
    let secret_report =
        secret_mgr::initialize_secret_mgr_like_cpp(&login_db, secret_mgr::SecretOwner::BnetServer)
            .await
            .context("Failed to initialize SecretMgr for bnetserver")?;
    tracing::debug!(
        totp_master_secret_present = secret_report.totp_master_secret_present,
        transitioned = secret_report.transitioned,
        "SecretMgr initialized for bnetserver"
    );

    // Load TLS certificates — separate configs for REST (HTTPS) and RPC (binary)
    let cert_file = wow_config::get_string_default("CertificatesFile", "./bnetserver.cert.pem");
    let key_file = wow_config::get_string_default("PrivateKeyFile", "./bnetserver.key.pem");
    let key_password = wow_config::get_string_default("PrivateKeyPassword", "");
    let key_password = if key_password.is_empty() {
        None
    } else {
        Some(key_password.as_str())
    };
    let (rest_tls_acceptor, rpc_tls_acceptor) =
        load_tls_acceptors(&cert_file, &key_file, key_password)
            .context("Failed to load TLS certificates")?;
    tracing::info!("TLS certificates loaded");

    // Build shared state
    let bind_ip = wow_config::get_string_default("BindIP", "0.0.0.0");
    let rest_port: u16 = wow_config::get_value("LoginREST.Port").unwrap_or(8081);
    let rpc_port: u16 = wow_config::get_value("BattlenetPort").unwrap_or(1119);
    let external_address = wow_config::get_string_default("LoginREST.ExternalAddress", "127.0.0.1");
    let local_address = wow_config::get_string_default("LoginREST.LocalAddress", "127.0.0.1");
    let rest_addresses =
        resolve_login_rest_addresses_like_cpp(&external_address, &local_address, rest_port).await?;
    tracing::info!(
        external_hostname = %rest_addresses.external_hostname,
        external_address = %rest_addresses.external_address,
        local_hostname = %rest_addresses.local_hostname,
        local_address = %rest_addresses.local_address,
        "LoginREST addresses resolved"
    );
    let ticket_duration: u64 = wow_config::get_value("LoginREST.TicketDuration").unwrap_or(3600);
    let wrong_pass_max: u32 = wow_config::get_value("WrongPass.MaxCount").unwrap_or(0);
    let wrong_pass_ban_time: u32 = wow_config::get_value("WrongPass.BanTime").unwrap_or(600);
    let wrong_pass_ban_type: u32 = wow_config::get_value("WrongPass.BanType").unwrap_or(0);
    let wrong_pass_logging: bool = wow_config::get_value_default("WrongPass.Logging", false);
    let realm_update_delay: u64 = wow_config::get_value("RealmsStateUpdateDelay").unwrap_or(10);
    let ban_check_interval: u64 = wow_config::get_value("BanExpiryCheckInterval").unwrap_or(60);
    let max_ping_time_minutes: u64 = wow_config::get_value("MaxPingTime").unwrap_or(30);
    let ip_location = load_ip_location_from_config_like_cpp();

    let state = Arc::new(AppState::new(
        login_db,
        ip_location,
        rest_addresses.external_hostname,
        rest_addresses.local_hostname,
        rest_port,
        rpc_port,
        ticket_duration,
        wrong_pass_max,
        wrong_pass_ban_time,
        wrong_pass_ban_type,
        wrong_pass_logging,
    ));

    // Initialize realm manager
    realm::init_realm_manager(Arc::clone(&state), realm_update_delay).await?;

    // Start ban expiry timer
    start_ban_expiry_timer(Arc::clone(&state), ban_check_interval);
    start_database_keep_alive_timer(Arc::clone(&state), max_ping_time_minutes);

    // Start REST API server (HTTPS)
    let rest_addr = format!("{bind_ip}:{rest_port}");
    let rest_listener = TcpListener::bind(&rest_addr)
        .await
        .with_context(|| format!("Failed to bind REST on {rest_addr}"))?;
    tracing::info!("REST API (HTTPS) listening on {rest_addr}");

    let rest_drain = rest::RestDrain::new();
    let rest_state = Arc::clone(&state);
    let rest_drain_for_accept = rest_drain.clone();
    let mut rest_handle = tokio::spawn(async move {
        loop {
            match rest_listener.accept().await {
                Ok((stream, addr)) => {
                    tracing::debug!("REST: new connection from {addr}");
                    let acceptor = rest_tls_acceptor.clone();
                    let state = Arc::clone(&rest_state);
                    let drain = rest_drain_for_accept.clone();
                    tokio::spawn(async move {
                        match state
                            .remote_ip_is_banned_like_cpp(&addr.ip().to_string())
                            .await
                        {
                            Ok(true) => {
                                tracing::debug!("{addr} tried to log in using banned IP");
                                return;
                            }
                            Ok(false) => {}
                            Err(error) => {
                                tracing::warn!(
                                    "Failed to check REST banned IP status for {addr}: {error}"
                                );
                            }
                        }

                        let tls_stream = match acceptor.accept(stream).await {
                            Ok(s) => s,
                            Err(e) => {
                                tracing::debug!("REST TLS handshake failed from {addr}: {e}");
                                return;
                            }
                        };
                        tracing::debug!("REST: TLS established with {addr}");
                        // Use raw HTTP handler — avoids hyper's TLS CloseNotify
                        // that the WoW client doesn't handle correctly.
                        rest::handle_rest_connection(tls_stream, state, addr, drain).await;
                    });
                }
                Err(e) => {
                    tracing::error!("REST accept error: {e}");
                }
            }
        }
    });

    // Start BNet RPC listener (TLS)
    let rpc_addr = format!("{bind_ip}:{rpc_port}");
    let rpc_listener = TcpListener::bind(&rpc_addr)
        .await
        .with_context(|| format!("Failed to bind RPC on {rpc_addr}"))?;
    tracing::info!("BNet RPC (TLS) listening on {rpc_addr}");

    let rpc_state = Arc::clone(&state);
    let mut rpc_handle = tokio::spawn(async move {
        rpc::accept_loop(rpc_listener, rpc_state, rpc_tls_acceptor).await;
    });

    tracing::info!("BNet Server ready");

    // Wait for shutdown. A listener task ending is not a graceful shutdown signal:
    // TC treats network initialization/runtime failures as fatal, while signals
    // are the normal path that stops the io_context.
    let shutdown_result = tokio::select! {
        result = &mut rest_handle => listener_task_exit_like_cpp("REST", result),
        result = &mut rpc_handle => listener_task_exit_like_cpp("RPC", result),
        signal = shutdown_signal_like_cpp() => {
            tracing::info!("Shutting down after {signal}...");
            rest_handle.abort();
            rpc_handle.abort();
            rest_drain.begin_shutdown();
            drain_rest_requests_like_cpp(&rest_drain, std::time::Duration::from_secs(5)).await;
            Ok(())
        },
    };

    state.login_db.close().await;
    tracing::info!("BNet Server stopped.");
    shutdown_result
}

async fn drain_rest_requests_like_cpp(drain: &rest::RestDrain, grace: std::time::Duration) {
    if drain.in_flight() == 0 {
        return;
    }

    tracing::info!(
        in_flight_rest_requests = drain.in_flight(),
        grace_ms = grace.as_millis(),
        "Waiting for in-flight REST requests to finish before closing LoginDatabase"
    );

    if tokio::time::timeout(grace, drain.wait_for_idle())
        .await
        .is_err()
    {
        tracing::warn!(
            in_flight_rest_requests = drain.in_flight(),
            "REST shutdown drain grace period expired"
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BnetCliLikeCpp {
    config_file: PathBuf,
    config_dir: PathBuf,
    update_databases_only: bool,
    show_version: bool,
    show_help: bool,
}

impl BnetCliLikeCpp {
    fn parse_from(args: impl IntoIterator<Item = String>) -> Self {
        let mut cli = Self::default();
        let mut args = args.into_iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--help" | "-h" => cli.show_help = true,
                "--version" | "-v" => cli.show_version = true,
                "--update-databases-only" | "-u" => cli.update_databases_only = true,
                "--config" | "-c" => {
                    if let Some(value) = args.next() {
                        cli.config_file = PathBuf::from(value);
                    }
                }
                "--config-dir" | "-cd" => {
                    if let Some(value) = args.next() {
                        cli.config_dir = PathBuf::from(value);
                    }
                }
                _ => {
                    if let Some(value) = arg.strip_prefix("--config=") {
                        cli.config_file = PathBuf::from(value);
                    } else if let Some(value) = arg.strip_prefix("--config-dir=") {
                        cli.config_dir = PathBuf::from(value);
                    }
                }
            }
        }

        cli
    }
}

impl Default for BnetCliLikeCpp {
    fn default() -> Self {
        Self {
            config_file: PathBuf::from(BNET_CONFIG_CANDIDATES[0]),
            config_dir: PathBuf::from(BNET_CONFIG_DIR),
            update_databases_only: false,
            show_version: false,
            show_help: false,
        }
    }
}

fn bnet_cli_help_like_cpp() -> &'static str {
    "Allowed options:\n  -h [ --help ]                  print usage message\n  -v [ --version ]               print version build info\n  -c [ --config ] <arg>          use <arg> as configuration file\n  -cd [ --config-dir ] <arg>     use <arg> as directory with additional config files\n  -u [ --update-databases-only ] updates databases only\n"
}

fn load_bnet_config(cli: &BnetCliLikeCpp) -> Result<LoadReport> {
    load_bnet_config_from(&cli.config_file, &cli.config_dir)
}

fn load_bnet_config_from(
    config_file: &std::path::Path,
    config_dir: &std::path::Path,
) -> Result<LoadReport> {
    let config_file = config_file.to_string_lossy();
    let config_dir = config_dir.to_string_lossy();
    let fallback_dist = format!("{config_file}.dist");
    let candidates = [config_file.as_ref(), fallback_dist.as_str()];
    let loaded_config = wow_config::load_config_with_fallbacks(&candidates, &config_dir)
        .context("Failed to load bnetserver.conf")?;

    if loaded_config.candidate_index > 0 {
        tracing::warn!(
            config = %loaded_config.initial_file,
            "Using .dist fallback config file"
        );
    }

    Ok(loaded_config)
}

async fn run_database_updates_like_cpp(login_db: &LoginDatabase, login_info: &DatabaseInfo) {
    let auto_setup = wow_config::get_string_default("Updates.AutoSetup", "1");
    if auto_setup == "0" || auto_setup.eq_ignore_ascii_case("false") {
        return;
    }

    use wow_database::updater::DbUpdater;
    let src = wow_config::get_string_default("Updates.SourcePath", ".");
    let auth_up = DbUpdater::new(
        login_db.pool().clone(),
        &login_info.host,
        &login_info.port_or_socket,
        &login_info.username,
        &login_info.password,
        &login_info.database,
        login_info.ssl,
    );
    if let Err(e) = auth_up
        .populate(&format!("{src}/sql/base/auth_database.sql"))
        .await
    {
        tracing::warn!("Auth populate skipped: {e}");
    }
    if let Err(e) = auth_up.update(&src).await {
        tracing::warn!("Auth update error: {e}");
    }
}

fn log_database_target_like_cpp(kind: &str, info: &DatabaseInfo) {
    tracing::info!(
        database_kind = kind,
        host = %info.host,
        port_or_socket = %info.port_or_socket,
        database = %info.database,
        "Connecting to database"
    );
}

fn log_startup_banner_like_cpp(config_report: &LoadReport) {
    tracing::info!("{}", bnet_full_version_like_cpp());
    tracing::info!(
        config = %config_report.initial_file,
        "Using configuration file"
    );
    for loaded_file in &config_report.loaded_files {
        tracing::info!(config = %loaded_file, "Using additional configuration file");
    }
    for overridden_key in &config_report.overridden_keys {
        tracing::info!(
            key = %overridden_key,
            "Configuration field was overridden with environment variable"
        );
    }
    tracing::info!(
        tls_backend = "rustls",
        rustls = "0.23",
        tokio_rustls = "0.26",
        sqlx = "0.8",
        "Using Rust dependency versions"
    );
}

fn bnet_full_version_like_cpp() -> String {
    let revision = option_env!("GIT_HASH")
        .or(option_env!("VERGEN_GIT_SHA"))
        .unwrap_or("unknown");
    format!(
        "RustyCore BNet Server {} (rev {revision})",
        env!("CARGO_PKG_VERSION")
    )
}

fn log_thread_config_like_cpp() {
    let config = bnet_thread_config_from_values_like_cpp(
        wow_config::get_value("Network.Threads").unwrap_or(1),
        wow_config::get_value("LoginREST.ThreadCount").unwrap_or(1),
    );

    tracing::info!(
        network_threads = config.network_threads,
        login_rest_thread_count = config.login_rest_thread_count,
        applies_to_bnet_acceptors = config.applies_to_bnet_acceptors,
        "BNet thread configuration loaded; Rust uses Tokio's multi-thread runtime and TrinityCore bnetserver does not gate startup on Network.Threads"
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BnetThreadConfigLikeCpp {
    network_threads: i32,
    login_rest_thread_count: i32,
    applies_to_bnet_acceptors: bool,
}

fn bnet_thread_config_from_values_like_cpp(
    network_threads: i32,
    login_rest_thread_count: i32,
) -> BnetThreadConfigLikeCpp {
    BnetThreadConfigLikeCpp {
        network_threads,
        login_rest_thread_count,
        applies_to_bnet_acceptors: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LoginRestResolvedAddressesLikeCpp {
    external_hostname: String,
    external_address: Ipv4Addr,
    local_hostname: String,
    local_address: Ipv4Addr,
}

async fn resolve_login_rest_addresses_like_cpp(
    external_hostname: &str,
    local_hostname: &str,
    port: u16,
) -> Result<LoginRestResolvedAddressesLikeCpp> {
    let external_address =
        resolve_login_rest_address_like_cpp("LoginREST.ExternalAddress", external_hostname, port)
            .await?;
    let local_address =
        resolve_login_rest_address_like_cpp("LoginREST.LocalAddress", local_hostname, port).await?;

    Ok(LoginRestResolvedAddressesLikeCpp {
        external_hostname: external_hostname.to_string(),
        external_address,
        local_hostname: local_hostname.to_string(),
        local_address,
    })
}

async fn resolve_login_rest_address_like_cpp(
    config_key: &str,
    hostname: &str,
    port: u16,
) -> Result<Ipv4Addr> {
    let endpoints = tokio::net::lookup_host((hostname, port))
        .await
        .with_context(|| format!("Could not resolve {config_key} {hostname}"))?;
    let address = first_ipv4_address_like_cpp(endpoints)
        .with_context(|| format!("Could not resolve {config_key} {hostname} to an IPv4 address"))?;

    tracing::info!(config_key, hostname, %address, "Resolved LoginREST address");
    Ok(address)
}

fn first_ipv4_address_like_cpp(
    endpoints: impl IntoIterator<Item = SocketAddr>,
) -> Option<Ipv4Addr> {
    endpoints
        .into_iter()
        .find_map(|endpoint| match endpoint.ip() {
            IpAddr::V4(address) => Some(address),
            IpAddr::V6(_) => None,
        })
}

fn listener_task_exit_like_cpp(
    service_name: &str,
    result: std::result::Result<(), tokio::task::JoinError>,
) -> Result<()> {
    match result {
        Ok(()) => anyhow::bail!("{service_name} listener stopped unexpectedly"),
        Err(error) => anyhow::bail!("{service_name} listener task failed: {error}"),
    }
}

fn load_ip_location_from_config_like_cpp() -> IpLocationStore {
    tracing::info!("Loading IP Location Database...");
    let database_file_path = wow_config::get_string_default("IPLocationFile", "");
    if database_file_path.is_empty() {
        return IpLocationStore::default();
    }

    if !std::path::Path::new(&database_file_path).exists() {
        tracing::error!("IPLocation: No ip database file exists ({database_file_path}).");
        return IpLocationStore::default();
    }

    let contents = match std::fs::read_to_string(&database_file_path) {
        Ok(contents) => contents,
        Err(error) => {
            tracing::error!(
                "IPLocation: Ip database file ({database_file_path}) can not be opened: {error}"
            );
            return IpLocationStore::default();
        }
    };

    let store = IpLocationStore::from_csv_like_cpp(&contents);
    tracing::info!(">> Loaded {} ip location entries.", store.len());
    store
}

fn create_pid_file_from_config_like_cpp() -> Result<Option<u32>> {
    let pid_file = wow_config::get_string_default("PidFile", "");
    if pid_file.is_empty() {
        return Ok(None);
    }

    let pid = create_pid_file_like_cpp(&pid_file)
        .with_context(|| format!("Cannot create PID file {pid_file}"))?;
    tracing::info!("Daemon PID: {pid}");
    Ok(Some(pid))
}

fn create_pid_file_like_cpp(path: impl AsRef<std::path::Path>) -> std::io::Result<u32> {
    let pid = std::process::id();
    std::fs::write(path, pid.to_string())?;
    Ok(pid)
}

#[cfg(unix)]
async fn shutdown_signal_like_cpp() -> &'static str {
    let ctrl_c = tokio::signal::ctrl_c();
    tokio::pin!(ctrl_c);

    let mut sigterm = match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    {
        Ok(signal) => signal,
        Err(error) => {
            tracing::warn!("Failed to install SIGTERM handler: {error}");
            let _ = ctrl_c.await;
            return "SIGINT";
        }
    };

    tokio::select! {
        result = &mut ctrl_c => {
            if let Err(error) = result {
                tracing::warn!("Failed while waiting for SIGINT: {error}");
            }
            "SIGINT"
        }
        _ = sigterm.recv() => "SIGTERM",
    }
}

#[cfg(not(unix))]
async fn shutdown_signal_like_cpp() -> &'static str {
    if let Err(error) = tokio::signal::ctrl_c().await {
        tracing::warn!("Failed while waiting for SIGINT: {error}");
    }
    "SIGINT"
}

/// Load TLS certificates and create two acceptors:
/// - REST acceptor: with ALPN for HTTP/1.1 (HTTPS)
/// - RPC acceptor: without ALPN (raw binary protocol)
fn load_tls_acceptors(
    cert_file_path: &str,
    private_key_file_path: &str,
    private_key_password: Option<&str>,
) -> Result<(TlsAcceptor, TlsAcceptor)> {
    use rustls::ServerConfig;
    use rustls::pki_types::CertificateDer;
    use std::io::BufReader;

    // Load certificates
    let cert_file = std::fs::File::open(cert_file_path)
        .with_context(|| format!("Failed to open {cert_file_path}"))?;
    let mut cert_reader = BufReader::new(cert_file);
    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("Failed to parse certificates")?;

    if certs.is_empty() {
        anyhow::bail!("No certificates found in {cert_file_path}");
    }

    let key = load_private_key_like_cpp(private_key_file_path, private_key_password)?;

    tracing::info!(
        "Loaded {} certificate(s) from {cert_file_path}; private key from {private_key_file_path}",
        certs.len()
    );

    // REST config — TLS 1.2 only, NO ALPN (matching C# SslStream which doesn't set ALPN)
    // (WoW 3.4.3 client expects TLS 1.2; matching C# SslProtocols.Tls12)
    let rest_config = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS12])
        .with_no_client_auth()
        .with_single_cert(certs.clone(), key.clone_key())
        .context("Failed to build REST TLS config")?;

    // RPC config — TLS 1.2 only, no ALPN (binary protobuf protocol)
    let rpc_config = ServerConfig::builder_with_protocol_versions(&[&rustls::version::TLS12])
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .context("Failed to build RPC TLS config")?;

    Ok((
        TlsAcceptor::from(Arc::new(rest_config)),
        TlsAcceptor::from(Arc::new(rpc_config)),
    ))
}

fn load_private_key_like_cpp(
    private_key_file_path: &str,
    private_key_password: Option<&str>,
) -> Result<rustls::pki_types::PrivateKeyDer<'static>> {
    use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
    use std::io::BufReader;

    match private_key_password {
        None => {
            let key_file = std::fs::File::open(private_key_file_path)
                .with_context(|| format!("Failed to open {private_key_file_path}"))?;
            let mut key_reader = BufReader::new(key_file);
            rustls_pemfile::private_key(&mut key_reader)
                .context("Failed to parse private key")?
                .ok_or_else(|| anyhow::anyhow!("No private key found in {private_key_file_path}"))
        }
        Some(password) => {
            let pem = std::fs::read_to_string(private_key_file_path)
                .with_context(|| format!("Failed to open {private_key_file_path}"))?;
            let decrypted =
                decrypt_pkcs8_private_key_pem_like_cpp(&pem, password).with_context(|| {
                    format!("Failed to decrypt private key {private_key_file_path}")
                })?;
            Ok(PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(decrypted)))
        }
    }
}

fn decrypt_pkcs8_private_key_pem_like_cpp(pem: &str, password: &str) -> Result<Vec<u8>> {
    use pkcs8::der::Decode;
    use pkcs8::der::pem::PemLabel;

    let (label, doc) = pkcs8::SecretDocument::from_pem(pem)
        .context("Failed to parse encrypted PKCS#8 private key PEM")?;
    if label != pkcs8::EncryptedPrivateKeyInfo::PEM_LABEL {
        anyhow::bail!(
            "PrivateKeyPassword is configured, but the key is '{label}', not an encrypted PKCS#8 private key"
        );
    }

    let encrypted = pkcs8::EncryptedPrivateKeyInfo::from_der(doc.as_bytes())
        .context("Failed to parse encrypted PKCS#8 private key")?;
    let decrypted = encrypted
        .decrypt(password.as_bytes())
        .context("Failed to decrypt encrypted PKCS#8 private key")?;
    Ok(decrypted.as_bytes().to_vec())
}

/// Periodically clean up expired bans.
fn start_ban_expiry_timer(state: Arc<AppState>, interval_secs: u64) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
        loop {
            interval.tick().await;
            let stmt = state.login_db.prepare(LoginStatements::DEL_EXPIRED_IP_BANS);
            if let Err(e) = state.login_db.execute(&stmt).await {
                tracing::warn!("Failed to delete expired IP bans: {e}");
            }
            let stmt = state
                .login_db
                .prepare(LoginStatements::UPD_EXPIRED_ACCOUNT_BANS);
            if let Err(e) = state.login_db.execute(&stmt).await {
                tracing::warn!("Failed to update expired account bans: {e}");
            }
            let stmt = state
                .login_db
                .prepare(LoginStatements::DEL_BNET_EXPIRED_ACCOUNT_BANNED);
            if let Err(e) = state.login_db.execute(&stmt).await {
                tracing::warn!("Failed to delete expired BNet account bans: {e}");
            }
        }
    });
}

/// Periodically ping LoginDatabase like TrinityCore's `KeepDatabaseAliveHandler`.
fn start_database_keep_alive_timer(state: Arc<AppState>, interval_minutes: u64) {
    if interval_minutes == 0 {
        tracing::warn!("MaxPingTime is 0; database keep-alive timer disabled");
        return;
    }

    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(db_keep_alive_interval_duration_like_cpp(interval_minutes));
        loop {
            interval.tick().await;
            tracing::info!("Ping MySQL to keep connection alive");
            if let Err(error) = state.login_db.direct_query("SELECT 1").await {
                tracing::warn!("Failed to keep LoginDatabase alive: {error}");
            }
        }
    });
}

fn db_keep_alive_interval_duration_like_cpp(interval_minutes: u64) -> std::time::Duration {
    std::time::Duration::from_secs(interval_minutes.saturating_mul(60))
}

#[cfg(test)]
mod tests {
    use super::{
        BnetCliLikeCpp, bnet_cli_help_like_cpp, bnet_full_version_like_cpp,
        bnet_thread_config_from_values_like_cpp, create_pid_file_like_cpp,
        db_keep_alive_interval_duration_like_cpp, decrypt_pkcs8_private_key_pem_like_cpp,
        first_ipv4_address_like_cpp, listener_task_exit_like_cpp, load_bnet_config_from,
        load_private_key_like_cpp,
    };
    use std::env;
    use std::fs;
    use std::net::{Ipv4Addr, SocketAddr};
    use std::path::PathBuf;
    use std::sync::Mutex;

    static CONFIG_TEST_LOCK: Mutex<()> = Mutex::new(());
    const ENCRYPTED_PKCS8_TEST_KEY: &str = r#"-----BEGIN ENCRYPTED PRIVATE KEY-----
MIIC3TBXBgkqhkiG9w0BBQ0wSjApBgkqhkiG9w0BBQwwHAQIUyBCum5/y54CAggA
MAwGCCqGSIb3DQIJBQAwHQYJYIZIAWUDBAEqBBC/XmvFo8zjwfieHYC70YDrBIIC
gOZC8gzx2anQD8lvyzVhWKpupCrl0KcOnF78xdY5tka278fTNZHZaiHgG3gN/2BA
XoZUcoggibt4R5Cv3gVOl+XJazTZVz905nabPKY2DX0mvlkC6QG1eD/QIQSJf3xy
JIJVz4/EMMpEfoRGzAopvYDT5KOoicWMyOT3wRGjFhQ7pkS8K1gknfOS/nJ2MReo
huAqvQWzv3QG1k0ywnBNfqLVJIYncAEdJ0EbveFK+iYD3/Ie2RjCRIPVUUx7mQZN
GdlQYWmJC0XD3YSJlCLwiKDS/4VFMnWZVIvo9Fja+0kVtRnq/Lh5rSXALRP65S+q
rY84agGD8YvnN1DjC0K/4chisdd4bTBr0U1G6gX6yieNsBzS/1LRIa3NpHvPL6Ta
atVzEs0R0Rnn2zhdBpixOBvFLOgge+NOPx5twOQUIBlCwgtHxBIFRBcz+9Au+mPH
bky5a18a2uwIR03v8DKCPX4zZnWsTy5IERcvu+y0m+D9bzNf5p9bob/CQPxx3kUB
dK42FcCpu0+mnP+SImsdNVufD9qCgmoxgM78kn2mInzdPs7y3otDwc4dfCCxSjyV
bCb2P11mgDUY1gODqvAmD7DEyghiZtUusCKcphBHFw+vobReIKXFAK9a3xrrux2Y
wK0J/RFBYJEw9aYFA5iHRQVVmzCyKro+EaQSrN9/Xi2n3YzRqMY/pduQ6qJ4xA5Q
DkpzLQyZJUrrBCu3ErEKKgJDB4zUoeA2Zx1QI0NffLwF4O0C+2jtVROs887b0kTx
7e6w3smBjkBREUiXdlDW+PYUpIUDFAqjWF8rxk3tg9H+9qeSz3a+vEnuT10pkq3A
5llBUo/cIM8wieR7BJNlnVs=
-----END ENCRYPTED PRIVATE KEY-----"#;

    #[test]
    fn bnet_config_resolution_prefers_lowercase_cpp_name() {
        let _guard = CONFIG_TEST_LOCK.lock().expect("config test lock poisoned");
        let root = unique_temp_dir("bnet_config_resolution");
        let lower = root.join("bnetserver.conf");
        let legacy = root.join("BNetServer.conf");

        fs::write(&lower, "BattlenetPort = 1119\n").expect("write lower failed");
        fs::write(&legacy, "BattlenetPort = 2222\n").expect("write legacy failed");

        let report = load_bnet_config_from(&lower, &root.join("bnetserver.conf.d"))
            .expect("config should load");

        assert_eq!(report.candidate_index, 0);
        assert_eq!(wow_config::get_value::<u16>("BattlenetPort"), Some(1119));

        fs::remove_dir_all(root).expect("cleanup failed");
    }

    #[test]
    fn encrypted_pkcs8_private_key_uses_private_key_password_like_cpp() {
        let der = decrypt_pkcs8_private_key_pem_like_cpp(ENCRYPTED_PKCS8_TEST_KEY, "secret")
            .expect("encrypted key should decrypt");
        assert!(matches!(der.first(), Some(0x30)));
        assert!(rustls::pki_types::PrivateKeyDer::try_from(der).is_ok());
    }

    #[test]
    fn encrypted_pkcs8_private_key_rejects_wrong_password_like_cpp() {
        let error = decrypt_pkcs8_private_key_pem_like_cpp(ENCRYPTED_PKCS8_TEST_KEY, "wrong")
            .expect_err("wrong password should fail");
        assert!(error.to_string().contains("Failed to decrypt"));
    }

    #[test]
    fn private_key_password_requires_encrypted_pkcs8_pem() {
        let root = unique_temp_dir("bnet_tls_password_requires_encrypted_pkcs8");
        let key_path = root.join("key.pem");
        fs::write(
            &key_path,
            r#"-----BEGIN PRIVATE KEY-----
MAoCAQAwBQYDK2Vw
-----END PRIVATE KEY-----
"#,
        )
        .expect("write key failed");

        let error = load_private_key_like_cpp(key_path.to_str().unwrap(), Some("secret"))
            .expect_err("non-encrypted key should fail when password is configured");
        assert!(format!("{error:#}").contains("not an encrypted PKCS#8"));

        fs::remove_dir_all(root).expect("cleanup failed");
    }

    #[test]
    fn bnet_config_loads_cpp_section_and_tls_paths_like_cpp() {
        let _guard = CONFIG_TEST_LOCK.lock().expect("config test lock poisoned");
        let root = unique_temp_dir("bnet_config_cpp_section");
        let lower = root.join("bnetserver.conf");

        fs::write(
            &lower,
            r#"
[bnetserver]
BattlenetPort = 1119
LoginREST.Port = 8081
CertificatesFile = "/tmp/bnetserver.cert.pem"
PrivateKeyFile = "/tmp/bnetserver.key.pem"
PrivateKeyPassword = "secret"
LoginDatabaseInfo = "127.0.0.1;3306;trinity;trinity;auth"
"#,
        )
        .expect("write lower failed");

        load_bnet_config_from(&lower, &root.join("bnetserver.conf.d")).expect("config should load");

        assert_eq!(
            wow_config::get_string_default("CertificatesFile", ""),
            "/tmp/bnetserver.cert.pem"
        );
        assert_eq!(
            wow_config::get_string_default("PrivateKeyFile", ""),
            "/tmp/bnetserver.key.pem"
        );
        assert_eq!(
            wow_config::get_string_default("PrivateKeyPassword", ""),
            "secret"
        );
        let info = wow_config::get_database_info_default(
            "Login",
            wow_config::DatabaseInfo::new("fallback", 1, "fallback", "fallback", "fallback"),
        );
        assert_eq!(info.database, "auth");

        fs::remove_dir_all(root).expect("cleanup failed");
    }

    #[test]
    fn bnet_config_resolution_uses_dist_fallback_for_explicit_config_like_cpp() {
        let _guard = CONFIG_TEST_LOCK.lock().expect("config test lock poisoned");
        let root = unique_temp_dir("bnet_config_dist_fallback");
        let config = root.join("custom-bnet.conf");
        let dist = root.join("custom-bnet.conf.dist");

        fs::write(&dist, "BattlenetPort = 3333\n").expect("write dist failed");

        let report = load_bnet_config_from(&config, &root.join("bnetserver.conf.d"))
            .expect("dist config should load");

        assert_eq!(report.candidate_index, 1);
        assert_eq!(wow_config::get_value::<u16>("BattlenetPort"), Some(3333));

        fs::remove_dir_all(root).expect("cleanup failed");
    }

    #[test]
    fn bnet_cli_parser_accepts_cpp_aliases_and_ignores_unknowns_like_cpp() {
        let cli = BnetCliLikeCpp::parse_from([
            "-c".to_string(),
            "custom.conf".to_string(),
            "-cd".to_string(),
            "custom.conf.d".to_string(),
            "-u".to_string(),
            "--unknown".to_string(),
        ]);

        assert_eq!(cli.config_file, PathBuf::from("custom.conf"));
        assert_eq!(cli.config_dir, PathBuf::from("custom.conf.d"));
        assert!(cli.update_databases_only);
        assert!(!cli.show_help);
        assert!(!cli.show_version);
    }

    #[test]
    fn bnet_cli_parser_accepts_long_equals_and_early_exit_flags_like_cpp() {
        let cli = BnetCliLikeCpp::parse_from([
            "--config=custom.conf".to_string(),
            "--config-dir=custom.conf.d".to_string(),
            "--help".to_string(),
            "--version".to_string(),
        ]);

        assert_eq!(cli.config_file, PathBuf::from("custom.conf"));
        assert_eq!(cli.config_dir, PathBuf::from("custom.conf.d"));
        assert!(cli.show_help);
        assert!(cli.show_version);
        assert!(bnet_cli_help_like_cpp().contains("--update-databases-only"));
    }

    #[test]
    fn db_keep_alive_interval_is_configured_in_minutes_like_cpp() {
        assert_eq!(
            db_keep_alive_interval_duration_like_cpp(30),
            std::time::Duration::from_secs(30 * 60)
        );
        assert_eq!(
            db_keep_alive_interval_duration_like_cpp(1),
            std::time::Duration::from_secs(60)
        );
    }

    #[test]
    fn bnet_thread_config_is_observed_but_not_applied_to_acceptors_like_cpp() {
        let config = bnet_thread_config_from_values_like_cpp(4, 8);

        assert_eq!(config.network_threads, 4);
        assert_eq!(config.login_rest_thread_count, 8);
        assert!(!config.applies_to_bnet_acceptors);
    }

    #[test]
    fn listener_task_clean_exit_is_fatal_like_cpp_network_failure() {
        let error =
            listener_task_exit_like_cpp("REST", Ok(())).expect_err("listener exit must fail");

        assert!(
            error
                .to_string()
                .contains("REST listener stopped unexpectedly")
        );
    }

    #[test]
    fn login_rest_resolution_selects_ipv4_endpoint_like_cpp() {
        let endpoints = [
            "[::1]:8081".parse::<SocketAddr>().unwrap(),
            "192.0.2.10:8081".parse::<SocketAddr>().unwrap(),
        ];

        assert_eq!(
            first_ipv4_address_like_cpp(endpoints),
            Some(Ipv4Addr::new(192, 0, 2, 10))
        );
        assert_eq!(
            first_ipv4_address_like_cpp(["[::1]:8081".parse::<SocketAddr>().unwrap()]),
            None
        );
    }

    #[test]
    fn create_pid_file_writes_current_process_id_like_cpp() {
        let root = unique_temp_dir("pid_file");
        fs::create_dir_all(&root).expect("create temp dir failed");
        let pid_file = root.join("bnetserver.pid");

        let pid = create_pid_file_like_cpp(&pid_file).expect("pid file should be created");

        assert_eq!(pid, std::process::id());
        assert_eq!(
            fs::read_to_string(&pid_file).expect("pid file should be readable"),
            std::process::id().to_string()
        );

        fs::remove_dir_all(root).expect("cleanup failed");
    }

    #[test]
    fn bnet_full_version_contains_package_version_like_cpp_banner() {
        let version = bnet_full_version_like_cpp();
        assert!(version.contains("RustyCore BNet Server"));
        assert!(version.contains(env!("CARGO_PKG_VERSION")));
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "rustycore_bnet_server_{name}_{}",
            std::process::id()
        ));

        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).expect("temp dir failed");
        path
    }
}
