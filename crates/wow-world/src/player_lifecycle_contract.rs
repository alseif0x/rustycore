//! Focused executable contract for the Player persistence port (#200).
//!
//! This deliberately describes lifecycle boundaries, not every statement in
//! `Player::SaveToDB`. The statement inventory is still incomplete in Rust;
//! freezing it here would turn that incompleteness into the target. C++ anchors:
//! `Player::LoadFromDB`, both `Player::SaveToDB` overloads, and
//! `WorldSession::LogoutPlayer`.

use serde::{Deserialize, Serialize};
use wow_database::persistence_trace::{CommitOutcome, ConnectionAffinity, LogicalDatabase};

const GOLDEN: &str = include_str!("../tests/fixtures/player-lifecycle-contract.json");

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ParameterClass {
    AccountId,
    PlayerGuid,
    PlayerSnapshot,
    WallClock,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "step", rename_all = "snake_case", deny_unknown_fields)]
enum Step {
    Boundary {
        name: String,
    },
    Fence {
        name: String,
    },
    Read {
        database: LogicalDatabase,
        connection: ConnectionAffinity,
        name: String,
        params: Vec<ParameterClass>,
    },
    TransactionBegin {
        database: LogicalDatabase,
    },
    WriteGroup {
        database: LogicalDatabase,
        connection: ConnectionAffinity,
        name: String,
        params: Vec<ParameterClass>,
    },
    Commit {
        database: LogicalDatabase,
        outcome: CommitOutcome,
    },
    Rollback {
        database: LogicalDatabase,
    },
    Publication {
        name: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    name: String,
    steps: Vec<Step>,
}

fn boundary(name: &str) -> Step {
    Step::Boundary { name: name.into() }
}

fn fence(name: &str) -> Step {
    Step::Fence { name: name.into() }
}

fn publication(name: &str) -> Step {
    Step::Publication { name: name.into() }
}

fn player_save_steps(outcome: CommitOutcome) -> Vec<Step> {
    let mut steps = vec![
        fence("player.save.mutations_closed"),
        fence("player.save.pending_durable_work_drained"),
        Step::TransactionBegin {
            database: LogicalDatabase::Character,
        },
        Step::WriteGroup {
            database: LogicalDatabase::Character,
            connection: ConnectionAffinity::Transaction,
            name: "player.save.represented_snapshot".into(),
            params: vec![
                ParameterClass::PlayerGuid,
                ParameterClass::PlayerSnapshot,
                ParameterClass::WallClock,
            ],
        },
    ];

    match outcome {
        CommitOutcome::Committed => {
            steps.push(Step::Commit {
                database: LogicalDatabase::Character,
                outcome,
            });
            steps.push(publication("player.save.dirty_state_clean"));
        }
        CommitOutcome::RolledBack => steps.push(Step::Rollback {
            database: LogicalDatabase::Character,
        }),
        CommitOutcome::Unknown => {
            steps.push(Step::Commit {
                database: LogicalDatabase::Character,
                outcome,
            });
            steps.push(fence("player.save.relogin_required"));
        }
    }
    steps
}

fn scenarios() -> Vec<Scenario> {
    let mut periodic = vec![boundary("player.save.periodic")];
    periodic.extend(player_save_steps(CommitOutcome::Committed));

    let mut manual_rollback = vec![boundary("player.save.manual")];
    manual_rollback.extend(player_save_steps(CommitOutcome::RolledBack));

    let mut manual_unknown = vec![boundary("player.save.manual")];
    manual_unknown.extend(player_save_steps(CommitOutcome::Unknown));

    let mut logout = vec![
        boundary("player.logout.started"),
        fence("player.logout.loot_idle"),
    ];
    logout.extend(player_save_steps(CommitOutcome::Committed));
    logout.extend([
        Step::TransactionBegin {
            database: LogicalDatabase::Login,
        },
        Step::WriteGroup {
            database: LogicalDatabase::Login,
            connection: ConnectionAffinity::Transaction,
            name: "player.logout.account_collections".into(),
            params: vec![ParameterClass::AccountId],
        },
        Step::Commit {
            database: LogicalDatabase::Login,
            outcome: CommitOutcome::Committed,
        },
        Step::WriteGroup {
            database: LogicalDatabase::Character,
            connection: ConnectionAffinity::Pooled,
            name: "player.logout.offline_state".into(),
            params: vec![ParameterClass::PlayerGuid, ParameterClass::AccountId],
        },
        publication("player.logout.runtime_removed"),
        publication("player.logout.offline_visible"),
        boundary("player.logout.login_claim_released"),
        boundary("player.login.reconnect_may_claim"),
    ]);

    vec![
        Scenario {
            name: "load_success".into(),
            steps: vec![
                boundary("player.login.claimed"),
                Step::Read {
                    database: LogicalDatabase::Character,
                    connection: ConnectionAffinity::Pooled,
                    name: "player.load.query_holder".into(),
                    params: vec![ParameterClass::PlayerGuid, ParameterClass::AccountId],
                },
                publication("player.login.runtime_published"),
            ],
        },
        Scenario {
            name: "periodic_save_success".into(),
            steps: periodic,
        },
        Scenario {
            name: "manual_save_precommit_failure".into(),
            steps: manual_rollback,
        },
        Scenario {
            name: "manual_save_commit_unknown".into(),
            steps: manual_unknown,
        },
        Scenario {
            name: "logout_success_then_reconnect".into(),
            steps: logout,
        },
    ]
}

#[test]
fn player_lifecycle_contract_matches_small_golden() {
    let expected: Vec<Scenario> = serde_json::from_str(GOLDEN).expect("valid lifecycle golden");
    assert_eq!(scenarios(), expected);
}

#[test]
fn publication_must_follow_a_successful_commit() {
    let steps = player_save_steps(CommitOutcome::Committed);
    let commit = steps
        .iter()
        .position(|step| {
            matches!(
                step,
                Step::Commit {
                    outcome: CommitOutcome::Committed,
                    ..
                }
            )
        })
        .expect("committed boundary");
    let publication = steps
        .iter()
        .position(|step| matches!(step, Step::Publication { .. }))
        .expect("post-commit publication");
    assert!(commit < publication);

    for outcome in [CommitOutcome::RolledBack, CommitOutcome::Unknown] {
        assert!(
            !player_save_steps(outcome)
                .iter()
                .any(|step| matches!(step, Step::Publication { .. })),
            "{outcome:?} must preserve dirty runtime state"
        );
    }
}

#[test]
fn rollback_and_unknown_commit_remain_distinct_terminal_states() {
    let rollback = player_save_steps(CommitOutcome::RolledBack);
    assert!(matches!(
        rollback.last(),
        Some(Step::Rollback {
            database: LogicalDatabase::Character
        })
    ));

    let unknown = player_save_steps(CommitOutcome::Unknown);
    assert!(unknown.iter().any(|step| matches!(
        step,
        Step::Commit {
            database: LogicalDatabase::Character,
            outcome: CommitOutcome::Unknown
        }
    )));
    assert!(matches!(
        unknown.last(),
        Some(Step::Fence { name }) if name == "player.save.relogin_required"
    ));
}

#[test]
fn player_save_writes_keep_character_transaction_affinity() {
    for outcome in [
        CommitOutcome::Committed,
        CommitOutcome::RolledBack,
        CommitOutcome::Unknown,
    ] {
        let writes = player_save_steps(outcome)
            .into_iter()
            .filter_map(|step| match step {
                Step::WriteGroup {
                    database,
                    connection,
                    ..
                } => Some((database, connection)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            writes,
            vec![(LogicalDatabase::Character, ConnectionAffinity::Transaction)]
        );
    }
}

#[test]
fn golden_detects_reordered_publication_and_changed_affinity() {
    let expected: Vec<Scenario> = serde_json::from_str(GOLDEN).expect("valid lifecycle golden");

    let mut reordered = scenarios();
    let steps = &mut reordered
        .iter_mut()
        .find(|scenario| scenario.name == "periodic_save_success")
        .expect("periodic scenario")
        .steps;
    let commit = steps
        .iter()
        .position(|step| matches!(step, Step::Commit { .. }))
        .expect("commit");
    steps.swap(commit, commit + 1);
    assert_ne!(reordered, expected, "publication order is contract data");

    let mut wrong_affinity = scenarios();
    let steps = &mut wrong_affinity
        .iter_mut()
        .find(|scenario| scenario.name == "periodic_save_success")
        .expect("periodic scenario")
        .steps;
    let write = steps
        .iter_mut()
        .find_map(|step| match step {
            Step::WriteGroup { connection, .. } => Some(connection),
            _ => None,
        })
        .expect("save write");
    *write = ConnectionAffinity::Pooled;
    assert_ne!(
        wrong_affinity, expected,
        "connection affinity is contract data"
    );
}
