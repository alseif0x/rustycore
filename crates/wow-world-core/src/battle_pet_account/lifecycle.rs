//! Lifecycle operations of battle_pet_account.
//!
//! Divided out of the single inherent impl under #705; every method keeps
//! its name, signature and body.

use super::*;

impl BattlePetAccountOwnerLikeCpp {
    pub(super) fn from_loaded_like_cpp(
        account_id: u32,
        realm_id: u16,
        virtual_realm_address: u32,
        persistence: Arc<dyn BattlePetPersistenceLikeCpp>,
        species_store: Arc<BattlePetSpeciesStore>,
        breed_quality_store: Arc<BattlePetBreedQualityStore>,
        breed_state_store: Arc<BattlePetBreedStateStore>,
        species_state_store: Arc<BattlePetSpeciesStateStore>,
        xp_game_table: Arc<BattlePetXpGameTableLikeCpp>,
        loaded: LoadedBattlePetAccountLikeCpp,
    ) -> Self {
        // C++ `BattlePetMgr::LoadFromDB` validates each row before it enters
        // `_pets`; capacity is therefore evaluated against only earlier valid
        // rows in the database result order.
        let mut pets = HashMap::new();
        for row in loaded.pets {
            let Some(species) = species_store.get(row.species) else {
                continue;
            };
            let not_account_wide =
                species.has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP);
            if not_account_wide != row.owner_guid_counter.is_some() {
                continue;
            }
            let owner_guid = row
                .owner_guid_counter
                .map(|counter| ObjectGuid::create_player(realm_id, counter as i64));
            let max = if species
                .has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP)
            {
                1
            } else {
                DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP
            };
            if count_materialized_species_like_cpp(&pets, row.species, owner_guid, species.flags)
                >= max
            {
                continue;
            }

            let guid = battle_pet_guid_like_cpp(row.guid_counter);
            let pet = materialize_pet_like_cpp(
                &row,
                realm_id,
                virtual_realm_address,
                &species_store,
                &breed_quality_store,
                &breed_state_store,
                &species_state_store,
            );
            pets.insert(guid, pet);
        }
        let mut slots =
            std::array::from_fn(|index| RepresentedBattlePetSlotLikeCpp::locked_empty(index as u8));
        for slot in loaded.slots {
            if let Some(target) = slots.get_mut(slot.index as usize) {
                target.pet_guid = slot
                    .pet_guid_counter
                    .map(battle_pet_guid_like_cpp)
                    .filter(|guid| pets.contains_key(guid));
                target.locked = slot.locked;
            }
        }
        Self {
            account_id,
            realm_id,
            virtual_realm_address,
            persistence,
            species_store,
            breed_quality_store,
            breed_state_store,
            species_state_store,
            xp_game_table,
            state: Mutex::new(BattlePetAccountStateLikeCpp {
                pets,
                slots,
                pending_adds: HashMap::new(),
                completed_adds: HashMap::new(),
                pending_pet_mutations: HashSet::new(),
                slots_pending: false,
            }),
            process_lease: Mutex::new(BattlePetProcessLeaseStateLikeCpp {
                guard: None,
                acquiring: false,
                attachments: 0,
                active_operations: 0,
                lease_holder: None,
            }),
            process_lease_changed: Notify::new(),
            operations_drained: Notify::new(),
            registry_identity: Mutex::new(None),
        }
    }

    pub(super) fn begin_operation_like_cpp(self: &Arc<Self>) -> BattlePetOperationGuardLikeCpp {
        self.process_lease
            .lock()
            .expect("battle-pet process lease poisoned")
            .active_operations += 1;
        BattlePetOperationGuardLikeCpp {
            owner: Arc::clone(self),
        }
    }

    pub(super) fn set_registry_identity_like_cpp(
        &self,
        accounts: &Arc<BattlePetAccountMapLikeCpp>,
        cell: &Arc<BattlePetAccountCellLikeCpp>,
    ) {
        let mut identity = self
            .registry_identity
            .lock()
            .expect("battle-pet registry identity poisoned");
        if identity.is_none() {
            *identity = Some(BattlePetAccountRegistryIdentityLikeCpp {
                accounts: Arc::downgrade(accounts),
                cell: Arc::downgrade(cell),
            });
        }
    }

    pub(super) fn try_evict_if_idle_like_cpp(&self) {
        let identity = self
            .registry_identity
            .lock()
            .expect("battle-pet registry identity poisoned")
            .clone();
        let Some(identity) = identity else {
            return;
        };
        let (Some(accounts), Some(cell)) = (identity.accounts.upgrade(), identity.cell.upgrade())
        else {
            return;
        };
        let dashmap::mapref::entry::Entry::Occupied(entry) = accounts.entry(self.account_id) else {
            return;
        };
        if !Arc::ptr_eq(entry.get(), &cell) {
            return;
        }
        let mut process = self
            .process_lease
            .lock()
            .expect("battle-pet process lease poisoned");
        if process.attachments == 0
            && process.active_operations == 0
            && process.lease_holder.is_none()
            && !process.acquiring
        {
            process.guard.take();
            entry.remove();
        }
    }

    pub(super) async fn wait_until_operations_drained_like_cpp(&self) {
        loop {
            let notified = self.operations_drained.notified();
            if self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned")
                .active_operations
                == 0
            {
                return;
            }
            notified.await;
        }
    }

    pub(super) async fn ensure_process_lease_like_cpp(self: &Arc<Self>) -> bool {
        loop {
            let wait = {
                let mut process = self
                    .process_lease
                    .lock()
                    .expect("battle-pet process lease poisoned");
                if process
                    .guard
                    .as_ref()
                    .is_some_and(|guard| guard.is_valid_like_cpp())
                {
                    return true;
                }
                process.guard.take();
                if process.acquiring {
                    Some(self.process_lease_changed.notified())
                } else {
                    process.acquiring = true;
                    None
                }
            };
            if let Some(wait) = wait {
                wait.await;
                continue;
            }

            let acquired = self
                .persistence
                .try_acquire_process_lease(self.account_id)
                .await;
            let mut guard = match acquired {
                Ok(guard) => guard,
                Err(_) => None,
            };
            if guard.is_some() {
                let loaded = self
                    .persistence
                    .load_account(self.account_id, self.realm_id)
                    .await;
                if let Ok(loaded) = loaded {
                    let refreshed = Self::from_loaded_like_cpp(
                        self.account_id,
                        self.realm_id,
                        self.virtual_realm_address,
                        Arc::clone(&self.persistence),
                        Arc::clone(&self.species_store),
                        Arc::clone(&self.breed_quality_store),
                        Arc::clone(&self.breed_state_store),
                        Arc::clone(&self.species_state_store),
                        Arc::clone(&self.xp_game_table),
                        loaded,
                    );
                    let refreshed = refreshed
                        .state
                        .into_inner()
                        .expect("refreshed battle-pet account state poisoned");
                    let mut state = self
                        .state
                        .lock()
                        .expect("battle-pet account state poisoned");
                    if state.pending_adds.is_empty()
                        && state.pending_pet_mutations.is_empty()
                        && !state.slots_pending
                    {
                        state.pets = refreshed.pets;
                        state.slots = refreshed.slots;
                        state.completed_adds.clear();
                    } else {
                        guard = None;
                    }
                } else {
                    guard = None;
                }
            }

            let acquired = guard.is_some();
            let mut process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            process.guard = guard;
            process.acquiring = false;
            drop(process);
            self.process_lease_changed.notify_waiters();
            return acquired;
        }
    }

    pub(super) async fn try_acquire_lease_like_cpp(
        self: &Arc<Self>,
        lease_id: BattlePetLeaseIdLikeCpp,
    ) -> bool {
        loop {
            if !self.ensure_process_lease_like_cpp().await {
                return false;
            }
            let mut process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            if !process
                .guard
                .as_ref()
                .is_some_and(|guard| guard.is_valid_like_cpp())
            {
                process.guard.take();
                continue;
            }
            return match process.lease_holder {
                None => {
                    process.lease_holder = Some(lease_id);
                    true
                }
                Some(holder) => holder == lease_id,
            };
        }
    }

    pub(super) fn has_lease_like_cpp(&self, lease_id: BattlePetLeaseIdLikeCpp) -> bool {
        let process = self
            .process_lease
            .lock()
            .expect("battle-pet process lease poisoned");
        process.lease_holder == Some(lease_id)
            && process
                .guard
                .as_ref()
                .is_some_and(|guard| guard.is_valid_like_cpp())
    }

    pub(super) fn has_process_fence_like_cpp(&self, fence: u64) -> bool {
        self.process_lease
            .lock()
            .expect("battle-pet process lease poisoned")
            .guard
            .as_ref()
            .is_some_and(|guard| guard.is_valid_like_cpp() && guard.fence_like_cpp() == fence)
    }
}
