//! Loot operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) fn install_loot_termination_token() -> Result<CancellationToken> {
    let token = CancellationToken::new();
    let signal_token = token.clone();
    #[cfg(unix)]
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())
        .context("Install loot-fixture SIGINT handler before mutation")?;
    #[cfg(unix)]
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .context("Install loot-fixture SIGTERM handler before mutation")?;
    if std::env::var("WOW_BOT_REQUIRE_PARENT_DEATH_GUARD").is_ok_and(|value| is_truthy(&value)) {
        #[cfg(target_os = "linux")]
        {
            // SAFETY: getppid takes no pointers and has no preconditions.
            let parent_before = unsafe { libc::getppid() };
            if parent_before <= 1 {
                bail!("loot parent-death guard has no live supervising parent");
            }
            // SAFETY: PR_SET_PDEATHSIG accepts an integer signal number. SIGTERM
            // is handled by the streams registered synchronously above.
            let result = unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) };
            if result != 0 {
                return Err(std::io::Error::last_os_error())
                    .context("Arm Linux parent-death SIGTERM before fixture mutation");
            }
            // Close the documented prctl race: if the supervisor disappeared
            // before PR_SET_PDEATHSIG was armed, refuse to enter the fixture.
            // SAFETY: getppid takes no pointers and has no preconditions.
            let parent_after = unsafe { libc::getppid() };
            if parent_after != parent_before {
                bail!("loot supervisor changed while arming parent-death guard");
            }
        }
        #[cfg(not(target_os = "linux"))]
        bail!("WOW_BOT_REQUIRE_PARENT_DEATH_GUARD is supported only on Linux");
    }
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            tokio::select! {
                _ = interrupt.recv() => {}
                _ = terminate.recv() => {}
            }
        }
        #[cfg(not(unix))]
        if let Err(error) = tokio::signal::ctrl_c().await {
            error!("Termination handler failed: {error}");
        }
        signal_token.cancel();
    });
    Ok(token)
}
pub(crate) async fn finish_guarded_loot_result(
    result: Result<Vec<BotRunResult>>,
) -> Result<Vec<BotRunResult>> {
    match result {
        Ok(results) => Ok(results),
        Err(primary) => match loot_race::recover_pending_fixture_if_present().await {
            Ok(true) => Err(primary.context(
                "loot workflow failed; the durable fixture journal was recovered before exit",
            )),
            Ok(false) => Err(primary),
            Err(recovery) => bail!(
                "loot workflow failed ({primary:#}) and durable fixture recovery also failed ({recovery:#}); leave the normal world stopped and run --recover-loot-fixture"
            ),
        },
    }
}
pub(crate) fn validate_exact_loot_bot_identities(bots: &[config::BotConfig]) -> Result<()> {
    let auth_opts = qa_mysql_opts(&auth_db_url()?, "auth")?;
    let char_opts = qa_mysql_opts(&characters_db_url()?, "characters")?;
    let mut auth_conn =
        mysql::Conn::new(auth_opts).map_err(|e| anyhow!("Connect to auth DB failed: {e}"))?;
    let mut character_conn =
        mysql::Conn::new(char_opts).map_err(|e| anyhow!("Connect to characters DB failed: {e}"))?;
    for bot in bots {
        validate_exact_bot_identity(&mut auth_conn, Some(&mut character_conn), bot)?;
    }
    Ok(())
}
