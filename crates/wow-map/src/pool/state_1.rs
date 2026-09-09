//! Pool template and group selection state definitions, part 1 of 2.
//!
//! Separated from the pool.rs root under #644. Behaviour is preserved.

use super::*;

/// C++ `PoolTemplateData { uint32 MaxLimit; int32 MapId; }`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PoolTemplateDataLikeCpp {
    pub max_limit: u32,
    pub map_id: i32,
}

impl PoolTemplateDataLikeCpp {
    #[must_use]
    pub const fn new(max_limit: u32, map_id: i32) -> Self {
        Self { max_limit, map_id }
    }
}

/// C++ `PoolObject { uint64 guid; float chance; }`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PoolObjectLikeCpp {
    pub guid: u64,
    pub chance: f32,
}

impl PoolObjectLikeCpp {
    #[must_use]
    pub const fn new(guid: u64, chance: f32) -> Self {
        Self { guid, chance }
    }
}

/// Tag for the C++ template parameter of `PoolGroup<T>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PoolMemberKindLikeCpp {
    Creature,
    GameObject,
    Pool,
}

/// Evidence returned by `PoolGroup<Pool>::RemoveOneRelation` representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PoolRelationRemovalLikeCpp {
    pub removed_explicit: bool,
    pub removed_equal: bool,
}

/// Planned side-effect placeholder for C++ `Spawn1Object`/`ReSpawn1Object`/
/// `DespawnObject` calls.
///
/// These actions intentionally do not create entities, write DB rows, call
/// `AddToMap`, recurse through live `PoolMgr::SpawnPool`, or fan out packets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolSpawnObjectActionLikeCpp {
    SpawnOne {
        kind: PoolMemberKindLikeCpp,
        guid: u64,
    },
    RespawnOne {
        kind: PoolMemberKindLikeCpp,
        guid: u64,
    },
    DespawnOne {
        kind: PoolMemberKindLikeCpp,
        guid: u64,
    },
    RemoveRespawnTime {
        kind: PoolMemberKindLikeCpp,
        guid: u64,
    },
}

/// Deterministic result of represented C++ `PoolGroup<T>::DespawnObject`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PoolDespawnObjectPlanLikeCpp {
    pub actions: Vec<PoolSpawnObjectActionLikeCpp>,
    pub requested_guid: u64,
    pub always_delete_respawn_time: bool,
    pub despawned: Vec<u64>,
    pub removed_respawn_times: Vec<(PoolMemberKindLikeCpp, u64)>,
    pub child_pool_plans: Vec<PoolDespawnPoolPlanLikeCpp>,
}

/// One represented specialization call of C++ `PoolMgr::DespawnPool<T>`.
#[derive(Debug, Clone, PartialEq)]
pub struct PoolTypedDespawnPlanLikeCpp {
    pub kind: PoolMemberKindLikeCpp,
    pub pool_id: u32,
    pub requested_guid: u64,
    pub always_delete_respawn_time: bool,
    pub object_plan: Option<PoolDespawnObjectPlanLikeCpp>,
    pub skip_reason: Option<PoolMgrPlanSkipReasonLikeCpp>,
}

/// Deterministic result of represented C++ `PoolMgr::DespawnPool(...)`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PoolDespawnPoolPlanLikeCpp {
    pub pool_id: u32,
    pub always_delete_respawn_time: bool,
    pub subplans: Vec<PoolTypedDespawnPlanLikeCpp>,
}

/// Deterministic result of represented C++ `PoolGroup<T>::SpawnObject`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PoolSpawnObjectPlanLikeCpp {
    pub actions: Vec<PoolSpawnObjectActionLikeCpp>,
    pub selected: Vec<PoolObjectLikeCpp>,
    pub despawned_trigger: Option<u64>,
    pub respawned_trigger: bool,
    /// Nested represented C++ `PoolGroup<Pool>::Spawn1Object` calls.
    ///
    /// C++ recurses through `sPoolMgr->SpawnPool(spawns, obj->guid)` for child
    /// pool Spawn1Object. `PoolGroup<Pool>::ReSpawn1Object` remains a no-op and
    /// therefore does not produce entries here.
    pub child_pool_spawn_plans: Vec<PoolSpawnPoolPlanLikeCpp>,
    /// Nested represented C++ `PoolGroup<Pool>::Despawn1Object` calls reached by
    /// `PoolGroup<Pool>::SpawnObject`'s final `DespawnObject(triggerFrom)`.
    pub child_pool_despawn_plans: Vec<PoolDespawnPoolPlanLikeCpp>,
}

/// Typed no-op/blocked reason recorded by represented `PoolMgr::SpawnPool` planning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolMgrPlanSkipReasonLikeCpp {
    MissingGroup,
    EmptyGroup,
}

/// Typed error for deterministic `PoolMgr` planning helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PoolMgrPlanErrorLikeCpp {
    MissingTemplate {
        pool_id: u32,
    },
    WrongGroupKind {
        expected: PoolMemberKindLikeCpp,
        actual: PoolMemberKindLikeCpp,
    },
    UnsupportedSpawnType {
        spawn_type: SpawnObjectType,
    },
    ChildPoolIdOverflow {
        child_pool_id: u64,
    },
    /// Rust guard for invalid pool dependency data that Trinity normally avoids
    /// during pool loading by removing circular relations.
    ChildPoolCycle {
        pool_id: u32,
    },
}

/// One represented specialization call of C++ `PoolMgr::SpawnPool<T>`.
#[derive(Debug, Clone, PartialEq)]
pub struct PoolTypedSpawnPlanLikeCpp {
    pub kind: PoolMemberKindLikeCpp,
    pub pool_id: u32,
    pub trigger_from: u64,
    pub max_limit: Option<u32>,
    pub object_plan: Option<PoolSpawnObjectPlanLikeCpp>,
    pub skip_reason: Option<PoolMgrPlanSkipReasonLikeCpp>,
}

/// Deterministic result of represented C++ `PoolMgr::SpawnPool(...)` orchestration.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PoolSpawnPoolPlanLikeCpp {
    pub pool_id: u32,
    pub subplans: Vec<PoolTypedSpawnPlanLikeCpp>,
}

/// Non-panicking report item for represented C++ `PoolMgr::InitPoolsForMap`.
#[derive(Debug, Clone, PartialEq)]
pub struct PoolInitForMapErrorLikeCpp {
    pub map_id: u32,
    pub pool_id: Option<u32>,
    pub error: PoolInitForMapErrorKindLikeCpp,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PoolInitForMapErrorKindLikeCpp {
    MapIdOutOfI32Range,
    PoolPlan(PoolMgrPlanErrorLikeCpp),
}

/// Deterministic represented result of C++ `PoolMgr::InitPoolsForMap(Map*)`.
///
/// C++ allocates fresh `SpawnedPoolData(map)`, looks up
/// `mAutoSpawnPoolsPerMap[map->GetId()]`, then calls `SpawnPool` for each pool
/// id in vector order. Rust keeps the caller-owned map `SpawnedPoolDataLikeCpp`
/// as source of truth, mutates it through `spawn_pool_plan_like_cpp`, and records
/// side-effect actions instead of creating/removing live entities.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PoolInitForMapPlanLikeCpp {
    pub map_id: u32,
    pub pools: Vec<PoolSpawnPoolPlanLikeCpp>,
    pub errors: Vec<PoolInitForMapErrorLikeCpp>,
}

impl PoolInitForMapPlanLikeCpp {
    #[must_use]
    pub fn attempted(&self) -> usize {
        self.pools.len()
            + self
                .errors
                .iter()
                .filter(|error| error.pool_id.is_some())
                .count()
    }

    #[must_use]
    pub fn planned(&self) -> usize {
        self.pools.len()
    }

    #[must_use]
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    #[must_use]
    pub fn spawn_one_actions(&self) -> usize {
        self.count_actions_like_cpp(|action| {
            matches!(action, PoolSpawnObjectActionLikeCpp::SpawnOne { .. })
        })
    }

    #[must_use]
    pub fn respawn_one_actions(&self) -> usize {
        self.count_actions_like_cpp(|action| {
            matches!(action, PoolSpawnObjectActionLikeCpp::RespawnOne { .. })
        })
    }

    #[must_use]
    pub fn despawn_one_actions(&self) -> usize {
        self.count_actions_like_cpp(|action| {
            matches!(action, PoolSpawnObjectActionLikeCpp::DespawnOne { .. })
        })
    }

    pub(super) fn count_actions_like_cpp(
        &self,
        mut matches_action: impl FnMut(&PoolSpawnObjectActionLikeCpp) -> bool,
    ) -> usize {
        self.pools
            .iter()
            .flat_map(|pool| pool.subplans.iter())
            .filter_map(|subplan| subplan.object_plan.as_ref())
            .flat_map(|object_plan| object_plan.actions.iter())
            .filter(|action| matches_action(action))
            .count()
    }
}

/// C++-shaped pure `PoolMgr` planner.
///
/// This struct owns only template/group/index data needed to plan C++
/// `PoolMgr::SpawnPool`/`UpdatePool` branch order. Runtime spawned state stays
/// exclusively in the caller-provided map-owned `SpawnedPoolDataLikeCpp`; live
/// entity creation, DB writes, recursive live PoolMgr execution, scripts, map
/// fanout, and server networking are intentionally represented only as action
/// records returned by the existing `PoolGroupLikeCpp::spawn_object_plan_like_cpp`.
#[derive(Debug, Clone, Default)]
pub struct PoolMgrLikeCpp {
    pub templates: HashMap<u32, PoolTemplateDataLikeCpp>,
    pub creature_groups: HashMap<u32, PoolGroupLikeCpp>,
    pub gameobject_groups: HashMap<u32, PoolGroupLikeCpp>,
    pub pool_groups: HashMap<u32, PoolGroupLikeCpp>,
    pub creature_spawn_to_pool: HashMap<SpawnId, u32>,
    pub gameobject_spawn_to_pool: HashMap<SpawnId, u32>,
    pub child_pool_to_parent: HashMap<u32, u32>,
    /// C++ `mAutoSpawnPoolsPerMap`; key intentionally stays signed to preserve
    /// `PoolTemplateData::MapId == -1` during honest load/report validation.
    pub auto_spawn_pools_per_map: HashMap<i32, Vec<u32>>,
}

impl PoolMgrLikeCpp {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_template_like_cpp(&mut self, pool_id: u32, template: PoolTemplateDataLikeCpp) {
        self.templates.insert(pool_id, template);
    }

    #[must_use]
    pub fn pool_template_like_cpp(&self, pool_id: u32) -> Option<&PoolTemplateDataLikeCpp> {
        self.templates.get(&pool_id)
    }

    pub fn insert_or_replace_group_like_cpp(
        &mut self,
        kind: PoolMemberKindLikeCpp,
        pool_id: u32,
        mut group: PoolGroupLikeCpp,
    ) -> Result<Option<PoolGroupLikeCpp>, PoolMgrPlanErrorLikeCpp> {
        if group.member_kind() != kind {
            return Err(PoolMgrPlanErrorLikeCpp::WrongGroupKind {
                expected: kind,
                actual: group.member_kind(),
            });
        }
        group.set_pool_id_like_cpp(pool_id);
        let replaced = match kind {
            PoolMemberKindLikeCpp::Creature => self.creature_groups.insert(pool_id, group),
            PoolMemberKindLikeCpp::GameObject => self.gameobject_groups.insert(pool_id, group),
            PoolMemberKindLikeCpp::Pool => self.pool_groups.insert(pool_id, group),
        };
        Ok(replaced)
    }

    pub fn register_spawn_pool_relation_like_cpp(
        &mut self,
        kind: PoolMemberKindLikeCpp,
        spawn_id: SpawnId,
        pool_id: u32,
    ) -> Result<Option<u32>, PoolMgrPlanErrorLikeCpp> {
        match kind {
            PoolMemberKindLikeCpp::Creature => {
                Ok(self.creature_spawn_to_pool.insert(spawn_id, pool_id))
            }
            PoolMemberKindLikeCpp::GameObject => {
                Ok(self.gameobject_spawn_to_pool.insert(spawn_id, pool_id))
            }
            PoolMemberKindLikeCpp::Pool => {
                self.register_child_pool_relation_like_cpp(spawn_id, pool_id)
            }
        }
    }

    pub fn register_child_pool_relation_like_cpp(
        &mut self,
        child_pool_id: u64,
        parent_pool_id: u32,
    ) -> Result<Option<u32>, PoolMgrPlanErrorLikeCpp> {
        let child_pool_id = u32::try_from(child_pool_id)
            .map_err(|_| PoolMgrPlanErrorLikeCpp::ChildPoolIdOverflow { child_pool_id })?;
        Ok(self
            .child_pool_to_parent
            .insert(child_pool_id, parent_pool_id))
    }

    pub fn add_auto_spawn_pool_like_cpp(&mut self, map_id: i32, pool_id: u32) {
        self.auto_spawn_pools_per_map
            .entry(map_id)
            .or_default()
            .push(pool_id);
    }

    #[must_use]
    pub fn auto_spawn_pools_for_map_like_cpp(&self, map_id: i32) -> &[u32] {
        self.auto_spawn_pools_per_map
            .get(&map_id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    #[must_use]
    pub fn auto_spawn_pools_per_map_like_cpp(&self) -> &HashMap<i32, Vec<u32>> {
        &self.auto_spawn_pools_per_map
    }

    pub fn init_pools_for_map_plan_like_cpp(
        &self,
        map_id: u32,
        spawns: &mut SpawnedPoolDataLikeCpp,
        mut explicit_roll_for: impl FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        mut choose_equal: impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
    ) -> PoolInitForMapPlanLikeCpp {
        let mut plan = PoolInitForMapPlanLikeCpp {
            map_id,
            ..PoolInitForMapPlanLikeCpp::default()
        };
        let Ok(map_id_key) = i32::try_from(map_id) else {
            plan.errors.push(PoolInitForMapErrorLikeCpp {
                map_id,
                pool_id: None,
                error: PoolInitForMapErrorKindLikeCpp::MapIdOutOfI32Range,
            });
            return plan;
        };

        for pool_id in self.auto_spawn_pools_for_map_like_cpp(map_id_key) {
            match self.spawn_pool_plan_like_cpp(
                spawns,
                *pool_id,
                &mut explicit_roll_for,
                &mut choose_equal,
            ) {
                Ok(pool_plan) => plan.pools.push(pool_plan),
                Err(error) => plan.errors.push(PoolInitForMapErrorLikeCpp {
                    map_id,
                    pool_id: Some(*pool_id),
                    error: PoolInitForMapErrorKindLikeCpp::PoolPlan(error),
                }),
            }
        }

        plan
    }

    pub fn remove_child_pool_relation_like_cpp(
        &mut self,
        child_pool_id: u32,
        parent_pool_id: u32,
    ) -> PoolRelationRemovalLikeCpp {
        let removal = self
            .pool_groups
            .get_mut(&parent_pool_id)
            .map(|group| group.remove_one_relation_like_cpp(child_pool_id))
            .unwrap_or_default();
        self.child_pool_to_parent.remove(&child_pool_id);
        removal
    }

    #[must_use]
    pub fn top_level_auto_spawn_candidate_like_cpp(&self, pool_id: u32) -> Option<i32> {
        if self.is_empty_like_cpp(pool_id) || !self.check_pool_like_cpp(pool_id) {
            return None;
        }
        if self.child_pool_to_parent.contains_key(&pool_id) {
            return None;
        }
        self.templates.get(&pool_id).map(|template| template.map_id)
    }

    pub fn is_part_of_a_pool_like_cpp(
        &self,
        spawn_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Result<u32, PoolMgrPlanErrorLikeCpp> {
        match spawn_type {
            SpawnObjectType::Creature => {
                Ok(*self.creature_spawn_to_pool.get(&spawn_id).unwrap_or(&0))
            }
            SpawnObjectType::GameObject => {
                Ok(*self.gameobject_spawn_to_pool.get(&spawn_id).unwrap_or(&0))
            }
            SpawnObjectType::AreaTrigger => Ok(0),
        }
    }

    pub fn spawn_pool_plan_like_cpp(
        &self,
        spawns: &mut SpawnedPoolDataLikeCpp,
        pool_id: u32,
        mut explicit_roll_for: impl FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        mut choose_equal: impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
    ) -> Result<PoolSpawnPoolPlanLikeCpp, PoolMgrPlanErrorLikeCpp> {
        let mut visiting = HashSet::new();
        self.spawn_pool_plan_with_visited_like_cpp(
            spawns,
            pool_id,
            &mut explicit_roll_for,
            &mut choose_equal,
            &mut visiting,
        )
    }

    pub(super) fn spawn_pool_plan_with_visited_like_cpp(
        &self,
        spawns: &mut SpawnedPoolDataLikeCpp,
        pool_id: u32,
        explicit_roll_for: &mut impl FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        choose_equal: &mut impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        visiting: &mut HashSet<u32>,
    ) -> Result<PoolSpawnPoolPlanLikeCpp, PoolMgrPlanErrorLikeCpp> {
        if !visiting.insert(pool_id) {
            return Err(PoolMgrPlanErrorLikeCpp::ChildPoolCycle { pool_id });
        }

        let mut plan = PoolSpawnPoolPlanLikeCpp {
            pool_id,
            subplans: Vec::new(),
        };
        for kind in [
            PoolMemberKindLikeCpp::Pool,
            PoolMemberKindLikeCpp::GameObject,
            PoolMemberKindLikeCpp::Creature,
        ] {
            plan.subplans
                .push(self.spawn_typed_pool_plan_with_visited_like_cpp(
                    kind,
                    spawns,
                    pool_id,
                    0,
                    explicit_roll_for,
                    choose_equal,
                    visiting,
                )?);
        }
        visiting.remove(&pool_id);
        Ok(plan)
    }

    pub fn spawn_typed_pool_plan_like_cpp(
        &self,
        kind: PoolMemberKindLikeCpp,
        spawns: &mut SpawnedPoolDataLikeCpp,
        pool_id: u32,
        trigger_from: u64,
        mut explicit_roll_for: impl FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        mut choose_equal: impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
    ) -> Result<PoolTypedSpawnPlanLikeCpp, PoolMgrPlanErrorLikeCpp> {
        let mut visiting = HashSet::new();
        self.spawn_typed_pool_plan_with_visited_like_cpp(
            kind,
            spawns,
            pool_id,
            trigger_from,
            &mut explicit_roll_for,
            &mut choose_equal,
            &mut visiting,
        )
    }

    pub(super) fn spawn_typed_pool_plan_with_visited_like_cpp(
        &self,
        kind: PoolMemberKindLikeCpp,
        spawns: &mut SpawnedPoolDataLikeCpp,
        pool_id: u32,
        trigger_from: u64,
        explicit_roll_for: &mut impl FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        choose_equal: &mut impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
        visiting: &mut HashSet<u32>,
    ) -> Result<PoolTypedSpawnPlanLikeCpp, PoolMgrPlanErrorLikeCpp> {
        let Some(group) = self.group_like_cpp(kind, pool_id) else {
            return Ok(PoolTypedSpawnPlanLikeCpp {
                kind,
                pool_id,
                trigger_from,
                max_limit: None,
                object_plan: None,
                skip_reason: Some(PoolMgrPlanSkipReasonLikeCpp::MissingGroup),
            });
        };
        if group.is_empty_like_cpp() {
            return Ok(PoolTypedSpawnPlanLikeCpp {
                kind,
                pool_id,
                trigger_from,
                max_limit: None,
                object_plan: None,
                skip_reason: Some(PoolMgrPlanSkipReasonLikeCpp::EmptyGroup),
            });
        }
        self.ensure_no_child_pool_overflow_like_cpp(group)?;
        let template = self
            .templates
            .get(&pool_id)
            .ok_or(PoolMgrPlanErrorLikeCpp::MissingTemplate { pool_id })?;
        let object_plan = if kind == PoolMemberKindLikeCpp::Pool {
            let recursive_state =
                std::cell::RefCell::new((explicit_roll_for, choose_equal, visiting));
            group.spawn_object_plan_with_child_pools_like_cpp(
                spawns,
                template.max_limit,
                trigger_from,
                || {
                    let mut state = recursive_state.borrow_mut();
                    let (explicit_roll_for, _, _) = &mut *state;
                    explicit_roll_for(kind, pool_id)
                },
                |candidates, count| {
                    let mut state = recursive_state.borrow_mut();
                    let (_, choose_equal, _) = &mut *state;
                    choose_equal(candidates, count)
                },
                |spawns, child_pool_id| {
                    let mut state = recursive_state.borrow_mut();
                    let (explicit_roll_for, choose_equal, visiting) = &mut *state;
                    self.spawn_pool_plan_with_visited_like_cpp(
                        spawns,
                        child_pool_id,
                        *explicit_roll_for,
                        *choose_equal,
                        *visiting,
                    )
                },
                |spawns, child_pool_id, _always_delete_respawn_time| {
                    let mut state = recursive_state.borrow_mut();
                    let (_, _, visiting) = &mut *state;
                    self.despawn_pool_plan_with_visited_like_cpp(
                        spawns,
                        child_pool_id,
                        false,
                        *visiting,
                    )
                },
            )?
        } else {
            group.spawn_object_plan_like_cpp(
                spawns,
                template.max_limit,
                trigger_from,
                || explicit_roll_for(kind, pool_id),
                &mut *choose_equal,
            )
        };
        Ok(PoolTypedSpawnPlanLikeCpp {
            kind,
            pool_id,
            trigger_from,
            max_limit: Some(template.max_limit),
            object_plan: Some(object_plan),
            skip_reason: None,
        })
    }

    pub fn update_pool_plan_like_cpp(
        &self,
        spawns: &mut SpawnedPoolDataLikeCpp,
        pool_id: u32,
        spawn_type: SpawnObjectType,
        spawn_id: SpawnId,
        mut explicit_roll_for: impl FnMut(PoolMemberKindLikeCpp, u32) -> f32,
        mut choose_equal: impl FnMut(&[PoolObjectLikeCpp], usize) -> Vec<usize>,
    ) -> Result<PoolTypedSpawnPlanLikeCpp, PoolMgrPlanErrorLikeCpp> {
        if spawn_type == SpawnObjectType::AreaTrigger {
            return Err(PoolMgrPlanErrorLikeCpp::UnsupportedSpawnType { spawn_type });
        }
        if let Some(&mother_pool_id) = self.child_pool_to_parent.get(&pool_id) {
            return self.spawn_typed_pool_plan_like_cpp(
                PoolMemberKindLikeCpp::Pool,
                spawns,
                mother_pool_id,
                u64::from(pool_id),
                &mut explicit_roll_for,
                choose_equal,
            );
        }
        let kind = match spawn_type {
            SpawnObjectType::Creature => PoolMemberKindLikeCpp::Creature,
            SpawnObjectType::GameObject => PoolMemberKindLikeCpp::GameObject,
            SpawnObjectType::AreaTrigger => unreachable!("AreaTrigger returned above"),
        };
        let mut visiting = HashSet::new();
        self.spawn_typed_pool_plan_with_visited_like_cpp(
            kind,
            spawns,
            pool_id,
            spawn_id,
            &mut explicit_roll_for,
            &mut choose_equal,
            &mut visiting,
        )
    }

    pub fn despawn_pool_plan_like_cpp(
        &self,
        spawns: &mut SpawnedPoolDataLikeCpp,
        pool_id: u32,
        always_delete_respawn_time: bool,
    ) -> Result<PoolDespawnPoolPlanLikeCpp, PoolMgrPlanErrorLikeCpp> {
        let mut visiting = HashSet::new();
        self.despawn_pool_plan_with_visited_like_cpp(
            spawns,
            pool_id,
            always_delete_respawn_time,
            &mut visiting,
        )
    }

    pub(super) fn despawn_pool_plan_with_visited_like_cpp(
        &self,
        spawns: &mut SpawnedPoolDataLikeCpp,
        pool_id: u32,
        always_delete_respawn_time: bool,
        visiting: &mut HashSet<u32>,
    ) -> Result<PoolDespawnPoolPlanLikeCpp, PoolMgrPlanErrorLikeCpp> {
        if !visiting.insert(pool_id) {
            return Err(PoolMgrPlanErrorLikeCpp::ChildPoolCycle { pool_id });
        }

        let mut plan = PoolDespawnPoolPlanLikeCpp {
            pool_id,
            always_delete_respawn_time,
            subplans: Vec::new(),
        };
        for kind in [
            PoolMemberKindLikeCpp::Creature,
            PoolMemberKindLikeCpp::GameObject,
            PoolMemberKindLikeCpp::Pool,
        ] {
            plan.subplans
                .push(self.despawn_typed_pool_plan_with_visited_like_cpp(
                    kind,
                    spawns,
                    pool_id,
                    0,
                    always_delete_respawn_time,
                    visiting,
                )?);
        }
        visiting.remove(&pool_id);
        Ok(plan)
    }

    pub fn despawn_typed_pool_plan_like_cpp(
        &self,
        kind: PoolMemberKindLikeCpp,
        spawns: &mut SpawnedPoolDataLikeCpp,
        pool_id: u32,
        requested_guid: u64,
        always_delete_respawn_time: bool,
    ) -> Result<PoolTypedDespawnPlanLikeCpp, PoolMgrPlanErrorLikeCpp> {
        let mut visiting = HashSet::new();
        visiting.insert(pool_id);
        self.despawn_typed_pool_plan_with_visited_like_cpp(
            kind,
            spawns,
            pool_id,
            requested_guid,
            always_delete_respawn_time,
            &mut visiting,
        )
    }

    pub(super) fn despawn_typed_pool_plan_with_visited_like_cpp(
        &self,
        kind: PoolMemberKindLikeCpp,
        spawns: &mut SpawnedPoolDataLikeCpp,
        pool_id: u32,
        requested_guid: u64,
        always_delete_respawn_time: bool,
        visiting: &mut HashSet<u32>,
    ) -> Result<PoolTypedDespawnPlanLikeCpp, PoolMgrPlanErrorLikeCpp> {
        let Some(group) = self.group_like_cpp(kind, pool_id) else {
            return Ok(PoolTypedDespawnPlanLikeCpp {
                kind,
                pool_id,
                requested_guid,
                always_delete_respawn_time,
                object_plan: None,
                skip_reason: Some(PoolMgrPlanSkipReasonLikeCpp::MissingGroup),
            });
        };
        if group.is_empty_like_cpp() {
            return Ok(PoolTypedDespawnPlanLikeCpp {
                kind,
                pool_id,
                requested_guid,
                always_delete_respawn_time,
                object_plan: None,
                skip_reason: Some(PoolMgrPlanSkipReasonLikeCpp::EmptyGroup),
            });
        }
        let object_plan = group.despawn_object_plan_like_cpp(
            spawns,
            requested_guid,
            always_delete_respawn_time,
            |spawns, child_pool_id, always_delete_respawn_time| {
                self.despawn_pool_plan_with_visited_like_cpp(
                    spawns,
                    child_pool_id,
                    always_delete_respawn_time,
                    visiting,
                )
            },
        )?;
        Ok(PoolTypedDespawnPlanLikeCpp {
            kind,
            pool_id,
            requested_guid,
            always_delete_respawn_time,
            object_plan: Some(object_plan),
            skip_reason: None,
        })
    }

    #[must_use]
    pub fn is_empty_like_cpp(&self, pool_id: u32) -> bool {
        let mut visiting = HashSet::new();
        self.is_empty_with_visited_like_cpp(pool_id, &mut visiting)
    }

    #[must_use]
    pub fn check_pool_like_cpp(&self, pool_id: u32) -> bool {
        for kind in [
            PoolMemberKindLikeCpp::GameObject,
            PoolMemberKindLikeCpp::Creature,
            PoolMemberKindLikeCpp::Pool,
        ] {
            if let Some(group) = self.group_like_cpp(kind, pool_id) {
                if !group.check_pool_like_cpp() {
                    return false;
                }
            }
        }
        true
    }

    pub(super) fn group_like_cpp(
        &self,
        kind: PoolMemberKindLikeCpp,
        pool_id: u32,
    ) -> Option<&PoolGroupLikeCpp> {
        match kind {
            PoolMemberKindLikeCpp::Creature => self.creature_groups.get(&pool_id),
            PoolMemberKindLikeCpp::GameObject => self.gameobject_groups.get(&pool_id),
            PoolMemberKindLikeCpp::Pool => self.pool_groups.get(&pool_id),
        }
    }

    pub(super) fn ensure_no_child_pool_overflow_like_cpp(
        &self,
        group: &PoolGroupLikeCpp,
    ) -> Result<(), PoolMgrPlanErrorLikeCpp> {
        if group.member_kind() != PoolMemberKindLikeCpp::Pool {
            return Ok(());
        }
        for child in group
            .explicitly_chanced_like_cpp()
            .iter()
            .chain(group.equal_chanced_like_cpp().iter())
        {
            if u32::try_from(child.guid).is_err() {
                return Err(PoolMgrPlanErrorLikeCpp::ChildPoolIdOverflow {
                    child_pool_id: child.guid,
                });
            }
        }
        Ok(())
    }

    pub(super) fn is_empty_with_visited_like_cpp(
        &self,
        pool_id: u32,
        visiting: &mut HashSet<u32>,
    ) -> bool {
        if !visiting.insert(pool_id) {
            return false;
        }
        for kind in [
            PoolMemberKindLikeCpp::GameObject,
            PoolMemberKindLikeCpp::Creature,
            PoolMemberKindLikeCpp::Pool,
        ] {
            if let Some(group) = self.group_like_cpp(kind, pool_id) {
                let empty = if kind == PoolMemberKindLikeCpp::Pool {
                    group.is_empty_deep_check_like_cpp(|child_pool_id| {
                        self.is_empty_with_visited_like_cpp(child_pool_id, visiting)
                    })
                } else {
                    group.is_empty_deep_check_like_cpp(|_| true)
                };
                if !empty {
                    visiting.remove(&pool_id);
                    return false;
                }
            }
        }
        visiting.remove(&pool_id);
        true
    }
}

/// C++-shaped `PoolGroup<T>` buckets and pure helpers.
#[derive(Debug, Clone, PartialEq)]
pub struct PoolGroupLikeCpp {
    pub(super) pool_id: u32,
    pub(super) member_kind: PoolMemberKindLikeCpp,
    pub(super) explicitly_chanced: Vec<PoolObjectLikeCpp>,
    pub(super) equal_chanced: Vec<PoolObjectLikeCpp>,
}
