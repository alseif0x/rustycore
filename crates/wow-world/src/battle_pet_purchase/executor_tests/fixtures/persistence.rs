//! Persistence packets.
//!
//! Separated from fixtures.rs under #709.

use super::*;

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

pub(crate) fn owner_of(fixture: &SagaFixtureLikeCpp) -> Arc<BattlePetAccountOwnerLikeCpp> {
    fixture
        .session
        .battle_pet_account_owner_lease_like_cpp()
        .expect("attached owner")
        .0
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
