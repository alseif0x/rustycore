//! SQLx-free persistence contract for C++ `InstanceLockMgr`.

use crate::PersistenceFutureLikeCpp;

/// DB-shaped `instance` row. Gameplay interpretation remains in
/// `wow-instances`; the adapter only preserves the C++ row contract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedInstanceLockPersistenceRowLikeCpp {
    pub instance_id: u32,
    pub data: String,
    pub completed_encounters_mask: u32,
    pub entrance_world_safe_loc_id: u32,
}

/// DB-shaped `character_instance_lock` row, including the ORDER BY
/// `instanceId` order required by C++ `MapManager::RegisterInstanceId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterInstanceLockPersistenceRowLikeCpp {
    pub player_guid_counter: u64,
    pub map_id: u32,
    pub lock_id: u32,
    pub instance_id: u32,
    pub difficulty_id: u8,
    pub data: String,
    pub completed_encounters_mask: u32,
    pub entrance_world_safe_loc_id: u32,
    pub expiry_time: u64,
    pub extended: bool,
}

/// One semantic Character DB mutation emitted by the instance-lock owner.
/// Variants intentionally retain C++ statement granularity so the adapter can
/// prove exact identity, bind order and delete-before-insert ordering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceLockPersistenceMutationLikeCpp {
    DeleteCharacterLock {
        player_guid_counter: u64,
        map_id: u32,
        lock_id: u32,
    },
    InsertCharacterLock {
        player_guid_counter: u64,
        map_id: u32,
        lock_id: u32,
        instance_id: u32,
        difficulty_id: u8,
        data: String,
        completed_encounters_mask: u32,
        entrance_world_safe_loc_id: u32,
        expiry_time: u64,
        extended: bool,
    },
    DeleteSharedInstance {
        instance_id: u32,
    },
    InsertSharedInstance {
        instance_id: u32,
        data: String,
        completed_encounters_mask: u32,
        entrance_world_safe_loc_id: u32,
    },
    UpdateCharacterLockExtension {
        extended: bool,
        player_guid_counter: u64,
        map_id: u32,
        lock_id: u32,
    },
    ForceExpireCharacterLock {
        expiry_time: u64,
        player_guid_counter: u64,
        map_id: u32,
        lock_id: u32,
    },
}

/// Atomic statement sequence. Callers build it while holding the instance
/// owner, release that lock, and only then await the adapter commit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstanceLockPersistencePlanLikeCpp {
    pub mutations: Vec<InstanceLockPersistenceMutationLikeCpp>,
}

impl InstanceLockPersistencePlanLikeCpp {
    pub fn push(&mut self, mutation: InstanceLockPersistenceMutationLikeCpp) {
        self.mutations.push(mutation);
    }

    pub fn is_empty(&self) -> bool {
        self.mutations.is_empty()
    }

    pub fn len(&self) -> usize {
        self.mutations.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceLockPersistenceLoadOutcomeLikeCpp {
    Loaded {
        shared_rows: Vec<SharedInstanceLockPersistenceRowLikeCpp>,
        character_rows: Vec<CharacterInstanceLockPersistenceRowLikeCpp>,
    },
    Failed {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstanceLockPersistenceOutcomeLikeCpp {
    Committed,
    Failed { reason: String },
}

pub trait InstanceLockPersistencePortLikeCpp: Send + Sync {
    fn load_all_like_cpp<'a>(
        &'a self,
    ) -> PersistenceFutureLikeCpp<'a, InstanceLockPersistenceLoadOutcomeLikeCpp>;

    fn commit_plan_like_cpp<'a>(
        &'a self,
        plan: InstanceLockPersistencePlanLikeCpp,
    ) -> PersistenceFutureLikeCpp<'a, InstanceLockPersistenceOutcomeLikeCpp>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_preserves_semantic_mutation_order() {
        let mut plan = InstanceLockPersistencePlanLikeCpp::default();
        plan.push(
            InstanceLockPersistenceMutationLikeCpp::DeleteCharacterLock {
                player_guid_counter: 7,
                map_id: 631,
                lock_id: 4,
            },
        );
        plan.push(
            InstanceLockPersistenceMutationLikeCpp::InsertCharacterLock {
                player_guid_counter: 7,
                map_id: 631,
                lock_id: 4,
                instance_id: 9001,
                difficulty_id: 3,
                data: "state".to_string(),
                completed_encounters_mask: 5,
                entrance_world_safe_loc_id: 9,
                expiry_time: 11,
                extended: true,
            },
        );

        assert_eq!(plan.len(), 2);
        assert!(matches!(
            plan.mutations.as_slice(),
            [
                InstanceLockPersistenceMutationLikeCpp::DeleteCharacterLock { .. },
                InstanceLockPersistenceMutationLikeCpp::InsertCharacterLock { .. }
            ]
        ));
    }
}
