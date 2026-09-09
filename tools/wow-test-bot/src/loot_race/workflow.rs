//! Loot-race workflow operations.
//!
//! Moved out of loot_race.rs under #634. Behaviour is preserved.

use super::*;

pub(crate) async fn run_workflow(
    mut bots: Vec<config::BotConfig>,
    cli: LootRaceCli,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    shutdown: CancellationToken,
) -> Result<Vec<BotRunResult>> {
    let workflow_deadline =
        tokio::time::Instant::now() + Duration::from_secs(cli.workflow_deadline_secs);
    if shutdown.is_cancelled() {
        bail!("loot-race cancelled before fixture setup");
    }
    bots.sort_by_key(|bot| {
        if bot.account.eq_ignore_ascii_case(&cli.account_a) {
            0
        } else {
            1
        }
    });
    let setup_bots = bots.clone();
    let setup_cli = cli.clone();
    let setup_shutdown = shutdown.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_fixture(
            &setup_bots,
            &setup_cli,
            LootFixturePurpose::Race,
            &setup_shutdown,
        )
    })
    .await
    .map_err(|error| anyhow!("loot-race fixture DB worker join failed: {error}"))??;

    let sync = Arc::new(LootRaceSync::new());
    let mut handles = tokio::task::JoinSet::new();
    for participant in 0..2 {
        let bot = bots[participant].clone();
        let options = LootRaceOptions {
            phase: LootRacePhase::Race,
            participant,
            character_guid: fixture.characters[participant].bot.character_guid,
            peer_name: fixture.characters[1 - participant].name.clone(),
            peer_character_guid: fixture.characters[1 - participant].bot.character_guid,
            killer_character_guid: fixture.characters[0].bot.character_guid,
            target: fixture.target.clone(),
            timeout_secs: cli.timeout_secs,
            sync: Arc::clone(&sync),
        };
        let task_sync = Arc::clone(&sync);
        handles.spawn(async move {
            let run = run_bot(
                bot,
                dungeon_id,
                lfg_secs,
                auto_teleport,
                false,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(options),
                None,
                None,
                None,
            )
            .await;
            match &run {
                Err(error) => task_sync.cancel(format!(
                    "participant {participant} transport/login failed: {error:#}"
                )),
                Ok(result) if result.loot_race_smoke_passed == Some(false) => {
                    task_sync.cancel(format!(
                        "participant {participant} failed: {}",
                        result
                            .loot_race_failure
                            .as_deref()
                            .unwrap_or("unknown loot-race failure")
                    ));
                }
                _ => {}
            }
            run
        });
    }

    let mut results = Vec::with_capacity(2);
    let mut task_error = None;
    loop {
        let joined = tokio::select! {
            joined = handles.join_next() => joined,
            _ = shutdown.cancelled() => {
                let message = "loot-race received SIGINT/SIGTERM".to_string();
                sync.cancel(message.clone());
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                task_error = Some(message);
                break;
            }
            _ = tokio::time::sleep_until(workflow_deadline) => {
                let message = format!(
                    "loot-race exceeded the {}s end-to-end deadline",
                    cli.workflow_deadline_secs
                );
                sync.cancel(message.clone());
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                task_error = Some(message);
                break;
            }
        };
        let Some(joined) = joined else { break };
        match joined {
            Ok(Ok(result)) => results.push(result),
            Ok(Err(error)) => {
                let message = error.to_string();
                sync.cancel(message.clone());
                handles.abort_all();
                task_error.get_or_insert(message);
            }
            Err(error) => {
                let message = format!("loot-race task join failed: {error}");
                sync.cancel(message.clone());
                handles.abort_all();
                task_error.get_or_insert(message);
            }
        }
    }
    results.sort_by_key(|result| result.account_id);
    if shutdown.is_cancelled() {
        task_error.get_or_insert_with(|| "loot-race received SIGINT/SIGTERM".to_string());
    } else if tokio::time::Instant::now() >= workflow_deadline {
        task_error.get_or_insert_with(|| {
            format!(
                "loot-race exceeded the {}s end-to-end deadline",
                cli.workflow_deadline_secs
            )
        });
    }

    if task_error.is_none()
        && results.len() == 2
        && results
            .iter()
            .all(|result| result.loot_race_smoke_passed == Some(true))
    {
        let expected_source_coins = results[0]
            .loot_race_loot_coins
            .map(u64::from)
            .ok_or_else(|| anyhow!("loot-race passed without recording source money"));
        let expected_item_grant = expected_persisted_item_grant(&fixture, &sync).await;
        let expected_money_grant = match expected_source_coins.as_ref() {
            Ok(source_coins) => {
                expected_persisted_money_grant(&fixture, &sync, *source_coins).await
            }
            Err(error) => Err(anyhow!(error.to_string())),
        };
        let verification = match (
            expected_source_coins,
            expected_item_grant,
            expected_money_grant,
        ) {
            (Ok(expected_source_coins), Ok(expected_item_grant), Ok(expected_money_grant)) => {
                if expected_source_coins != expected_money_grant.amount {
                    Err(anyhow!(
                        "wire money winner amount {} did not consume exact source pool {expected_source_coins}",
                        expected_money_grant.amount
                    ))
                } else {
                    let fixture_for_db = fixture.clone();
                    match tokio::task::spawn_blocking(move || {
                        verify_persisted_grants(
                            &fixture_for_db,
                            expected_money_grant,
                            expected_item_grant,
                        )
                    })
                    .await
                    {
                        Ok(verification) => verification,
                        Err(error) => Err(anyhow!(
                            "loot-race verification DB worker join failed: {error}"
                        )),
                    }
                }
            }
            (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => Err(error),
        };
        match verification {
            Ok((item_total, money_delta, item_grant, money_grant)) => {
                for result in &mut results {
                    result.loot_race_db_item_total = Some(item_total);
                    result.loot_race_db_money_delta = Some(money_delta);
                }
                let mut relog_ok = true;
                for participant in 0..2 {
                    let options = LootRaceOptions {
                        phase: LootRacePhase::VerifyRelog,
                        participant,
                        character_guid: fixture.characters[participant].bot.character_guid,
                        peer_name: fixture.characters[1 - participant].name.clone(),
                        peer_character_guid: fixture.characters[1 - participant].bot.character_guid,
                        killer_character_guid: fixture.characters[0].bot.character_guid,
                        target: fixture.target.clone(),
                        timeout_secs: cli.timeout_secs,
                        sync: Arc::clone(&sync),
                    };
                    let relog = tokio::select! {
                        _ = shutdown.cancelled() => {
                            Err(anyhow!("loot-race relog cancelled by SIGINT/SIGTERM"))
                        }
                        result = tokio::time::timeout_at(workflow_deadline, run_bot(
                            bots[participant].clone(),
                            dungeon_id,
                            lfg_secs,
                            auto_teleport,
                            false,
                            None,
                            None,
                            None,
                            None,
                            None,
                            None,
                            Some(options),
                            None,
                            None,
                            None,
                        )) => match result {
                            Ok(run) => run,
                            Err(_) => Err(anyhow!(
                                "loot-race relog exceeded the {}s end-to-end deadline",
                                cli.workflow_deadline_secs
                            )),
                        },
                    };
                    match relog {
                        Ok(relog) if relog.loot_race_relog_verified => {
                            if let Some(result) = results
                                .iter_mut()
                                .find(|result| result.account_id == bots[participant].account_id)
                            {
                                result.loot_race_relog_verified = true;
                            }
                        }
                        Ok(_) | Err(_) => relog_ok = false,
                    }
                }
                let fixture_for_relog = fixture.clone();
                let after_relog = match tokio::task::spawn_blocking(move || {
                    verify_persisted_grants(&fixture_for_relog, money_grant, item_grant)
                })
                .await
                {
                    Ok(verification) => verification,
                    Err(error) => Err(anyhow!("loot-race relog DB worker join failed: {error}")),
                };
                if !matches!(
                    after_relog,
                    Ok(persisted) if persisted == (item_total, money_delta, item_grant, money_grant)
                ) {
                    relog_ok = false;
                }
                for result in &mut results {
                    result.loot_race_smoke_passed = Some(
                        result.loot_race_smoke_passed.unwrap_or(false)
                            && relog_ok
                            && result.loot_race_relog_verified,
                    );
                    if !result.loot_race_smoke_passed.unwrap_or(false)
                        && result.loot_race_failure.is_none()
                    {
                        result.loot_race_failure = Some(
                            "loot grants did not survive a clean logout/relogin unchanged".into(),
                        );
                    }
                }
            }
            Err(error) => {
                for result in &mut results {
                    result.loot_race_smoke_passed = Some(false);
                    result.loot_race_failure = Some(error.to_string());
                }
            }
        }
    }

    if let Some(ref error) = task_error {
        for result in &mut results {
            result.loot_race_smoke_passed = Some(false);
            result.loot_race_failure.get_or_insert(error.clone());
        }
    }

    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || cleanup_fixture(&fixture_for_cleanup))
        .await
        .map_err(|error| anyhow!("loot-race cleanup DB worker join failed: {error}"))?;
    if let Err(error) = cleanup {
        bail!("loot-race fixture cleanup failed: {error:#}");
    }
    if results.len() != 2 {
        bail!(
            "loot-race produced {} results instead of 2{}",
            results.len(),
            task_error
                .as_deref()
                .map(|error| format!(": {error}"))
                .unwrap_or_default()
        );
    }
    Ok(results)
}
/// Record one deterministic, item-only loot action with exactly one connected
/// client. The second guarded bot remains offline; it is retained only because
/// the shared disposable-fixture snapshot/cleanup predates this capture mode.
pub(crate) async fn run_single_item_capture_workflow(
    mut bots: Vec<config::BotConfig>,
    cli: LootRaceCli,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    shutdown: CancellationToken,
) -> Result<Vec<BotRunResult>> {
    let workflow_deadline =
        tokio::time::Instant::now() + Duration::from_secs(cli.workflow_deadline_secs);
    if shutdown.is_cancelled() {
        bail!("loot-item capture cancelled before fixture setup");
    }
    bots.sort_by_key(|bot| {
        if bot.account.eq_ignore_ascii_case(&cli.account_a) {
            0
        } else {
            1
        }
    });
    let setup_bots = bots.clone();
    let setup_cli = cli.clone();
    let setup_shutdown = shutdown.clone();
    let fixture = tokio::task::spawn_blocking(move || {
        prepare_fixture(
            &setup_bots,
            &setup_cli,
            LootFixturePurpose::CaptureItem,
            &setup_shutdown,
        )
    })
    .await
    .map_err(|error| anyhow!("loot-item capture fixture DB worker join failed: {error}"))??;

    let sync = Arc::new(LootRaceSync::new());
    let options = LootRaceOptions {
        phase: LootRacePhase::CaptureItem,
        participant: 0,
        character_guid: fixture.characters[0].bot.character_guid,
        peer_name: fixture.characters[1].name.clone(),
        peer_character_guid: fixture.characters[1].bot.character_guid,
        killer_character_guid: fixture.characters[0].bot.character_guid,
        target: fixture.target.clone(),
        timeout_secs: cli.timeout_secs,
        sync: Arc::clone(&sync),
    };
    let run = tokio::select! {
        _ = shutdown.cancelled() => Err(anyhow!("loot-item capture received SIGINT/SIGTERM")),
        result = tokio::time::timeout_at(workflow_deadline, run_bot(
            bots[0].clone(),
            dungeon_id,
            lfg_secs,
            auto_teleport,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(options),
            None,
            None,
            None,
        )) => match result {
            Ok(run) => run,
            Err(_) => Err(anyhow!(
                "loot-item capture exceeded the {}s end-to-end deadline",
                cli.workflow_deadline_secs
            )),
        },
    };

    let mut result = match run {
        Ok(result) => Some(result),
        Err(error) => {
            let fixture_for_cleanup = fixture.clone();
            let cleanup =
                tokio::task::spawn_blocking(move || cleanup_fixture(&fixture_for_cleanup))
                    .await
                    .map_err(|join| {
                        anyhow!("loot-item capture cleanup DB worker join failed: {join}")
                    })?;
            if let Err(cleanup_error) = cleanup {
                bail!(
                    "loot-item capture failed ({error:#}) and fixture cleanup also failed ({cleanup_error:#})"
                );
            }
            return Err(error);
        }
    };

    if result
        .as_ref()
        .is_some_and(|result| result.loot_race_smoke_passed == Some(true))
    {
        let fixture_for_verification = fixture.clone();
        match tokio::task::spawn_blocking(move || {
            verify_single_item_capture_persistence(&fixture_for_verification)
        })
        .await
        {
            Ok(Ok(item_total)) => {
                let result = result
                    .as_mut()
                    .expect("successful wire result remains available");
                result.loot_race_db_item_total = Some(item_total);
                result.loot_race_db_money_delta = Some(0);
            }
            Ok(Err(error)) => {
                let result = result
                    .as_mut()
                    .expect("successful wire result remains available");
                result.loot_race_smoke_passed = Some(false);
                result.loot_race_failure = Some(error.to_string());
            }
            Err(error) => {
                let result = result
                    .as_mut()
                    .expect("successful wire result remains available");
                result.loot_race_smoke_passed = Some(false);
                result.loot_race_failure = Some(format!(
                    "loot-item capture verification DB worker join failed: {error}"
                ));
            }
        }
    }

    if result
        .as_ref()
        .is_some_and(|result| result.loot_race_smoke_passed == Some(true))
    {
        let relog_options = LootRaceOptions {
            phase: LootRacePhase::VerifyRelog,
            participant: 0,
            character_guid: fixture.characters[0].bot.character_guid,
            peer_name: fixture.characters[1].name.clone(),
            peer_character_guid: fixture.characters[1].bot.character_guid,
            killer_character_guid: fixture.characters[0].bot.character_guid,
            target: fixture.target.clone(),
            timeout_secs: cli.timeout_secs,
            sync,
        };
        let relog = tokio::select! {
            _ = shutdown.cancelled() => {
                Err(anyhow!("loot-item capture relog cancelled by SIGINT/SIGTERM"))
            }
            relog = tokio::time::timeout_at(workflow_deadline, run_bot(
                bots[0].clone(),
                dungeon_id,
                lfg_secs,
                auto_teleport,
                false,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(relog_options),
                None,
                None,
                None,
            )) => match relog {
                Ok(relog) => relog,
                Err(_) => Err(anyhow!(
                    "loot-item capture relog exceeded the {}s end-to-end deadline",
                    cli.workflow_deadline_secs
                )),
            },
        };
        match relog {
            Ok(relog) if relog.loot_race_relog_verified => {
                let fixture_for_verification = fixture.clone();
                let persisted_after_relog = tokio::task::spawn_blocking(move || {
                    verify_single_item_capture_persistence(&fixture_for_verification)
                })
                .await;
                let result = result
                    .as_mut()
                    .expect("successful capture result remains available for relog merge");
                result.world_auth &= relog.world_auth;
                result.enum_characters &= relog.enum_characters;
                result.player_login_verified &= relog.player_login_verified;
                result.seen_opcodes.extend(relog.seen_opcodes);
                match persisted_after_relog {
                    Ok(Ok(item_total)) if Some(item_total) == result.loot_race_db_item_total => {
                        result.loot_race_relog_verified = true;
                    }
                    Ok(Ok(item_total)) => {
                        result.loot_race_smoke_passed = Some(false);
                        result.loot_race_failure = Some(format!(
                            "loot-item capture persisted item total changed across relog: before {:?}, after {item_total}",
                            result.loot_race_db_item_total
                        ));
                    }
                    Ok(Err(error)) => {
                        result.loot_race_smoke_passed = Some(false);
                        result.loot_race_failure = Some(format!(
                            "loot-item capture post-relog persistence verification failed: {error}"
                        ));
                    }
                    Err(error) => {
                        result.loot_race_smoke_passed = Some(false);
                        result.loot_race_failure = Some(format!(
                            "loot-item capture post-relog DB worker join failed: {error}"
                        ));
                    }
                }
            }
            Ok(_) => {
                let result = result
                    .as_mut()
                    .expect("successful capture result remains available for relog failure");
                result.loot_race_smoke_passed = Some(false);
                result.loot_race_failure = Some(
                    "loot-item capture relog did not verify a clean logout/login cycle".into(),
                );
            }
            Err(error) => {
                let result = result
                    .as_mut()
                    .expect("successful capture result remains available for relog error");
                result.loot_race_smoke_passed = Some(false);
                result.loot_race_failure = Some(error.to_string());
            }
        }
    }

    let deadline_exceeded_before_cleanup = tokio::time::Instant::now() >= workflow_deadline;
    let fixture_for_cleanup = fixture.clone();
    let cleanup = tokio::task::spawn_blocking(move || cleanup_fixture(&fixture_for_cleanup))
        .await
        .map_err(|error| anyhow!("loot-item capture cleanup DB worker join failed: {error}"))?;
    if let Err(error) = cleanup {
        bail!("loot-item capture fixture cleanup failed: {error:#}");
    }

    let workflow_failure = if shutdown.is_cancelled() {
        Some("loot-item capture was cancelled before end-to-end verification completed")
    } else if deadline_exceeded_before_cleanup {
        Some("loot-item capture exceeded its end-to-end deadline during final verification")
    } else {
        None
    };
    if let Some(failure) = workflow_failure {
        let result = result
            .as_mut()
            .expect("successful bot run remains available after cleanup");
        result.loot_race_smoke_passed = Some(false);
        result
            .loot_race_failure
            .get_or_insert_with(|| failure.to_owned());
    }

    Ok(vec![result.expect("successful bot run remains available")])
}
pub(crate) async fn run_group_capacity_workflow(
    mut bots: Vec<config::BotConfig>,
    cli: GroupCapacityRaceCli,
    dungeon_id: u32,
    lfg_secs: u64,
    auto_teleport: bool,
    shutdown: CancellationToken,
) -> Result<Vec<BotRunResult>> {
    let fixture = {
        let bots = bots.clone();
        let cli = cli.clone();
        tokio::task::spawn_blocking(move || load_group_capacity_fixture(&bots, &cli))
            .await
            .map_err(|error| anyhow!("group-capacity fixture preflight worker failed: {error}"))??
    };
    bots.sort_by_key(|bot| {
        if bot.account.eq_ignore_ascii_case(&cli.leader_account) {
            0
        } else if bot.account.eq_ignore_ascii_case(&cli.candidate_a_account) {
            1
        } else {
            2
        }
    });
    if bots.len() != 3 {
        bail!("group-capacity race requires exactly three selected bot accounts");
    }

    let auth_serial = Arc::new(Mutex::new(()));
    let sync = Arc::new(GroupCapacityRaceSync::new());
    let mut handles = tokio::task::JoinSet::new();
    for (index, bot) in bots.iter().cloned().enumerate() {
        let options = GroupCapacityRaceOptions {
            role: match index {
                0 => GroupCapacityRaceRole::Leader,
                1 => GroupCapacityRaceRole::CandidateA,
                _ => GroupCapacityRaceRole::CandidateB,
            },
            character_guid: bot.character_guid,
            leader_guid: fixture.leader_guid,
            candidate_names: fixture.candidate_names.clone(),
            candidate_guids: fixture.candidate_guids,
            initial_member_guids: fixture.initial_member_guids,
            party_settings: fixture.party_settings,
            group_db_store_id: cli.group_db_store_id,
            timeout_secs: cli.timeout_secs,
            auth_serial: Arc::clone(&auth_serial),
            sync: Arc::clone(&sync),
        };
        let task_sync = Arc::clone(&sync);
        handles.spawn(async move {
            let run = run_bot(
                bot,
                dungeon_id,
                lfg_secs,
                auto_teleport,
                false,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(options),
                None,
                None,
            )
            .await;
            if let Err(error) = &run {
                task_sync.cancel(format!("group-capacity transport/login failed: {error:#}"));
            }
            run
        });
    }

    let deadline = tokio::time::Instant::now()
        + Duration::from_secs(cli.timeout_secs.saturating_mul(4).saturating_add(180));
    let mut results = Vec::with_capacity(3);
    while !handles.is_empty() {
        let joined = tokio::select! {
            joined = handles.join_next() => joined,
            _ = shutdown.cancelled() => {
                sync.cancel("group-capacity race received SIGINT/SIGTERM");
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                bail!("group-capacity race received SIGINT/SIGTERM");
            }
            _ = tokio::time::sleep_until(deadline) => {
                sync.cancel("group-capacity race exceeded its end-to-end deadline");
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                bail!("group-capacity race exceeded its end-to-end deadline");
            }
        };
        match joined {
            Some(Ok(Ok(result))) => results.push(result),
            Some(Ok(Err(error))) => {
                sync.cancel(error.to_string());
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                return Err(error);
            }
            Some(Err(error)) => {
                sync.cancel(error.to_string());
                handles.abort_all();
                while handles.join_next().await.is_some() {}
                bail!("group-capacity task join failed: {error}");
            }
            None => break,
        }
    }
    results.sort_by_key(|result| result.account_id);

    let participant_failures: Vec<_> = results
        .iter()
        .filter_map(|result| {
            result
                .group_capacity_failure
                .as_deref()
                .map(|failure| format!("{}: {failure}", result.account))
        })
        .collect();
    if !participant_failures.is_empty() {
        bail!(
            "group-capacity participant failure(s): {}",
            participant_failures.join("; ")
        );
    }

    let candidate_outcomes: Vec<_> = results
        .iter()
        .filter_map(|result| result.group_capacity_outcome.as_deref())
        .filter(|outcome| *outcome != "leader-observer")
        .collect();
    if candidate_outcomes
        .iter()
        .filter(|outcome| **outcome == "added")
        .count()
        != 1
        || candidate_outcomes
            .iter()
            .filter(|outcome| **outcome == "full")
            .count()
            != 1
    {
        bail!(
            "group-capacity wire outcomes were {candidate_outcomes:?}; expected one added and one full"
        );
    }
    let wire_winner_guid = results
        .iter()
        .find(|result| result.group_capacity_outcome.as_deref() == Some("added"))
        .map(|result| result.character_guid)
        .ok_or_else(|| anyhow!("group-capacity race did not identify the wire winner"))?;
    let persistence = {
        let cli = cli.clone();
        let fixture = fixture.clone();
        tokio::task::spawn_blocking(move || verify_group_capacity_fixture(&cli, &fixture))
            .await
            .map_err(|error| anyhow!("group-capacity DB verification worker failed: {error}"))??
    };
    validate_group_capacity_winner_consistency(
        wire_winner_guid,
        persistence.winning_candidate_guid,
    )?;
    for result in &mut results {
        result.group_capacity_final_member_count = Some(persistence.final_member_count);
        result.group_capacity_race_smoke_passed = Some(true);
    }
    Ok(results)
}
