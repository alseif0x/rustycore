use super::*;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

fn snapshot() -> QuestCharacterSnapshot {
    QuestCharacterSnapshot {
        race: 1,
        class: 1,
        level: 10,
        map_id: 0,
        zone_id: 12,
        instance_id: 0,
        x: -8949.95,
        y: -132.493,
        z: 83.5312,
        orientation: 0.0,
        quest_status_present: Some(false),
    }
}

fn counting_restore(
    calls: &Arc<AtomicUsize>,
    outcome: fn() -> Result<()>,
) -> impl FnOnce(QuestCharacterSnapshot) -> std::future::Ready<Result<()>> {
    let calls = Arc::clone(calls);
    move |restored| {
        assert_eq!(restored, snapshot());
        calls.fetch_add(1, Ordering::SeqCst);
        std::future::ready(outcome())
    }
}

/// An early `bail!` (world auth, enum chars, login verify) never reaches the
/// quest branch; the guard must still restore and keep the original error.
#[tokio::test]
async fn early_error_still_restores_and_keeps_the_original_error() {
    let calls = Arc::new(AtomicUsize::new(0));
    let outcome: Result<BotRunResult> = Err(anyhow!("Did not receive SMSG_ENUM_CHARACTERS_RESULT"));
    let error = finish_quest_run_with_restore(
        1,
        outcome,
        Some(snapshot()),
        counting_restore(&calls, || Ok(())),
    )
    .await
    .unwrap_err();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        error.to_string(),
        "Did not receive SMSG_ENUM_CHARACTERS_RESULT"
    );
}

#[tokio::test]
async fn early_error_with_failed_restore_reports_both() {
    let calls = Arc::new(AtomicUsize::new(0));
    let outcome: Result<BotRunResult> = Err(anyhow!("Login verification failed"));
    let error = finish_quest_run_with_restore(
        1,
        outcome,
        Some(snapshot()),
        counting_restore(&calls, || Err(anyhow!("character 1 remained online"))),
    )
    .await
    .unwrap_err();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(error.root_cause().to_string(), "Login verification failed");
    let rendered = format!("{error:#}");
    assert!(
        rendered.contains("character 1 remained online"),
        "{rendered}"
    );
    assert!(rendered.contains("Login verification failed"), "{rendered}");
}

/// A branch that returns `Ok` before the quest phase is restored too, and a
/// failed restore is surfaced and fails the quest verdict.
#[tokio::test]
async fn ok_without_in_band_restore_is_restored_and_failure_fails_the_run() {
    let calls = Arc::new(AtomicUsize::new(0));
    let mut result = BotRunResult {
        quest_smoke: true,
        world_auth: true,
        enum_characters: true,
        player_login_verified: true,
        quest_smoke_passed: Some(true),
        ..Default::default()
    };
    assert!(result.success(false, false, false));
    result = finish_quest_run_with_restore(
        1,
        Ok(result),
        Some(snapshot()),
        counting_restore(&calls, || Err(anyhow!("restore broke"))),
    )
    .await
    .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(result.quest_character_restored, Some(false));
    assert_eq!(
        result.quest_restore_failure.as_deref(),
        Some("restore broke")
    );
    assert!(!result.success(false, false, false));
}

#[tokio::test]
async fn in_band_restore_and_no_snapshot_do_not_restore_again() {
    let calls = Arc::new(AtomicUsize::new(0));
    let restored = BotRunResult {
        quest_character_restored: Some(true),
        ..Default::default()
    };
    let result = finish_quest_run_with_restore(
        1,
        Ok(restored),
        Some(snapshot()),
        counting_restore(&calls, || Ok(())),
    )
    .await
    .unwrap();
    assert_eq!(result.quest_character_restored, Some(true));
    let error = finish_quest_run_with_restore(
        1,
        Err(anyhow!("no setup")),
        None,
        counting_restore(&calls, || Ok(())),
    )
    .await
    .unwrap_err();
    assert_eq!(error.to_string(), "no setup");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}
