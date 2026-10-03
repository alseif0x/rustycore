//! Build-70170 account-phase server, explicitly restricted to the disposable
//! local target while character/world operations are being ported.
mod appearance;
mod bootstrap;
mod character_capture;
mod connection;
mod name_regex;
mod names;

use anyhow::Result;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .init();
    if run().await.is_err() {
        // Source errors may contain row contents/URLs. Never display chains.
        eprintln!(
            "Forever target failed its prerequisites or session operation; no playable-world claim."
        );
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let runtime = bootstrap::load().await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:18085").await?;
    println!("Forever build-70170 isolated account-phase server ready on 127.0.0.1:18085.");
    // One sequential owner, no detached sessions, mirror or background writer.
    loop {
        let (stream, peer) = tokio::select! {
            result = listener.accept() => result?,
            _ = tokio::signal::ctrl_c() => break,
        };
        if !peer.ip().is_loopback() {
            continue;
        }
        match connection::run(&runtime, stream).await {
            Ok(true) => break,
            Ok(false) => {}
            Err(error) if error.is::<connection::DurableStateUncertain>() => return Err(error),
            Err(_) => {
                eprintln!("Forever connection ended/rejected; no secret-bearing errors rendered.")
            }
        }
    }
    Ok(())
}
