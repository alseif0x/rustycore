//! Money fixtures retain the real commands, fanout routes, trackers and worker handle.
//! Mounted only for unit tests or the explicit test-fixtures feature.

use super::*;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use crate::session::DurableLootMoneyPersistenceTrackerLikeCpp;
use wow_persistence::GroupLootMoneyPersistencePortLikeCpp;

pub async fn consume_stored_money_with_port_for_test(
    session: &WorldSession,
    item: ObjectGuid,
    cached_amount: u64,
) -> Option<(Arc<AtomicBool>, Arc<AtomicBool>, u64, u64)> {
    session.persist_and_consume_stored_item_money_like_cpp(item, cached_amount).await
}

pub async fn open_money_loot_normally_for_test(
    session: &mut WorldSession,
    gameobject_guid: ObjectGuid,
    source: GameObjectLootSource,
) {
    let template_money = session
        .world_query_catalogs_like_cpp()
        .and_then(|catalogs| catalogs.gameobject.get(gameobject_guid.entry()))
        .map(|row| (row.min_money, row.max_money))
        .unwrap_or((0, 0));
    let generators = session.id_generators_for_test_like_cpp();
    let item_valuation = session.item_valuation_catalogs_for_test_like_cpp();
    session.open_represented_gameobject_chest_with_template_money_like_cpp(
        generators.item.as_ref(),
        &item_valuation,
        gameobject_guid,
        source,
        template_money,
    )
    .await;
}

pub async fn open_personal_money_loot_for_test(
    session: &mut WorldSession,
    gameobject_guid: ObjectGuid,
    source: GameObjectLootSource,
) {
    let template_money = session
        .world_query_catalogs_like_cpp()
        .and_then(|catalogs| catalogs.gameobject.get(gameobject_guid.entry()))
        .map(|row| (row.min_money, row.max_money))
        .unwrap_or((0, 0));
    let generators = session.id_generators_for_test_like_cpp();
    let item_valuation = session.item_valuation_catalogs_for_test_like_cpp();
    session.open_gameobject_chest_with_policy(
        generators.item.as_ref(),
        &item_valuation,
        gameobject_guid,
        source,
        template_money,
        LootCyclePolicy::LegacyFixture,
    )
    .await;
}

pub async fn consume_personal_money_loot_for_test(session: &mut WorldSession, pkt: WorldPacket) {
    let generators = session.id_generators_for_test_like_cpp();
    session.consume_money_from_loot_views(
        generators.item.as_ref(), pkt, LootCyclePolicy::LegacyFixture,
    ).await;
}

pub struct LootMoneyTracker(Arc<DurableLootMoneyPersistenceTrackerLikeCpp>);
pub struct LootMoneyApplication(ApplyLootMoneyLikeCppCommand);
pub struct LootMoneyFanout(LootMoneyViewerFanoutLikeCpp);
pub struct LootMoneyDelivery((LootMoneyDeliveryAddressLikeCpp, SessionCommand));

impl Default for LootMoneyTracker {
    fn default() -> Self {
        Self(Default::default())
    }
}

pub fn money_tracker_for_test(session: &WorldSession) -> LootMoneyTracker {
    LootMoneyTracker(session.durable_loot_money_persistence_tracker_like_cpp())
}

impl LootMoneyApplication {
    pub fn new(
        recipient: ObjectGuid,
        loot_owner: ObjectGuid,
        loot_obj: ObjectGuid,
        amount: u64,
        durable_applied_amount: Arc<AtomicU64>,
        tracker: LootMoneyTracker,
        sole_looter: bool,
        authority: OwnedLootAuthority,
        authority_generation: u64,
        authority_committed: Arc<AtomicBool>,
        send_coin_removed: Arc<AtomicBool>,
        applied: Arc<AtomicBool>,
        published: Arc<AtomicBool>,
    ) -> Self {
        Self(ApplyLootMoneyLikeCppCommand {
            recipient, loot_owner, loot_obj, amount, durable_applied_amount,
            durable_persistence_tracker: tracker.0,
            sole_looter, authority, authority_generation, authority_committed,
            send_coin_removed, applied, published,
        })
    }
}

impl LootMoneyFanout {
    pub fn new(
        scope_player: ObjectGuid,
        source_player: ObjectGuid,
        source_command_tx: flume::Sender<SessionCommand>,
        player_registry: Option<Arc<PlayerRegistry>>,
        map_id: u16,
        instance_id: u32,
        loot_owner: ObjectGuid,
        loot_obj: ObjectGuid,
        authority: OwnedLootAuthority,
        authority_generation: u64,
        payout_recipients: HashSet<ObjectGuid>,
    ) -> Self {
        Self(LootMoneyViewerFanoutLikeCpp {
            scope_player, source_player, source_command_tx, player_registry,
            map_id, instance_id, loot_owner, loot_obj, authority,
            authority_generation, payout_recipients,
        })
    }
}

pub fn source_money_delivery_for_test(
    sender: flume::Sender<SessionCommand>,
    application: LootMoneyApplication,
) -> LootMoneyDelivery {
    LootMoneyDelivery((
        LootMoneyDeliveryAddressLikeCpp::Source(sender),
        SessionCommand::ApplyLootMoneyLikeCpp(application.0),
    ))
}

#[derive(Debug)]
pub enum LootMoneyWorkerError {
    MissingPlayer,
    MissingCharacterDatabase,
    WorkerTerminated,
    Claim(LootClaimCommitError),
    Persistence(String),
    CommitOutcomeUnknownPersistence(String),
}

impl From<LootMoneyPersistenceErrorLikeCpp> for LootMoneyWorkerError {
    fn from(error: LootMoneyPersistenceErrorLikeCpp) -> Self {
        match error {
            LootMoneyPersistenceErrorLikeCpp::MissingPlayer => Self::MissingPlayer,
            LootMoneyPersistenceErrorLikeCpp::MissingCharacterDatabase => Self::MissingCharacterDatabase,
            LootMoneyPersistenceErrorLikeCpp::WorkerTerminated => Self::WorkerTerminated,
            LootMoneyPersistenceErrorLikeCpp::Claim(error) => Self::Claim(error),
            LootMoneyPersistenceErrorLikeCpp::Persistence(reason) => Self::Persistence(reason),
            LootMoneyPersistenceErrorLikeCpp::CommitOutcomeUnknownPersistence(reason) => {
                Self::CommitOutcomeUnknownPersistence(reason)
            }
        }
    }
}

pub struct LootMoneyWorker {
    handle: tokio::task::JoinHandle<Result<(), LootMoneyPersistenceErrorLikeCpp>>,
}

impl Future for LootMoneyWorker {
    type Output = Result<Result<(), LootMoneyWorkerError>, tokio::task::JoinError>;

    fn poll(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.get_mut().handle).poll(context).map(|joined| {
            joined.map(|result| result.map_err(LootMoneyWorkerError::from))
        })
    }
}

pub fn spawn_group_money_worker_for_test(
    session: &WorldSession,
    payouts: Vec<(ObjectGuid, u64)>,
    claim: LootClaimLease,
    deliveries: Vec<LootMoneyDelivery>,
    authority_committed: Arc<AtomicBool>,
    fanout: LootMoneyFanout,
) -> Result<LootMoneyWorker, LootMoneyWorkerError> {
    session.spawn_group_loot_money_persistence_like_cpp(
        payouts,
        claim,
        deliveries.into_iter().map(|delivery| delivery.0).collect(),
        authority_committed,
        fanout.0,
    ).map(|handle| LootMoneyWorker { handle }).map_err(LootMoneyWorkerError::from)
}

pub async fn apply_money_command_for_test(
    session: &mut WorldSession,
    application: LootMoneyApplication,
) {
    let generators = session.id_generators_for_test_like_cpp();
    session.handle_apply_loot_money_with_generator_like_cpp_command(
        generators.item.as_ref(),
        application.0,
    ).await;
}

pub fn accepted_money_delta_for_test(current: u64, delta: u64) -> (u64, u64) {
    loot_money_durable_outcome_like_cpp(current, delta)
}

pub fn money_recipients_for_test(session: &WorldSession, owner: ObjectGuid) -> Vec<ObjectGuid> {
    session.represented_loot_money_recipients_like_cpp(owner)
}

pub fn allow_money_looter_for_test(session: &mut WorldSession, owner: ObjectGuid, player: ObjectGuid) {
    session.loot_table.get_mut(&owner).unwrap().allowed_looters.push(player);
}

pub fn notify_money_removed_for_test(session: &mut WorldSession, owner: ObjectGuid) {
    session.represented_notify_money_removed_like_cpp(owner);
}

pub fn active_money_generation_for_test(session: &WorldSession, owner: ObjectGuid) -> Option<u64> {
    session.loot_views.generation(&owner)
}

pub fn money_instance_for_test(session: &WorldSession) -> Option<u32> {
    session.current_canonical_player_map_key_like_cpp().map(|key| key.instance_id)
}

pub fn money_map_for_test(session: &WorldSession) -> u16 {
    session.player_map_id_like_cpp()
}

pub fn clear_money_persistence_outcome_for_test(session: &mut WorldSession) {
    session.clear_loot_money_persistence_test_result_like_cpp();
}

pub fn set_group_money_port_for_test(
    session: &mut WorldSession,
    port: Arc<dyn GroupLootMoneyPersistencePortLikeCpp>,
) {
    session.set_group_loot_money_persistence_port_like_cpp(port);
}

pub async fn generate_creature_money_loot_for_test(
    session: &mut WorldSession,
    owner: ObjectGuid,
    player: ObjectGuid,
    level: u8,
    entry: u32,
    loot_id: u32,
    min_money: u32,
    max_money: u32,
    encounter: u32,
) -> Option<CreatureLoot> {
    session.generate_represented_creature_loot_like_cpp(
        owner, player, level, entry, loot_id, min_money, max_money, encounter,
    ).await
}

pub async fn generate_chest_money_loot_for_test(
    session: &mut WorldSession,
    owner: ObjectGuid,
    player: ObjectGuid,
    source: GameObjectLootSource,
    looters: &[ObjectGuid],
    money: (u32, u32),
) -> Option<CreatureLoot> {
    session.generate_represented_gameobject_chest_loot_with_template_money_like_cpp(
        owner, player, source, looters, money,
    ).await
}

pub fn sync_creature_loot_fixture_for_test(
    session: &mut WorldSession,
    owner: ObjectGuid,
    player: ObjectGuid,
) -> Option<()> {
    session.sync_represented_creature_loot_to_canonical_like_cpp(owner, player)
}

pub fn ensure_money_player_map_for_test(session: &mut WorldSession) -> Option<wow_map::CreateMapDecision> {
    session.ensure_canonical_world_map_for_current_player_like_cpp()
}

pub fn attach_money_player_controller_for_test(
    session: &mut WorldSession,
    guid: ObjectGuid,
    name: String,
    position: wow_core::Position,
    map_id: u16,
    race: u8,
    class: u8,
    level: u8,
    gender: u8,
) {
    session.attach_player_controller_for_fixture(crate::session::SessionPlayerController::new(
        guid, name, position, map_id, race, class, level, gender,
    ));
}
