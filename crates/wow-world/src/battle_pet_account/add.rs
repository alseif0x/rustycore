//! Add operations of battle_pet_account.
//!
//! Divided out of the single inherent impl under #705; every method keeps
//! its name, signature and body.

use super::*;

impl BattlePetAccountOwnerLikeCpp {
    /// Receipt probe for an account other than this owner's, serialized by
    /// the original account's process fence. Any in-flight original-account
    /// insert holds that fence through its owner guard, so
    /// `AuthorityUnavailable` must defer rather than guess: without the
    /// fence a negative read is only a snapshot that a detached worker can
    /// falsify immediately afterwards.
    pub(crate) async fn receipt_probe_for_account_fenced_like_cpp(
        &self,
        account_id: u32,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> Result<BattlePetFencedReceiptProbeLikeCpp, BattlePetAddFailureLikeCpp> {
        let guard = self
            .persistence
            .try_acquire_process_lease(account_id)
            .await
            .map_err(add_persistence_error_like_cpp)?;
        let Some(_guard) = guard else {
            return Ok(BattlePetFencedReceiptProbeLikeCpp::AuthorityUnavailable);
        };
        let committed = self
            .persistence
            .lookup_add_request(account_id, request_key)
            .await
            .map_err(add_persistence_error_like_cpp)?
            .is_some();
        Ok(if committed {
            BattlePetFencedReceiptProbeLikeCpp::Committed
        } else {
            BattlePetFencedReceiptProbeLikeCpp::Absent
        })
    }

    /// Receipt probe for an account other than this owner's — used by the
    /// #161 purchase saga when a character changed Battle.net accounts
    /// mid-purchase: the receipt authority stays the original account.
    pub(crate) async fn receipt_committed_for_account_like_cpp(
        &self,
        account_id: u32,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> Result<bool, BattlePetAddFailureLikeCpp> {
        self.persistence
            .lookup_add_request(account_id, request_key)
            .await
            .map(|receipt| receipt.is_some())
            .map_err(add_persistence_error_like_cpp)
    }

    pub(crate) async fn add_request_committed_like_cpp(
        &self,
        request_key: BattlePetAddRequestKeyLikeCpp,
    ) -> Result<bool, BattlePetAddFailureLikeCpp> {
        if self
            .state
            .lock()
            .expect("battle-pet account state poisoned")
            .completed_adds
            .contains_key(&request_key)
        {
            return Ok(true);
        }
        self.persistence
            .lookup_add_request(self.account_id, request_key)
            .await
            .map(|receipt| receipt.is_some())
            .map_err(add_persistence_error_like_cpp)
    }

    pub(crate) async fn try_add_pet_like_cpp(
        self: &Arc<Self>,
        lease_id: BattlePetLeaseIdLikeCpp,
        request: BattlePetAddRequestLikeCpp,
    ) -> Result<BattlePetAddOutcomeLikeCpp, BattlePetAddFailureLikeCpp> {
        let operation_guard = self.begin_operation_like_cpp();
        {
            let process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            validate_process_add_lease_like_cpp(&process, lease_id)?;
            let state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            if let Some((guid, durable)) = state.completed_adds.get(&request.request_key) {
                if !request_matches_durable_like_cpp(&request, durable, &self.species_store) {
                    return Err(BattlePetAddFailureLikeCpp::DuplicateRequest);
                }
                let pet = state
                    .pets
                    .get(guid)
                    .expect("completed battle-pet request must name canonical pet");
                return Ok(BattlePetAddOutcomeLikeCpp::Replayed(
                    pet.packet_info_like_cpp(*guid),
                ));
            }
        }

        let persisted_request = self
            .persistence
            .lookup_add_request(self.account_id, request.request_key)
            .await
            .map_err(add_persistence_error_like_cpp)?;
        if let Some(receipt) = persisted_request {
            if !request_matches_durable_like_cpp(
                &request,
                &receipt.requested_pet,
                &self.species_store,
            ) {
                return Err(BattlePetAddFailureLikeCpp::DuplicateRequest);
            }
            let Some(current_pet) = receipt.current_pet else {
                return Err(BattlePetAddFailureLikeCpp::DuplicateRequest);
            };
            let guid = battle_pet_guid_like_cpp(current_pet.guid_counter);
            let process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            validate_process_add_lease_like_cpp(&process, lease_id)?;
            let mut state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            if state.pending_pet_mutations.contains(&guid) {
                return Err(BattlePetAddFailureLikeCpp::Busy);
            }
            let packet = state
                .pets
                .get(&guid)
                .ok_or(BattlePetAddFailureLikeCpp::DuplicateRequest)?
                .packet_info_like_cpp(guid);
            state
                .completed_adds
                .insert(request.request_key, (guid, receipt.requested_pet));
            return Ok(BattlePetAddOutcomeLikeCpp::Replayed(packet));
        }

        let fence = loop {
            let (wait, reserved_fence) = {
                let process = self
                    .process_lease
                    .lock()
                    .expect("battle-pet process lease poisoned");
                let fence = validate_process_add_lease_like_cpp(&process, lease_id)?;
                let mut state = self
                    .state
                    .lock()
                    .expect("battle-pet account state poisoned");
                if let Some((guid, durable)) = state.completed_adds.get(&request.request_key) {
                    if !request_matches_durable_like_cpp(&request, durable, &self.species_store) {
                        return Err(BattlePetAddFailureLikeCpp::DuplicateRequest);
                    }
                    let pet = state
                        .pets
                        .get(guid)
                        .expect("completed battle-pet request must name canonical pet");
                    return Ok(BattlePetAddOutcomeLikeCpp::Replayed(
                        pet.packet_info_like_cpp(*guid),
                    ));
                }
                if let Some(pending) = state.pending_adds.get(&request.request_key) {
                    if pending.request != request {
                        return Err(BattlePetAddFailureLikeCpp::DuplicateRequest);
                    }
                    (Some(pending.completion.subscribe()), None)
                } else {
                    let Some(species) = self.species_store.get(request.species) else {
                        return Err(BattlePetAddFailureLikeCpp::InvalidSpecies);
                    };
                    if !species.has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP) {
                        return Err(BattlePetAddFailureLikeCpp::InvalidSpecies);
                    }
                    if species.has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP)
                        && request.owner_guid.is_none()
                    {
                        return Err(BattlePetAddFailureLikeCpp::MissingAuthority);
                    }
                    let max = if species
                        .has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP)
                    {
                        1
                    } else {
                        DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP
                    };
                    let persisted_count = count_species_like_cpp(
                        &state,
                        request.species,
                        request.owner_guid,
                        species.flags,
                    );
                    let reserved_count = state
                        .pending_adds
                        .values()
                        .filter(|pending| {
                            pending.request.species == request.species
                                && owner_matches_like_cpp(
                                    species.flags,
                                    pending.request.owner_guid,
                                    request.owner_guid,
                                )
                        })
                        .count();
                    if usize::from(persisted_count) + reserved_count >= usize::from(max) {
                        return Err(BattlePetAddFailureLikeCpp::Capacity);
                    }
                    let (completion, _) = watch::channel(false);
                    state.pending_adds.insert(
                        request.request_key,
                        PendingAddLikeCpp {
                            request: request.clone(),
                            completion,
                        },
                    );
                    (None, Some(fence))
                }
            };
            if let Some(mut wait) = wait {
                if !*wait.borrow() {
                    let _ = wait.changed().await;
                }
                continue;
            }
            break reserved_fence.expect("new pending add must capture its process fence");
        };

        let owner = Arc::clone(self);
        tokio::spawn(async move {
            let _operation_guard = operation_guard;
            owner.finish_add_pet_like_cpp(request, fence).await
        })
        .await
        .map_err(|error| {
            BattlePetAddFailureLikeCpp::DatabaseFailure(format!(
                "battle-pet add worker failed: {error}"
            ))
        })?
    }

    async fn finish_add_pet_like_cpp(
        self: Arc<Self>,
        request: BattlePetAddRequestLikeCpp,
        fence: u64,
    ) -> Result<BattlePetAddOutcomeLikeCpp, BattlePetAddFailureLikeCpp> {
        let mut persistence_result = async {
            let counter = self.persistence.allocate_guid_counter_like_cpp().await?;
            let row =
                self.durable_new_pet_like_cpp(counter, &request)
                    .map_err(|error| match error {
                        BattlePetAddFailureLikeCpp::GuidCollision => {
                            BattlePetPersistenceErrorLikeCpp::GuidCollision
                        }
                        other => BattlePetPersistenceErrorLikeCpp::Database(format!(
                            "could not materialize allocated battle pet: {other:?}"
                        )),
                    })?;
            let outcome = self
                .persistence
                .insert_pet_idempotently(DurableBattlePetAddLikeCpp {
                    account_id: self.account_id,
                    realm_id: self.realm_id,
                    request_key: request.request_key,
                    max_per_scope: self
                        .species_store
                        .get(request.species)
                        .map(|species| {
                            if species.has_flag_like_cpp(
                                BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP,
                            ) {
                                1
                            } else {
                                DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP
                            }
                        })
                        .ok_or_else(|| {
                            BattlePetPersistenceErrorLikeCpp::Database(
                                "validated battle-pet species disappeared".to_string(),
                            )
                        })?,
                    fence,
                    pet: row.clone(),
                })
                .await?;
            Ok::<_, BattlePetPersistenceErrorLikeCpp>((row, outcome))
        }
        .await;

        if persistence_result.is_ok() && !self.has_process_fence_like_cpp(fence) {
            persistence_result = Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
        }

        let mut state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        let pending = state
            .pending_adds
            .remove(&request.request_key)
            .expect("battle-pet reservation disappeared before persistence completed");
        let result = match persistence_result {
            Ok((row, outcome)) => {
                let replayed =
                    matches!(outcome, PersistBattlePetAddOutcomeLikeCpp::Replayed { .. });
                let durable = match outcome {
                    PersistBattlePetAddOutcomeLikeCpp::Inserted => row,
                    PersistBattlePetAddOutcomeLikeCpp::Replayed {
                        pet,
                        still_present: true,
                    } => pet,
                    PersistBattlePetAddOutcomeLikeCpp::Replayed {
                        still_present: false,
                        ..
                    } => {
                        let _ = pending.completion.send(true);
                        return Err(BattlePetAddFailureLikeCpp::DuplicateRequest);
                    }
                };
                let guid = battle_pet_guid_like_cpp(durable.guid_counter);
                let pet = materialize_pet_like_cpp(
                    &durable,
                    self.realm_id,
                    self.virtual_realm_address,
                    &self.species_store,
                    &self.breed_quality_store,
                    &self.breed_state_store,
                    &self.species_state_store,
                );
                let packet = pet.packet_info_like_cpp(guid);
                state.pets.insert(guid, pet);
                state
                    .completed_adds
                    .insert(request.request_key, (guid, durable.clone()));
                if replayed {
                    Ok(BattlePetAddOutcomeLikeCpp::Replayed(packet))
                } else {
                    Ok(BattlePetAddOutcomeLikeCpp::Added(packet))
                }
            }
            Err(BattlePetPersistenceErrorLikeCpp::GuidCollision) => {
                Err(BattlePetAddFailureLikeCpp::GuidCollision)
            }
            Err(BattlePetPersistenceErrorLikeCpp::Capacity) => {
                Err(BattlePetAddFailureLikeCpp::Capacity)
            }
            Err(BattlePetPersistenceErrorLikeCpp::DuplicateRequest) => {
                Err(BattlePetAddFailureLikeCpp::DuplicateRequest)
            }
            Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority) => {
                Err(BattlePetAddFailureLikeCpp::MissingAuthority)
            }
            Err(BattlePetPersistenceErrorLikeCpp::Database(error)) => {
                Err(BattlePetAddFailureLikeCpp::DatabaseFailure(error))
            }
        };
        let _ = pending.completion.send(true);
        result
    }

    fn durable_new_pet_like_cpp(
        &self,
        guid_counter: u64,
        request: &BattlePetAddRequestLikeCpp,
    ) -> Result<DurableBattlePetRowLikeCpp, BattlePetAddFailureLikeCpp> {
        let species = self
            .species_store
            .get(request.species)
            .ok_or(BattlePetAddFailureLikeCpp::InvalidSpecies)?;
        let stats = calculate_battle_pet_stats_like_cpp(
            request.breed,
            request.species,
            request.quality,
            request.level,
            &self.breed_state_store,
            &self.species_state_store,
            &self.breed_quality_store,
        );
        Ok(DurableBattlePetRowLikeCpp {
            guid_counter,
            species: request.species,
            breed: request.breed,
            display_id: request.display_id,
            level: request.level,
            exp: 0,
            health: stats.map_or(0, |stats| stats.max_health),
            quality: request.quality,
            flags: 0,
            name: String::new(),
            name_timestamp: 0,
            owner_guid_counter: species
                .has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP)
                .then(|| request.owner_guid.map(|guid| guid.counter() as u64))
                .flatten(),
            declined_names: None,
        })
    }
}
