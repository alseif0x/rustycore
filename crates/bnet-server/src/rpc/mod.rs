//! BNet RPC session layer.
//!
//! Handles TLS connections with binary protobuf framing on port 1119.

pub mod services;
pub mod session;

use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;

use crate::state::AppState;

/// Accept loop for BNet RPC connections with TLS.
pub async fn accept_loop(listener: TcpListener, state: Arc<AppState>, tls_acceptor: TlsAcceptor) {
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                tracing::debug!("New RPC connection from {addr}");
                let state = Arc::clone(&state);
                let acceptor = tls_acceptor.clone();
                tokio::spawn(async move {
                    // TLS handshake
                    let tls_stream = match acceptor.accept(stream).await {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::debug!("TLS handshake failed for {addr}: {e}");
                            return;
                        }
                    };
                    let mut session = session::RpcSession::new(tls_stream, addr, state);
                    if let Err(e) = session.run().await {
                        tracing::debug!("RPC session {addr} ended: {e}");
                    }
                });
            }
            Err(e) => {
                tracing::error!("Failed to accept RPC connection: {e}");
            }
        }
    }
}
