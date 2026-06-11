//! BNet Authentication Server — entry point.
//!
//! Handles Battle.net account login via REST (HTTPS) and BNet RPC (TLS).
//! This is a drop-in replacement for the C# BNetServer.

mod realm;
mod rest;
mod rpc;
mod state;

use anyhow::{Context, Result};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use wow_config::{DatabaseInfo, LoadReport};
use wow_database::{LoginDatabase, LoginStatements, build_connection_string};

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

    tracing::info!("RustyCore BNet Server starting...");

    load_bnet_config()?;

    // Database connection
    let login_info = wow_config::get_database_info_default(
        "Login",
        DatabaseInfo::new("127.0.0.1", 3306, "trinity", "trinity", "auth"),
    );
    log_database_target_like_cpp("login", &login_info);

    let conn_str = build_connection_string(
        &login_info.host,
        &login_info.port_or_socket,
        &login_info.username,
        &login_info.password,
        &login_info.database,
    );
    let login_db = LoginDatabase::open(&conn_str)
        .await
        .context("Failed to connect to login database")?;

    tracing::info!("Connected to login database");

    // ── Database auto-update (auth only) ─────────────────────────────────
    let auto_setup = wow_config::get_string_default("Updates.AutoSetup", "1");
    if auto_setup != "0" && auto_setup.to_lowercase() != "false" {
        use wow_database::updater::DbUpdater;
        let src = wow_config::get_string_default("Updates.SourcePath", ".");
        let auth_up = DbUpdater::new(
            login_db.pool().clone(),
            &login_info.host,
            &login_info.port_or_socket,
            &login_info.username,
            &login_info.password,
            &login_info.database,
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
    // ─────────────────────────────────────────────────────────────────────

    // Load TLS certificates — separate configs for REST (HTTPS) and RPC (binary)
    let cert_file = wow_config::get_string_default("CertificatesFile", "./bnetserver.cert.pem");
    let key_file = wow_config::get_string_default("PrivateKeyFile", "./bnetserver.key.pem");
    let (rest_tls_acceptor, rpc_tls_acceptor) =
        load_tls_acceptors(&cert_file, &key_file).context("Failed to load TLS certificates")?;
    tracing::info!("TLS certificates loaded");

    // Build shared state
    let bind_ip = wow_config::get_string_default("BindIP", "0.0.0.0");
    let rest_port: u16 = wow_config::get_value("LoginREST.Port").unwrap_or(8081);
    let rpc_port: u16 = wow_config::get_value("BattlenetPort").unwrap_or(1119);
    let external_address = wow_config::get_string_default("LoginREST.ExternalAddress", "127.0.0.1");
    let local_address = wow_config::get_string_default("LoginREST.LocalAddress", "127.0.0.1");
    let ticket_duration: u64 = wow_config::get_value("LoginREST.TicketDuration").unwrap_or(3600);
    let wrong_pass_max: u32 = wow_config::get_value("WrongPass.MaxCount").unwrap_or(0);
    let wrong_pass_ban_time: u32 = wow_config::get_value("WrongPass.BanTime").unwrap_or(600);
    let wrong_pass_ban_type: u32 = wow_config::get_value("WrongPass.BanType").unwrap_or(0);
    let wrong_pass_logging: bool = wow_config::get_value_default("WrongPass.Logging", false);
    let realm_update_delay: u64 = wow_config::get_value("RealmsStateUpdateDelay").unwrap_or(10);
    let ban_check_interval: u64 = wow_config::get_value("BanExpiryCheckInterval").unwrap_or(60);

    let state = Arc::new(AppState::new(
        login_db,
        external_address,
        local_address,
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

    // Start REST API server (HTTPS)
    let rest_addr = format!("{bind_ip}:{rest_port}");
    let rest_listener = TcpListener::bind(&rest_addr)
        .await
        .with_context(|| format!("Failed to bind REST on {rest_addr}"))?;
    tracing::info!("REST API (HTTPS) listening on {rest_addr}");

    let rest_state = Arc::clone(&state);
    let rest_handle = tokio::spawn(async move {
        loop {
            match rest_listener.accept().await {
                Ok((stream, addr)) => {
                    tracing::debug!("REST: new connection from {addr}");
                    let acceptor = rest_tls_acceptor.clone();
                    let state = Arc::clone(&rest_state);
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
                        rest::handle_rest_connection(tls_stream, state, addr).await;
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
    let rpc_handle = tokio::spawn(async move {
        rpc::accept_loop(rpc_listener, rpc_state, rpc_tls_acceptor).await;
    });

    tracing::info!("BNet Server ready");

    // Wait for shutdown
    tokio::select! {
        _ = rest_handle => tracing::warn!("REST server stopped"),
        _ = rpc_handle => tracing::warn!("RPC server stopped"),
        _ = tokio::signal::ctrl_c() => tracing::info!("Shutting down..."),
    }

    state.login_db.close().await;
    tracing::info!("BNet Server stopped.");
    Ok(())
}

fn load_bnet_config() -> Result<LoadReport> {
    load_bnet_config_from(BNET_CONFIG_CANDIDATES, BNET_CONFIG_DIR)
}

fn load_bnet_config_from(config_candidates: &[&str], config_dir: &str) -> Result<LoadReport> {
    let loaded_config = wow_config::load_config_with_fallbacks(config_candidates, config_dir)
        .context("Failed to load bnetserver.conf")?;

    if loaded_config.candidate_index > 1 {
        tracing::warn!(
            config = %loaded_config.initial_file,
            "Using legacy Rust config filename; prefer bnetserver.conf"
        );
    }

    Ok(loaded_config)
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

/// Load TLS certificates and create two acceptors:
/// - REST acceptor: with ALPN for HTTP/1.1 (HTTPS)
/// - RPC acceptor: without ALPN (raw binary protocol)
fn load_tls_acceptors(
    cert_file_path: &str,
    private_key_file_path: &str,
) -> Result<(TlsAcceptor, TlsAcceptor)> {
    use rustls::ServerConfig;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
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

    // Load private key
    let key_file = std::fs::File::open(private_key_file_path)
        .with_context(|| format!("Failed to open {private_key_file_path}"))?;
    let mut key_reader = BufReader::new(key_file);
    let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut key_reader)
        .context("Failed to parse private key")?
        .ok_or_else(|| anyhow::anyhow!("No private key found in {private_key_file_path}"))?;

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

#[cfg(test)]
mod tests {
    use super::load_bnet_config_from;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Mutex;

    static CONFIG_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn bnet_config_resolution_prefers_lowercase_cpp_name() {
        let _guard = CONFIG_TEST_LOCK.lock().expect("config test lock poisoned");
        let root = unique_temp_dir("bnet_config_resolution");
        let lower = root.join("bnetserver.conf");
        let legacy = root.join("BNetServer.conf");

        fs::write(&lower, "BattlenetPort = 1119\n").expect("write lower failed");
        fs::write(&legacy, "BattlenetPort = 2222\n").expect("write legacy failed");

        let report = load_bnet_config_from(
            &[
                lower.to_str().expect("utf8 path"),
                legacy.to_str().expect("utf8 path"),
            ],
            root.join("bnetserver.conf.d").to_str().expect("utf8 path"),
        )
        .expect("config should load");

        assert_eq!(report.candidate_index, 0);
        assert_eq!(wow_config::get_value::<u16>("BattlenetPort"), Some(1119));

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
LoginDatabaseInfo = "127.0.0.1;3306;trinity;trinity;auth"
"#,
        )
        .expect("write lower failed");

        load_bnet_config_from(
            &[lower.to_str().expect("utf8 path")],
            root.join("bnetserver.conf.d").to_str().expect("utf8 path"),
        )
        .expect("config should load");

        assert_eq!(
            wow_config::get_string_default("CertificatesFile", ""),
            "/tmp/bnetserver.cert.pem"
        );
        assert_eq!(
            wow_config::get_string_default("PrivateKeyFile", ""),
            "/tmp/bnetserver.key.pem"
        );
        let info = wow_config::get_database_info_default(
            "Login",
            wow_config::DatabaseInfo::new("fallback", 1, "fallback", "fallback", "fallback"),
        );
        assert_eq!(info.database, "auth");

        fs::remove_dir_all(root).expect("cleanup failed");
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
