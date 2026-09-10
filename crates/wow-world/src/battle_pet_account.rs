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
    BattlePetSpeciesStore, BattlePetXpGameTableLikeCpp, calculate_battle_pet_stats_like_cpp,
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
    xp_game_table: Arc<BattlePetXpGameTableLikeCpp>,
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
    xp_game_table: Arc<BattlePetXpGameTableLikeCpp>,
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
        xp_game_table: Arc<BattlePetXpGameTableLikeCpp>,
        realm_id: u16,
        virtual_realm_address: u32,
    ) -> Self {
        Self::new_with_persistence_like_cpp(
            persistence,
            species_store,
            breed_quality_store,
            breed_state_store,
            species_state_store,
            xp_game_table,
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
        xp_game_table: Arc<BattlePetXpGameTableLikeCpp>,
        realm_id: u16,
        virtual_realm_address: u32,
    ) -> Self {
        Self {
            persistence,
            species_store,
            breed_quality_store,
            breed_state_store,
            species_state_store,
            xp_game_table,
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
                            Arc::clone(&self.xp_game_table),
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
#[path = "battle_pet_account_tests/mod.rs"]
mod tests;

mod add;
mod lifecycle;
mod mutate;
mod queries;
