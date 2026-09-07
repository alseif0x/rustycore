//! Bounded logout/disconnect-save evidence; no fixture setup or SQL writes here.
//! C++ CharacterPackets.cpp LogoutRequest::Read (IdleLogout=false),
//! WorldSession.cpp LogoutPlayer (SaveToDB before LogoutComplete), and
//! Player.cpp SaveToDB/_SaveSpells/_SaveSkills/_SaveEquipmentSets.
use super::*;
use mysql::prelude::Queryable;
use std::collections::BTreeMap;
mod portal;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub(super) struct Projection {
    sha256: String,
    rows: usize,
    #[serde(skip)]
    row_hashes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct Evidence {
    logout_confirmed: bool,
    disconnect_confirmed: bool,
    login_account_offline: Option<bool>,
    pending_portal: Option<portal::Receipt>,
    saved_map: u32,
    saved_position: [f32; 3],
    offline: bool,
    retained_existing_rows: bool,
    logout_time_before: u64,
    logout_time_after: u64,
    before: BTreeMap<String, Projection>,
    after: BTreeMap<String, Projection>,
    known_spells: Vec<u32>,
    favorite_spells: Vec<u32>,
}

pub(super) struct Before {
    logout_time: u64,
    projection: BTreeMap<String, Projection>,
    disconnect: bool,
    portal: bool,
}

pub(super) fn enabled() -> bool {
    flag("WOW_BOT_LOGIN_SAVE_CHECK") || flag("WOW_BOT_LOGIN_DISCONNECT_CHECK")
}

fn flag(name: &str) -> bool {
    std::env::var(name).is_ok_and(|v| is_truthy(&v))
}

fn connect(url: String) -> Result<mysql::Conn> {
    // Never include a connection URL or driver error containing credentials.
    let opts = mysql::Opts::from_url(&url).map_err(|_| anyhow!("invalid QA DB options"))?;
    mysql::Conn::new(opts).map_err(|_| anyhow!("QA database connection failed"))
}

fn projections(conn: &mut mysql::Conn, guid: u64) -> Result<BTreeMap<String, Projection>> {
    let mut out = BTreeMap::new();
    // Selected stable save families only: not a whole-character parity assertion.
    for (table, order) in [
        ("character_spell", "spell"),
        ("character_spell_favorite", "spell"),
        ("character_skills", "skill"),
        ("character_equipmentsets", "setguid"),
        ("character_transmog_outfits", "setguid"),
        ("character_reputation", "faction"),
    ] {
        let rows: Vec<mysql::Row> = conn
            .exec(
                format!("SELECT * FROM {table} WHERE guid = ? ORDER BY {order}"),
                (guid,),
            )
            .with_context(|| format!("read save projection {table}"))?;
        // MySQL value debug encoding preserves type/length/escaping. Digest is
        // comparable within this pinned bot/schema, not a cross-version format.
        let encoded = format!("{rows:?}");
        let row_hashes = rows
            .iter()
            .map(|row| hex::encode(Sha256::digest(format!("{row:?}").as_bytes())))
            .collect();
        out.insert(
            table.to_string(),
            Projection {
                sha256: hex::encode(Sha256::digest(encoded.as_bytes())),
                rows: rows.len(),
                row_hashes,
            },
        );
    }
    Ok(out)
}

pub(super) fn preflight(bot: &config::BotConfig) -> Result<Before> {
    if flag("WOW_BOT_LOGIN_SAVE_CHECK") && flag("WOW_BOT_LOGIN_DISCONNECT_CHECK") {
        bail!("choose normal logout or transport disconnect, not both");
    }
    if portal::enabled() && !flag("WOW_BOT_LOGIN_DISCONNECT_CHECK") {
        bail!("pending portal requires the disconnect-save mode");
    }
    if !bot.account.eq_ignore_ascii_case("TESTBOT1@bot.local") {
        bail!("bounded login-save QA is pinned to TESTBOT1@bot.local");
    }
    let mut auth = connect(auth_db_url()?)?;
    let owner: Option<String> = auth.exec_first(
        "SELECT ba.email FROM account a JOIN battlenet_accounts ba ON ba.id = a.battlenet_account WHERE a.id = ?", (bot.account_id,),
    )?;
    if !owner.is_some_and(|email| email.eq_ignore_ascii_case(&bot.account)) {
        bail!("login-save account ownership mismatch");
    }
    let mut conn = connect(characters_db_url()?)?;
    let rows: Vec<(u64, u8, u64)> = conn.exec(
        "SELECT guid, online, logout_time FROM characters WHERE account = ?",
        (bot.account_id,),
    )?;
    if rows.len() != 1 || rows[0].0 != bot.character_guid || rows[0].1 != 0 {
        bail!("login-save requires the exact sole offline character of the approved account");
    }
    if portal::enabled() {
        portal::preflight(&mut conn, bot)?;
    }
    Ok(Before {
        logout_time: rows[0].2,
        projection: projections(&mut conn, bot.character_guid)?,
        disconnect: flag("WOW_BOT_LOGIN_DISCONNECT_CHECK"),
        portal: portal::enabled(),
    })
}

/// Protocol termination belongs to this scenario, not the bot's main dispatcher.
/// Disconnect sends FIN on both authenticated transports without a logout request.
pub(super) async fn complete(
    bot_index: usize,
    bot: &config::BotConfig,
    before: Before,
    known: LoginKnownSpellsLikeCpp,
    stream: &mut TcpStream,
    crypt: &mut WorldCrypt,
    inflater: &mut ServerPacketInflater,
    realm: &mut Option<EncryptedWorldConnection>,
    result: &mut BotRunResult,
) -> Result<Evidence> {
    let pending_portal = if before.portal {
        Some(portal::begin(stream, crypt, inflater, realm, result).await?)
    } else {
        None
    };
    if before.disconnect {
        let mut realm = realm
            .take()
            .context("disconnect requires both authenticated transports")?;
        let (instance_result, realm_result) =
            tokio::join!(stream.shutdown(), realm.stream.shutdown());
        instance_result.context("instance transport shutdown failed")?;
        realm_result.context("realm transport shutdown failed")?;
    } else {
        let confirmed = loot_race::logout_and_wait_routed_like_cpp(
            bot_index,
            stream,
            crypt,
            inflater,
            realm.as_mut(),
            bot.character_guid,
            result,
        )
        .await?;
        if !confirmed {
            bail!("save check requires SMSG_LOGOUT_COMPLETE, not socket-loss fallback");
        }
    }
    let selected = bot.clone();
    tokio::task::spawn_blocking(move || finish(&selected, before, known, pending_portal)).await?
}

fn finish(
    bot: &config::BotConfig,
    before: Before,
    known: LoginKnownSpellsLikeCpp,
    mut pending_portal: Option<portal::Receipt>,
) -> Result<Evidence> {
    let mut conn = connect(characters_db_url()?)?;
    if before.disconnect {
        let mut auth = connect(auth_db_url()?)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(90);
        loop {
            let row: Option<(u8, u64)> = conn.exec_first(
                "SELECT online, logout_time FROM characters WHERE guid = ? AND account = ?",
                (bot.character_guid, bot.account_id),
            )?;
            let account_online: Option<u8> =
                auth.exec_first("SELECT online FROM account WHERE id = ?", (bot.account_id,))?;
            if row.is_some_and(|(online, stamp)| {
                offline_save_observed(online, stamp, before.logout_time)
            }) && account_online == Some(0)
            {
                break;
            }
            if std::time::Instant::now() >= deadline {
                bail!("transport disconnect did not confirm character save and account offline within 90s");
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    let (online, logout_time, saved_map, x, y, z): (u8, u64, u32, f32, f32, f32) = conn
        .exec_first(
            "SELECT online, logout_time, map, position_x, position_y, position_z FROM characters WHERE guid = ? AND account = ?",
            (bot.character_guid, bot.account_id),
        )?
        .context("saved character disappeared")?;
    if !offline_save_observed(online, logout_time, before.logout_time) {
        bail!("session termination did not produce a new offline save marker");
    }
    if let Some(receipt) = &mut pending_portal {
        portal::verify_saved(&mut conn, bot, receipt)?;
    }
    let after = projections(&mut conn, bot.character_guid)?;
    for (table, saved) in &before.projection {
        let current = after
            .get(table)
            .context("save projection family disappeared")?;
        if !retains_existing(saved, current) {
            bail!("login/save changed or removed a pre-existing {table} row; inspect before accepting QA");
        }
    }
    Ok(Evidence {
        logout_confirmed: !before.disconnect,
        disconnect_confirmed: before.disconnect,
        login_account_offline: before.disconnect.then_some(true),
        pending_portal,
        saved_map,
        saved_position: [x, y, z],
        offline: true,
        retained_existing_rows: true,
        logout_time_before: before.logout_time,
        logout_time_after: logout_time,
        before: before.projection,
        after,
        known_spells: known.known_spells,
        favorite_spells: known.favorite_spells,
    })
}

fn offline_save_observed(online: u8, after: u64, before: u64) -> bool {
    online == 0 && after > before
}

fn retains_existing(before: &Projection, after: &Projection) -> bool {
    before
        .row_hashes
        .iter()
        .all(|row| after.row_hashes.contains(row))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnect_requires_new_save_and_offline_not_just_socket_shutdown() {
        assert!(offline_save_observed(0, 11, 10));
        assert!(!offline_save_observed(1, 11, 10));
        assert!(!offline_save_observed(0, 10, 10));
        assert!(!offline_save_observed(0, 9, 10));
    }

    fn projection(rows: &[&str]) -> Projection {
        Projection {
            sha256: String::new(),
            rows: rows.len(),
            row_hashes: rows.iter().map(|row| row.to_string()).collect(),
        }
    }

    #[test]
    fn login_defaults_may_be_added_but_existing_rows_must_survive() {
        assert!(retains_existing(
            &projection(&["a"]),
            &projection(&["a", "b"])
        ));
        assert!(retains_existing(&projection(&[]), &projection(&["a"])));
        assert!(!retains_existing(&projection(&["a"]), &projection(&[])));
        assert!(!retains_existing(
            &projection(&["a"]),
            &projection(&["changed"])
        ));
    }
}
