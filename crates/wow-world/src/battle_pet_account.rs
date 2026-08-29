//! Canonical account-scoped battle-pet ownership.
//!
//! C++ stores the journal in each `WorldSession::BattlePetMgr` and relies on
//! `World::IsBattlePetJournalLockAcquired` to keep one session authoritative.
//! Its capacity check and `AddPet` mutation are nevertheless separate.  This
//! module preserves the journal/packet model while deliberately closing that
//! race: lease validation, capacity, GUID reservation, durable insert and
//! canonical publication are one result-bearing operation.

use std::collections::{HashMap, HashSet};
use std::sync::{
    Arc, Mutex, Weak,
    atomic::{AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tokio::sync::{Notify, OnceCell, watch};
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_data::{
    BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP,
    BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP, BATTLE_PET_SPECIES_FLAG_WELL_KNOWN_LIKE_CPP,
    BattlePetBreedQualityStore, BattlePetBreedStateStore, BattlePetSpeciesStateStore,
    BattlePetSpeciesStore, calculate_battle_pet_stats_like_cpp,
};
use wow_packet::packets::misc::{
    BattlePetJournal, BattlePetJournalPet, BattlePetJournalPetOwnerInfo, BattlePetJournalSlot,
    DeclinedNamesLikeCpp, empty_battle_pet_guid_like_cpp,
};
use wow_persistence::BattlePetDeclinedNamesLikeCpp;
pub(crate) use wow_persistence::{
    BattlePetAccountPersistencePortLikeCpp as BattlePetPersistenceLikeCpp,
    BattlePetAddRequestKeyLikeCpp, BattlePetPersistenceErrorLikeCpp, BattlePetProcessLeaseLikeCpp,
    DurableBattlePetAddLikeCpp, DurableBattlePetRowLikeCpp, DurableBattlePetSlotLikeCpp,
    LoadedBattlePetAccountLikeCpp, PersistBattlePetAddOutcomeLikeCpp,
};
#[cfg(test)]
pub(crate) use wow_persistence::{
    DurableBattlePetAddReceiptLikeCpp, PersistenceFutureLikeCpp as PersistenceFuture,
};

use crate::session::{
    BATTLE_PET_SLOT_COUNT_LIKE_CPP, DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP,
    RepresentedBattlePetDataLikeCpp, RepresentedBattlePetSaveInfoLikeCpp,
    RepresentedBattlePetSlotLikeCpp,
};

struct BattlePetProcessLeaseStateLikeCpp {
    guard: Option<Box<dyn BattlePetProcessLeaseLikeCpp>>,
    acquiring: bool,
    attachments: usize,
    active_operations: usize,
    lease_holder: Option<BattlePetLeaseIdLikeCpp>,
}
fn validate_add_lease_like_cpp(
    lease_holder: Option<BattlePetLeaseIdLikeCpp>,
    lease_id: BattlePetLeaseIdLikeCpp,
) -> Result<(), BattlePetAddFailureLikeCpp> {
    if lease_holder == Some(lease_id) {
        return Ok(());
    }
    Err(if lease_holder.is_some() {
        BattlePetAddFailureLikeCpp::JournalLocked
    } else {
        BattlePetAddFailureLikeCpp::MissingAuthority
    })
}

fn request_matches_durable_like_cpp(
    request: &BattlePetAddRequestLikeCpp,
    persisted: &DurableBattlePetRowLikeCpp,
    species_store: &BattlePetSpeciesStore,
) -> bool {
    let Some(species) = species_store.get(request.species) else {
        return false;
    };
    let requested_owner = species
        .has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP)
        .then(|| request.owner_guid.map(|guid| guid.counter() as u64))
        .flatten();
    request.species == persisted.species
        && request.breed == persisted.breed
        && request.display_id == persisted.display_id
        && request.level == persisted.level
        && request.quality == persisted.quality
        && requested_owner == persisted.owner_guid_counter
}

fn validate_mutation_lease_like_cpp(
    lease_holder: Option<BattlePetLeaseIdLikeCpp>,
    lease_id: BattlePetLeaseIdLikeCpp,
) -> Result<(), BattlePetMutationFailureLikeCpp> {
    if lease_holder == Some(lease_id) {
        return Ok(());
    }
    Err(if lease_holder.is_some() {
        BattlePetMutationFailureLikeCpp::JournalLocked
    } else {
        BattlePetMutationFailureLikeCpp::MissingAuthority
    })
}

fn validate_process_add_lease_like_cpp(
    process: &BattlePetProcessLeaseStateLikeCpp,
    lease_id: BattlePetLeaseIdLikeCpp,
) -> Result<u64, BattlePetAddFailureLikeCpp> {
    let Some(guard) = process
        .guard
        .as_ref()
        .filter(|guard| guard.is_valid_like_cpp())
    else {
        return Err(BattlePetAddFailureLikeCpp::MissingAuthority);
    };
    validate_add_lease_like_cpp(process.lease_holder, lease_id)?;
    Ok(guard.fence_like_cpp())
}

fn validate_process_mutation_lease_like_cpp(
    process: &BattlePetProcessLeaseStateLikeCpp,
    lease_id: BattlePetLeaseIdLikeCpp,
) -> Result<u64, BattlePetMutationFailureLikeCpp> {
    let Some(guard) = process
        .guard
        .as_ref()
        .filter(|guard| guard.is_valid_like_cpp())
    else {
        return Err(BattlePetMutationFailureLikeCpp::MissingAuthority);
    };
    validate_mutation_lease_like_cpp(process.lease_holder, lease_id)?;
    Ok(guard.fence_like_cpp())
}

fn add_persistence_error_like_cpp(
    error: BattlePetPersistenceErrorLikeCpp,
) -> BattlePetAddFailureLikeCpp {
    match error {
        BattlePetPersistenceErrorLikeCpp::Database(error) => {
            BattlePetAddFailureLikeCpp::DatabaseFailure(error)
        }
        BattlePetPersistenceErrorLikeCpp::Capacity => BattlePetAddFailureLikeCpp::Capacity,
        BattlePetPersistenceErrorLikeCpp::GuidCollision => {
            BattlePetAddFailureLikeCpp::GuidCollision
        }
        BattlePetPersistenceErrorLikeCpp::DuplicateRequest => {
            BattlePetAddFailureLikeCpp::DuplicateRequest
        }
        BattlePetPersistenceErrorLikeCpp::StaleAuthority => {
            BattlePetAddFailureLikeCpp::MissingAuthority
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BattlePetLeaseIdLikeCpp(u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BattlePetAddRequestLikeCpp {
    pub request_key: BattlePetAddRequestKeyLikeCpp,
    pub species: u32,
    pub display_id: u32,
    pub breed: u16,
    pub quality: u8,
    pub level: u16,
    pub owner_guid: Option<ObjectGuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BattlePetAddFailureLikeCpp {
    MissingAuthority,
    JournalLocked,
    InvalidSpecies,
    Capacity,
    DuplicateRequest,
    Busy,
    DatabaseFailure(String),
    GuidCollision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BattlePetAddOutcomeLikeCpp {
    Added(BattlePetJournalPet),
    Replayed(BattlePetJournalPet),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BattlePetMutationFailureLikeCpp {
    MissingAuthority,
    JournalLocked,
    UnknownPet,
    Busy,
    DatabaseFailure(String),
}

struct PendingAddLikeCpp {
    request: BattlePetAddRequestLikeCpp,
    completion: watch::Sender<bool>,
}

struct BattlePetAccountStateLikeCpp {
    pets: HashMap<ObjectGuid, RepresentedBattlePetDataLikeCpp>,
    slots: [RepresentedBattlePetSlotLikeCpp; BATTLE_PET_SLOT_COUNT_LIKE_CPP],
    pending_adds: HashMap<BattlePetAddRequestKeyLikeCpp, PendingAddLikeCpp>,
    completed_adds:
        HashMap<BattlePetAddRequestKeyLikeCpp, (ObjectGuid, DurableBattlePetRowLikeCpp)>,
    pending_pet_mutations: HashSet<ObjectGuid>,
    slots_pending: bool,
}

type BattlePetAccountCellLikeCpp = OnceCell<Arc<BattlePetAccountOwnerLikeCpp>>;
type BattlePetAccountMapLikeCpp = DashMap<u32, Arc<BattlePetAccountCellLikeCpp>>;

#[derive(Clone)]
struct BattlePetAccountRegistryIdentityLikeCpp {
    accounts: Weak<BattlePetAccountMapLikeCpp>,
    cell: Weak<BattlePetAccountCellLikeCpp>,
}

pub(crate) struct BattlePetAccountOwnerLikeCpp {
    account_id: u32,
    realm_id: u16,
    virtual_realm_address: u32,
    persistence: Arc<dyn BattlePetPersistenceLikeCpp>,
    species_store: Arc<BattlePetSpeciesStore>,
    breed_quality_store: Arc<BattlePetBreedQualityStore>,
    breed_state_store: Arc<BattlePetBreedStateStore>,
    species_state_store: Arc<BattlePetSpeciesStateStore>,
    state: Mutex<BattlePetAccountStateLikeCpp>,
    process_lease: Mutex<BattlePetProcessLeaseStateLikeCpp>,
    process_lease_changed: Notify,
    operations_drained: Notify,
    registry_identity: Mutex<Option<BattlePetAccountRegistryIdentityLikeCpp>>,
}

/// Outcome of a cross-account receipt probe serialized by the #160 process
/// fence (issue #161): the answer is only meaningful when the fence could be
/// held across the read, so no in-flight original-account insert can race it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BattlePetFencedReceiptProbeLikeCpp {
    Committed,
    Absent,
    AuthorityUnavailable,
}

impl BattlePetAccountOwnerLikeCpp {
    fn from_loaded_like_cpp(
        account_id: u32,
        realm_id: u16,
        virtual_realm_address: u32,
        persistence: Arc<dyn BattlePetPersistenceLikeCpp>,
        species_store: Arc<BattlePetSpeciesStore>,
        breed_quality_store: Arc<BattlePetBreedQualityStore>,
        breed_state_store: Arc<BattlePetBreedStateStore>,
        species_state_store: Arc<BattlePetSpeciesStateStore>,
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

    fn begin_operation_like_cpp(self: &Arc<Self>) -> BattlePetOperationGuardLikeCpp {
        self.process_lease
            .lock()
            .expect("battle-pet process lease poisoned")
            .active_operations += 1;
        BattlePetOperationGuardLikeCpp {
            owner: Arc::clone(self),
        }
    }

    fn set_registry_identity_like_cpp(
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

    fn try_evict_if_idle_like_cpp(&self) {
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

    async fn wait_until_operations_drained_like_cpp(&self) {
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

    async fn ensure_process_lease_like_cpp(self: &Arc<Self>) -> bool {
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

    async fn try_acquire_lease_like_cpp(
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

    fn has_lease_like_cpp(&self, lease_id: BattlePetLeaseIdLikeCpp) -> bool {
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

    fn has_process_fence_like_cpp(&self, fence: u64) -> bool {
        self.process_lease
            .lock()
            .expect("battle-pet process lease poisoned")
            .guard
            .as_ref()
            .is_some_and(|guard| guard.is_valid_like_cpp() && guard.fence_like_cpp() == fence)
    }

    pub(crate) fn journal_like_cpp(
        &self,
        lease_id: BattlePetLeaseIdLikeCpp,
        player_guid: Option<ObjectGuid>,
    ) -> BattlePetJournal {
        let has_journal_lock = self.has_lease_like_cpp(lease_id);
        let state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        let mut pets: Vec<_> = state
            .pets
            .iter()
            .filter(|(_, pet)| pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::Removed)
            .filter(|(_, pet)| {
                pet.owner_info
                    .is_none_or(|owner| Some(owner.guid) == player_guid)
            })
            .map(|(guid, pet)| pet.packet_info_like_cpp(*guid))
            .collect();
        pets.sort_by_key(|pet| pet.guid.counter());
        BattlePetJournal {
            trap: 0,
            has_journal_lock,
            slots: state
                .slots
                .iter()
                .map(|slot| {
                    let mut packet = slot.packet_slot_like_cpp();
                    if !packet.pet_guid.is_empty() && !state.pets.contains_key(&packet.pet_guid) {
                        packet.pet_guid = empty_battle_pet_guid_like_cpp();
                    }
                    packet
                })
                .collect(),
            pets,
        }
    }

    pub(crate) fn pet_snapshot_like_cpp(
        &self,
        pet_guid: ObjectGuid,
    ) -> Option<RepresentedBattlePetDataLikeCpp> {
        self.state
            .lock()
            .expect("battle-pet account state poisoned")
            .pets
            .get(&pet_guid)
            .cloned()
    }

    pub(crate) fn max_pet_level_like_cpp(&self) -> u16 {
        self.state
            .lock()
            .expect("battle-pet account state poisoned")
            .pets
            .values()
            .filter(|pet| pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::Removed)
            .map(|pet| pet.level)
            .max()
            .unwrap_or(0)
    }

    pub(crate) fn has_max_pet_count_like_cpp(
        &self,
        species: u32,
        owner_guid: Option<ObjectGuid>,
    ) -> bool {
        let Some(entry) = self.species_store.get(species) else {
            return false;
        };
        let max = if entry.has_flag_like_cpp(BATTLE_PET_SPECIES_FLAG_LEGACY_ACCOUNT_UNIQUE_LIKE_CPP)
        {
            1
        } else {
            DEFAULT_MAX_BATTLE_PETS_PER_SPECIES_LIKE_CPP
        };
        let state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        count_species_like_cpp(&state, species, owner_guid, entry.flags) >= max
    }

    pub(crate) fn pet_count_like_cpp(&self, species: u32, owner_guid: Option<ObjectGuid>) -> u8 {
        let Some(entry) = self.species_store.get(species) else {
            return 0;
        };
        count_species_like_cpp(
            &self
                .state
                .lock()
                .expect("battle-pet account state poisoned"),
            species,
            owner_guid,
            entry.flags,
        )
    }

    pub(crate) fn unique_species_count_like_cpp(&self) -> u32 {
        let state = self
            .state
            .lock()
            .expect("battle-pet account state poisoned");
        u32::try_from(
            state
                .pets
                .values()
                .map(|pet| pet.species)
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
        )
        .unwrap_or(u32::MAX)
    }

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

fn count_species_like_cpp(
    state: &BattlePetAccountStateLikeCpp,
    species: u32,
    owner_guid: Option<ObjectGuid>,
    species_flags: i32,
) -> u8 {
    let count = state
        .pets
        .values()
        .filter(|pet| pet.species == species)
        .filter(|pet| pet.save_info != RepresentedBattlePetSaveInfoLikeCpp::Removed)
        .filter(|pet| {
            if species_flags & BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP != 0 {
                if let (Some(owner_guid), Some(owner_info)) = (owner_guid, pet.owner_info) {
                    return owner_info.guid == owner_guid;
                }
            }
            true
        })
        .count();
    u8::try_from(count).unwrap_or(u8::MAX)
}

fn count_materialized_species_like_cpp(
    pets: &HashMap<ObjectGuid, RepresentedBattlePetDataLikeCpp>,
    species: u32,
    owner_guid: Option<ObjectGuid>,
    species_flags: i32,
) -> u8 {
    let count = pets
        .values()
        .filter(|pet| pet.species == species)
        .filter(|pet| {
            species_flags & BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP == 0
                || pet.owner_info.map(|owner| owner.guid) == owner_guid
        })
        .count();
    u8::try_from(count).unwrap_or(u8::MAX)
}

fn owner_matches_like_cpp(
    species_flags: i32,
    left: Option<ObjectGuid>,
    right: Option<ObjectGuid>,
) -> bool {
    species_flags & BATTLE_PET_SPECIES_FLAG_NOT_ACCOUNT_WIDE_LIKE_CPP == 0 || left == right
}

fn materialize_pet_like_cpp(
    row: &DurableBattlePetRowLikeCpp,
    realm_id: u16,
    virtual_realm_address: u32,
    species_store: &BattlePetSpeciesStore,
    breed_quality_store: &BattlePetBreedQualityStore,
    breed_state_store: &BattlePetBreedStateStore,
    species_state_store: &BattlePetSpeciesStateStore,
) -> RepresentedBattlePetDataLikeCpp {
    let species = species_store.get(row.species);
    let stats = calculate_battle_pet_stats_like_cpp(
        row.breed,
        row.species,
        row.quality,
        row.level,
        breed_state_store,
        species_state_store,
        breed_quality_store,
    );
    RepresentedBattlePetDataLikeCpp {
        species: row.species,
        creature_id: species
            .and_then(|entry| u32::try_from(entry.creature_id).ok())
            .unwrap_or_default(),
        display_id: row.display_id,
        breed: row.breed,
        level: row.level,
        exp: row.exp,
        flags: row.flags,
        power: stats.map_or(0, |stats| stats.power),
        health: row.health,
        max_health: stats.map_or(0, |stats| stats.max_health),
        speed: stats.map_or(0, |stats| stats.speed),
        quality: row.quality,
        owner_info: row
            .owner_guid_counter
            .map(|counter| BattlePetJournalPetOwnerInfo {
                guid: ObjectGuid::create_player(realm_id, counter as i64),
                player_virtual_realm: virtual_realm_address,
                player_native_realm: virtual_realm_address,
            }),
        name: row.name.clone(),
        name_timestamp: row.name_timestamp,
        declined_names: row
            .declined_names
            .as_ref()
            .map(|declined| DeclinedNamesLikeCpp {
                names: declined.names.clone(),
            }),
        save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
    }
}

fn durable_from_pet_like_cpp(
    guid: ObjectGuid,
    pet: &RepresentedBattlePetDataLikeCpp,
) -> DurableBattlePetRowLikeCpp {
    DurableBattlePetRowLikeCpp {
        guid_counter: guid.counter() as u64,
        species: pet.species,
        breed: pet.breed,
        display_id: pet.display_id,
        level: pet.level,
        exp: pet.exp,
        health: pet.health,
        quality: pet.quality,
        flags: pet.flags,
        name: pet.name.clone(),
        name_timestamp: pet.name_timestamp,
        owner_guid_counter: pet.owner_info.map(|owner| owner.guid.counter() as u64),
        declined_names: pet
            .declined_names
            .as_ref()
            .map(|declined| BattlePetDeclinedNamesLikeCpp {
                names: declined.names.clone(),
            }),
    }
}

fn durable_slot_like_cpp(slot: &RepresentedBattlePetSlotLikeCpp) -> DurableBattlePetSlotLikeCpp {
    DurableBattlePetSlotLikeCpp {
        index: slot.index,
        pet_guid_counter: slot.pet_guid.map(|guid| guid.counter() as u64),
        locked: slot.locked,
    }
}

fn battle_pet_guid_like_cpp(counter: u64) -> ObjectGuid {
    ObjectGuid::create_global(HighGuid::BattlePet, 0, counter as i64)
}

fn mutation_persistence_error_like_cpp(
    error: BattlePetPersistenceErrorLikeCpp,
) -> BattlePetMutationFailureLikeCpp {
    if error == BattlePetPersistenceErrorLikeCpp::StaleAuthority {
        return BattlePetMutationFailureLikeCpp::MissingAuthority;
    }
    BattlePetMutationFailureLikeCpp::DatabaseFailure(match error {
        BattlePetPersistenceErrorLikeCpp::Database(error) => error,
        BattlePetPersistenceErrorLikeCpp::Capacity => "unexpected capacity failure".to_string(),
        BattlePetPersistenceErrorLikeCpp::GuidCollision => "unexpected GUID collision".to_string(),
        BattlePetPersistenceErrorLikeCpp::DuplicateRequest => {
            "unexpected duplicate request".to_string()
        }
        BattlePetPersistenceErrorLikeCpp::StaleAuthority => unreachable!(),
    })
}

struct BattlePetOperationGuardLikeCpp {
    owner: Arc<BattlePetAccountOwnerLikeCpp>,
}

impl Drop for BattlePetOperationGuardLikeCpp {
    fn drop(&mut self) {
        let drained = {
            let mut process = self
                .owner
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            debug_assert!(process.active_operations != 0);
            process.active_operations -= 1;
            let drained = process.active_operations == 0;
            if process.attachments == 0 && drained && process.lease_holder.is_none() {
                process.guard.take();
            }
            drained
        };
        if drained {
            self.owner.operations_drained.notify_waiters();
        }
        self.owner.try_evict_if_idle_like_cpp();
    }
}

pub struct BattlePetAccountAttachmentLikeCpp {
    owner: Arc<BattlePetAccountOwnerLikeCpp>,
    lease_id: BattlePetLeaseIdLikeCpp,
}

impl BattlePetAccountAttachmentLikeCpp {
    pub(crate) async fn try_acquire_lease_like_cpp(&self) -> bool {
        self.owner.try_acquire_lease_like_cpp(self.lease_id).await
    }

    pub(crate) fn has_lease_like_cpp(&self) -> bool {
        self.owner.has_lease_like_cpp(self.lease_id)
    }

    pub(crate) fn owner_like_cpp(&self) -> &Arc<BattlePetAccountOwnerLikeCpp> {
        &self.owner
    }

    pub(crate) fn lease_id_like_cpp(&self) -> BattlePetLeaseIdLikeCpp {
        self.lease_id
    }
}

impl Drop for BattlePetAccountAttachmentLikeCpp {
    fn drop(&mut self) {
        {
            let mut process = self
                .owner
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned");
            if process.lease_holder == Some(self.lease_id) {
                process.lease_holder = None;
            }
            debug_assert!(
                process.attachments != 0,
                "battle-pet attachment count underflow"
            );
            process.attachments -= 1;
            if process.attachments == 0
                && process.active_operations == 0
                && process.lease_holder.is_none()
            {
                process.guard.take();
            }
        }
        self.owner.try_evict_if_idle_like_cpp();
    }
}

pub struct BattlePetAccountRegistryLikeCpp {
    persistence: Arc<dyn BattlePetPersistenceLikeCpp>,
    species_store: Arc<BattlePetSpeciesStore>,
    breed_quality_store: Arc<BattlePetBreedQualityStore>,
    breed_state_store: Arc<BattlePetBreedStateStore>,
    species_state_store: Arc<BattlePetSpeciesStateStore>,
    realm_id: u16,
    virtual_realm_address: u32,
    next_lease_id: AtomicU64,
    accounts: Arc<BattlePetAccountMapLikeCpp>,
}

impl BattlePetAccountRegistryLikeCpp {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        persistence: Arc<dyn BattlePetPersistenceLikeCpp>,
        species_store: Arc<BattlePetSpeciesStore>,
        breed_quality_store: Arc<BattlePetBreedQualityStore>,
        breed_state_store: Arc<BattlePetBreedStateStore>,
        species_state_store: Arc<BattlePetSpeciesStateStore>,
        realm_id: u16,
        virtual_realm_address: u32,
    ) -> Self {
        Self::new_with_persistence_like_cpp(
            persistence,
            species_store,
            breed_quality_store,
            breed_state_store,
            species_state_store,
            realm_id,
            virtual_realm_address,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_with_persistence_like_cpp(
        persistence: Arc<dyn BattlePetPersistenceLikeCpp>,
        species_store: Arc<BattlePetSpeciesStore>,
        breed_quality_store: Arc<BattlePetBreedQualityStore>,
        breed_state_store: Arc<BattlePetBreedStateStore>,
        species_state_store: Arc<BattlePetSpeciesStateStore>,
        realm_id: u16,
        virtual_realm_address: u32,
    ) -> Self {
        Self {
            persistence,
            species_store,
            breed_quality_store,
            breed_state_store,
            species_state_store,
            realm_id,
            virtual_realm_address,
            next_lease_id: AtomicU64::new(1),
            accounts: Arc::new(DashMap::new()),
        }
    }

    pub async fn attach_like_cpp(
        &self,
        account_id: u32,
    ) -> Result<BattlePetAccountAttachmentLikeCpp, String> {
        if account_id == 0 {
            return Err("battle-pet journal requires a nonzero Battle.net account".to_string());
        }
        loop {
            let cell = self
                .accounts
                .entry(account_id)
                .or_insert_with(|| Arc::new(OnceCell::new()))
                .clone();
            let owner = cell
                .get_or_try_init(|| async {
                    let loaded = self
                        .persistence
                        .load_account(account_id, self.realm_id)
                        .await
                        .map_err(|error| format!("failed to load battle-pet account: {error:?}"))?;
                    Ok::<_, String>(Arc::new(
                        BattlePetAccountOwnerLikeCpp::from_loaded_like_cpp(
                            account_id,
                            self.realm_id,
                            self.virtual_realm_address,
                            Arc::clone(&self.persistence),
                            Arc::clone(&self.species_store),
                            Arc::clone(&self.breed_quality_store),
                            Arc::clone(&self.breed_state_store),
                            Arc::clone(&self.species_state_store),
                            loaded,
                        ),
                    ))
                })
                .await?
                .clone();
            owner.set_registry_identity_like_cpp(&self.accounts, &cell);

            let dashmap::mapref::entry::Entry::Occupied(entry) = self.accounts.entry(account_id)
            else {
                continue;
            };
            if !Arc::ptr_eq(entry.get(), &cell) {
                continue;
            }
            owner
                .process_lease
                .lock()
                .expect("battle-pet process lease poisoned")
                .attachments += 1;
            drop(entry);
            let lease_id =
                BattlePetLeaseIdLikeCpp(self.next_lease_id.fetch_add(1, Ordering::Relaxed));
            return Ok(BattlePetAccountAttachmentLikeCpp { owner, lease_id });
        }
    }

    /// Wait for cancellation-safe persistence workers after the network has
    /// stopped admitting sessions. The single deadline bounds orderly
    /// shutdown even when a database connection is unhealthy.
    pub async fn drain_like_cpp(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        let owners: Vec<_> = self
            .accounts
            .iter()
            .filter_map(|entry| entry.value().get().cloned())
            .collect();
        for owner in owners {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            if tokio::time::timeout(remaining, owner.wait_until_operations_drained_like_cpp())
                .await
                .is_err()
            {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
#[path = "battle_pet_account_tests.rs"]
mod tests;
