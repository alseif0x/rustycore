//! Battle-pet account regressions.
//!
//! Separated from the battle_pet_account_tests.rs root under #683.

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
        Arc::new(wow_data::BattlePetXpGameTableLikeCpp::from_rows([])),
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

mod scenarios_1;
mod scenarios_2;
