//! Strict one-shot world-auth QA, NOT a playable realm or authentication bypass.
//! Mutates only the disposable account's continued-session key, after proof.
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{env, fs, path::Path, time::Duration};
use tokio::{net::TcpListener, time::timeout};
use wow_database::{LoginDatabase, PreparedStatement, build_connection_string_with_ssl_like_cpp};
use wow_network::forever::{AUTH_RESPONSE, ForeverSocket, PING, PONG};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Ticket {
    game_account: String,
    platform: u32,
    client_arch: u32,
    #[serde(rename = "type")]
    kind: u32,
}

fn admits_ticket(raw: &str) -> bool {
    let Ok(ticket) = serde_json::from_str::<Ticket>(raw) else {
        return false;
    };
    ticket.game_account == "1#1"
        && ticket.platform == u32::from_be_bytes(*b"\0Win")
        && ticket.client_arch == u32::from_be_bytes(*b"\0x64")
        && ticket.kind == u32::from_be_bytes(*b"WoWB")
}

fn ping_serial(opcode: u32, payload: &[u8]) -> Result<[u8; 4]> {
    // WorldSocket::HandlePing / Ping::Read: serial:u32, latency:u32.
    // Native Pong dispatcher 0xA1AF50 reads only the four-byte serial.
    if opcode != PING || payload.len() != 8 {
        bail!("not the bounded native ping");
    }
    Ok(payload[..4].try_into().expect("validated ping length"))
}

#[tokio::main]
async fn main() {
    // Database/serde errors may contain request data: never render their chain.
    if run().await.is_err() {
        eprintln!("Isolated world authentication probe failed; no character access claimed.");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 3 || args[0] != "--ack-isolated-probe" {
        bail!("usage: --ack-isolated-probe <bnet-config> <private-build-key-file>");
    }
    wow_config::load_config(args[1].to_str().context("config path")?)?;
    let info = wow_config::parse_database_info(
        "LoginDatabaseInfo",
        &wow_config::get_string_default("LoginDatabaseInfo", ""),
    )?;
    if info.host != "127.0.0.1"
        || info.port_or_socket != "13316"
        || info.database != "auth_forever_70170"
        || wow_config::get_string_default("BindIP", "") != "127.0.0.1"
    {
        bail!("non-fixture target");
    }
    let key_path = Path::new(&args[2]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(key_path)?.permissions().mode() & 0o077 != 0 {
            bail!("key file is not private");
        }
    }
    let build_key: [u8; 16] = fs::read(key_path)?
        .try_into()
        .map_err(|_| anyhow::anyhow!("build-key length"))?;
    let url = build_connection_string_with_ssl_like_cpp(
        &info.host,
        &info.port_or_socket,
        &info.username,
        &info.password,
        &info.database,
        info.ssl,
    );
    let db = LoginDatabase::open_with_pool_size(&url, 1).await?;
    wow_database::migration::validate_runtime_schema(
        db.pool(),
        &wow_database::migration::bundled_manifest()?,
        wow_database::migration::DatabaseKind::Auth,
    )
    .await?;
    let listener = TcpListener::bind("127.0.0.1:18085").await?;
    println!("Isolated strict world-auth probe ready on 127.0.0.1:18085");
    let (stream, peer) = timeout(Duration::from_secs(120), listener.accept()).await??;
    if !peer.ip().is_loopback() {
        bail!("non-loopback peer");
    }
    // WorldSocket.cpp::LoadSessionPermissionsCallback uses this independent
    // certificate keyring selector; it is not the realm's geographic Region.
    let region_group =
        wow_config::get_value_default("Network.EnterEncryptedModeRegionGroup", 0_i32);
    let mut socket = ForeverSocket::new(stream, 0x02010001, region_group);
    socket.start().await?;
    if !admits_ticket(socket.join_ticket()?) {
        bail!("fixture variant rejected");
    }
    // Same account/link/build/ban/IP/country admission as WorldSocket.cpp,
    // with deliberately narrower fixture-only account and realm predicates.
    let account = db.direct_query(concat!(
        "SELECT a.session_key_bnet FROM account a JOIN battlenet_accounts b ON a.battlenet_account=b.id ",
        "WHERE a.id=1 AND a.username='1#1' AND a.client_build=70170 AND LENGTH(a.session_key_bnet)=64 ",
        "AND b.id=1 AND b.email='FOREVER@LOCAL.TEST' AND (b.locked=0 OR b.last_ip='127.0.0.1') ",
        "AND b.lock_country IN ('','00') ",
        "AND NOT EXISTS (SELECT 1 FROM account_banned ab WHERE ab.id=a.id AND ab.active=1 AND (ab.unbandate>UNIX_TIMESTAMP() OR ab.unbandate=ab.bandate)) ",
        "AND NOT EXISTS (SELECT 1 FROM battlenet_account_bans bb WHERE bb.id=b.id AND (bb.unbandate>UNIX_TIMESTAMP() OR bb.unbandate=bb.bandate)) ",
        "AND EXISTS (SELECT 1 FROM realmlist r WHERE r.id=1 AND r.gamebuild=70170 AND r.Region=2 AND r.Battlegroup=1 AND r.port=18085 AND r.flag=0 AND r.icon=1 AND r.allowedSecurityLevel=0)"
    )).await?;
    let join_key: [u8; 64] = account
        .try_read::<Vec<u8>>(0)
        .context("fixture admission")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("join-key length"))?;
    socket.verify_credentials(&join_key, &build_key)?;
    println!("Native AuthSession digest verified (24 bytes); no verification bypass.");
    // Guard the source's 64 -> 40 transition against a concurrent fresh BNet join.
    let mut persist = PreparedStatement::new(
        "UPDATE account SET session_key_bnet=? WHERE id=1 AND client_build=70170 AND session_key_bnet=?",
    );
    persist.set_bytes(0, socket.session_key()?.to_vec());
    persist.set_bytes(1, join_key.to_vec());
    let rows = db.execute(&persist).await?;
    if let Err(error) = socket.complete_encryption(rows).await {
        // This error type carries only phase/shape metadata, never key or bytes.
        eprintln!("Encryption transition rejected: {error}");
        return Err(error.into());
    }
    println!("Signed encryption offer acknowledged; 40-byte session key persisted.");
    let frame = timeout(Duration::from_secs(30), socket.receive()).await??;
    let serial = ping_serial(frame.opcode(), frame.payload())?;
    println!(
        "Encrypted client response authenticated: opcode=0x{:06X}, payload_bytes={}; character_selection_tested=false",
        frame.opcode(),
        frame.payload().len()
    );
    socket.send(PONG, &serial).await?;
    println!(
        "Encrypted Pong sent with the native serial-only layout; native parsing not inferred."
    );
    // Native decoder RVA 0x7EEFC0 and target AuthResponse::Write:
    // ERROR_DENIED=3, absent success/wait option bits. No fake session success.
    socket.send(AUTH_RESPONSE, &[3, 0, 0, 0, 0]).await?;
    // Give the client a bounded chance to process the terminal response before
    // closing. Transport delivery alone is not native parser acceptance.
    match timeout(Duration::from_secs(5), socket.receive()).await {
        Ok(Ok(frame)) => println!(
            "Post-denial client frame: opcode=0x{:06X}, payload_bytes={}",
            frame.opcode(),
            frame.payload().len()
        ),
        Ok(Err(_)) => println!("Client transport closed after denial; UI result not inferred."),
        Err(_) => {
            println!("No post-denial client frame within five seconds; UI result not inferred.")
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture_ticket() -> serde_json::Value {
        serde_json::json!({"gameAccount":"1#1", "platform":u32::from_be_bytes(*b"\0Win"),
            "clientArch":u32::from_be_bytes(*b"\0x64"), "type":u32::from_be_bytes(*b"WoWB")})
    }
    #[test]
    fn exact_fixture_variant_is_admitted() {
        assert!(admits_ticket(&fixture_ticket().to_string()));
    }
    #[test]
    fn malformed_incomplete_unknown_and_duplicate_fields_fail_closed() {
        for raw in [
            "",
            "null",
            "{}",
            "[]",
            r#"{"gameAccount":"1#1","gameAccount":"1#1"}"#,
        ] {
            assert!(!admits_ticket(raw));
        }
        let mut unknown = fixture_ticket();
        unknown["extra"] = 1.into();
        assert!(!admits_ticket(&unknown.to_string()));
    }
    #[test]
    fn other_account_or_platform_is_not_this_fixture() {
        for (field, value) in [
            ("gameAccount", serde_json::json!("2#1")),
            ("platform", serde_json::json!(0)),
            ("clientArch", serde_json::json!(0)),
            ("type", serde_json::json!(0)),
        ] {
            let mut wrong = fixture_ticket();
            wrong[field] = value;
            assert!(!admits_ticket(&wrong.to_string()));
        }
    }

    #[test]
    fn native_ping_returns_only_its_serial() {
        assert_eq!(
            ping_serial(PING, &[0x12, 0x34, 0x56, 0x78, 1, 2, 3, 4]).unwrap(),
            [0x12, 0x34, 0x56, 0x78]
        );
        for len in [0, 4, 7, 9] {
            assert!(ping_serial(PING, &vec![0; len]).is_err());
        }
        assert!(ping_serial(PING + 1, &[0; 8]).is_err());
        assert_eq!(PONG, 0x4D0009);
    }
}
