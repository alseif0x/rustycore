//! Executor-test fixtures: in-memory Login DB persistence, saga sessions
//! over flume channels and the fault-injection matrix.
//!
//! Separated from the executor_tests root under #656.

use super::*;

pub(crate) const PLAYER_COUNTER: i64 = 42;

pub(crate) const OTHER_PLAYER_COUNTER: i64 = 43;

pub(crate) const ACCOUNT_ID: u32 = 1;

pub(crate) const TRAINER_ID: u32 = 7;

pub(crate) const SAGA_SPELL_ID: u32 = 54_330;

pub(crate) const SAGA_SPECIES: u32 = 11;

pub(crate) const LEGACY_UNIQUE_SPECIES: u32 = 12;

pub(crate) const SAGA_PRICE: u32 = 250;

pub(crate) const SAGA_MONEY: u64 = 1_000;

pub(crate) const REALM_ID: u16 = 7;

pub(crate) const VIRTUAL_REALM: u32 = 0x0102_0007;

pub(crate) type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Default)]
pub(crate) struct FakeSagaPersistenceStateLikeCpp {
    pub(crate) pets: Vec<DurableBattlePetRowLikeCpp>,
    pub(crate) slots: Vec<DurableBattlePetSlotLikeCpp>,
    pub(crate) receipts: HashMap<BattlePetAddRequestKeyLikeCpp, (u32, DurableBattlePetRowLikeCpp)>,
}

/// In-memory Login DB with the #160 persistence contract (receipt
/// replay, capacity, fence validation) plus the fault gates the saga
/// matrix needs: pre-commit insert failure, lost insert reply and a
/// blocking insert for cancellation/drain tests.
#[derive(Default)]
pub(crate) struct FakeSagaPersistenceLikeCpp {
    pub(crate) state: StdMutex<FakeSagaPersistenceStateLikeCpp>,
    pub(crate) process_lease: Arc<AtomicBool>,
    pub(crate) current_fence: Arc<AtomicU64>,
    pub(crate) next_guid: AtomicU64,
    pub(crate) insert_calls: AtomicUsize,
    pub(crate) fail_next_insert: AtomicBool,
    pub(crate) lose_next_insert_reply: AtomicBool,
    pub(crate) reconcile_next_insert_after_commit: AtomicBool,
    pub(crate) block_next_insert: AtomicBool,
    pub(crate) insert_started: Notify,
    pub(crate) allow_insert: Notify,
}

pub(crate) struct FakeSagaLeaseGuardLikeCpp {
    pub(crate) held: Arc<AtomicBool>,
    pub(crate) fence: u64,
}

impl BattlePetProcessLeaseLikeCpp for FakeSagaLeaseGuardLikeCpp {
    fn is_valid_like_cpp(&self) -> bool {
        self.held.load(Ordering::Acquire)
    }

    fn fence_like_cpp(&self) -> u64 {
        self.fence
    }
}

impl Drop for FakeSagaLeaseGuardLikeCpp {
    fn drop(&mut self) {
        self.held.store(false, Ordering::Release);
    }
}

impl FakeSagaPersistenceLikeCpp {
    pub(crate) fn with_seeded_pets(pets: Vec<DurableBattlePetRowLikeCpp>) -> Self {
        let persistence = Self::default();
        persistence.next_guid.store(
            pets.iter().map(|pet| pet.guid_counter).max().unwrap_or(0) + 10,
            Ordering::Release,
        );
        persistence
            .state
            .lock()
            .expect("fake saga persistence poisoned")
            .pets = pets;
        persistence
    }

    pub(crate) fn pet_count(&self) -> usize {
        self.state
            .lock()
            .expect("fake saga persistence poisoned")
            .pets
            .len()
    }

    pub(crate) fn species_count(&self, species: u32) -> usize {
        self.state
            .lock()
            .expect("fake saga persistence poisoned")
            .pets
            .iter()
            .filter(|pet| pet.species == species)
            .count()
    }

    pub(crate) fn receipt_count(&self) -> usize {
        self.state
            .lock()
            .expect("fake saga persistence poisoned")
            .receipts
            .len()
    }

    pub(crate) fn receipt(&self, request_key: [u8; 16]) -> Option<DurableBattlePetRowLikeCpp> {
        self.state
            .lock()
            .expect("fake saga persistence poisoned")
            .receipts
            .get(&BattlePetAddRequestKeyLikeCpp::from_bytes(request_key))
            .map(|(_, pet)| pet.clone())
    }

    /// Simulate another process winning the named lock: the current
    /// guard dies and the next acquisition observes a higher fence.
    pub(crate) fn simulate_process_takeover_like_cpp(&self) {
        self.process_lease.store(false, Ordering::Release);
        self.current_fence.fetch_add(1, Ordering::AcqRel);
    }
}

pub(crate) fn request_matches_row_like_cpp(
    pet: &DurableBattlePetRowLikeCpp,
    existing: &DurableBattlePetRowLikeCpp,
) -> bool {
    pet.species == existing.species
        && pet.breed == existing.breed
        && pet.display_id == existing.display_id
        && pet.quality == existing.quality
        && pet.level == existing.level
        && pet.owner_guid_counter == existing.owner_guid_counter
}

impl BattlePetPersistenceLikeCpp for FakeSagaPersistenceLikeCpp {
    fn try_acquire_process_lease<'a>(
        &'a self,
        _account_id: u32,
    ) -> BoxFuture<
        'a,
        Result<Option<Box<dyn BattlePetProcessLeaseLikeCpp>>, BattlePetPersistenceErrorLikeCpp>,
    > {
        Box::pin(async move {
            Ok(self
                .process_lease
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
                .then(|| {
                    let fence = self.current_fence.fetch_add(1, Ordering::AcqRel) + 1;
                    Box::new(FakeSagaLeaseGuardLikeCpp {
                        held: Arc::clone(&self.process_lease),
                        fence,
                    }) as Box<dyn BattlePetProcessLeaseLikeCpp>
                }))
        })
    }

    fn load_account<'a>(
        &'a self,
        _account_id: u32,
        _realm_id: u16,
    ) -> BoxFuture<'a, Result<LoadedBattlePetAccountLikeCpp, BattlePetPersistenceErrorLikeCpp>>
    {
        Box::pin(async move {
            let state = self.state.lock().expect("fake saga persistence poisoned");
            Ok(LoadedBattlePetAccountLikeCpp {
                pets: state.pets.clone(),
                slots: state.slots.clone(),
            })
        })
    }

    fn allocate_guid_counter_like_cpp(
        &self,
    ) -> BoxFuture<'_, Result<u64, BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move { Ok(self.next_guid.fetch_add(1, Ordering::AcqRel)) })
    }

    fn insert_pet_idempotently<'a>(
        &'a self,
        request: DurableBattlePetAddLikeCpp,
    ) -> BoxFuture<'a, Result<PersistBattlePetAddOutcomeLikeCpp, BattlePetPersistenceErrorLikeCpp>>
    {
        Box::pin(async move {
            self.insert_calls.fetch_add(1, Ordering::Relaxed);
            tokio::task::yield_now().await;
            if self.block_next_insert.swap(false, Ordering::AcqRel) {
                self.insert_started.notify_one();
                self.allow_insert.notified().await;
            }
            if self.current_fence.load(Ordering::Acquire) != request.fence {
                return Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
            }
            if self.fail_next_insert.swap(false, Ordering::AcqRel) {
                return Err(BattlePetPersistenceErrorLikeCpp::Database(
                    "injected insert failure".to_string(),
                ));
            }
            let applied = {
                let mut state = self.state.lock().expect("fake saga persistence poisoned");
                if let Some((receipt_account_id, existing)) =
                    state.receipts.get(&request.request_key).cloned()
                {
                    let still_present = state
                        .pets
                        .iter()
                        .any(|pet| pet.guid_counter == existing.guid_counter);
                    return if receipt_account_id == request.account_id
                        && request_matches_row_like_cpp(&request.pet, &existing)
                    {
                        Ok(PersistBattlePetAddOutcomeLikeCpp::Replayed {
                            pet: existing,
                            still_present,
                        })
                    } else {
                        Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest)
                    };
                }
                let scoped_count = state
                    .pets
                    .iter()
                    .filter(|pet| pet.species == request.pet.species)
                    .filter(|pet| pet.owner_guid_counter == request.pet.owner_guid_counter)
                    .count();
                if scoped_count >= usize::from(request.max_per_scope) {
                    return Err(BattlePetPersistenceErrorLikeCpp::Capacity);
                }
                if state
                    .pets
                    .iter()
                    .any(|pet| pet.guid_counter == request.pet.guid_counter)
                {
                    return Err(BattlePetPersistenceErrorLikeCpp::GuidCollision);
                }
                state.pets.push(request.pet.clone());
                state
                    .receipts
                    .insert(request.request_key, (request.account_id, request.pet));
                true
            };
            let _ = applied;
            if self.lose_next_insert_reply.swap(false, Ordering::AcqRel) {
                return Err(BattlePetPersistenceErrorLikeCpp::Database(
                    "injected lost insert reply".to_string(),
                ));
            }
            if self
                .reconcile_next_insert_after_commit
                .swap(false, Ordering::AcqRel)
            {
                // Production reconciles its own lost COMMIT reply through
                // the receipt as `Inserted`, preserving the fact that this
                // invocation owns the one C++ new-pet criteria update.
                return Ok(PersistBattlePetAddOutcomeLikeCpp::Inserted);
            }
            Ok(PersistBattlePetAddOutcomeLikeCpp::Inserted)
        })
    }

    fn lookup_add_request<'a>(
        &'a self,
        account_id: u32,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> BoxFuture<
        'a,
        Result<Option<DurableBattlePetAddReceiptLikeCpp>, BattlePetPersistenceErrorLikeCpp>,
    > {
        Box::pin(async move {
            let state = self.state.lock().expect("fake saga persistence poisoned");
            let Some((receipt_account_id, pet)) = state.receipts.get(&request_key).cloned() else {
                return Ok(None);
            };
            if receipt_account_id != account_id {
                return Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest);
            }
            Ok(Some(DurableBattlePetAddReceiptLikeCpp {
                account_id: receipt_account_id,
                requested_pet: pet.clone(),
                current_pet: state
                    .pets
                    .iter()
                    .find(|existing| existing.guid_counter == pet.guid_counter)
                    .cloned(),
            }))
        })
    }

    fn update_pet<'a>(
        &'a self,
        _account_id: u32,
        fence: u64,
        pet: DurableBattlePetRowLikeCpp,
    ) -> BoxFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            if self.current_fence.load(Ordering::Acquire) != fence {
                return Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
            }
            let mut state = self.state.lock().expect("fake saga persistence poisoned");
            let Some(existing) = state
                .pets
                .iter_mut()
                .find(|existing| existing.guid_counter == pet.guid_counter)
            else {
                return Err(BattlePetPersistenceErrorLikeCpp::Database(
                    "unknown fake pet".to_string(),
                ));
            };
            *existing = pet;
            Ok(())
        })
    }

    fn delete_pet<'a>(
        &'a self,
        _account_id: u32,
        fence: u64,
        pet_guid_counter: u64,
        _slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> BoxFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            if self.current_fence.load(Ordering::Acquire) != fence {
                return Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
            }
            self.state
                .lock()
                .expect("fake saga persistence poisoned")
                .pets
                .retain(|pet| pet.guid_counter != pet_guid_counter);
            Ok(())
        })
    }

    fn replace_slots<'a>(
        &'a self,
        _account_id: u32,
        fence: u64,
        slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> BoxFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            if self.current_fence.load(Ordering::Acquire) != fence {
                return Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
            }
            self.state
                .lock()
                .expect("fake saga persistence poisoned")
                .slots = slots;
            Ok(())
        })
    }
}

pub(crate) fn saga_species_store_like_cpp() -> Arc<BattlePetSpeciesStore> {
    Arc::new(BattlePetSpeciesStore::from_entries([
        BattlePetSpeciesEntry {
            id: SAGA_SPECIES,
            description: String::new(),
            source_text: String::new(),
            creature_id: 99,
            summon_spell_id: 0,
            icon_file_data_id: 0,
            pet_type_enum: 0,
            flags: BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP,
            source_type_enum: 0,
            card_ui_model_scene_id: 0,
            loadout_ui_model_scene_id: 0,
        },
        BattlePetSpeciesEntry {
            id: LEGACY_UNIQUE_SPECIES,
            description: String::new(),
            source_text: String::new(),
            creature_id: 100,
            summon_spell_id: 0,
            icon_file_data_id: 0,
            pet_type_enum: 0,
            flags: BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP
                | wow_data::BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP,
            source_type_enum: 0,
            card_ui_model_scene_id: 0,
            loadout_ui_model_scene_id: 0,
        },
    ]))
}

pub(crate) fn saga_stat_stores_like_cpp() -> (
    Arc<BattlePetBreedQualityStore>,
    Arc<BattlePetBreedStateStore>,
    Arc<BattlePetSpeciesStateStore>,
) {
    (
        Arc::new(BattlePetBreedQualityStore::from_entries([
            BattlePetBreedQualityEntry {
                id: 1,
                state_multiplier: 1.0,
                quality_enum: 1,
            },
        ])),
        Arc::new(BattlePetBreedStateStore::from_entries([
            BattlePetBreedStateEntry {
                id: 1,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
                value: 500,
                battle_pet_breed_id: 7,
            },
            BattlePetBreedStateEntry {
                id: 2,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_POWER_LIKE_CPP,
                value: 300,
                battle_pet_breed_id: 7,
            },
            BattlePetBreedStateEntry {
                id: 3,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
                value: 200,
                battle_pet_breed_id: 7,
            },
        ])),
        Arc::new(BattlePetSpeciesStateStore::from_entries([
            BattlePetSpeciesStateEntry {
                id: 1,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
                value: 100,
                battle_pet_species_id: SAGA_SPECIES,
            },
            BattlePetSpeciesStateEntry {
                id: 2,
                battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
                value: 100,
                battle_pet_species_id: LEGACY_UNIQUE_SPECIES,
            },
        ])),
    )
}

pub(crate) fn saga_selection_like_cpp(species: u32) -> BattlePetTrainerSelectionLikeCpp {
    BattlePetTrainerSelectionLikeCpp {
        species,
        breed: 7,
        quality: 1,
        display_id: 123,
        level: 1,
    }
}

pub(crate) fn saga_durable_pet_row_like_cpp(
    guid_counter: u64,
    species: u32,
    owner_guid_counter: Option<u64>,
) -> DurableBattlePetRowLikeCpp {
    DurableBattlePetRowLikeCpp {
        guid_counter,
        species,
        breed: 7,
        display_id: 123,
        level: 1,
        exp: 0,
        health: 100,
        quality: 1,
        flags: 0,
        name: String::new(),
        name_timestamp: 0,
        owner_guid_counter,
        declined_names: None,
    }
}

pub(crate) fn saga_trainer_guid_like_cpp() -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 100, 1)
}

pub(crate) struct SagaFixtureLikeCpp {
    pub(crate) session: WorldSession,
    pub(crate) send_rx: flume::Receiver<Vec<u8>>,
    pub(crate) store: Arc<FakeBattlePetPurchaseStoreLikeCpp>,
    pub(crate) persistence: Arc<FakeSagaPersistenceLikeCpp>,
    pub(crate) registry: Arc<BattlePetAccountRegistryLikeCpp>,
}

pub(crate) fn make_saga_session_like_cpp(
    player_counter: i64,
    money: u64,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(64);
    let mut session = WorldSession::new(
        ACCOUNT_ID,
        "SagaTest".into(),
        0,
        2,
        9,
        54_261,
        vec![0; 40],
        "enUS".into(),
        pkt_rx,
        send_tx,
    );
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        ObjectGuid::create_player(1, player_counter),
        "Buyer".to_string(),
        Position::ZERO,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_battlenet_account_id(ACCOUNT_ID);
    session.set_player_gold_like_cpp(money);
    session.set_battle_pet_species_store(saga_species_store_like_cpp());
    session.set_battle_pet_purchase_selection_override_like_cpp(Some(saga_selection_like_cpp(
        SAGA_SPECIES,
    )));
    (session, send_rx)
}

pub(crate) fn saga_registry_like_cpp(
    persistence: Arc<FakeSagaPersistenceLikeCpp>,
) -> Arc<BattlePetAccountRegistryLikeCpp> {
    let (qualities, breed_states, species_states) = saga_stat_stores_like_cpp();
    Arc::new(
        BattlePetAccountRegistryLikeCpp::new_with_persistence_like_cpp(
            persistence,
            saga_species_store_like_cpp(),
            qualities,
            breed_states,
            species_states,
            Arc::new(wow_data::BattlePetXpGameTableLikeCpp::from_rows([])),
            REALM_ID,
            VIRTUAL_REALM,
        ),
    )
}

pub(crate) async fn saga_fixture_like_cpp(
    money: u64,
    seeded_pets: Vec<DurableBattlePetRowLikeCpp>,
) -> SagaFixtureLikeCpp {
    let persistence = Arc::new(FakeSagaPersistenceLikeCpp::with_seeded_pets(seeded_pets));
    let store =
        Arc::new(FakeBattlePetPurchaseStoreLikeCpp::new().with_money(PLAYER_COUNTER as u64, money));
    let registry = saga_registry_like_cpp(Arc::clone(&persistence));
    let (mut session, send_rx) = make_saga_session_like_cpp(PLAYER_COUNTER, money);
    session.set_battle_pet_purchase_persistence_port_like_cpp(store_handle_like_cpp(&store));
    let attachment = registry
        .attach_like_cpp(ACCOUNT_ID)
        .await
        .expect("saga account attaches");
    session.set_battle_pet_account_attachment_like_cpp(attachment);
    SagaFixtureLikeCpp {
        session,
        send_rx,
        store,
        persistence,
        registry,
    }
}

/// A "process restart": the old session/registry are gone and a fresh
/// registry wraps the same durable fakes (Login/Character DB survive).
pub(crate) async fn restart_saga_session_like_cpp(
    store: Arc<FakeBattlePetPurchaseStoreLikeCpp>,
    persistence: Arc<FakeSagaPersistenceLikeCpp>,
    money: u64,
) -> SagaFixtureLikeCpp {
    let registry = saga_registry_like_cpp(Arc::clone(&persistence));
    let (mut session, send_rx) = make_saga_session_like_cpp(PLAYER_COUNTER, money);
    session.set_battle_pet_purchase_persistence_port_like_cpp(store_handle_like_cpp(&store));
    let attachment = registry
        .attach_like_cpp(ACCOUNT_ID)
        .await
        .expect("saga account attaches after restart");
    session.set_battle_pet_account_attachment_like_cpp(attachment);
    SagaFixtureLikeCpp {
        session,
        send_rx,
        store,
        persistence,
        registry,
    }
}

pub(crate) fn store_handle_like_cpp(
    store: &Arc<FakeBattlePetPurchaseStoreLikeCpp>,
) -> Arc<dyn BattlePetPurchaseStoreLikeCpp> {
    store.clone()
}

pub(crate) fn saga_offer_like_cpp(price: u32) -> PreparedBattlePetTrainerOfferLikeCpp {
    PreparedBattlePetTrainerOfferLikeCpp {
        source_spell_id: SAGA_SPELL_ID,
        effective_price: price,
        species_id: SAGA_SPECIES,
    }
}

pub(crate) async fn execute_saga_purchase_like_cpp(
    fixture: &mut SagaFixtureLikeCpp,
    offer: PreparedBattlePetTrainerOfferLikeCpp,
) -> BattlePetPurchaseExecutionLikeCpp {
    let guard = fixture
        .session
        .begin_exclusive_player_money_persistence_like_cpp()
        .await
        .expect("money exclusivity");
    fixture
        .session
        .execute_battle_pet_trainer_purchase_like_cpp(
            guard,
            saga_trainer_guid_like_cpp(),
            TRAINER_ID,
            offer,
        )
        .await
}

pub(crate) fn owner_of(fixture: &SagaFixtureLikeCpp) -> Arc<BattlePetAccountOwnerLikeCpp> {
    fixture
        .session
        .battle_pet_account_owner_lease_like_cpp()
        .expect("attached owner")
        .0
}

pub(crate) fn expected_pet_packet_like_cpp(
    fixture: &SagaFixtureLikeCpp,
    pet_guid: ObjectGuid,
) -> wow_packet::packets::misc::BattlePetJournalPet {
    owner_of(fixture)
        .pet_snapshot_like_cpp(pet_guid)
        .expect("durable pet snapshot")
        .packet_info_like_cpp(pet_guid)
}

pub(crate) fn assert_no_packets(fixture: &SagaFixtureLikeCpp) {
    assert!(
        fixture.send_rx.try_recv().is_err(),
        "no packets must be published on this path"
    );
}

pub(crate) fn expect_money_update_packet_like_cpp(
    fixture: &SagaFixtureLikeCpp,
    money: u64,
) -> Vec<u8> {
    wow_packet::packets::update::UpdateObject::player_money_update(
        fixture.session.player_guid().expect("player guid"),
        fixture.session.player_map_id_like_cpp(),
        money,
        None,
    )
    .to_bytes()
}

/// Account-wide species rows carry no owner counter (C++ only sets
/// `owner`/`ownerRealmId` for `NotAccountWide` species); the saga species
/// is account-wide, so capacity fixtures seed ownerless rows.
pub(crate) fn seed_third_pet_into_persistence_like_cpp(fixture: &SagaFixtureLikeCpp) {
    fixture
        .persistence
        .state
        .lock()
        .expect("fake saga persistence poisoned")
        .pets
        .push(saga_durable_pet_row_like_cpp(900, SAGA_SPECIES, None));
}

/// Run one async body on a 32 MiB stack, mirroring `run_player_stack_test`
/// in `wow-entities` (`object_accessor.rs`). `#[tokio::test]` runs the body
/// on a default 2 MiB thread, which cannot hold two joined debug-profile
/// purchase state machines at once.
pub(crate) fn run_async_stack_test<F>(test: impl FnOnce() -> F + Send + 'static)
where
    F: Future<Output = ()>,
{
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("current-thread runtime")
                .block_on(test());
        })
        .expect("stack test thread")
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload));
}

pub(crate) async fn concurrent_sessions_charge_once_grant_once_and_compensate_once() {
    let seeded = vec![
        saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
        saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
    ];
    let mut first = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
    let store = Arc::clone(&first.store);
    let registry = Arc::clone(&first.registry);
    let persistence = Arc::clone(&first.persistence);
    store.seed_money_like_cpp(OTHER_PLAYER_COUNTER as u64, SAGA_MONEY);
    let (mut second_session, second_rx) =
        make_saga_session_like_cpp(OTHER_PLAYER_COUNTER, SAGA_MONEY);
    second_session.set_battle_pet_purchase_persistence_port_like_cpp(store_handle_like_cpp(&store));
    second_session.set_battle_pet_account_attachment_like_cpp(
        registry
            .attach_like_cpp(ACCOUNT_ID)
            .await
            .expect("second attachment"),
    );

    let first_purchase = async {
        let guard = first
            .session
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
            .expect("money exclusivity");
        first
            .session
            .execute_battle_pet_trainer_purchase_like_cpp(
                guard,
                saga_trainer_guid_like_cpp(),
                TRAINER_ID,
                saga_offer_like_cpp(SAGA_PRICE),
            )
            .await
    };
    let second_purchase = async {
        let guard = second_session
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
            .expect("money exclusivity");
        second_session
            .execute_battle_pet_trainer_purchase_like_cpp(
                guard,
                saga_trainer_guid_like_cpp(),
                TRAINER_ID,
                saga_offer_like_cpp(SAGA_PRICE),
            )
            .await
    };
    let (first_outcome, second_outcome) = tokio::join!(first_purchase, second_purchase);
    let outcomes = [first_outcome, second_outcome];
    // The #160 journal lease serializes same-account sessions: exactly
    // one session is admitted and purchases; the other receives the
    // structured journal-lock result without a charge or a command.
    let purchased = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome,
                BattlePetPurchaseExecutionLikeCpp::Purchased {
                    published: true,
                    ..
                }
            )
        })
        .count();
    let locked = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome,
                BattlePetPurchaseExecutionLikeCpp::Unavailable(
                    BattlePetPurchaseAdmissionFailureLikeCpp::JournalLocked
                )
            )
        })
        .count();
    assert_eq!((purchased, locked), (1, 1), "{outcomes:?}");

    // Exactly one new pet (third of the species), one Completed
    // command, one charge; the locked session was never charged.
    assert_eq!(persistence.species_count(SAGA_SPECIES), 3);
    assert_eq!(persistence.receipt_count(), 1);
    let statuses: Vec<_> = store
        .commands_snapshot()
        .into_iter()
        .map(|command| command.status)
        .collect();
    assert_eq!(statuses, vec![BattlePetPurchaseStatusLikeCpp::Completed]);
    let balances: Vec<_> = [PLAYER_COUNTER as u64, OTHER_PLAYER_COUNTER as u64]
        .into_iter()
        .map(|guid| store.money(guid).expect("money row"))
        .collect();
    assert!(
        balances.contains(&750) && balances.contains(&SAGA_MONEY),
        "winner charged once, loser never charged: {balances:?}"
    );
    assert_eq!(store.money_mutations(), 1);
    drop(second_rx);

    // Releasing the winner frees the journal; the loser's retry then
    // meets the filled capacity as a structured unavailable, still
    // without a charge — two sessions can never duplicate the outcome.
    let (winning_session, mut losing_session) = if matches!(
        outcomes[0],
        BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
    ) {
        (first.session, second_session)
    } else {
        (second_session, first.session)
    };
    drop(winning_session);
    let retry_guard = losing_session
        .begin_exclusive_player_money_persistence_like_cpp()
        .await
        .expect("money exclusivity");
    let retry_outcome = losing_session
        .execute_battle_pet_trainer_purchase_like_cpp(
            retry_guard,
            saga_trainer_guid_like_cpp(),
            TRAINER_ID,
            saga_offer_like_cpp(SAGA_PRICE),
        )
        .await;
    assert_eq!(
        retry_outcome,
        BattlePetPurchaseExecutionLikeCpp::Unavailable(
            BattlePetPurchaseAdmissionFailureLikeCpp::Capacity
        )
    );
    assert_eq!(persistence.species_count(SAGA_SPECIES), 3);
    assert_eq!(persistence.receipt_count(), 1);
    assert_eq!(store.money_mutations(), 1);
    assert_eq!(store.commands_snapshot().len(), 1);
}

pub(crate) const SAGA_CREATURE_ENTRY: u32 = 123;

pub(crate) const SAGA_SUMMON_PROPERTIES_ID: u32 = 700;

pub(crate) const SAGA_SUMMON_SLOT_MINIPET_RAW: i64 = 5;

pub(crate) const SAGA_SUMMON_FROM_JOURNAL_RAW: i64 = 0x0020_0000;

pub(crate) fn saga_learn_effect_like_cpp(
    record_id: u32,
    wrapper_spell_id: u32,
    learned_spell_id: u32,
) -> wow_data::SpellAcquisitionEffectLikeCpp {
    wow_data::SpellAcquisitionEffectLikeCpp {
        record_id,
        spell_id_raw: i64::from(wrapper_spell_id),
        difficulty_id_raw: 0,
        effect_index_raw: 1,
        effect_type_raw: 36, // C++ SPELL_EFFECT_LEARN_SPELL
        effect_aura_raw: 0,
        effect_mechanic_raw: 0,
        effect_attributes_raw: 0,
        effect_base_points_raw: 0,
        effect_die_sides_raw: 0,
        effect_chain_targets_raw: 0,
        effect_points_per_resource_bits: 0.0_f32.to_bits(),
        effect_real_points_per_level_bits: 0.0_f32.to_bits(),
        effect_coefficient_bits: 0.0_f32.to_bits(),
        effect_variance_bits: 0.0_f32.to_bits(),
        effect_trigger_spell_raw: i64::from(learned_spell_id),
        effect_item_type_raw: 0,
        effect_misc_value_raw: [0, 0],
        implicit_target_raw: [1, 0],
    }
}

pub(crate) fn saga_summon_effect_like_cpp(
    spell_id: u32,
) -> wow_data::SpellAcquisitionEffectLikeCpp {
    wow_data::SpellAcquisitionEffectLikeCpp {
        record_id: 1,
        spell_id_raw: i64::from(spell_id),
        difficulty_id_raw: 0,
        effect_index_raw: 0,
        effect_type_raw: 28, // C++ SPELL_EFFECT_SUMMON
        effect_aura_raw: 0,
        effect_mechanic_raw: 0,
        effect_attributes_raw: 0,
        effect_base_points_raw: 0,
        effect_die_sides_raw: 0,
        effect_chain_targets_raw: 0,
        effect_points_per_resource_bits: 0.0_f32.to_bits(),
        effect_real_points_per_level_bits: 0.0_f32.to_bits(),
        effect_coefficient_bits: 0.0_f32.to_bits(),
        effect_variance_bits: 0.0_f32.to_bits(),
        effect_trigger_spell_raw: 0,
        effect_item_type_raw: 0,
        effect_misc_value_raw: [99, i64::from(SAGA_SUMMON_PROPERTIES_ID)],
        implicit_target_raw: [1, 0],
    }
}

pub(crate) fn insert_saga_trainer_creature_like_cpp(
    manager: &Arc<std::sync::Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
) {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(SAGA_CREATURE_ENTRY);
    creature.unit_mut().world_mut().set_map(0, 0).unwrap();
    creature
        .unit_mut()
        .world_mut()
        .relocate(Position::new(1.0, 0.0, 0.0, 0.0));
    creature.unit_mut().world_mut().set_combat_reach(1.0);
    creature.unit_mut().set_level(80);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature.set_ai_identity_runtime(
        1,
        35,
        wow_constants::unit::NPCFlags1::TRAINER.bits()
            | wow_constants::unit::NPCFlags1::TRAINER_CLASS.bits()
            | wow_constants::unit::NPCFlags1::TRAINER_PROFESSION.bits(),
        0,
    );
    creature.unit_mut().world_mut().object_mut().add_to_world();
    manager
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .expect("canonical test map")
        .map_mut()
        .insert_map_object_record(wow_entities::MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
}

/// A fully rigged handler session: the trainer list/buy path runs the
/// real admission composition (membership, gates, conditions, price,
/// classification) before the saga. `wrapper_learned_spell` adds a
/// `SPELL_EFFECT_LEARN_SPELL` effect so the trainer spell is castable
/// (C++ `IsCastable()`), turning the offer into the normal wrapper
/// acquisition that retains its battle-pet species classification.
pub(crate) async fn saga_handler_fixture_like_cpp(
    money: u64,
    battle_pet_price: u32,
    wrapper_learned_spell: Option<u32>,
    seeded_pets: Vec<DurableBattlePetRowLikeCpp>,
) -> SagaFixtureLikeCpp {
    let persistence = Arc::new(FakeSagaPersistenceLikeCpp::with_seeded_pets(seeded_pets));
    let store =
        Arc::new(FakeBattlePetPurchaseStoreLikeCpp::new().with_money(PLAYER_COUNTER as u64, money));
    let registry = saga_registry_like_cpp(Arc::clone(&persistence));
    let (mut session, send_rx) = make_saga_session_like_cpp(PLAYER_COUNTER, money);
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_trainer_store_like_cpp(Arc::new(
        wow_data::TrainerStoreLikeCpp::from_rows_like_cpp(
            vec![wow_data::TrainerRowLikeCpp {
                id: TRAINER_ID,
                trainer_type: 2,
                greeting: "Train".to_string(),
            }],
            vec![wow_data::TrainerSpellRowLikeCpp {
                trainer_id: TRAINER_ID,
                spell: wow_data::TrainerSpellLikeCpp {
                    spell_id: SAGA_SPELL_ID,
                    money_cost: battle_pet_price,
                    req_skill_line: 0,
                    req_skill_rank: 0,
                    req_ability: [0; 3],
                    req_level: 1,
                },
            }],
            Vec::new(),
            vec![wow_data::CreatureTrainerRowLikeCpp {
                creature_id: SAGA_CREATURE_ENTRY,
                trainer_id: TRAINER_ID,
                menu_id: 0,
                option_id: 0,
            }],
            |_| true,
            |_| true,
            |_| true,
            |_, _| true,
        )
        .store,
    ));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_disable_mgr(Arc::new(wow_data::DisableMgrLikeCpp::default()));
    session.set_player_aura_authority_complete_like_cpp(true);
    session.set_condition_store(Arc::new(wow_data::ConditionEntriesByTypeStore::default()));
    session.set_skill_store(Arc::new(
        wow_data::SkillStore::from_skill_line_abilities_and_race_class_like_cpp([], []),
    ));
    session.set_skill_line_store(Arc::new(wow_data::SkillLineStore::from_entries([])));
    session.set_skill_tiers_store(Arc::new(wow_data::SkillTiersStoreLikeCpp::default()));
    session.set_trait_definition_store(Arc::new(
        wow_data::trait_tree::TraitDefinitionStore::from_entries([]),
    ));
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([])));
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_spell_custom_attribute_store(Arc::new(
        wow_data::SpellCustomAttributeStoreLikeCpp::default(),
    ));
    let mut learn_skills = wow_data::SpellLearnSkillStoreLikeCpp::default();
    learn_skills.covered_spell_ids.extend([SAGA_SPELL_ID]);
    if let Some(learned) = wrapper_learned_spell {
        learn_skills.covered_spell_ids.extend([learned]);
    }
    session.set_spell_learn_skill_store(Arc::new(learn_skills));
    if wrapper_learned_spell.is_some() {
        session.set_spell_acquisition_static_authority_like_cpp([SAGA_SPELL_ID], []);
        session.set_loot_money_persistence_test_result_like_cpp(true);
    }
    session.set_spell_learn_spell_store(Arc::new(wow_data::SpellLearnSpellStoreLikeCpp::default()));
    session.set_spell_required_store(Arc::new(wow_data::SpellRequiredStoreLikeCpp::default()));
    session.set_spell_linked_store(Arc::new(wow_data::SpellLinkedStoreLikeCpp::default()));
    session.set_spell_pet_aura_store(Arc::new(wow_data::SpellPetAuraStoreLikeCpp::default()));
    session.set_spell_target_restrictions_store(Arc::new(
        wow_data::SpellTargetRestrictionsStore::from_entries([]),
    ));
    session.set_spell_aura_restrictions_store(Arc::new(
        wow_data::SpellAuraRestrictionsStore::from_entries([]),
    ));
    let mut coverage = vec![wow_data::SpellAcquisitionCoverageSeedLikeCpp::covered(
        SAGA_SPELL_ID,
        0,
    )];
    let mut spell_effects = vec![saga_summon_effect_like_cpp(SAGA_SPELL_ID)];
    if let Some(learned) = wrapper_learned_spell {
        coverage.push(wow_data::SpellAcquisitionCoverageSeedLikeCpp::covered(
            learned, 0,
        ));
        spell_effects.push(saga_learn_effect_like_cpp(2, SAGA_SPELL_ID, learned));
    }
    session.set_spell_acquisition_catalog(Arc::new(
        wow_data::SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            coverage,
            wow_data::EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects,
                summon_properties: vec![wow_data::SpellAcquisitionSummonPropertiesLikeCpp {
                    record_id: SAGA_SUMMON_PROPERTIES_ID,
                    slot_raw: SAGA_SUMMON_SLOT_MINIPET_RAW,
                    flags_1_raw: SAGA_SUMMON_FROM_JOURNAL_RAW,
                }],
                battle_pet_species: vec![wow_data::SpellAcquisitionBattlePetSpeciesLikeCpp {
                    species_id: SAGA_SPECIES,
                    creature_id_raw: 99,
                }],
                ..Default::default()
            },
            wow_data::SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        ),
    ));
    session.set_known_spells_like_cpp(Vec::new());
    assert!(session.set_complete_represented_player_spell_rows_like_cpp([]));
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.set_complete_represented_override_spells_like_cpp([]));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical player map");
    // Production login hydrates Player::SetFactionForRace before the
    // trainer can be used. This synthetic fixture has no ChrRaces store,
    // so install that canonical interaction prerequisite explicitly.
    session.set_player_faction_template_like_cpp(1);
    assert!(
        session.set_complete_player_skill_records_like_cpp(std::collections::HashMap::new(), 0)
    );
    insert_saga_trainer_creature_like_cpp(&canonical, saga_trainer_guid_like_cpp());
    session.set_player_trainer_interaction_like_cpp(saga_trainer_guid_like_cpp(), TRAINER_ID);
    session.set_battle_pet_purchase_persistence_port_like_cpp(store_handle_like_cpp(&store));
    let attachment = registry
        .attach_like_cpp(ACCOUNT_ID)
        .await
        .expect("saga account attaches");
    session.set_battle_pet_account_attachment_like_cpp(attachment);
    SagaFixtureLikeCpp {
        session,
        send_rx,
        store,
        persistence,
        registry,
    }
}

pub(crate) fn saga_buy_packet_like_cpp(spell_id: i32) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&saga_trainer_guid_like_cpp());
    packet.write_int32(TRAINER_ID as i32);
    packet.write_int32(spell_id);
    packet.reset_read();
    packet
}

pub(crate) const SAGA_WRAPPER_LEARNED_SPELL: u32 = 54_331;

pub(crate) const SAGA_WRAPPER_PRICE: u32 = 25;
