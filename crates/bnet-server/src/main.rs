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
use wow_database::{LoginDatabase, LoginStatements, build_connection_string};

use crate::state::AppState;

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

    // Load configuration
    let _loaded = wow_config::load_config("BNetServer.conf")
        .or_else(|_| wow_config::load_config("BNetServer.conf.dist"))
        .context("Failed to load BNetServer.conf")?;

    // Database connection
    let db_host = wow_config::get_string_default("LoginDatabaseInfo.Host", "127.0.0.1");
    let db_port: u16 = wow_config::get_value("LoginDatabaseInfo.Port").unwrap_or(3306);
    let db_user = wow_config::get_string_default("LoginDatabaseInfo.Username", "root");
    let db_pass = wow_config::get_string_default("LoginDatabaseInfo.Password", "");
    let db_name = wow_config::get_string_default("LoginDatabaseInfo.Database", "auth");

    let conn_str = build_connection_string(&db_host, db_port, &db_user, &db_pass, &db_name);
    let login_db = LoginDatabase::open(&conn_str)
        .await
        .context("Failed to connect to login database")?;

    tracing::info!("Connected to login database");

    // Load TLS certificates — separate configs for REST (HTTPS) and RPC (binary)
    let cert_file = wow_config::get_string_default("CertificatesFile", "./BNetServer.pfx");
    let (rest_tls_acceptor, rpc_tls_acceptor) = load_tls_acceptors(&cert_file)
        .context("Failed to load TLS certificates")?;
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

/// Load TLS certificates and create two acceptors:
/// - REST acceptor: with ALPN for HTTP/1.1 (HTTPS)
/// - RPC acceptor: without ALPN (raw binary protocol)
fn load_tls_acceptors(_cert_config: &str) -> Result<(TlsAcceptor, TlsAcceptor)> {
    use rustls::ServerConfig;
    use rustls::pki_types::{CertificateDer, PrivateKeyDer};
    use std::io::BufReader;

    let cert_path = "bnet_cert.pem";
    let key_path = "bnet_key.pem";
    let fullchain_path = "bnet_fullchain.pem";

    let cert_file_path = if std::path::Path::new(fullchain_path).exists() {
        fullchain_path
    } else {
        cert_path
    };

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
    let key_file = std::fs::File::open(key_path)
        .with_context(|| format!("Failed to open {key_path}"))?;
    let mut key_reader = BufReader::new(key_file);
    let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut key_reader)
        .context("Failed to parse private key")?
        .ok_or_else(|| anyhow::anyhow!("No private key found in {key_path}"))?;

    tracing::info!("Loaded {} certificate(s) from {cert_file_path}", certs.len());

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
            let stmt = state.login_db.prepare(LoginStatements::UPD_EXPIRED_ACCOUNT_BANS);
            if let Err(e) = state.login_db.execute(&stmt).await {
                tracing::warn!("Failed to update expired account bans: {e}");
            }
            let stmt = state.login_db.prepare(LoginStatements::DEL_BNET_EXPIRED_ACCOUNT_BANNED);
            if let Err(e) = state.login_db.execute(&stmt).await {
                tracing::warn!("Failed to delete expired BNet account bans: {e}");
            }
        }
    });
}
