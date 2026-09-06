//! Bounded normal logout-save evidence; no fixture setup or SQL writes here.
//! C++ CharacterPackets.cpp LogoutRequest::Read (IdleLogout=false),
//! WorldSession.cpp LogoutPlayer (SaveToDB before LogoutComplete), and
//! Player.cpp SaveToDB/_SaveSpells/_SaveSkills/_SaveEquipmentSets.
use super::*;
use mysql::prelude::Queryable;
use std::collections::BTreeMap;

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
}

pub(super) fn enabled() -> bool {
    std::env::var("WOW_BOT_LOGIN_SAVE_CHECK").is_ok_and(|v| is_truthy(&v))
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
    Ok(Before {
        logout_time: rows[0].2,
        projection: projections(&mut conn, bot.character_guid)?,
    })
}

pub(super) fn finish(
    bot: &config::BotConfig,
    before: Before,
    known: LoginKnownSpellsLikeCpp,
) -> Result<Evidence> {
    let mut conn = connect(characters_db_url()?)?;
    let (online, logout_time): (u8, u64) = conn
        .exec_first(
            "SELECT online, logout_time FROM characters WHERE guid = ? AND account = ?",
            (bot.character_guid, bot.account_id),
        )?
        .context("saved character disappeared")?;
    if online != 0 || logout_time <= before.logout_time {
        bail!("normal logout did not produce a new offline save marker");
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
        logout_confirmed: true,
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

fn retains_existing(before: &Projection, after: &Projection) -> bool {
    before
        .row_hashes
        .iter()
        .all(|row| after.row_hashes.contains(row))
}

#[cfg(test)]
mod tests {
    use super::*;

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
