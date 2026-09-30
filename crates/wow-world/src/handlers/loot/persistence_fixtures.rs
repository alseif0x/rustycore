//! Opaque recovery fixtures backed by the real loot persistence workers.
//! Available only to tests and the explicit test-fixtures feature.

use super::*;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug)]
pub enum LootPersistenceError<E> {
    Persistence(E),
    Claim(LootClaimCommitError),
}

/// Polls the original worker handle; dropping this adapter still detaches it.
pub struct LootPersistenceWorker<E> {
    handle: tokio::task::JoinHandle<Result<(), LootClaimPersistenceWorkerError<E>>>,
}

impl<E> LootPersistenceWorker<E> {
    pub fn abort(&self) {
        self.handle.abort();
    }
}

impl<E> Future for LootPersistenceWorker<E> {
    type Output = Result<Result<(), LootPersistenceError<E>>, tokio::task::JoinError>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().handle).poll(context).map(|joined| {
            joined.map(|result| result.map_err(|error| match error {
                LootClaimPersistenceWorkerError::Persistence(error) => {
                    LootPersistenceError::Persistence(error)
                }
                LootClaimPersistenceWorkerError::Claim(error) => {
                    LootPersistenceError::Claim(error)
                }
            }))
        })
    }
}

pub struct LootPersistenceGuard(DurableItemLootPersistenceGuardLikeCpp);
pub struct LootCompletion(DurableItemLootCompletionLikeCpp);
pub struct LootItemFanout(DurableLootItemFanoutLikeCpp);

impl LootPersistenceGuard {
    pub fn mark_committed(&mut self, completion: LootCompletion) {
        self.0.mark_committed_like_cpp(completion.0);
    }
}

impl LootCompletion {
    pub fn new(
        owner_guid: ObjectGuid,
        loot_list_id: u8,
        player_guid: ObjectGuid,
        item_owner_auto_release: bool,
        durable_item_money_applied_amount: Option<u64>,
        durable_item_money_notified_amount: Option<u64>,
        durable_item_money_balance_applied: Option<Arc<AtomicBool>>,
        item_fanout: Option<LootItemFanout>,
        runtime_inventory_applied: Arc<AtomicBool>,
    ) -> Self {
        Self(DurableItemLootCompletionLikeCpp {
            owner_guid,
            loot_list_id,
            player_guid,
            item_owner_auto_release,
            durable_item_money_applied_amount,
            durable_item_money_notified_amount,
            durable_item_money_balance_applied,
            item_fanout: item_fanout.map(|fanout| fanout.0),
            runtime_inventory_applied,
        })
    }
}

impl LootItemFanout {
    pub fn precommit_viewer_count(&self) -> usize {
        self.0.precommit_snapshot.loot.players_looting.len()
    }
}

pub fn begin_loot_persistence_for_test(session: &WorldSession) -> LootPersistenceGuard {
    LootPersistenceGuard(session.begin_durable_item_loot_persistence_like_cpp())
}

pub fn spawn_loot_claim_worker_for_test<F, E>(
    persistence: F,
    claim: Option<LootClaimLease>,
    completion: Option<(LootPersistenceGuard, LootCompletion)>,
) -> Result<LootPersistenceWorker<E>, LootClaimCommitError>
where
    F: Future<Output = Result<(), E>> + Send + 'static,
    E: Send + 'static,
{
    spawn_loot_claim_persistence_worker_like_cpp(
        persistence,
        claim,
        completion.map(|(guard, completion)| (guard.0, completion.0)),
    )
    .map(|handle| LootPersistenceWorker { handle })
}

pub fn spawn_loot_item_worker_for_test<F>(
    persistence: F,
    claim: Option<LootClaimLease>,
    completion: Option<(LootPersistenceGuard, LootCompletion)>,
    command_tx: flume::Sender<SessionCommand>,
) -> Result<LootPersistenceWorker<String>, LootClaimCommitError>
where
    F: Future<Output = PersistenceOutcomeLikeCpp> + Send + 'static,
{
    spawn_loot_item_persistence_worker_like_cpp(
        persistence,
        claim,
        completion.map(|(guard, completion)| (guard.0, completion.0)),
        command_tx,
    )
    .map(|handle| LootPersistenceWorker { handle })
}

pub fn prepare_loot_item_fanout_for_test(
    session: &mut WorldSession,
    claim: &LootClaimLease,
    owner_guid: ObjectGuid,
    loot_obj: ObjectGuid,
    loot_list_id: u8,
    player_guid: ObjectGuid,
    free_for_all: bool,
) -> Option<LootItemFanout> {
    session.prepare_durable_loot_item_fanout_like_cpp(
        claim,
        LootItemClaimCommitContextLikeCpp {
            owner_guid,
            loot_obj,
            loot_list_id,
            player_guid,
            free_for_all,
        },
    ).map(LootItemFanout)
}

pub async fn apply_loot_completions_for_test(session: &mut WorldSession) {
    let generators = session.id_generators_for_test_like_cpp();
    session.apply_pending_durable_item_loot_completions_with_generator_like_cpp(
        generators.item.as_ref(),
    ).await;
}

pub async fn wait_for_loot_persistence_for_test(session: &mut WorldSession) {
    let generators = session.id_generators_for_test_like_cpp();
    session.wait_for_active_loot_persistence_with_generator_like_cpp(generators.item.as_ref())
        .await;
}

pub async fn disconnect_loot_cleanup_for_test(session: &mut WorldSession) {
    let generators = session.id_generators_for_test_like_cpp();
    session.wait_for_active_loot_persistence_with_generator_like_cpp(generators.item.as_ref())
        .await;
    if let Some(player_guid) = session.player_guid()
        && session.has_active_loot_views_like_cpp()
    {
        session.do_loot_release_all_like_cpp(player_guid).await;
    }
    session.cleanup_shared_runtime_state();
}

pub async fn disconnect_loot_save_for_test(session: &mut WorldSession) -> crate::FinalizationReport {
    let generators = session.id_generators_for_test_like_cpp();
    session.finalize_session_with_generator_like_cpp(
        crate::finalization::FinalizationMode::Disconnect,
        generators.item.as_ref(),
    ).await
}

pub fn loot_recovery_authority_for_test(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
) -> Option<OwnedLootAuthority> {
    session.represented_owned_loot_authority_like_cpp(owner_guid)
}

pub fn loot_recovery_cache_for_test(
    session: &WorldSession,
    owner_guid: ObjectGuid,
) -> Option<&CreatureLoot> {
    session.loot_table.get(&owner_guid)
}

pub fn loot_recovery_cache_values_for_test(
    session: &WorldSession,
) -> impl Iterator<Item = &CreatureLoot> {
    session.loot_table.values()
}

pub fn install_loot_recovery_authority_for_test(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
    loot: CreatureLoot,
) {
    session.loot_table.insert(owner_guid, loot);
    session.sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, player_guid).unwrap();
}

pub fn open_loot_recovery_view_for_test(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
) {
    let loot = &session.loot_table[&owner_guid];
    let response = LootResponse {
        owner: owner_guid,
        loot_obj: loot.loot_guid,
        failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
        acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
        loot_method: loot.loot_method,
        threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
        coins: loot.coins,
        items: represented_loot_response_items_like_cpp(loot, player_guid),
        currencies: Vec::new(),
        acquired: true,
        ae_looting: false,
    };
    let item_valuation = session.item_valuation_catalogs_for_test_like_cpp();
    session.represented_on_loot_opened_with_catalogs_like_cpp(
        &item_valuation,
        owner_guid,
        player_guid,
        response,
    );
}

pub fn reconcile_loot_recovery_cache_for_test(
    session: &mut WorldSession,
    owner_guid: ObjectGuid,
    player_guid: ObjectGuid,
) -> bool {
    session.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid)
}
