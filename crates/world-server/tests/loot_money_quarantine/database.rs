use anyhow::{Context, Result, ensure};
use std::{
    env,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::Notify;
use wow_database::{
    CharacterDatabase, database::build_connection_string_with_ssl_like_cpp,
    group_loot_money_adapter::MariaDbGroupLootMoneyPersistenceAdapterLikeCpp,
};
use wow_persistence::{
    GroupLootMoneyPersistenceAttemptLikeCpp as Attempt,
    GroupLootMoneyPersistenceOutcomeLikeCpp as Outcome,
    GroupLootMoneyPersistencePortLikeCpp as Port,
    GroupLootMoneyPersistenceRequestLikeCpp as Request,
    GroupLootMoneyReconciliationLikeCpp as Reconciliation, PersistenceFutureLikeCpp,
};

pub(super) const RECIPIENT: u64 = 54;
pub(super) const BEFORE: u64 = 10;
pub(super) const COINS: u64 = 7;

pub(super) struct Database {
    admin: CharacterDatabase,
    name: String,
    pub db: Arc<CharacterDatabase>,
}

impl Database {
    pub async fn create(rollback: bool) -> Result<Self> {
        let socket = env::var("RUSTYCORE_LOOT_TEST_SOCKET")
            .context("supply an isolated MariaDB Unix socket")?;
        ensure!(
            std::path::Path::new(&socket).is_absolute(),
            "expected an absolute socket path"
        );
        let user = env::var("USER")?;
        let url = |schema: &str| {
            build_connection_string_with_ssl_like_cpp(
                "localhost",
                &socket,
                &user,
                "",
                schema,
                false,
            )
        };
        let admin = CharacterDatabase::open(&url("mysql")).await?;
        let name = format!(
            "rc_quarantine_{}_{}",
            std::process::id(),
            u8::from(rollback)
        );
        admin
            .direct_execute(&format!("CREATE DATABASE `{name}`"))
            .await?;
        let db = Arc::new(CharacterDatabase::open(&url(&name)).await?);
        db.direct_execute("CREATE TABLE characters (guid BIGINT UNSIGNED PRIMARY KEY, money BIGINT UNSIGNED NOT NULL) ENGINE=InnoDB").await?;
        db.direct_execute("INSERT INTO characters (guid, money) VALUES (54, 10)")
            .await?;
        let ddl = include_str!(
            "../../../../sql/updates/characters/wotlk_classic/2026_11_22_00_characters.sql"
        )
        .lines()
        .filter(|line| !line.trim_start().starts_with("--"))
        .collect::<Vec<_>>()
        .join("\n");
        for statement in ddl.split(';').filter(|s| !s.trim().is_empty()) {
            db.direct_execute(statement).await?;
        }
        if rollback {
            // The adapter has already updated the character row when it writes the receipt.
            db.direct_execute("CREATE TRIGGER reject_receipt BEFORE UPDATE ON loot_money_claim_receipt FOR EACH ROW SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT = 'injected receipt rollback'").await?;
        }
        Ok(Self { admin, name, db })
    }

    pub async fn restore_receipt(&self, rollback: bool) -> Result<()> {
        self.db
            .direct_execute("RENAME TABLE held_receipt TO loot_money_claim_receipt")
            .await?;
        if rollback {
            self.db
                .direct_execute("DROP TRIGGER reject_receipt")
                .await?;
        }
        Ok(())
    }

    pub async fn counts(&self) -> Result<(u64, u64)> {
        let rows = self.db.direct_query("SELECT money, CAST((SELECT COUNT(*) FROM loot_money_claim_receipt) AS UNSIGNED) FROM characters WHERE guid=54").await?;
        ensure!(rows.count() == 1);
        Ok((rows.read(0), rows.read(1)))
    }

    pub async fn finish(self) -> Result<()> {
        self.db.pool().close().await;
        self.admin
            .direct_execute(&format!("DROP DATABASE `{}`", self.name))
            .await?;
        Ok(())
    }
}

/// Only the attempt's acknowledgement is replaced. Every reconciliation goes
/// to the production writer. Renaming its table makes the first read fail in SQL.
pub(super) struct LostAcknowledgement {
    db: Arc<CharacterDatabase>,
    adapter: MariaDbGroupLootMoneyPersistenceAdapterLikeCpp,
    rollback: bool,
    pub started: Notify,
    pub resume: Notify,
    pub settled: Notify,
    pub attempts: AtomicUsize,
    pub indeterminate_reads: AtomicUsize,
    pub settled_reads: AtomicUsize,
}

impl LostAcknowledgement {
    pub fn new(db: Arc<CharacterDatabase>, rollback: bool) -> Self {
        Self {
            adapter: MariaDbGroupLootMoneyPersistenceAdapterLikeCpp::new(Arc::clone(&db)),
            db,
            rollback,
            started: Notify::new(),
            resume: Notify::new(),
            settled: Notify::new(),
            attempts: 0.into(),
            indeterminate_reads: 0.into(),
            settled_reads: 0.into(),
        }
    }
}

impl Port for LostAcknowledgement {
    fn attempt_group_loot_money_like_cpp(
        &self,
        request: Request,
    ) -> PersistenceFutureLikeCpp<'_, Attempt> {
        Box::pin(async move {
            assert_eq!(
                self.attempts.fetch_add(1, Ordering::SeqCst),
                0,
                "recovery submitted a second payout"
            );
            self.started.notify_one();
            self.resume.notified().await;
            let outcomes = vec![Outcome {
                recipient_guid: RECIPIENT,
                before: BEFORE,
                after: BEFORE + COINS,
                applied_delta: COINS,
                criteria: None,
            }];
            match self
                .adapter
                .attempt_group_loot_money_like_cpp(request)
                .await
            {
                Attempt::Applied(actual) if !self.rollback => assert_eq!(actual, outcomes),
                Attempt::DefinitelyRolledBack { reason, .. } if self.rollback => {
                    assert!(reason.contains("injected receipt rollback"), "{reason}")
                }
                other => panic!("unexpected writer outcome {other:?}"),
            }
            self.db
                .direct_execute("RENAME TABLE loot_money_claim_receipt TO held_receipt")
                .await
                .unwrap();
            Attempt::CommitOutcomeUnknown {
                reason: "fixture dropped the writer acknowledgement".into(),
                outcomes,
            }
        })
    }

    fn reconcile_group_loot_money_like_cpp(
        &self,
        id: Option<[u8; 16]>,
        outcomes: Vec<Outcome>,
    ) -> PersistenceFutureLikeCpp<'_, Reconciliation> {
        Box::pin(async move {
            let result = self
                .adapter
                .reconcile_group_loot_money_like_cpp(id, outcomes)
                .await;
            match result {
                Reconciliation::Indeterminate { .. } => {
                    self.indeterminate_reads.fetch_add(1, Ordering::SeqCst);
                }
                _ => {
                    self.settled_reads.fetch_add(1, Ordering::SeqCst);
                    self.settled.notify_one();
                }
            }
            result
        })
    }
}
