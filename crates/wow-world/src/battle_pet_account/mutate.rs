//! Mutate operations of battle_pet_account.
//!
//! Divided out of the single inherent impl under #705; every method keeps
//! its name, signature and body.

use super::*;

impl BattlePetAccountOwnerLikeCpp {
    pub(crate) async fn try_mutate_pet_like_cpp<R, F>(
        self: &Arc<Self>,
        lease_id: BattlePetLeaseIdLikeCpp,
        pet_guid: ObjectGuid,
        mutation: F,
    ) -> Result<(R, BattlePetJournalPet), BattlePetMutationFailureLikeCpp>
    where
        R: Send + 'static,
        F: FnOnce(&mut RepresentedBattlePetDataLikeCpp) -> R,
    {
        self.try_mutate_pet_with_optional_lease_like_cpp(Some(lease_id), pet_guid, mutation)
            .await
    }

    /// C++ `BattlePetMgr::ClearFanfare` is intentionally the one represented
    /// durable pet mutation without a `HasJournalLock()` gate. It still uses
    /// the canonical owner's per-pet mutation serialization and persistence.
    pub(crate) async fn try_mutate_pet_without_lease_like_cpp<R, F>(
        self: &Arc<Self>,
        pet_guid: ObjectGuid,
        mutation: F,
    ) -> Result<(R, BattlePetJournalPet), BattlePetMutationFailureLikeCpp>
    where
        R: Send + 'static,
        F: FnOnce(&mut RepresentedBattlePetDataLikeCpp) -> R,
    {
        if !self.ensure_process_lease_like_cpp().await {
            return Err(BattlePetMutationFailureLikeCpp::MissingAuthority);
        }
        self.try_mutate_pet_with_optional_lease_like_cpp(None, pet_guid, mutation)
            .await
    }

    async fn try_mutate_pet_with_optional_lease_like_cpp<R, F>(
        self: &Arc<Self>,
        lease_id: Option<BattlePetLeaseIdLikeCpp>,
        pet_guid: ObjectGuid,
        mutation: F,
    ) -> Result<(R, BattlePetJournalPet), BattlePetMutationFailureLikeCpp>
    where
        R: Send + 'static,
        F: FnOnce(&mut RepresentedBattlePetDataLikeCpp) -> R,
    {
        let operation_guard = self.begin_operation_like_cpp();
        let (mut changed, fence) = {
            let process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            let fence = if let Some(lease_id) = lease_id {
                validate_process_mutation_lease_like_cpp(&process, lease_id)?
            } else if !process
                .guard
                .as_ref()
                .is_some_and(|guard| guard.is_valid_like_cpp())
            {
                return Err(BattlePetMutationFailureLikeCpp::MissingAuthority);
            } else {
                process
                    .guard
                    .as_ref()
                    .expect("validated battle-pet process guard disappeared")
                    .fence_like_cpp()
            };
            let mut state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            if !state.pending_pet_mutations.insert(pet_guid) {
                return Err(BattlePetMutationFailureLikeCpp::Busy);
            }
            let Some(current) = state.pets.get(&pet_guid).cloned() else {
                state.pending_pet_mutations.remove(&pet_guid);
                return Err(BattlePetMutationFailureLikeCpp::UnknownPet);
            };
            (current, fence)
        };
        let outcome = mutation(&mut changed);
        changed.save_info = RepresentedBattlePetSaveInfoLikeCpp::Unchanged;
        let owner = Arc::clone(self);
        tokio::spawn(async move {
            let _operation_guard = operation_guard;
            owner
                .finish_mutate_pet_like_cpp(pet_guid, outcome, changed, fence)
                .await
        })
        .await
        .map_err(|error| {
            BattlePetMutationFailureLikeCpp::DatabaseFailure(format!(
                "battle-pet update worker failed: {error}"
            ))
        })?
    }

    async fn finish_mutate_pet_like_cpp<R: Send + 'static>(
        self: Arc<Self>,
        pet_guid: ObjectGuid,
        outcome: R,
        changed: RepresentedBattlePetDataLikeCpp,
        fence: u64,
    ) -> Result<(R, BattlePetJournalPet), BattlePetMutationFailureLikeCpp> {
        let durable = durable_from_pet_like_cpp(pet_guid, &changed);
        let mut persistence = self
            .persistence
            .update_pet(self.account_id, fence, durable)
            .await;
        if persistence.is_ok() && !self.has_process_fence_like_cpp(fence) {
            persistence = Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
        }
        let mut state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        state.pending_pet_mutations.remove(&pet_guid);
        match persistence {
            Ok(()) => {
                let packet = changed.packet_info_like_cpp(pet_guid);
                state.pets.insert(pet_guid, changed);
                Ok((outcome, packet))
            }
            Err(error) => Err(mutation_persistence_error_like_cpp(error)),
        }
    }

    pub(crate) async fn try_remove_pet_like_cpp(
        self: &Arc<Self>,
        lease_id: BattlePetLeaseIdLikeCpp,
        pet_guid: ObjectGuid,
    ) -> Result<(), BattlePetMutationFailureLikeCpp> {
        let operation_guard = self.begin_operation_like_cpp();
        let (changed_slots, fence) = {
            let process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            let fence = validate_process_mutation_lease_like_cpp(&process, lease_id)?;
            let mut state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            if !state.pets.contains_key(&pet_guid) {
                return Err(BattlePetMutationFailureLikeCpp::UnknownPet);
            }
            if !state.pending_pet_mutations.insert(pet_guid) {
                return Err(BattlePetMutationFailureLikeCpp::Busy);
            }
            if state.slots_pending {
                state.pending_pet_mutations.remove(&pet_guid);
                return Err(BattlePetMutationFailureLikeCpp::Busy);
            }
            state.slots_pending = true;
            let mut changed = state.slots;
            for slot in &mut changed {
                if slot.pet_guid == Some(pet_guid) {
                    slot.pet_guid = None;
                }
            }
            (changed, fence)
        };
        let owner = Arc::clone(self);
        tokio::spawn(async move {
            let _operation_guard = operation_guard;
            owner
                .finish_remove_pet_like_cpp(pet_guid, changed_slots, fence)
                .await
        })
        .await
        .map_err(|error| {
            BattlePetMutationFailureLikeCpp::DatabaseFailure(format!(
                "battle-pet delete worker failed: {error}"
            ))
        })?
    }

    async fn finish_remove_pet_like_cpp(
        self: Arc<Self>,
        pet_guid: ObjectGuid,
        changed_slots: [RepresentedBattlePetSlotLikeCpp; BATTLE_PET_SLOT_COUNT_LIKE_CPP],
        fence: u64,
    ) -> Result<(), BattlePetMutationFailureLikeCpp> {
        let mut persistence = self
            .persistence
            .delete_pet(
                self.account_id,
                fence,
                pet_guid.counter() as u64,
                changed_slots.iter().map(durable_slot_like_cpp).collect(),
            )
            .await;
        if persistence.is_ok() && !self.has_process_fence_like_cpp(fence) {
            persistence = Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
        }
        let mut state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        state.pending_pet_mutations.remove(&pet_guid);
        state.slots_pending = false;
        match persistence {
            Ok(()) => {
                state.pets.remove(&pet_guid);
                state.slots = changed_slots;
                Ok(())
            }
            Err(error) => Err(mutation_persistence_error_like_cpp(error)),
        }
    }

    pub(crate) async fn try_set_slot_like_cpp(
        self: &Arc<Self>,
        lease_id: BattlePetLeaseIdLikeCpp,
        pet_guid: ObjectGuid,
        slot_index: u8,
    ) -> Result<BattlePetJournalSlot, BattlePetMutationFailureLikeCpp> {
        let operation_guard = self.begin_operation_like_cpp();
        let (changed, fence) = {
            let process = self
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            let fence = validate_process_mutation_lease_like_cpp(&process, lease_id)?;
            let mut state = self
                .state
                .lock()
                .expect("battle-pet account state poisoned");
            if !state.pets.contains_key(&pet_guid) {
                return Err(BattlePetMutationFailureLikeCpp::UnknownPet);
            }
            if state.slots_pending {
                return Err(BattlePetMutationFailureLikeCpp::Busy);
            }
            let mut changed = state.slots;
            let Some(slot) = changed.get_mut(slot_index as usize) else {
                return Err(BattlePetMutationFailureLikeCpp::UnknownPet);
            };
            slot.pet_guid = Some(pet_guid);
            state.slots_pending = true;
            (changed, fence)
        };
        let owner = Arc::clone(self);
        tokio::spawn(async move {
            let _operation_guard = operation_guard;
            owner
                .finish_set_slot_like_cpp(slot_index, changed, fence)
                .await
        })
        .await
        .map_err(|error| {
            BattlePetMutationFailureLikeCpp::DatabaseFailure(format!(
                "battle-pet slot worker failed: {error}"
            ))
        })?
    }

    async fn finish_set_slot_like_cpp(
        self: Arc<Self>,
        slot_index: u8,
        changed: [RepresentedBattlePetSlotLikeCpp; BATTLE_PET_SLOT_COUNT_LIKE_CPP],
        fence: u64,
    ) -> Result<BattlePetJournalSlot, BattlePetMutationFailureLikeCpp> {
        let durable = changed.iter().map(durable_slot_like_cpp).collect();
        let mut persistence = self
            .persistence
            .replace_slots(self.account_id, fence, durable)
            .await;
        if persistence.is_ok() && !self.has_process_fence_like_cpp(fence) {
            persistence = Err(BattlePetPersistenceErrorLikeCpp::StaleAuthority);
        }
        let mut state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        state.slots_pending = false;
        match persistence {
            Ok(()) => {
                state.slots = changed;
                Ok(state.slots[slot_index as usize].packet_slot_like_cpp())
            }
            Err(error) => Err(mutation_persistence_error_like_cpp(error)),
        }
    }
}
