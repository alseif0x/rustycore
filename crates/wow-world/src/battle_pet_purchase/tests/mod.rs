//! Battle-pet purchase tests.
//!
//! Separated from the battle_pet_purchase.rs root under #656.

use super::*;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use tokio::sync::Notify;

use super::*;

/// In-memory Character DB with the same transition guards as the
/// production SQL. Fault flags model the crash boundaries: a raw commit
/// that fails before applying (`fail_*_pre_commit`), and a raw commit
/// that applies but loses the reply (`lose_*_reply`), which must drive
/// the shared reconcile path. Blocking gates (`block_*`) pause a step
/// mid-flight so cancellation tests can abort the saga future exactly
/// before or after the durable apply.
pub(crate) struct FakeBattlePetPurchaseStoreLikeCpp {
    inner: Mutex<FakeStoreInnerLikeCpp>,
    pub(crate) fail_next_charge_pre_commit: AtomicBool,
    pub(crate) lose_next_charge_reply: AtomicBool,
    pub(crate) fail_next_compensate_pre_commit: AtomicBool,
    pub(crate) lose_next_compensate_reply: AtomicBool,
    pub(crate) fail_next_compensate_post_apply_read: AtomicBool,
    pub(crate) fail_next_mark: AtomicBool,
    pub(crate) fail_marks_remaining: AtomicUsize,
    pub(crate) block_next_charge_pre_apply: AtomicBool,
    pub(crate) block_next_charge_post_apply: AtomicBool,
    pub(crate) block_next_compensate_pre_apply: AtomicBool,
    pub(crate) block_next_compensate_post_apply: AtomicBool,
    pub(crate) gate_started: Notify,
    pub(crate) allow_gate: Notify,
    pub(crate) charge_attempts: AtomicUsize,
    pub(crate) compensate_attempts: AtomicUsize,
    money_mutations: AtomicUsize,
}

#[derive(Default)]
struct FakeStoreInnerLikeCpp {
    commands: BTreeMap<[u8; 16], BattlePetPurchaseCommandLikeCpp>,
    money: BTreeMap<u64, u64>,
}

impl FakeBattlePetPurchaseStoreLikeCpp {
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(FakeStoreInnerLikeCpp::default()),
            fail_next_charge_pre_commit: AtomicBool::new(false),
            lose_next_charge_reply: AtomicBool::new(false),
            fail_next_compensate_pre_commit: AtomicBool::new(false),
            lose_next_compensate_reply: AtomicBool::new(false),
            fail_next_compensate_post_apply_read: AtomicBool::new(false),
            fail_next_mark: AtomicBool::new(false),
            fail_marks_remaining: AtomicUsize::new(0),
            block_next_charge_pre_apply: AtomicBool::new(false),
            block_next_charge_post_apply: AtomicBool::new(false),
            block_next_compensate_pre_apply: AtomicBool::new(false),
            block_next_compensate_post_apply: AtomicBool::new(false),
            gate_started: Notify::new(),
            allow_gate: Notify::new(),
            charge_attempts: AtomicUsize::new(0),
            compensate_attempts: AtomicUsize::new(0),
            money_mutations: AtomicUsize::new(0),
        }
    }

    pub(crate) fn with_money(self, guid: u64, money: u64) -> Self {
        self.inner
            .lock()
            .expect("fake purchase store poisoned")
            .money
            .insert(guid, money);
        self
    }

    pub(crate) fn money(&self, guid: u64) -> Option<u64> {
        self.inner
            .lock()
            .expect("fake purchase store poisoned")
            .money
            .get(&guid)
            .copied()
    }

    pub(crate) fn command(&self, request_key: [u8; 16]) -> Option<BattlePetPurchaseCommandLikeCpp> {
        self.inner
            .lock()
            .expect("fake purchase store poisoned")
            .commands
            .get(&request_key)
            .cloned()
    }

    pub(crate) fn seed_command(&self, command: BattlePetPurchaseCommandLikeCpp) {
        self.inner
            .lock()
            .expect("fake purchase store poisoned")
            .commands
            .insert(command.request_key, command);
    }

    pub(crate) fn seed_money_like_cpp(&self, guid: u64, money: u64) {
        self.inner
            .lock()
            .expect("fake purchase store poisoned")
            .money
            .insert(guid, money);
    }

    pub(crate) fn remove_money_row_for_test_like_cpp(&self, guid: u64) {
        self.inner
            .lock()
            .expect("fake purchase store poisoned")
            .money
            .remove(&guid);
    }

    pub(crate) fn commands_snapshot(&self) -> Vec<BattlePetPurchaseCommandLikeCpp> {
        self.inner
            .lock()
            .expect("fake purchase store poisoned")
            .commands
            .values()
            .cloned()
            .collect()
    }

    pub(crate) fn money_mutations(&self) -> usize {
        self.money_mutations.load(Ordering::SeqCst)
    }

    fn fail_mark_now_like_cpp(&self) -> bool {
        if self.fail_next_mark.swap(false, Ordering::SeqCst) {
            return true;
        }
        let remaining = self.fail_marks_remaining.load(Ordering::SeqCst);
        remaining > 0
            && self
                .fail_marks_remaining
                .compare_exchange(remaining, remaining - 1, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok()
    }
}

pub(crate) fn test_command(request_key: [u8; 16], guid: u64) -> BattlePetPurchaseCommandLikeCpp {
    BattlePetPurchaseCommandLikeCpp {
        request_key,
        character_guid: guid,
        account_id: 7,
        trainer_id: 11,
        spell_id: 12345,
        species: 42,
        breed: 3,
        quality: 0,
        display_id: 999,
        level: 1,
        price: 250,
        money_before: 1_000,
        money_after: 750,
        status: BattlePetPurchaseStatusLikeCpp::PendingApplication,
        published: false,
        failure_reason: None,
    }
}

pub(crate) fn test_money_commit_fence_like_cpp() -> Box<dyn BattlePetPurchaseCommitFenceLikeCpp> {
    Box::new(
        PlayerMoneyCommitCancellationFenceLikeCpp::new_disarmed_like_cpp(Arc::new(
            crate::loot_persistence::DurableLootMoneyPersistenceTrackerLikeCpp::default(),
        )),
    )
}

impl BattlePetPurchaseStoreLikeCpp for FakeBattlePetPurchaseStoreLikeCpp {
    fn charge_and_insert_command<'a>(
        &'a self,
        command: BattlePetPurchaseCommandLikeCpp,
        mut cancellation_fence: Box<dyn BattlePetPurchaseCommitFenceLikeCpp>,
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseChargeOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            self.charge_attempts.fetch_add(1, Ordering::SeqCst);
            if self
                .block_next_charge_pre_apply
                .swap(false, Ordering::SeqCst)
            {
                self.gate_started.notify_one();
                self.allow_gate.notified().await;
            }
            if self
                .fail_next_charge_pre_commit
                .swap(false, Ordering::SeqCst)
            {
                return Ok(BattlePetPurchaseChargeOutcomeLikeCpp::RolledBack);
            }
            cancellation_fence.arm_like_cpp();
            let applied = {
                let mut inner = self.inner.lock().expect("fake purchase store poisoned");
                let guard_ok =
                    inner.money.get(&command.character_guid).copied() == Some(command.money_before);
                let key_free = !inner.commands.contains_key(&command.request_key);
                if guard_ok && key_free {
                    if command.money_after != command.money_before {
                        inner
                            .money
                            .insert(command.character_guid, command.money_after);
                        self.money_mutations.fetch_add(1, Ordering::SeqCst);
                    }
                    inner.commands.insert(command.request_key, command.clone());
                    true
                } else {
                    false
                }
            };
            if applied
                && self
                    .block_next_charge_post_apply
                    .swap(false, Ordering::SeqCst)
            {
                self.gate_started.notify_one();
                self.allow_gate.notified().await;
            }
            let outcome = if applied && self.lose_next_charge_reply.swap(false, Ordering::SeqCst) {
                // The commit happened; the reply was lost. The shared
                // reconcile must attribute it by the durable row.
                let row = self.command(command.request_key);
                Ok(reconcile_battle_pet_purchase_charge_like_cpp(
                    row.as_ref(),
                    &command,
                ))
            } else if applied {
                Ok(BattlePetPurchaseChargeOutcomeLikeCpp::Charged)
            } else {
                // Guard or unique-key failure: the raw transaction rolls
                // back, then reconcile-by-row decides (a same-token retry
                // finds its own earlier commit and must not charge again).
                let row = self.command(command.request_key);
                Ok(reconcile_battle_pet_purchase_charge_like_cpp(
                    row.as_ref(),
                    &command,
                ))
            };
            cancellation_fence.disarm_like_cpp();
            outcome
        })
    }

    fn load_pending_commands<'a>(
        &'a self,
        character_guid: u64,
        limit: u32,
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<Vec<BattlePetPurchaseCommandLikeCpp>, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            let inner = self.inner.lock().expect("fake purchase store poisoned");
            Ok(inner
                .commands
                .values()
                .filter(|command| {
                    command.character_guid == character_guid
                        && (matches!(
                            command.status,
                            BattlePetPurchaseStatusLikeCpp::PendingApplication
                                | BattlePetPurchaseStatusLikeCpp::CompensationPending
                        ) || (command.status == BattlePetPurchaseStatusLikeCpp::Completed
                            && !command.published))
                })
                .take(limit as usize)
                .cloned()
                .collect())
        })
    }

    fn mark_published<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            if self.fail_mark_now_like_cpp() {
                let row = self.command(request_key);
                return match row {
                    Some(row) if row.published => {
                        Ok(BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied)
                    }
                    Some(row) if row.status.is_terminal_like_cpp() => {
                        Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                            "publication mark on terminal command".to_string(),
                        ))
                    }
                    Some(_) => Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(
                        "publication mark did not commit".to_string(),
                    )),
                    None => Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                        "missing command".to_string(),
                    )),
                };
            }
            let mut inner = self.inner.lock().expect("fake purchase store poisoned");
            let Some(row) = inner.commands.get_mut(&request_key) else {
                return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                    "missing command".to_string(),
                ));
            };
            Ok(match row.status {
                BattlePetPurchaseStatusLikeCpp::PendingApplication
                | BattlePetPurchaseStatusLikeCpp::Completed
                | BattlePetPurchaseStatusLikeCpp::CompensationPending => {
                    if row.published {
                        BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied
                    } else {
                        row.published = true;
                        BattlePetPurchaseMarkOutcomeLikeCpp::Applied
                    }
                }
                _ => BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompensated,
            })
        })
    }

    fn mark_completed<'a>(
        &'a self,
        request_key: [u8; 16],
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            if self.fail_mark_now_like_cpp() {
                let row = self.command(request_key);
                return reconcile_battle_pet_purchase_mark_like_cpp(
                    row.as_ref(),
                    BattlePetPurchaseStatusLikeCpp::PendingApplication,
                    BattlePetPurchaseStatusLikeCpp::Completed,
                );
            }
            let mut inner = self.inner.lock().expect("fake purchase store poisoned");
            let Some(row) = inner.commands.get_mut(&request_key) else {
                return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                    "missing command".to_string(),
                ));
            };
            Ok(match row.status {
                BattlePetPurchaseStatusLikeCpp::PendingApplication
                | BattlePetPurchaseStatusLikeCpp::CompensationPending => {
                    row.status = BattlePetPurchaseStatusLikeCpp::Completed;
                    row.failure_reason = None;
                    BattlePetPurchaseMarkOutcomeLikeCpp::Applied
                }
                BattlePetPurchaseStatusLikeCpp::Completed => {
                    BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied
                }
                BattlePetPurchaseStatusLikeCpp::Compensated
                | BattlePetPurchaseStatusLikeCpp::TerminalFailure => {
                    BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompensated
                }
            })
        })
    }

    fn mark_compensation_pending<'a>(
        &'a self,
        request_key: [u8; 16],
        reason: &'static str,
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseMarkOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            if self.fail_mark_now_like_cpp() {
                let row = self.command(request_key);
                return reconcile_battle_pet_purchase_mark_like_cpp(
                    row.as_ref(),
                    BattlePetPurchaseStatusLikeCpp::PendingApplication,
                    BattlePetPurchaseStatusLikeCpp::CompensationPending,
                );
            }
            let mut inner = self.inner.lock().expect("fake purchase store poisoned");
            let Some(row) = inner.commands.get_mut(&request_key) else {
                return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                    "missing command".to_string(),
                ));
            };
            Ok(match row.status {
                BattlePetPurchaseStatusLikeCpp::PendingApplication => {
                    row.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
                    row.failure_reason = Some(reason.to_string());
                    BattlePetPurchaseMarkOutcomeLikeCpp::Applied
                }
                BattlePetPurchaseStatusLikeCpp::CompensationPending => {
                    BattlePetPurchaseMarkOutcomeLikeCpp::AlreadyApplied
                }
                BattlePetPurchaseStatusLikeCpp::Completed => {
                    BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompleted
                }
                BattlePetPurchaseStatusLikeCpp::Compensated
                | BattlePetPurchaseStatusLikeCpp::TerminalFailure => {
                    BattlePetPurchaseMarkOutcomeLikeCpp::ConflictedCompensated
                }
            })
        })
    }

    fn compensate<'a>(
        &'a self,
        request_key: [u8; 16],
        max_money: u64,
        mut cancellation_fence: Box<dyn BattlePetPurchaseCommitFenceLikeCpp>,
    ) -> BattlePetPurchaseFuture<
        'a,
        Result<BattlePetPurchaseCompensationOutcomeLikeCpp, BattlePetPurchaseStoreErrorLikeCpp>,
    > {
        Box::pin(async move {
            self.compensate_attempts.fetch_add(1, Ordering::SeqCst);
            let command = match self.command(request_key) {
                Some(command) => command,
                None => {
                    return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                        "missing command".to_string(),
                    ));
                }
            };
            match command.status {
                BattlePetPurchaseStatusLikeCpp::Compensated => {
                    let durable_money = self.money(command.character_guid).ok_or_else(|| {
                        BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                            "compensated command has no money row".to_string(),
                        )
                    })?;
                    return Ok(
                        BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated {
                            durable_money,
                        },
                    );
                }
                BattlePetPurchaseStatusLikeCpp::Completed => {
                    return Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::ConflictedCompleted);
                }
                BattlePetPurchaseStatusLikeCpp::CompensationPending => {}
                status => {
                    return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(format!(
                        "compensation from {status:?}"
                    )));
                }
            }
            if self
                .block_next_compensate_pre_apply
                .swap(false, Ordering::SeqCst)
            {
                self.gate_started.notify_one();
                self.allow_gate.notified().await;
            }
            if self
                .fail_next_compensate_pre_commit
                .swap(false, Ordering::SeqCst)
            {
                let character_present = self
                    .inner
                    .lock()
                    .expect("fake purchase store poisoned")
                    .money
                    .contains_key(&command.character_guid);
                return if character_present {
                    Err(BattlePetPurchaseStoreErrorLikeCpp::Retryable(
                        "injected pre-commit compensation failure".to_string(),
                    ))
                } else {
                    Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::CharacterMissing)
                };
            }
            cancellation_fence.arm_like_cpp();
            let applied = {
                let mut inner = self.inner.lock().expect("fake purchase store poisoned");
                match inner.money.get_mut(&command.character_guid) {
                    Some(money) => {
                        if command.price != 0 {
                            *money = (*money + u64::from(command.price)).min(max_money);
                            self.money_mutations.fetch_add(1, Ordering::SeqCst);
                        }
                        inner
                            .commands
                            .get_mut(&request_key)
                            .expect("seeded command")
                            .status = BattlePetPurchaseStatusLikeCpp::Compensated;
                        true
                    }
                    None => false,
                }
            };
            if !applied {
                cancellation_fence.disarm_like_cpp();
                return Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::CharacterMissing);
            }
            if self
                .block_next_compensate_post_apply
                .swap(false, Ordering::SeqCst)
            {
                self.gate_started.notify_one();
                self.allow_gate.notified().await;
            }
            if self
                .fail_next_compensate_post_apply_read
                .swap(false, Ordering::SeqCst)
            {
                return Err(BattlePetPurchaseStoreErrorLikeCpp::Indeterminate(
                    "injected durable money read failure after committed refund".to_string(),
                ));
            }
            if self
                .lose_next_compensate_reply
                .swap(false, Ordering::SeqCst)
            {
                // Reply lost after this call's own refund committed; the
                // status read is decisive and attributes the refund to
                // this call.
                let durable_money = self.money(command.character_guid).expect("money row");
                cancellation_fence.disarm_like_cpp();
                return Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated {
                    durable_money,
                });
            }
            let durable_money = self.money(command.character_guid).expect("money row");
            cancellation_fence.disarm_like_cpp();
            Ok(BattlePetPurchaseCompensationOutcomeLikeCpp::Compensated { durable_money })
        })
    }

    fn mark_terminal_failure<'a>(
        &'a self,
        request_key: [u8; 16],
        reason: &'static str,
    ) -> BattlePetPurchaseFuture<'a, Result<(), BattlePetPurchaseStoreErrorLikeCpp>> {
        Box::pin(async move {
            let mut inner = self.inner.lock().expect("fake purchase store poisoned");
            let Some(row) = inner.commands.get_mut(&request_key) else {
                return Err(BattlePetPurchaseStoreErrorLikeCpp::Terminal(
                    "missing command".to_string(),
                ));
            };
            if row.status == BattlePetPurchaseStatusLikeCpp::CompensationPending {
                row.status = BattlePetPurchaseStatusLikeCpp::TerminalFailure;
                row.failure_reason = Some(reason.to_string());
            }
            Ok(())
        })
    }
}

mod scenarios_1;
