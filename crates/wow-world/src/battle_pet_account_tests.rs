//! Behaviour tests for [`super`].
//!
//! Extracted from `battle_pet_account.rs`, which was 4,380 lines of which
//! 1,315 — 30% — were this one `mod tests`. The production code and its
//! module boundaries are untouched: moving tests moves no invariant. Dedenting by
//! one level lets rustfmt collapse some argument lists onto a single line, which
//! drops their trailing commas; that is the only difference from the original text.

#![cfg(test)]

use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use wow_data::{
    BATTLE_PET_STATE_STAT_POWER_LIKE_CPP, BATTLE_PET_STATE_STAT_SPEED_LIKE_CPP,
    BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP, BattlePetBreedQualityEntry, BattlePetBreedStateEntry,
    BattlePetSpeciesEntry, BattlePetSpeciesStateEntry,
};

#[derive(Default)]
struct FakePersistenceStateLikeCpp {
    pets: Vec<DurableBattlePetRowLikeCpp>,
    slots: Vec<DurableBattlePetSlotLikeCpp>,
    receipts: HashMap<BattlePetAddRequestKeyLikeCpp, (u32, DurableBattlePetRowLikeCpp)>,
}

#[derive(Default)]
struct FakePersistenceLikeCpp {
    state: Mutex<FakePersistenceStateLikeCpp>,
    process_lease: Arc<AtomicBool>,
    current_fence: Arc<AtomicU64>,
    next_guid: AtomicU64,
    insert_calls: AtomicUsize,
    fail_next_insert: AtomicBool,
    fail_next_update: AtomicBool,
    fail_next_delete: AtomicBool,
    fail_next_slots: AtomicBool,
    block_next_insert: AtomicBool,
    insert_started: Notify,
    allow_insert: Notify,
}

fn add_request_matches_like_cpp(
    requested: &DurableBattlePetRowLikeCpp,
    persisted: &DurableBattlePetRowLikeCpp,
) -> bool {
    requested.species == persisted.species
        && requested.breed == persisted.breed
        && requested.display_id == persisted.display_id
        && requested.level == persisted.level
        && requested.quality == persisted.quality
        && requested.owner_guid_counter == persisted.owner_guid_counter
}

struct FakeProcessLeaseLikeCpp {
    held: Arc<AtomicBool>,
    fence: u64,
}

impl BattlePetProcessLeaseLikeCpp for FakeProcessLeaseLikeCpp {
    fn is_valid_like_cpp(&self) -> bool {
        self.held.load(Ordering::Acquire)
    }

    fn fence_like_cpp(&self) -> u64 {
        self.fence
    }
}

impl Drop for FakeProcessLeaseLikeCpp {
    fn drop(&mut self) {
        self.held.store(false, Ordering::Release);
    }
}

impl BattlePetPersistenceLikeCpp for FakePersistenceLikeCpp {
    fn try_acquire_process_lease<'a>(
        &'a self,
        _account_id: u32,
    ) -> PersistenceFuture<
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
                    Box::new(FakeProcessLeaseLikeCpp {
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
    ) -> PersistenceFuture<
        'a,
        Result<LoadedBattlePetAccountLikeCpp, BattlePetPersistenceErrorLikeCpp>,
    > {
        Box::pin(async move {
            let state = self.state.lock().expect("fake persistence poisoned");
            Ok(LoadedBattlePetAccountLikeCpp {
                pets: state.pets.clone(),
                slots: state.slots.clone(),
            })
        })
    }

    fn allocate_guid_counter_like_cpp(
        &self,
    ) -> PersistenceFuture<'_, Result<u64, BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            let counter = self.next_guid.fetch_add(1, Ordering::AcqRel);
            if counter == 0 {
                Err(BattlePetPersistenceErrorLikeCpp::GuidCollision)
            } else {
                Ok(counter)
            }
        })
    }

    fn insert_pet_idempotently<'a>(
        &'a self,
        request: DurableBattlePetAddLikeCpp,
    ) -> PersistenceFuture<
        'a,
        Result<PersistBattlePetAddOutcomeLikeCpp, BattlePetPersistenceErrorLikeCpp>,
    > {
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
            let mut state = self.state.lock().expect("fake persistence poisoned");
            if let Some((receipt_account_id, existing)) =
                state.receipts.get(&request.request_key).cloned()
            {
                let still_present = state
                    .pets
                    .iter()
                    .any(|pet| pet.guid_counter == existing.guid_counter);
                return if receipt_account_id == request.account_id
                    && add_request_matches_like_cpp(&request.pet, &existing)
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
                .filter(|pet| {
                    if request.pet.owner_guid_counter.is_none() {
                        pet.owner_guid_counter.is_none()
                    } else {
                        pet.owner_guid_counter == request.pet.owner_guid_counter
                    }
                })
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
            Ok(PersistBattlePetAddOutcomeLikeCpp::Inserted)
        })
    }

    fn lookup_add_request<'a>(
        &'a self,
        account_id: u32,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> PersistenceFuture<
        'a,
        Result<Option<DurableBattlePetAddReceiptLikeCpp>, BattlePetPersistenceErrorLikeCpp>,
    > {
        Box::pin(async move {
            let state = self.state.lock().expect("fake persistence poisoned");
            let Some((receipt_account_id, pet)) = state.receipts.get(&request_key).cloned() else {
                return Ok(None);
            };
            if receipt_account_id != account_id {
                return Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest);
            }
            Ok(Some({
                let current_pet = state
                    .pets
                    .iter()
                    .find(|existing| existing.guid_counter == pet.guid_counter)
                    .cloned();
                DurableBattlePetAddReceiptLikeCpp {
                    account_id: receipt_account_id,
                    requested_pet: pet,
                    current_pet,
                }
            }))
        })
    }

    fn update_pet<'a>(
        &'a self,
        _account_id: u32,
        fence: u64,
        pet: DurableBattlePetRowLikeCpp,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            if self.current_fence.load(Ordering::Acquire) != fence {
                return Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
            }
            if self.fail_next_update.swap(false, Ordering::AcqRel) {
                return Err(BattlePetPersistenceErrorLikeCpp::Database(
                    "injected update failure".to_string(),
                ));
            }
            let mut state = self.state.lock().expect("fake persistence poisoned");
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
        slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            if self.current_fence.load(Ordering::Acquire) != fence {
                return Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
            }
            if self.fail_next_delete.swap(false, Ordering::AcqRel) {
                return Err(BattlePetPersistenceErrorLikeCpp::Database(
                    "injected delete failure".to_string(),
                ));
            }
            let mut state = self.state.lock().expect("fake persistence poisoned");
            state
                .pets
                .retain(|pet| pet.guid_counter != pet_guid_counter);
            state.slots = slots;
            Ok(())
        })
    }

    fn replace_slots<'a>(
        &'a self,
        _account_id: u32,
        fence: u64,
        slots: Vec<DurableBattlePetSlotLikeCpp>,
    ) -> PersistenceFuture<'a, Result<(), BattlePetPersistenceErrorLikeCpp>> {
        Box::pin(async move {
            if self.current_fence.load(Ordering::Acquire) != fence {
                return Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
            }
            if self.fail_next_slots.swap(false, Ordering::AcqRel) {
                return Err(BattlePetPersistenceErrorLikeCpp::Database(
                    "injected slot failure".to_string(),
                ));
            }
            self.state.lock().expect("fake persistence poisoned").slots = slots;
            Ok(())
        })
    }
}

fn stores_like_cpp(
    species_flags: i32,
) -> (
    Arc<BattlePetSpeciesStore>,
    Arc<BattlePetBreedQualityStore>,
    Arc<BattlePetBreedStateStore>,
    Arc<BattlePetSpeciesStateStore>,
) {
    let species = Arc::new(BattlePetSpeciesStore::from_entries([
        BattlePetSpeciesEntry {
            id: 11,
            description: String::new(),
            source_text: String::new(),
            creature_id: 99,
            summon_spell_id: 0,
            icon_file_data_id: 0,
            pet_type_enum: 0,
            flags: BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP | species_flags,
            source_type_enum: 0,
            card_ui_model_scene_id: 0,
            loadout_ui_model_scene_id: 0,
        },
        BattlePetSpeciesEntry {
            id: 12,
            description: String::new(),
            source_text: String::new(),
            creature_id: 100,
            summon_spell_id: 0,
            icon_file_data_id: 0,
            pet_type_enum: 0,
            flags: BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP
                | BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP,
            source_type_enum: 0,
            card_ui_model_scene_id: 0,
            loadout_ui_model_scene_id: 0,
        },
    ]));
    let qualities = Arc::new(BattlePetBreedQualityStore::from_entries([
        BattlePetBreedQualityEntry {
            id: 1,
            state_multiplier: 1.0,
            quality_enum: 1,
        },
    ]));
    let breed_states = Arc::new(BattlePetBreedStateStore::from_entries([
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
    ]));
    let species_states = Arc::new(BattlePetSpeciesStateStore::from_entries([
        BattlePetSpeciesStateEntry {
            id: 1,
            battle_pet_state_id: BATTLE_PET_STATE_STAT_STAMINA_LIKE_CPP,
            value: 100,
            battle_pet_species_id: 11,
        },
    ]));
    (species, qualities, breed_states, species_states)
}

fn registry_like_cpp(
    persistence: Arc<FakePersistenceLikeCpp>,
    species_flags: i32,
    next_guid: i64,
) -> BattlePetAccountRegistryLikeCpp {
    persistence
        .next_guid
        .store(next_guid as u64, Ordering::Release);
    let (species, qualities, breed_states, species_states) = stores_like_cpp(species_flags);
    BattlePetAccountRegistryLikeCpp::new_with_persistence_like_cpp(
        persistence,
        species,
        qualities,
        breed_states,
        species_states,
        7,
        0x0102_0007,
    )
}

fn add_request_like_cpp(key: u8, species: u32, owner: u64) -> BattlePetAddRequestLikeCpp {
    let mut request_key = [0; 16];
    request_key[0] = key;
    BattlePetAddRequestLikeCpp {
        request_key: BattlePetAddRequestKeyLikeCpp::from_bytes(request_key),
        species,
        display_id: 123,
        breed: 7,
        quality: 1,
        level: 1,
        owner_guid: Some(ObjectGuid::create_player(7, owner as i64)),
    }
}

fn durable_pet_row_like_cpp(
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

#[test]
fn source_item_guid_is_a_restart_stable_add_request_identity_like_cpp() {
    let first = ObjectGuid::create_item(7, 41);
    let second = ObjectGuid::create_item(7, 42);
    assert_eq!(
        BattlePetAddRequestKeyLikeCpp::from_source_guid_bytes_like_cpp(first.to_raw_bytes())
            .expect("nonempty item guid")
            .as_bytes(),
        first.to_raw_bytes()
    );
    assert_ne!(
        BattlePetAddRequestKeyLikeCpp::from_source_guid_bytes_like_cpp(first.to_raw_bytes()),
        BattlePetAddRequestKeyLikeCpp::from_source_guid_bytes_like_cpp(second.to_raw_bytes())
    );
    assert_eq!(
        BattlePetAddRequestKeyLikeCpp::from_source_guid_bytes_like_cpp(
            ObjectGuid::EMPTY.to_raw_bytes()
        ),
        None
    );
}

#[tokio::test]
async fn zero_battlenet_account_never_enters_the_shared_registry() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    let registry = registry_like_cpp(persistence, 0, 10);
    assert!(registry.attach_like_cpp(0).await.is_err());
    assert!(registry.accounts.is_empty());
}

#[tokio::test]
async fn lost_process_lease_is_revalidated_before_more_mutation() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    let registry = registry_like_cpp(Arc::clone(&persistence), 0, 20);
    let attachment = registry.attach_like_cpp(77).await.expect("attach");
    assert!(attachment.try_acquire_lease_like_cpp().await);

    // Models the monitor observing that MySQL released the dedicated
    // session lock. A stale in-memory guard must stop authorizing writes.
    persistence.process_lease.store(false, Ordering::Release);
    assert!(!attachment.has_lease_like_cpp());
    assert!(matches!(
        attachment
            .owner_like_cpp()
            .try_add_pet_like_cpp(
                attachment.lease_id_like_cpp(),
                add_request_like_cpp(41, 11, 1),
            )
            .await,
        Err(BattlePetAddFailureLikeCpp::MissingAuthority)
    ));

    assert!(attachment.try_acquire_lease_like_cpp().await);
    assert!(attachment.has_lease_like_cpp());
}

#[tokio::test]
async fn one_source_item_request_cannot_grant_pets_to_two_accounts() {
    let persistence = FakePersistenceLikeCpp::default();
    persistence.next_guid.store(30, Ordering::Release);
    let request_key = BattlePetAddRequestKeyLikeCpp::from_bytes([7; 16]);
    let first = DurableBattlePetAddLikeCpp {
        account_id: 77,
        realm_id: 7,
        request_key,
        max_per_scope: 3,
        fence: 0,
        pet: durable_pet_row_like_cpp(30, 11, None),
    };
    assert_eq!(
        persistence.insert_pet_idempotently(first.clone()).await,
        Ok(PersistBattlePetAddOutcomeLikeCpp::Inserted)
    );
    let mut second = first;
    second.account_id = 88;
    second.pet.guid_counter = 31;
    assert_eq!(
        persistence.insert_pet_idempotently(second).await,
        Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest)
    );
    assert_eq!(
        persistence
            .state
            .lock()
            .expect("fake persistence poisoned")
            .pets
            .len(),
        1
    );
}

#[tokio::test]
async fn invalid_owner_shape_rows_do_not_consume_account_wide_capacity() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    persistence
        .state
        .lock()
        .expect("fake persistence poisoned")
        .pets = vec![
        durable_pet_row_like_cpp(1, 11, Some(100)),
        durable_pet_row_like_cpp(2, 11, Some(101)),
        durable_pet_row_like_cpp(3, 11, Some(102)),
    ];
    let registry = registry_like_cpp(Arc::clone(&persistence), 0, 40);
    let attachment = registry.attach_like_cpp(77).await.expect("attach");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    assert!(
        attachment
            .owner_like_cpp()
            .try_add_pet_like_cpp(
                attachment.lease_id_like_cpp(),
                add_request_like_cpp(42, 11, 1),
            )
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn registry_evicts_only_after_the_last_detached_operation_finishes() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    persistence.block_next_insert.store(true, Ordering::Release);
    let registry = Arc::new(registry_like_cpp(Arc::clone(&persistence), 0, 50));
    let attachment = registry.attach_like_cpp(77).await.expect("attach");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let owner = Arc::clone(attachment.owner_like_cpp());
    let lease = attachment.lease_id_like_cpp();
    let task = tokio::spawn(async move {
        owner
            .try_add_pet_like_cpp(lease, add_request_like_cpp(43, 11, 1))
            .await
    });
    persistence.insert_started.notified().await;
    drop(attachment);
    assert_eq!(registry.accounts.len(), 1);

    persistence.allow_insert.notify_one();
    task.await.expect("add task").expect("durable add");
    tokio::task::yield_now().await;
    assert!(registry.accounts.is_empty());
}

#[tokio::test]
async fn former_process_cannot_commit_or_publish_after_fence_handoff() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    persistence.block_next_insert.store(true, Ordering::Release);
    let first_registry = registry_like_cpp(Arc::clone(&persistence), 0, 70);
    let first = first_registry
        .attach_like_cpp(77)
        .await
        .expect("attach first");
    assert!(first.try_acquire_lease_like_cpp().await);
    let first_owner = Arc::clone(first.owner_like_cpp());
    let first_lease = first.lease_id_like_cpp();
    let stale = tokio::spawn(async move {
        first_owner
            .try_add_pet_like_cpp(first_lease, add_request_like_cpp(44, 11, 1))
            .await
    });
    persistence.insert_started.notified().await;

    // Models MySQL releasing the old process' named lock and another
    // world-server advancing the durable account epoch before the queued
    // insert enters its transaction.
    persistence.process_lease.store(false, Ordering::Release);
    let second_registry = registry_like_cpp(Arc::clone(&persistence), 0, 80);
    let second = second_registry
        .attach_like_cpp(77)
        .await
        .expect("attach replacement");
    assert!(second.try_acquire_lease_like_cpp().await);

    persistence.allow_insert.notify_one();
    assert_eq!(
        stale.await.expect("stale add task"),
        Err(BattlePetAddFailureLikeCpp::MissingAuthority)
    );
    assert!(
        persistence
            .state
            .lock()
            .expect("fake persistence poisoned")
            .pets
            .is_empty()
    );
}

#[test]
fn durable_uncage_schema_uses_global_key_and_unsigned_account_domain() {
    let sql = include_str!("../../../sql/updates/auth/wotlk_classic/2026_08_03_00_auth.sql");
    assert!(sql.contains("`battlenetAccountId` int unsigned NOT NULL"));
    assert!(sql.contains("DELETE duplicateReceipt"));
    assert!(sql.contains("canonicalReceipt.`battlePetGuid` < duplicateReceipt.`battlePetGuid`"));
    assert!(sql.contains("ADD PRIMARY KEY (`requestKey`)"));
    assert!(sql.contains("DROP PRIMARY KEY"));
    assert!(sql.contains("CREATE TABLE IF NOT EXISTS `battle_pet_account_fences`"));
    assert!(!sql.contains("PRIMARY KEY (`battlenetAccountId`, `requestKey`)"));
}

#[tokio::test]
async fn two_realm_owners_share_one_global_guid_sequence_without_startup_exclusion() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    persistence.next_guid.store(500, Ordering::Release);

    // Two world-server processes may share the Login DB. Their allocator
    // calls serialize only the individual reservation and neither needs a
    // process-lifetime lease over the database.
    let (first, second) = tokio::join!(
        persistence.allocate_guid_counter_like_cpp(),
        persistence.allocate_guid_counter_like_cpp()
    );
    let allocated = HashSet::from([first.unwrap(), second.unwrap()]);
    assert_eq!(allocated, HashSet::from([500, 501]));
    assert_eq!(persistence.next_guid.load(Ordering::Acquire), 502);
}

#[test]
fn loaded_rows_apply_cpp_species_owner_capacity_and_slot_validation_like_cpp() {
    let (species, qualities, breed_states, species_states) =
        stores_like_cpp(BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP);
    let loaded = LoadedBattlePetAccountLikeCpp {
        pets: vec![
            durable_pet_row_like_cpp(1, 999, None),
            durable_pet_row_like_cpp(2, 11, None),
            durable_pet_row_like_cpp(3, 11, Some(100)),
            durable_pet_row_like_cpp(4, 11, Some(100)),
            durable_pet_row_like_cpp(5, 11, Some(100)),
            durable_pet_row_like_cpp(6, 11, Some(100)),
            durable_pet_row_like_cpp(7, 11, Some(200)),
            durable_pet_row_like_cpp(8, 12, Some(100)),
            durable_pet_row_like_cpp(9, 12, None),
            durable_pet_row_like_cpp(10, 12, None),
        ],
        slots: vec![
            DurableBattlePetSlotLikeCpp {
                index: 0,
                pet_guid_counter: Some(3),
                locked: false,
            },
            DurableBattlePetSlotLikeCpp {
                index: 1,
                pet_guid_counter: Some(6),
                locked: false,
            },
        ],
    };
    let owner = BattlePetAccountOwnerLikeCpp::from_loaded_like_cpp(
        77,
        7,
        0x0102_0007,
        Arc::new(FakePersistenceLikeCpp::default()),
        species,
        qualities,
        breed_states,
        species_states,
        loaded,
    );
    let state = owner.state.lock().expect("battle-pet account state");
    let accepted = state
        .pets
        .keys()
        .map(|guid| guid.counter() as u64)
        .collect::<HashSet<_>>();
    assert_eq!(accepted, HashSet::from([3, 4, 5, 7, 9]));
    assert_eq!(state.slots[0].pet_guid, Some(battle_pet_guid_like_cpp(3)));
    assert_eq!(state.slots[1].pet_guid, None);
}

#[test]
fn unique_species_criteria_count_keeps_pending_removed_pets_like_cpp() {
    let (species, qualities, breed_states, species_states) = stores_like_cpp(0);
    let owner = BattlePetAccountOwnerLikeCpp::from_loaded_like_cpp(
        77,
        7,
        0x0102_0007,
        Arc::new(FakePersistenceLikeCpp::default()),
        species,
        qualities,
        breed_states,
        species_states,
        LoadedBattlePetAccountLikeCpp {
            pets: vec![
                durable_pet_row_like_cpp(1, 11, None),
                durable_pet_row_like_cpp(2, 12, None),
            ],
            slots: Vec::new(),
        },
    );
    owner
        .state
        .lock()
        .expect("battle-pet account state")
        .pets
        .get_mut(&battle_pet_guid_like_cpp(1))
        .expect("loaded pet")
        .save_info = RepresentedBattlePetSaveInfoLikeCpp::Removed;

    assert_eq!(owner.unique_species_count_like_cpp(), 2);
}

#[tokio::test]
async fn one_account_lease_and_pending_capacity_are_atomic_like_cpp() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    let registry = registry_like_cpp(Arc::clone(&persistence), 0, 100);
    let first = registry.attach_like_cpp(77).await.expect("first attach");
    let second = registry.attach_like_cpp(77).await.expect("second attach");
    assert!(first.try_acquire_lease_like_cpp().await);
    assert!(!second.try_acquire_lease_like_cpp().await);

    let owner = Arc::clone(first.owner_like_cpp());
    let lease = first.lease_id_like_cpp();
    assert!(matches!(
        owner
            .try_add_pet_like_cpp(second.lease_id_like_cpp(), add_request_like_cpp(99, 11, 1),)
            .await,
        Err(BattlePetAddFailureLikeCpp::JournalLocked)
    ));
    let (a, b, c, d) = tokio::join!(
        owner.try_add_pet_like_cpp(lease, add_request_like_cpp(1, 11, 1)),
        owner.try_add_pet_like_cpp(lease, add_request_like_cpp(2, 11, 1)),
        owner.try_add_pet_like_cpp(lease, add_request_like_cpp(3, 11, 1)),
        owner.try_add_pet_like_cpp(lease, add_request_like_cpp(4, 11, 1)),
    );
    let outcomes = [a, b, c, d];
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 3);
    assert_eq!(
        outcomes
            .iter()
            .filter(|result| matches!(result, Err(BattlePetAddFailureLikeCpp::Capacity)))
            .count(),
        1
    );
    let state = persistence.state.lock().expect("fake persistence poisoned");
    assert_eq!(state.pets.len(), 3);
    assert_eq!(
        state
            .pets
            .iter()
            .map(|pet| pet.guid_counter)
            .collect::<HashSet<_>>()
            .len(),
        3
    );
}

#[tokio::test]
async fn cross_process_lease_handoff_reloads_before_next_realm_mutates() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    persistence.next_guid.store(300, Ordering::Release);
    let (species, qualities, breed_states, species_states) = stores_like_cpp(0);
    let first_registry = BattlePetAccountRegistryLikeCpp::new_with_persistence_like_cpp(
        persistence.clone(),
        species.clone(),
        qualities.clone(),
        breed_states.clone(),
        species_states.clone(),
        7,
        0x0102_0007,
    );
    let second_registry = BattlePetAccountRegistryLikeCpp::new_with_persistence_like_cpp(
        persistence.clone(),
        species,
        qualities,
        breed_states,
        species_states,
        8,
        0x0102_0008,
    );
    let first = first_registry.attach_like_cpp(77).await.expect("realm one");
    let second = second_registry
        .attach_like_cpp(77)
        .await
        .expect("realm two");
    assert!(first.try_acquire_lease_like_cpp().await);
    assert!(!second.try_acquire_lease_like_cpp().await);
    first
        .owner_like_cpp()
        .try_add_pet_like_cpp(first.lease_id_like_cpp(), add_request_like_cpp(1, 11, 1))
        .await
        .expect("first durable pet");
    first
        .owner_like_cpp()
        .try_add_pet_like_cpp(first.lease_id_like_cpp(), add_request_like_cpp(2, 11, 1))
        .await
        .expect("second durable pet");

    drop(first);

    assert!(second.try_acquire_lease_like_cpp().await);
    assert_eq!(
        second
            .owner_like_cpp()
            .journal_like_cpp(second.lease_id_like_cpp(), None)
            .pets
            .len(),
        2
    );
    second
        .owner_like_cpp()
        .try_add_pet_like_cpp(second.lease_id_like_cpp(), add_request_like_cpp(3, 11, 1))
        .await
        .expect("third durable pet after process handoff");
    assert_eq!(
        second
            .owner_like_cpp()
            .try_add_pet_like_cpp(second.lease_id_like_cpp(), add_request_like_cpp(4, 11, 1),)
            .await,
        Err(BattlePetAddFailureLikeCpp::Capacity)
    );
    assert_eq!(
        persistence
            .state
            .lock()
            .expect("fake persistence poisoned")
            .pets
            .len(),
        3
    );
}

#[tokio::test]
async fn clear_fanfare_persists_without_journal_lease_like_cpp() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    let registry = registry_like_cpp(persistence, 0, 400);
    let holder = registry.attach_like_cpp(77).await.expect("holder");
    let non_holder = registry.attach_like_cpp(77).await.expect("non-holder");
    assert!(holder.try_acquire_lease_like_cpp().await);
    assert!(!non_holder.try_acquire_lease_like_cpp().await);
    let added = holder
        .owner_like_cpp()
        .try_add_pet_like_cpp(holder.lease_id_like_cpp(), add_request_like_cpp(1, 11, 1))
        .await
        .expect("add pet");
    let guid = match added {
        BattlePetAddOutcomeLikeCpp::Added(pet) => pet.guid,
        BattlePetAddOutcomeLikeCpp::Replayed(_) => panic!("fresh add must not replay"),
    };
    holder
        .owner_like_cpp()
        .try_mutate_pet_like_cpp(holder.lease_id_like_cpp(), guid, |pet| {
            pet.flags |= crate::session::BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP;
        })
        .await
        .expect("seed fanfare");
    assert!(matches!(
        non_holder
            .owner_like_cpp()
            .try_mutate_pet_like_cpp(non_holder.lease_id_like_cpp(), guid, |_| {})
            .await,
        Err(BattlePetMutationFailureLikeCpp::JournalLocked)
    ));
    let (_, packet) = non_holder
        .owner_like_cpp()
        .try_mutate_pet_without_lease_like_cpp(guid, |pet| {
            pet.flags &= !crate::session::BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP;
        })
        .await
        .expect("C++ clear-fanfare exception");
    assert_eq!(
        packet.flags & crate::session::BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP,
        0
    );
}

#[tokio::test]
async fn concurrent_same_request_inserts_and_publishes_once_like_cpp() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    let registry = registry_like_cpp(Arc::clone(&persistence), 0, 200);
    let attachment = registry.attach_like_cpp(77).await.expect("attach");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let owner = attachment.owner_like_cpp();
    let lease = attachment.lease_id_like_cpp();
    let request = add_request_like_cpp(9, 11, 1);
    let (first, second) = tokio::join!(
        owner.try_add_pet_like_cpp(lease, request.clone()),
        owner.try_add_pet_like_cpp(lease, request),
    );
    let outcomes = [first, second];
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Ok(BattlePetAddOutcomeLikeCpp::Added(_))))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| matches!(outcome, Ok(BattlePetAddOutcomeLikeCpp::Replayed(_))))
            .count(),
        1
    );
    assert_eq!(persistence.insert_calls.load(Ordering::Relaxed), 1);
    assert_eq!(
        persistence
            .state
            .lock()
            .expect("fake persistence poisoned")
            .pets
            .len(),
        1
    );
}

#[tokio::test]
async fn failed_insert_publishes_nothing_and_retry_can_commit_like_cpp() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    persistence.fail_next_insert.store(true, Ordering::Release);
    let registry = registry_like_cpp(Arc::clone(&persistence), 0, 300);
    let attachment = registry.attach_like_cpp(77).await.expect("attach");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let owner = attachment.owner_like_cpp();
    let lease = attachment.lease_id_like_cpp();
    let request = add_request_like_cpp(10, 11, 1);
    assert!(matches!(
        owner.try_add_pet_like_cpp(lease, request.clone()).await,
        Err(BattlePetAddFailureLikeCpp::DatabaseFailure(_))
    ));
    assert!(
        owner
            .journal_like_cpp(lease, request.owner_guid)
            .pets
            .is_empty()
    );
    assert!(matches!(
        owner.try_add_pet_like_cpp(lease, request.clone()).await,
        Ok(BattlePetAddOutcomeLikeCpp::Added(_))
    ));
    assert_eq!(
        owner.journal_like_cpp(lease, request.owner_guid).pets.len(),
        1
    );
}

#[tokio::test]
async fn durable_pet_and_slots_reload_without_a_session_mirror_like_cpp() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    let registry = registry_like_cpp(
        Arc::clone(&persistence),
        BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP,
        400,
    );
    let attachment = registry.attach_like_cpp(77).await.expect("attach");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let owner = attachment.owner_like_cpp();
    let lease = attachment.lease_id_like_cpp();
    let request = add_request_like_cpp(11, 11, 1);
    let pet_guid = match owner
        .try_add_pet_like_cpp(lease, request.clone())
        .await
        .expect("add")
    {
        BattlePetAddOutcomeLikeCpp::Added(pet) | BattlePetAddOutcomeLikeCpp::Replayed(pet) => {
            pet.guid
        }
    };
    owner
        .try_set_slot_like_cpp(lease, pet_guid, 0)
        .await
        .expect("persist slot");
    drop(attachment);
    drop(registry);

    let restarted = registry_like_cpp(
        Arc::clone(&persistence),
        BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP,
        401,
    );
    let attachment = restarted.attach_like_cpp(77).await.expect("reload");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let journal = attachment
        .owner_like_cpp()
        .journal_like_cpp(attachment.lease_id_like_cpp(), request.owner_guid);
    assert_eq!(journal.pets.len(), 1);
    assert_eq!(journal.slots[0].pet_guid, pet_guid);
    assert_eq!(
        journal.pets[0].owner_info.unwrap().player_virtual_realm,
        0x0102_0007
    );
}

#[tokio::test]
async fn legacy_unique_and_owner_specific_caps_use_the_effective_scope_like_cpp() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    let registry = registry_like_cpp(
        Arc::clone(&persistence),
        BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP,
        500,
    );
    let attachment = registry.attach_like_cpp(77).await.expect("attach");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let owner = attachment.owner_like_cpp();
    let lease = attachment.lease_id_like_cpp();
    for key in 1..=3 {
        owner
            .try_add_pet_like_cpp(lease, add_request_like_cpp(key, 11, 1))
            .await
            .expect("first owner normal-cap pet");
    }
    assert!(matches!(
        owner
            .try_add_pet_like_cpp(lease, add_request_like_cpp(4, 11, 1))
            .await,
        Err(BattlePetAddFailureLikeCpp::Capacity)
    ));
    owner
        .try_add_pet_like_cpp(lease, add_request_like_cpp(5, 11, 2))
        .await
        .expect("second owner has independent normal cap");
    owner
        .try_add_pet_like_cpp(lease, add_request_like_cpp(6, 12, 1))
        .await
        .expect("first legacy-unique pet");
    assert!(matches!(
        owner
            .try_add_pet_like_cpp(lease, add_request_like_cpp(7, 12, 1))
            .await,
        Err(BattlePetAddFailureLikeCpp::Capacity)
    ));
}

#[tokio::test]
async fn cancelled_session_does_not_cancel_reserved_insert_like_cpp() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    persistence.block_next_insert.store(true, Ordering::Release);
    let registry = registry_like_cpp(Arc::clone(&persistence), 0, 600);
    let attachment = registry.attach_like_cpp(77).await.expect("attach");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let owner = Arc::clone(attachment.owner_like_cpp());
    let lease = attachment.lease_id_like_cpp();
    let request = add_request_like_cpp(21, 11, 1);
    let request_for_worker = request.clone();
    let owner_for_worker = Arc::clone(&owner);
    let session_task = tokio::spawn(async move {
        owner_for_worker
            .try_add_pet_like_cpp(lease, request_for_worker)
            .await
    });

    persistence.insert_started.notified().await;
    session_task.abort();
    let _ = session_task.await;
    persistence.allow_insert.notify_one();

    assert!(registry.drain_like_cpp(Duration::from_secs(1)).await);
    assert_eq!(
        owner.journal_like_cpp(lease, request.owner_guid).pets.len(),
        1
    );
    assert!(matches!(
        owner.try_add_pet_like_cpp(lease, request).await,
        Ok(BattlePetAddOutcomeLikeCpp::Replayed(_))
    ));
    assert_eq!(persistence.insert_calls.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn historical_retry_receipt_does_not_revive_deleted_pet_like_cpp() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    let registry = registry_like_cpp(Arc::clone(&persistence), 0, 700);
    let attachment = registry.attach_like_cpp(77).await.expect("attach");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let owner = attachment.owner_like_cpp();
    let lease = attachment.lease_id_like_cpp();
    let request = add_request_like_cpp(22, 11, 1);
    let pet_guid = match owner
        .try_add_pet_like_cpp(lease, request.clone())
        .await
        .expect("add")
    {
        BattlePetAddOutcomeLikeCpp::Added(pet) | BattlePetAddOutcomeLikeCpp::Replayed(pet) => {
            pet.guid
        }
    };
    owner
        .try_remove_pet_like_cpp(lease, pet_guid)
        .await
        .expect("delete");
    drop(attachment);
    drop(registry);

    let restarted = registry_like_cpp(Arc::clone(&persistence), 0, 701);
    let attachment = restarted.attach_like_cpp(77).await.expect("reload");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let owner = attachment.owner_like_cpp();
    assert!(matches!(
        owner
            .try_add_pet_like_cpp(attachment.lease_id_like_cpp(), request.clone())
            .await,
        Err(BattlePetAddFailureLikeCpp::DuplicateRequest)
    ));
    assert!(
        owner
            .journal_like_cpp(attachment.lease_id_like_cpp(), request.owner_guid)
            .pets
            .is_empty()
    );
}

#[tokio::test]
async fn restart_retry_is_replayed_before_capacity_check_like_cpp() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    let registry = registry_like_cpp(Arc::clone(&persistence), 0, 750);
    let attachment = registry.attach_like_cpp(77).await.expect("attach");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let owner = attachment.owner_like_cpp();
    let lease = attachment.lease_id_like_cpp();
    let first_request = add_request_like_cpp(31, 11, 1);
    let first_guid = match owner
        .try_add_pet_like_cpp(lease, first_request.clone())
        .await
        .expect("first capped pet")
    {
        BattlePetAddOutcomeLikeCpp::Added(pet) | BattlePetAddOutcomeLikeCpp::Replayed(pet) => {
            pet.guid
        }
    };
    for request in [
        add_request_like_cpp(32, 11, 1),
        add_request_like_cpp(33, 11, 1),
    ] {
        owner
            .try_add_pet_like_cpp(lease, request)
            .await
            .expect("fill species cap");
    }
    owner
        .try_mutate_pet_like_cpp(lease, first_guid, |pet| pet.level = 9)
        .await
        .expect("persist post-add mutation");
    drop(attachment);
    drop(registry);

    let restarted = registry_like_cpp(Arc::clone(&persistence), 0, 751);
    let attachment = restarted.attach_like_cpp(77).await.expect("reload");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let replayed = attachment
        .owner_like_cpp()
        .try_add_pet_like_cpp(attachment.lease_id_like_cpp(), first_request)
        .await
        .expect("retry before cap");
    assert!(matches!(
        replayed,
        BattlePetAddOutcomeLikeCpp::Replayed(ref pet) if pet.level == 9
    ));
    assert!(matches!(
        attachment
            .owner_like_cpp()
            .try_add_pet_like_cpp(
                attachment.lease_id_like_cpp(),
                add_request_like_cpp(31, 11, 1),
            )
            .await,
        Ok(BattlePetAddOutcomeLikeCpp::Replayed(ref pet)) if pet.level == 9
    ));
}

#[tokio::test]
async fn unchanged_pet_fields_can_still_replace_declined_names_like_cpp() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    let registry = registry_like_cpp(Arc::clone(&persistence), 0, 775);
    let attachment = registry.attach_like_cpp(77).await.expect("attach");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let owner = attachment.owner_like_cpp();
    let lease = attachment.lease_id_like_cpp();
    let pet_guid = match owner
        .try_add_pet_like_cpp(lease, add_request_like_cpp(34, 11, 1))
        .await
        .expect("add")
    {
        BattlePetAddOutcomeLikeCpp::Added(pet) | BattlePetAddOutcomeLikeCpp::Replayed(pet) => {
            pet.guid
        }
    };
    let first = DeclinedNamesLikeCpp {
        names: std::array::from_fn(|index| format!("first-{index}")),
    };
    owner
        .try_mutate_pet_like_cpp(lease, pet_guid, move |pet| pet.declined_names = Some(first))
        .await
        .expect("first declined forms");
    let replacement = DeclinedNamesLikeCpp {
        names: std::array::from_fn(|index| format!("replacement-{index}")),
    };
    let durable_replacement = BattlePetDeclinedNamesLikeCpp {
        names: replacement.names.clone(),
    };
    let replacement_for_mutation = replacement.clone();
    owner
        .try_mutate_pet_like_cpp(lease, pet_guid, move |pet| {
            pet.declined_names = Some(replacement_for_mutation)
        })
        .await
        .expect("replace declined forms without changing main row");
    assert_eq!(
        owner
            .pet_snapshot_like_cpp(pet_guid)
            .expect("published pet")
            .declined_names,
        Some(replacement.clone())
    );
    assert_eq!(
        persistence
            .state
            .lock()
            .expect("fake persistence poisoned")
            .pets
            .iter()
            .find(|pet| pet.guid_counter == pet_guid.counter() as u64)
            .expect("durable pet")
            .declined_names,
        Some(durable_replacement)
    );
}

#[tokio::test]
async fn failed_update_delete_and_slot_persistence_publish_nothing_like_cpp() {
    let persistence = Arc::new(FakePersistenceLikeCpp::default());
    let registry = registry_like_cpp(Arc::clone(&persistence), 0, 800);
    let attachment = registry.attach_like_cpp(77).await.expect("attach");
    assert!(attachment.try_acquire_lease_like_cpp().await);
    let owner = attachment.owner_like_cpp();
    let lease = attachment.lease_id_like_cpp();
    let request = add_request_like_cpp(23, 11, 1);
    let pet_guid = match owner
        .try_add_pet_like_cpp(lease, request.clone())
        .await
        .expect("add")
    {
        BattlePetAddOutcomeLikeCpp::Added(pet) | BattlePetAddOutcomeLikeCpp::Replayed(pet) => {
            pet.guid
        }
    };
    let original = owner.pet_snapshot_like_cpp(pet_guid).expect("pet");

    persistence.fail_next_update.store(true, Ordering::Release);
    assert!(matches!(
        owner
            .try_mutate_pet_like_cpp(lease, pet_guid, |pet| pet.level = 9)
            .await,
        Err(BattlePetMutationFailureLikeCpp::DatabaseFailure(_))
    ));
    assert_eq!(owner.pet_snapshot_like_cpp(pet_guid), Some(original));

    persistence.fail_next_slots.store(true, Ordering::Release);
    assert!(matches!(
        owner.try_set_slot_like_cpp(lease, pet_guid, 0).await,
        Err(BattlePetMutationFailureLikeCpp::DatabaseFailure(_))
    ));
    assert_eq!(
        owner.journal_like_cpp(lease, request.owner_guid).slots[0].pet_guid,
        empty_battle_pet_guid_like_cpp()
    );

    persistence.fail_next_delete.store(true, Ordering::Release);
    assert!(matches!(
        owner.try_remove_pet_like_cpp(lease, pet_guid).await,
        Err(BattlePetMutationFailureLikeCpp::DatabaseFailure(_))
    ));
    assert!(owner.pet_snapshot_like_cpp(pet_guid).is_some());
}
