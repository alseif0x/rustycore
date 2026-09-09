//! Social operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn validate_linked_group_capacity_bot_identities(
    bots: &[config::BotConfig],
) -> Result<()> {
    use mysql::prelude::Queryable;

    let auth_opts = qa_mysql_opts(&auth_db_url()?, "auth")?;
    let char_opts = qa_mysql_opts(&characters_db_url()?, "characters")?;
    let mut auth_conn =
        mysql::Conn::new(auth_opts).map_err(|e| anyhow!("Connect to auth DB failed: {e}"))?;
    let mut character_conn =
        mysql::Conn::new(char_opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;

    for bot in bots {
        validate_local_bot_character_owner(&mut character_conn, bot)?;
        let expected_email = bot_srp6::utf8_to_upper_only_latin_like_cpp(&bot.account);
        let expected_username = game_account_username(&bot.account)?;
        let game_account = auth_conn
            .exec_first::<(
                String,
                String,
                String,
                Option<u32>,
                Option<u8>,
                u8,
                u32,
                u8,
                u8,
            ), _, _>(
                "SELECT username, reg_mail, email, battlenet_account, battlenet_index, expansion, \
                        failed_logins, locked, online FROM account WHERE id = ?",
                (bot.account_id,),
            )
            .map_err(|e| {
                anyhow!(
                    "Load linked group-capacity game account {}: {e}",
                    bot.account_id
                )
            })?
            .ok_or_else(|| {
                anyhow!(
                    "No linked group-capacity game account for id {}",
                    bot.account_id
                )
            })?;
        let bnet_id = game_account.3.ok_or_else(|| {
            anyhow!(
                "Group-capacity game account {} has no linked BNet identity",
                bot.account_id
            )
        })?;
        if !game_account.0.eq_ignore_ascii_case(&expected_username)
            || !game_account.1.eq_ignore_ascii_case(&expected_email)
            || !game_account.2.eq_ignore_ascii_case(&expected_email)
            || game_account.4 != Some(1)
            || game_account.5 != 9
            || game_account.6 != 0
            || game_account.7 != 0
            || game_account.8 != 0
        {
            bail!(
                "Linked group-capacity game account {} does not match configured identity/offline state",
                bot.account_id
            );
        }

        let bnet_account = auth_conn
            .exec_first::<(String, i8, Vec<u8>, Vec<u8>, u32, u8, u8), _, _>(
                "SELECT email, srp_version, salt, verifier, failed_logins, locked, online \
                 FROM battlenet_accounts WHERE id = ?",
                (bnet_id,),
            )
            .map_err(|e| anyhow!("Load linked group-capacity BNet identity {bnet_id}: {e}"))?
            .ok_or_else(|| anyhow!("No linked group-capacity BNet identity for id {bnet_id}"))?;
        // These long-lived group fixtures predate the current create-only SRP
        // provisioning helper, so their stored verifier is not reproducible by
        // `bnet_v1_verifier_for_salt_like_cpp`. Pin the exact linked identity,
        // SRP shape, offline state, and bans here. The World authentication that
        // follows proves possession of either the configured 64-byte fixture
        // key or a key derived by the live BNet fallback.
        if !bnet_account.0.eq_ignore_ascii_case(&expected_email)
            || bnet_account.1 != 1
            || bnet_account.2.len() != 32
            || bnet_account.3.len() != 128
            || bnet_account.4 != 0
            || bnet_account.5 != 0
            || bnet_account.6 != 0
        {
            bail!(
                "Linked group-capacity BNet identity {} does not match configured credentials/offline state",
                bot.account
            );
        }

        let bnet_bans: u64 = auth_conn
            .exec_first(
                "SELECT COUNT(*) FROM battlenet_account_bans WHERE id = ?",
                (bnet_id,),
            )
            .map_err(|e| anyhow!("Check linked group-capacity BNet bans: {e}"))?
            .unwrap_or(0);
        let game_bans: u64 = auth_conn
            .exec_first(
                "SELECT COUNT(*) FROM account_banned WHERE id = ? AND active <> 0",
                (bot.account_id,),
            )
            .map_err(|e| anyhow!("Check linked group-capacity game-account bans: {e}"))?
            .unwrap_or(0);
        if bnet_bans != 0 || game_bans != 0 {
            bail!(
                "Configured group-capacity bot is banned (bnet rows={bnet_bans}, active game rows={game_bans})"
            );
        }
    }
    Ok(())
}
pub(crate) fn cleanup_bot_group_state(bots: &[config::BotConfig]) -> Result<()> {
    use mysql::prelude::Queryable;

    let guids: Vec<u64> = bots.iter().map(|bot| bot.character_guid).collect();
    if guids.is_empty() {
        return Ok(());
    }

    let db_url = characters_db_url()?;
    let opts =
        mysql::Opts::from_url(&db_url).map_err(|e| anyhow!("Bad characters DB URL: {}", e))?;
    let mut conn =
        mysql::Conn::new(opts).map_err(|e| anyhow!("Connect to characters DB failed: {}", e))?;

    let placeholders = std::iter::repeat("?")
        .take(guids.len())
        .collect::<Vec<_>>()
        .join(",");
    let params = mysql::Params::Positional(guids.iter().copied().map(mysql::Value::from).collect());

    conn.exec_drop(
        format!(
            "DELETE FROM group_member WHERE memberGuid IN ({})",
            placeholders
        ),
        params.clone(),
    )
    .map_err(|e| anyhow!("DELETE group_member for bots: {}", e))?;

    conn.exec_drop(
        format!("DELETE FROM groups WHERE leaderGuid IN ({})", placeholders),
        params,
    )
    .map_err(|e| anyhow!("DELETE groups for bots: {}", e))?;

    info!(
        "Cleaned stale group rows for {} configured bot GUIDs",
        bots.len()
    );
    Ok(())
}
