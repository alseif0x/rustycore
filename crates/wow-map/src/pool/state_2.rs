//! Pool template and group selection state definitions, part 2 of 2.
//!
//! Separated from the pool.rs root under #644. Behaviour is preserved.

use super::*;

impl PoolGroupLikeCpp {
    /// C++ constructor initializes `poolId` to zero.
    #[must_use]
    pub const fn new(member_kind: PoolMemberKindLikeCpp) -> Self {
        Self {
            pool_id: 0,
            member_kind,
            explicitly_chanced: Vec::new(),
            equal_chanced: Vec::new(),
        }
    }

    #[must_use]
    pub const fn with_pool_id(member_kind: PoolMemberKindLikeCpp, pool_id: u32) -> Self {
        Self {
            pool_id,
            member_kind,
            explicitly_chanced: Vec::new(),
            equal_chanced: Vec::new(),
        }
    }

    pub const fn set_pool_id_like_cpp(&mut self, pool_id: u32) {
        self.pool_id = pool_id;
    }

    #[must_use]
    pub const fn pool_id_like_cpp(&self) -> u32 {
        self.pool_id
    }

    #[must_use]
    pub const fn member_kind(&self) -> PoolMemberKindLikeCpp {
        self.member_kind
    }

    #[must_use]
    pub fn explicitly_chanced_like_cpp(&self) -> &[PoolObjectLikeCpp] {
        &self.explicitly_chanced
    }

    #[must_use]
    pub fn equal_chanced_like_cpp(&self) -> &[PoolObjectLikeCpp] {
        &self.equal_chanced
    }

    /// C++ `isEmpty()`: both chance buckets are empty.
    #[must_use]
    pub fn is_empty_like_cpp(&self) -> bool {
        self.explicitly_chanced.is_empty() && self.equal_chanced.is_empty()
    }

    /// C++ `isEmptyDeepCheck()`.
    ///
    /// For Creature/GameObject groups this is the normal `isEmpty()` helper and
    /// the child-pool closure is not called. For Pool-of-Pools groups this
    /// represents `sPoolMgr->IsEmpty(child_guid)`. Child GUIDs above `u32::MAX`
    /// are treated as non-empty rather than truncated silently.
    pub fn is_empty_deep_check_like_cpp(
        &self,
        mut is_child_pool_empty: impl FnMut(u32) -> bool,
    ) -> bool {
        if self.member_kind != PoolMemberKindLikeCpp::Pool {
            return self.is_empty_like_cpp();
        }

        for child in self
            .explicitly_chanced
            .iter()
            .chain(self.equal_chanced.iter())
        {
            let Ok(child_pool_id) = u32::try_from(child.guid) else {
                return false;
            };
            if !is_child_pool_empty(child_pool_id) {
                return false;
            }
        }

        true
    }

    /// C++ `AddEntry`: non-zero chance with maxentries one is explicit;
    /// everything else is equal-chanced.
    pub fn add_entry_like_cpp(&mut self, pool_object: PoolObjectLikeCpp, maxentries: u32) {
        if pool_object.chance != 0.0 && maxentries == 1 {
            self.explicitly_chanced.push(pool_object);
        } else {
            self.equal_chanced.push(pool_object);
        }
    }

    /// C++ `CheckPool`: validate explicit total only when equal-chanced is empty.
    #[must_use]
    pub fn check_pool_like_cpp(&self) -> bool {
        if self.equal_chanced.is_empty() {
            let chance = self
                .explicitly_chanced
                .iter()
                .map(|entry| entry.chance)
                .sum::<f32>();
            if chance != 100.0 && chance != 0.0 {
                return false;
            }
        }

        true
    }

    /// Deterministic representation of C++ `PoolGroup<T>::SpawnObject`.
    ///
    /// Source-of-truth spawned-pool state is the caller-provided map-owned
    /// `SpawnedPoolDataLikeCpp`. This helper mutates only that state for the
    /// represented `spawns.AddSpawn<T>` and final `DespawnObject(...triggerFrom)`
    /// branches, then returns explicit action records for live side effects that
    /// still belong to future owners (`Spawn1Object`, `ReSpawn1Object`,
    /// `DespawnObject`, recursive live `PoolMgr::SpawnPool`). The deterministic
    /// API requires a lazy explicit-roll provider so callers cannot skip C++
    /// `rand_chance()` when `ExplicitlyChanced` is non-empty, while still
    /// avoiding RNG consumption for C++ paths that do not call it.
    pub fn spawn_object_plan_like_cpp(
        &self,
        spawns: &mut SpawnedPoolDataLikeCpp,
        limit: u32,
        trigger_from: u64,
        explicit_roll: impl FnMut() -> f32,
        choose_equal: impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
    ) -> PoolSpawnObjectPlanLikeCpp {
        self.spawn_object_plan_with_child_pools_like_cpp(
            spawns,
            limit,
            trigger_from,
            explicit_roll,
            choose_equal,
            |_spawns, _child_pool_id| Ok(PoolSpawnPoolPlanLikeCpp::default()),
            |_spawns, _child_pool_id, _always_delete_respawn_time| {
                Ok(PoolDespawnPoolPlanLikeCpp::default())
            },
        )
        .unwrap_or_default()
    }

    pub fn spawn_object_plan_with_child_pools_like_cpp(
        &self,
        spawns: &mut SpawnedPoolDataLikeCpp,
        limit: u32,
        trigger_from: u64,
        mut explicit_roll: impl FnMut() -> f32,
        mut choose_equal: impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        mut spawn_child_pool: impl FnMut(
            &mut SpawnedPoolDataLikeCpp,
            u32,
        )
            -> Result<PoolSpawnPoolPlanLikeCpp, PoolMgrPlanErrorLikeCpp>,
        mut despawn_child_pool: impl FnMut(
            &mut SpawnedPoolDataLikeCpp,
            u32,
            bool,
        ) -> Result<
            PoolDespawnPoolPlanLikeCpp,
            PoolMgrPlanErrorLikeCpp,
        >,
    ) -> Result<PoolSpawnObjectPlanLikeCpp, PoolMgrPlanErrorLikeCpp> {
        let mut plan = PoolSpawnObjectPlanLikeCpp::default();
        let mut trigger_from = trigger_from;
        let spawned = i64::from(spawns.get_spawned_objects_like_cpp(self.pool_id));
        let mut count = i64::from(limit) - spawned;
        if trigger_from != 0 {
            count += 1;
        }

        if count > 0 {
            let mut rolled_objects = Vec::new();

            if !self.explicitly_chanced.is_empty() {
                let mut roll = explicit_roll();
                for obj in &self.explicitly_chanced {
                    roll -= obj.chance;
                    if roll < 0.0
                        && (obj.guid == trigger_from
                            || !self.is_spawned_in_map_like_cpp(spawns, obj.guid))
                    {
                        rolled_objects.push(*obj);
                        break;
                    }
                }
            }

            if !self.equal_chanced.is_empty() && rolled_objects.is_empty() {
                let candidates = self
                    .equal_chanced
                    .iter()
                    .copied()
                    .filter(|obj| {
                        obj.guid == trigger_from
                            || !self.is_spawned_in_map_like_cpp(spawns, obj.guid)
                    })
                    .collect::<Vec<_>>();
                let requested = match usize::try_from(count) {
                    Ok(value) => value.min(candidates.len()),
                    Err(_) => candidates.len(),
                };
                let chosen_indices = choose_equal(&candidates, requested);
                let mut used = vec![false; candidates.len()];
                for index in chosen_indices {
                    if rolled_objects.len() >= requested {
                        break;
                    }
                    if let Some(candidate) = candidates.get(index).copied() {
                        if !used[index] {
                            used[index] = true;
                            rolled_objects.push(candidate);
                        }
                    }
                }
            }

            for obj in rolled_objects {
                plan.selected.push(obj);
                if obj.guid == trigger_from {
                    plan.respawned_trigger = true;
                    if self.member_kind != PoolMemberKindLikeCpp::Pool {
                        plan.actions.push(PoolSpawnObjectActionLikeCpp::RespawnOne {
                            kind: self.member_kind,
                            guid: obj.guid,
                        });
                    }
                    trigger_from = 0;
                } else if self.add_spawn_to_map_like_cpp(spawns, obj.guid) {
                    plan.actions.push(PoolSpawnObjectActionLikeCpp::SpawnOne {
                        kind: self.member_kind,
                        guid: obj.guid,
                    });
                    if self.member_kind == PoolMemberKindLikeCpp::Pool {
                        let child_pool_id = u32::try_from(obj.guid).map_err(|_| {
                            PoolMgrPlanErrorLikeCpp::ChildPoolIdOverflow {
                                child_pool_id: obj.guid,
                            }
                        })?;
                        plan.child_pool_spawn_plans
                            .push(spawn_child_pool(spawns, child_pool_id)?);
                    }
                }
            }
        }

        if trigger_from != 0
            && self.contains_guid_like_cpp(trigger_from)
            && self.is_spawned_in_map_like_cpp(spawns, trigger_from)
        {
            plan.actions.push(PoolSpawnObjectActionLikeCpp::DespawnOne {
                kind: self.member_kind,
                guid: trigger_from,
            });
            if self.member_kind == PoolMemberKindLikeCpp::Pool {
                let child_pool_id = u32::try_from(trigger_from).map_err(|_| {
                    PoolMgrPlanErrorLikeCpp::ChildPoolIdOverflow {
                        child_pool_id: trigger_from,
                    }
                })?;
                plan.child_pool_despawn_plans.push(despawn_child_pool(
                    spawns,
                    child_pool_id,
                    false,
                )?);
            }
            if self.remove_spawn_from_map_like_cpp(spawns, trigger_from) {
                plan.despawned_trigger = Some(trigger_from);
            }
        }

        Ok(plan)
    }

    /// Deterministic representation of C++ `PoolGroup<T>::DespawnObject`.
    ///
    /// Bucket order is exactly C++: `EqualChanced` first, then
    /// `ExplicitlyChanced`. The helper mutates only caller-owned
    /// `SpawnedPoolDataLikeCpp` when C++ would call `RemoveSpawn<T>`, records
    /// `Despawn1Object`/respawn-time delete side effects as plan actions, and
    /// delegates child-pool recursion before removing the child from the parent
    /// spawned relation.
    pub fn despawn_object_plan_like_cpp(
        &self,
        spawns: &mut SpawnedPoolDataLikeCpp,
        requested_guid: u64,
        always_delete_respawn_time: bool,
        mut despawn_child_pool: impl FnMut(
            &mut SpawnedPoolDataLikeCpp,
            u32,
            bool,
        ) -> Result<
            PoolDespawnPoolPlanLikeCpp,
            PoolMgrPlanErrorLikeCpp,
        >,
    ) -> Result<PoolDespawnObjectPlanLikeCpp, PoolMgrPlanErrorLikeCpp> {
        let mut plan = PoolDespawnObjectPlanLikeCpp {
            requested_guid,
            always_delete_respawn_time,
            ..PoolDespawnObjectPlanLikeCpp::default()
        };

        for object in self
            .equal_chanced
            .iter()
            .chain(self.explicitly_chanced.iter())
        {
            if self.member_kind == PoolMemberKindLikeCpp::Pool
                && u32::try_from(object.guid).is_err()
            {
                return Err(PoolMgrPlanErrorLikeCpp::ChildPoolIdOverflow {
                    child_pool_id: object.guid,
                });
            }
            let spawned = self.is_spawned_in_map_like_cpp(spawns, object.guid);
            if spawned {
                if requested_guid == 0 || object.guid == requested_guid {
                    plan.actions.push(PoolSpawnObjectActionLikeCpp::DespawnOne {
                        kind: self.member_kind,
                        guid: object.guid,
                    });
                    if self.member_kind == PoolMemberKindLikeCpp::Pool {
                        let child_pool_id = u32::try_from(object.guid).map_err(|_| {
                            PoolMgrPlanErrorLikeCpp::ChildPoolIdOverflow {
                                child_pool_id: object.guid,
                            }
                        })?;
                        let child_plan =
                            despawn_child_pool(spawns, child_pool_id, always_delete_respawn_time)?;
                        plan.child_pool_plans.push(child_plan);
                    }
                    if self.remove_spawn_from_map_like_cpp(spawns, object.guid) {
                        plan.despawned.push(object.guid);
                    }
                }
            } else if always_delete_respawn_time && self.member_kind != PoolMemberKindLikeCpp::Pool
            {
                plan.actions
                    .push(PoolSpawnObjectActionLikeCpp::RemoveRespawnTime {
                        kind: self.member_kind,
                        guid: object.guid,
                    });
                plan.removed_respawn_times
                    .push((self.member_kind, object.guid));
            }
        }

        Ok(plan)
    }

    /// C++ specialization `PoolGroup<Pool>::RemoveOneRelation`.
    ///
    /// Creature/GameObject groups have no specialization in C++; this pure Rust
    /// helper treats them as an explicit no-op. For Pool groups, it removes the
    /// first matching child from `ExplicitlyChanced` and then the first matching
    /// child from `EqualChanced`, so one match can be removed from each bucket.
    pub fn remove_one_relation_like_cpp(
        &mut self,
        child_pool_id: u32,
    ) -> PoolRelationRemovalLikeCpp {
        if self.member_kind != PoolMemberKindLikeCpp::Pool {
            return PoolRelationRemovalLikeCpp::default();
        }

        let mut removal = PoolRelationRemovalLikeCpp::default();
        let child_pool_id = u64::from(child_pool_id);

        if let Some(index) = self
            .explicitly_chanced
            .iter()
            .position(|entry| entry.guid == child_pool_id)
        {
            self.explicitly_chanced.remove(index);
            removal.removed_explicit = true;
        }

        if let Some(index) = self
            .equal_chanced
            .iter()
            .position(|entry| entry.guid == child_pool_id)
        {
            self.equal_chanced.remove(index);
            removal.removed_equal = true;
        }

        removal
    }

    pub(super) fn contains_guid_like_cpp(&self, guid: u64) -> bool {
        self.explicitly_chanced
            .iter()
            .chain(self.equal_chanced.iter())
            .any(|entry| entry.guid == guid)
    }

    pub(super) fn is_spawned_in_map_like_cpp(
        &self,
        spawns: &SpawnedPoolDataLikeCpp,
        guid: u64,
    ) -> bool {
        match self.member_kind {
            PoolMemberKindLikeCpp::Creature => spawns.is_spawned_creature_like_cpp(guid),
            PoolMemberKindLikeCpp::GameObject => spawns.is_spawned_gameobject_like_cpp(guid),
            PoolMemberKindLikeCpp::Pool => u32::try_from(guid)
                .ok()
                .is_some_and(|sub_pool_id| spawns.is_spawned_pool_like_cpp(sub_pool_id)),
        }
    }

    pub(super) fn add_spawn_to_map_like_cpp(
        &self,
        spawns: &mut SpawnedPoolDataLikeCpp,
        guid: u64,
    ) -> bool {
        match self.member_kind {
            PoolMemberKindLikeCpp::Creature => spawns
                .add_spawn_like_cpp(SpawnObjectType::Creature, guid as SpawnId, self.pool_id)
                .is_ok(),
            PoolMemberKindLikeCpp::GameObject => spawns
                .add_spawn_like_cpp(SpawnObjectType::GameObject, guid as SpawnId, self.pool_id)
                .is_ok(),
            PoolMemberKindLikeCpp::Pool => {
                let Ok(sub_pool_id) = u32::try_from(guid) else {
                    return false;
                };
                spawns.add_pool_spawn_like_cpp(sub_pool_id, self.pool_id);
                true
            }
        }
    }

    pub(super) fn remove_spawn_from_map_like_cpp(
        &self,
        spawns: &mut SpawnedPoolDataLikeCpp,
        guid: u64,
    ) -> bool {
        match self.member_kind {
            PoolMemberKindLikeCpp::Creature => spawns
                .remove_spawn_like_cpp(SpawnObjectType::Creature, guid as SpawnId, self.pool_id)
                .is_ok(),
            PoolMemberKindLikeCpp::GameObject => spawns
                .remove_spawn_like_cpp(SpawnObjectType::GameObject, guid as SpawnId, self.pool_id)
                .is_ok(),
            PoolMemberKindLikeCpp::Pool => {
                let Ok(sub_pool_id) = u32::try_from(guid) else {
                    return false;
                };
                spawns.remove_pool_spawn_like_cpp(sub_pool_id, self.pool_id);
                true
            }
        }
    }
}
