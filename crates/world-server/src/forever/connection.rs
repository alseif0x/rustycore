//! Encrypted handoff -> canonical session -> ordered transport publication.
use super::bootstrap::Runtime;
use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::TcpStream;
use wow_database::PreparedStatement;
use wow_network::forever::ForeverSocket;
use wow_world::forever::{Identity, Request, Session};

/// Stop the listener when durable ownership cannot be established/released.
/// Continuing would hide a stuck online flag behind ordinary reconnect errors.
#[derive(Debug)]
pub(super) struct DurableStateUncertain;
impl std::fmt::Display for DurableStateUncertain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("isolated session durable state requires reconciliation")
    }
}
impl std::error::Error for DurableStateUncertain {}

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

pub(super) async fn run(runtime: &Runtime, stream: TcpStream) -> Result<bool> {
    let mut socket = ForeverSocket::new(stream, 0x02010001, runtime.region_group);
    socket.start().await?;
    if !admits_ticket(socket.join_ticket()?) {
        bail!("target variant rejected");
    }
    let row = runtime.auth.direct_query(concat!(
        "SELECT a.session_key_bnet,a.expansion,r.name FROM account a JOIN battlenet_accounts b ON a.battlenet_account=b.id JOIN realmlist r ON r.id=1 ",
        "WHERE a.id=1 AND a.username='1#1' AND a.client_build=70170 AND LENGTH(a.session_key_bnet)=64 AND a.online=0 ",
        "AND b.id=1 AND b.email='FOREVER@LOCAL.TEST' AND (b.locked=0 OR b.last_ip='127.0.0.1') AND b.lock_country IN ('','00') ",
        "AND NOT EXISTS (SELECT 1 FROM account_banned ab WHERE ab.id=a.id AND ab.active=1 AND (ab.unbandate>UNIX_TIMESTAMP() OR ab.unbandate=ab.bandate)) ",
        "AND NOT EXISTS (SELECT 1 FROM battlenet_account_bans bb WHERE bb.id=b.id AND (bb.unbandate>UNIX_TIMESTAMP() OR bb.unbandate=bb.bandate)) ",
        "AND NOT EXISTS (SELECT 1 FROM ip_banned ib WHERE ib.ip='127.0.0.1' AND (ib.unbandate>UNIX_TIMESTAMP() OR ib.unbandate=ib.bandate)) ",
        "AND r.gamebuild=70170 AND r.Region=2 AND r.Battlegroup=1 AND r.port=18085 AND r.flag=0 AND r.icon=1 AND r.allowedSecurityLevel=0"
    )).await?;
    let join_key: [u8; 64] = row
        .try_read::<Vec<u8>>(0)
        .context("admission")?
        .try_into()
        .map_err(|_| anyhow::anyhow!("join key length"))?;
    let account_expansion: u8 = row.try_read(1).context("account expansion")?;
    let realm_name: String = row.try_read(2).context("realm name")?;
    socket.verify_credentials(&join_key, &runtime.build_key)?;
    let mut persist = PreparedStatement::new(
        "UPDATE account SET session_key_bnet=?,online=1 WHERE id=1 AND client_build=70170 AND online=0 AND session_key_bnet=?",
    );
    persist.set_bytes(0, socket.session_key()?.to_vec());
    persist.set_bytes(1, join_key.to_vec());
    let rows = match runtime.auth.execute(&persist).await {
        Ok(rows) => rows,
        Err(_) => {
            // Autocommit acknowledgement can be lost. Admit only after a fresh
            // read confirms our exact key+online projection; unknown is failure.
            let mut confirm = PreparedStatement::new(
                "SELECT id FROM account WHERE id=1 AND client_build=70170 AND online=1 AND session_key_bnet=?",
            );
            confirm.set_bytes(0, socket.session_key()?.to_vec());
            let confirmed = runtime
                .auth
                .query(&confirm)
                .await
                .map_err(|_| DurableStateUncertain)?;
            if confirmed.count() != 1 {
                return Err(DurableStateUncertain.into());
            }
            1
        }
    };
    // Sequential accept owner is the sole fixture session writer. Cleanup is
    // attempted even after encryption/init/publication failure. A concurrent
    // BNet fresh key does not constitute another admitted world session here.
    let mut shutdown = false;
    let outcome = tokio::select! {
        outcome = admitted(runtime, &mut socket, rows, account_expansion, realm_name) => outcome,
        _ = tokio::signal::ctrl_c() => { shutdown = true; Ok(()) },
    };
    if rows == 1 {
        let clean = PreparedStatement::new(
            "UPDATE account SET online=0 WHERE id=1 AND client_build=70170 AND online=1",
        );
        if runtime
            .auth
            .execute(&clean)
            .await
            .map_err(|_| DurableStateUncertain)?
            != 1
        {
            return Err(DurableStateUncertain.into());
        }
    }
    outcome.map(|_| shutdown)
}

async fn admitted(
    runtime: &Runtime,
    socket: &mut ForeverSocket,
    rows: u64,
    account_expansion: u8,
    realm_name: String,
) -> Result<()> {
    // WorldSocket::LoadSessionPermissionsCallback precedes encrypted-mode offer.
    // This disposable ordinary account has no explicit grants/denials. Nonempty
    // customized RBAC requires its target policy integration, not silent ignore.
    let permissions = runtime.auth.direct_query("SELECT permissionId,granted FROM rbac_account_permissions WHERE accountId=1 AND (realmId=1 OR realmId=-1) ORDER BY permissionId,realmId").await?;
    if !permissions.is_empty() {
        bail!("explicit target RBAC policy required");
    }
    socket.complete_encryption(rows).await?;
    println!("Forever native digest and encryption ACK accepted; initializing real account state.");
    let mut session = Session::after_encryption(
        Identity {
            account_id: 1,
            battlenet_id: 1,
            realm_address: 0x02010001,
            account_expansion,
        },
        runtime.session_repository.clone(),
        runtime.hotfixes.clone(),
    )
    .map_err(|_| anyhow::anyhow!("session construction"))?;
    let policy = wow_world::forever::InitializationPolicy {
        realm_name: realm_name.clone(),
        normalized_realm_name: realm_name
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect(),
        timezone: runtime.policy.timezone.clone(),
        cache_version: runtime.policy.cache_version,
        content_set: runtime.policy.content_set,
        max_characters: runtime.policy.max_characters,
    };
    // No realm-name stand-in: the initialized virtual realm uses its real row.
    let time = i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())?;
    let init = session
        .initialize(&runtime.catalog, &policy, time)
        .await
        .map_err(|error| {
            eprintln!("Forever initialization rejected: {error:?}");
            anyhow::anyhow!("session initialization")
        })?;
    for message in init {
        socket.send(message.opcode(), message.payload()).await?;
    }
    println!(
        "Real account snapshot loaded; ordered AuthResponse/init batch published (native UI not inferred)."
    );
    loop {
        let frame = socket
            .receive_with_idle_timeout(
                session
                    .character_idle_remaining(runtime.character_idle_timeout)
                    .context("character session idle")?,
            )
            .await?;
        println!(
            "Forever encrypted request: opcode=0x{:06X}, payload_bytes={}",
            frame.opcode(),
            frame.payload().len()
        );
        // Read-only metadata for the still-unported DBQueryBulk boundary.
        // This does not dispatch, admit a record or publish an absent-record reply.
        if frame.opcode() == 0x440010 && frame.payload().len() >= 6 {
            let bytes = frame.payload();
            let table = u32::from_le_bytes(bytes[..4].try_into().expect("bounded metadata"));
            let count = u16::from(bytes[4]) << 5 | u16::from(bytes[5] >> 3);
            println!(
                "Unported DBQueryBulk metadata: table_hash=0x{table:08X}, requested_records={count}"
            );
        }
        let output = session
            .dispatch(
                &runtime.catalog,
                Request {
                    opcode: frame.opcode(),
                    payload: frame.payload().to_vec(),
                },
            )
            .await
            .map_err(|error| {
                eprintln!("Forever operation rejected: {error:?}");
                anyhow::anyhow!("session operation")
            })?;
        if let Some(output) = output {
            let enumerated = output.iter().any(|message| message.opcode() == 0x460018);
            for message in output {
                socket.send(message.opcode(), message.payload()).await?;
            }
            if session.is_closed() {
                return Ok(());
            }
            if enumerated {
                println!(
                    "Database-backed empty enum published; native character UI still requires observation."
                );
            }
        } else {
            println!("Unregistered target opcode ignored; no invented response.");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::admits_ticket;

    #[test]
    fn ticket_admission_is_exact_and_does_not_accept_other_accounts_or_variants() {
        let ticket = serde_json::json!({ "gameAccount": "1#1", "platform": u32::from_be_bytes(*b"\0Win"), "clientArch": u32::from_be_bytes(*b"\0x64"), "type": u32::from_be_bytes(*b"WoWB") });
        assert!(admits_ticket(&ticket.to_string()));
        for (field, value) in [
            ("gameAccount", serde_json::json!("2#1")),
            ("platform", serde_json::json!(0)),
            ("clientArch", serde_json::json!(0)),
            ("type", serde_json::json!(u32::from_be_bytes(*b"WoW\0"))),
            ("extra", serde_json::json!(1)),
        ] {
            let mut invalid = ticket.clone();
            invalid[field] = value;
            assert!(!admits_ticket(&invalid.to_string()));
        }
        assert!(!admits_ticket("{}"));
        assert!(!admits_ticket("not-json"));
    }
}
