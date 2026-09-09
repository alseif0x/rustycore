//! Spawn object and group model state definitions, part 1 of 2.
//!
//! Separated from the spawn.rs root under #644. Behaviour is preserved.

use super::*;

pub type SpawnId = u64;

pub type Difficulty = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum SpawnObjectType {
    Creature = 0,
    GameObject = 1,
    AreaTrigger = 2,
}

impl SpawnObjectType {
    pub const fn type_has_data(self) -> bool {
        matches!(self, Self::Creature | Self::GameObject | Self::AreaTrigger)
    }

    pub const fn mask(self) -> u32 {
        1 << self as u8
    }

    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::Creature),
            1 => Some(Self::GameObject),
            2 => Some(Self::AreaTrigger),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LinkedRespawnTypeLikeCpp {
    CreatureToCreature = 0,
    CreatureToGameObject = 1,
    GameObjectToGameObject = 2,
    GameObjectToCreature = 3,
}

impl LinkedRespawnTypeLikeCpp {
    pub const fn from_raw(raw: u8) -> Option<Self> {
        match raw {
            0 => Some(Self::CreatureToCreature),
            1 => Some(Self::CreatureToGameObject),
            2 => Some(Self::GameObjectToGameObject),
            3 => Some(Self::GameObjectToCreature),
            _ => None,
        }
    }

    pub const fn slave_type(self) -> SpawnObjectType {
        match self {
            Self::CreatureToCreature | Self::CreatureToGameObject => SpawnObjectType::Creature,
            Self::GameObjectToGameObject | Self::GameObjectToCreature => {
                SpawnObjectType::GameObject
            }
        }
    }

    pub const fn master_type(self) -> SpawnObjectType {
        match self {
            Self::CreatureToCreature | Self::GameObjectToCreature => SpawnObjectType::Creature,
            Self::CreatureToGameObject | Self::GameObjectToGameObject => {
                SpawnObjectType::GameObject
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkedRespawnRowLikeCpp {
    pub guid: SpawnId,
    pub linked_guid: SpawnId,
    pub link_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkedRespawnLoadIssueKindLikeCpp {
    InvalidType,
    MissingSlave,
    MissingMaster,
    NotInstanceableOrMapMismatch,
    DifficultyMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedRespawnLoadIssueLikeCpp {
    pub kind: LinkedRespawnLoadIssueKindLikeCpp,
    pub guid: SpawnId,
    pub linked_guid: SpawnId,
    pub link_type: u8,
    pub slave_type: Option<SpawnObjectType>,
    pub master_type: Option<SpawnObjectType>,
    pub slave_map_id: Option<u32>,
    pub master_map_id: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinkedRespawnLoadReportLikeCpp {
    pub rows: usize,
    pub inserted: usize,
    pub invalid_type: usize,
    pub missing_slave: usize,
    pub missing_master: usize,
    pub not_instanceable_or_map_mismatch: usize,
    pub difficulty_mismatch: usize,
    pub issues: Vec<LinkedRespawnLoadIssueLikeCpp>,
}

impl LinkedRespawnLoadReportLikeCpp {
    pub fn push(&mut self, issue: LinkedRespawnLoadIssueLikeCpp) {
        match issue.kind {
            LinkedRespawnLoadIssueKindLikeCpp::InvalidType => self.invalid_type += 1,
            LinkedRespawnLoadIssueKindLikeCpp::MissingSlave => self.missing_slave += 1,
            LinkedRespawnLoadIssueKindLikeCpp::MissingMaster => self.missing_master += 1,
            LinkedRespawnLoadIssueKindLikeCpp::NotInstanceableOrMapMismatch => {
                self.not_instanceable_or_map_mismatch += 1;
            }
            LinkedRespawnLoadIssueKindLikeCpp::DifficultyMismatch => self.difficulty_mismatch += 1,
        }
        self.issues.push(issue);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LinkedRespawnStoreLikeCpp {
    pub(super) linked_respawns: BTreeMap<ObjectGuid, ObjectGuid>,
}

impl LinkedRespawnStoreLikeCpp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_like_cpp(
        &mut self,
        guid: ObjectGuid,
        linked_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        self.linked_respawns.insert(guid, linked_guid)
    }

    pub fn get_linked_respawn_guid_like_cpp(&self, guid: ObjectGuid) -> ObjectGuid {
        self.linked_respawns
            .get(&guid)
            .copied()
            .unwrap_or(ObjectGuid::EMPTY)
    }

    pub fn len(&self) -> usize {
        self.linked_respawns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.linked_respawns.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpawnGroupFlags(pub u32);

impl SpawnGroupFlags {
    pub const NONE: Self = Self(0x00);
    pub const SYSTEM: Self = Self(0x01);
    pub const COMPATIBILITY_MODE: Self = Self(0x02);
    pub const MANUAL_SPAWN: Self = Self(0x04);
    pub const DYNAMIC_SPAWN_RATE: Self = Self(0x08);
    pub const ESCORTQUESTNPC: Self = Self(0x10);
    pub const DESPAWN_ON_CONDITION_FAILURE: Self = Self(0x20);
    pub const ALL: Self = Self(
        Self::SYSTEM.0
            | Self::COMPATIBILITY_MODE.0
            | Self::MANUAL_SPAWN.0
            | Self::DYNAMIC_SPAWN_RATE.0
            | Self::ESCORTQUESTNPC.0
            | Self::DESPAWN_ON_CONDITION_FAILURE.0,
    );

    pub const fn contains(self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }

    pub const fn truncate_to_all(self) -> Self {
        Self(self.0 & Self::ALL.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnPosition {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub orientation: f32,
}

impl SpawnPosition {
    pub const fn new(x: f32, y: f32, z: f32, orientation: f32) -> Self {
        Self {
            x,
            y,
            z,
            orientation,
        }
    }
}

pub const SPAWNGROUP_MAP_UNSET: u32 = 0xffff_ffff;

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnGroupTemplateData {
    pub group_id: u32,
    pub name: String,
    pub map_id: u32,
    pub flags: SpawnGroupFlags,
}

impl SpawnGroupTemplateData {
    pub fn default_group() -> Self {
        Self {
            group_id: 0,
            name: "Default Group".to_string(),
            map_id: 0,
            flags: SpawnGroupFlags::SYSTEM,
        }
    }

    pub fn legacy_group() -> Self {
        Self {
            group_id: 1,
            name: "Legacy Group".to_string(),
            map_id: 0,
            flags: SpawnGroupFlags(
                SpawnGroupFlags::SYSTEM.0 | SpawnGroupFlags::COMPATIBILITY_MODE.0,
            ),
        }
    }

    pub const fn is_system(&self) -> bool {
        self.flags.contains(SpawnGroupFlags::SYSTEM)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnGroupActiveChange {
    MissingGroup,
    SystemGroup,
    Toggled,
    ClearedToggle,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpawnGroupRuntimeState {
    pub(super) toggled_spawn_group_ids: BTreeSet<u32>,
}

impl SpawnGroupRuntimeState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_spawn_group_active_like_cpp(
        &mut self,
        group: Option<&SpawnGroupTemplateData>,
        state: bool,
    ) -> SpawnGroupActiveChange {
        let Some(group) = group else {
            return SpawnGroupActiveChange::MissingGroup;
        };
        if group.is_system() {
            return SpawnGroupActiveChange::SystemGroup;
        }

        if state != spawn_group_default_active_like_cpp(group) {
            self.toggled_spawn_group_ids.insert(group.group_id);
            SpawnGroupActiveChange::Toggled
        } else {
            self.toggled_spawn_group_ids.remove(&group.group_id);
            SpawnGroupActiveChange::ClearedToggle
        }
    }

    pub fn is_spawn_group_active_like_cpp(&self, group: Option<&SpawnGroupTemplateData>) -> bool {
        let Some(group) = group else {
            return false;
        };
        if group.is_system() {
            return true;
        }

        self.toggled_spawn_group_ids.contains(&group.group_id)
            != spawn_group_default_active_like_cpp(group)
    }

    pub fn is_toggled(&self, group_id: u32) -> bool {
        self.toggled_spawn_group_ids.contains(&group_id)
    }

    pub fn toggled_spawn_group_ids(&self) -> &BTreeSet<u32> {
        &self.toggled_spawn_group_ids
    }
}

pub(super) fn spawn_group_default_active_like_cpp(group: &SpawnGroupTemplateData) -> bool {
    !group.flags.contains(SpawnGroupFlags::MANUAL_SPAWN)
}

#[derive(Debug, Clone)]
pub struct SpawnGridLoadStateLikeCpp<'a> {
    pub(super) spawn_store: &'a SpawnStore,
    pub(super) spawn_group_state: &'a SpawnGroupRuntimeState,
    pub(super) respawn_timers: BTreeSet<(SpawnObjectType, SpawnId)>,
    pub(super) pool_spawned_objects: BTreeSet<(SpawnObjectType, SpawnId)>,
}

impl<'a> SpawnGridLoadStateLikeCpp<'a> {
    pub fn new(spawn_store: &'a SpawnStore, spawn_group_state: &'a SpawnGroupRuntimeState) -> Self {
        Self {
            spawn_store,
            spawn_group_state,
            respawn_timers: BTreeSet::new(),
            pool_spawned_objects: BTreeSet::new(),
        }
    }

    pub fn with_respawn_timers(
        mut self,
        respawn_timers: impl IntoIterator<Item = (SpawnObjectType, SpawnId)>,
    ) -> Self {
        self.respawn_timers.extend(respawn_timers);
        self
    }

    pub fn with_pool_spawned_objects(
        mut self,
        pool_spawned_objects: impl IntoIterator<Item = (SpawnObjectType, SpawnId)>,
    ) -> Self {
        self.pool_spawned_objects.extend(pool_spawned_objects);
        self
    }

    pub fn add_respawn_timer(&mut self, object_type: SpawnObjectType, spawn_id: SpawnId) {
        self.respawn_timers.insert((object_type, spawn_id));
    }

    pub fn add_pool_spawned_object(&mut self, object_type: SpawnObjectType, spawn_id: SpawnId) {
        self.pool_spawned_objects.insert((object_type, spawn_id));
    }

    pub fn should_be_spawned_on_grid_load(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> bool {
        if !object_type.type_has_data() {
            return false;
        }

        // C++ `Map::ShouldBeSpawnedOnGridLoad` checks respawn timers before
        // consulting spawn metadata, spawn group state, or pool state.
        if self.respawn_timers.contains(&(object_type, spawn_id)) {
            return false;
        }

        let Some(spawn_data) = self.spawn_store.spawn_data(object_type, spawn_id) else {
            return false;
        };
        let spawn_group = &spawn_data.spawn_group;
        if !spawn_group.is_system()
            && !self
                .spawn_group_state
                .is_spawn_group_active_like_cpp(Some(spawn_group))
        {
            return false;
        }

        if spawn_data.pool_id != 0 && !self.pool_spawned_objects.contains(&(object_type, spawn_id))
        {
            return false;
        }

        true
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnData {
    pub object_type: SpawnObjectType,
    pub spawn_id: SpawnId,
    pub map_id: u32,
    pub db_data: bool,
    pub spawn_group: SpawnGroupTemplateData,
    pub id: u32,
    pub spawn_point: SpawnPosition,
    pub phase_use_flags: u8,
    pub phase_id: u32,
    pub phase_group: u32,
    pub terrain_swap_map: i32,
    pub pool_id: u32,
    pub spawn_time_secs: i32,
    pub spawn_difficulties: Vec<Difficulty>,
    pub script_id: u32,
    pub string_id: String,
}

impl SpawnData {
    pub fn cell_id(&self) -> u32 {
        compute_cell_coord(self.spawn_point.x, self.spawn_point.y).get_id()
    }

    pub const fn spawn_group_id(&self) -> u32 {
        self.spawn_group.group_id
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpawnGroupMemberRow {
    pub group_id: u32,
    pub spawn_type: u8,
    pub spawn_id: SpawnId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpawnGroupMember {
    pub object_type: SpawnObjectType,
    pub spawn_id: SpawnId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnGroupApplyIssueKind {
    InvalidType,
    MissingSpawn,
    DuplicateSpawnGroup,
    MissingGroup,
    MapMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnGroupApplyIssue {
    pub kind: SpawnGroupApplyIssueKind,
    pub group_id: u32,
    pub spawn_type: u8,
    pub spawn_id: SpawnId,
    pub existing_group_id: Option<u32>,
    pub group_map_id: Option<u32>,
    pub spawn_map_id: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SpawnGroupApplyReport {
    pub assigned: usize,
    pub invalid_type: usize,
    pub missing_spawn: usize,
    pub duplicate_spawn_group: usize,
    pub missing_group: usize,
    pub map_mismatch: usize,
    pub issues: Vec<SpawnGroupApplyIssue>,
}

impl SpawnGroupApplyReport {
    pub(super) fn push(&mut self, issue: SpawnGroupApplyIssue) {
        match issue.kind {
            SpawnGroupApplyIssueKind::InvalidType => self.invalid_type += 1,
            SpawnGroupApplyIssueKind::MissingSpawn => self.missing_spawn += 1,
            SpawnGroupApplyIssueKind::DuplicateSpawnGroup => self.duplicate_spawn_group += 1,
            SpawnGroupApplyIssueKind::MissingGroup => self.missing_group += 1,
            SpawnGroupApplyIssueKind::MapMismatch => self.map_mismatch += 1,
        }
        self.issues.push(issue);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CellSpawnGuids {
    pub creatures: BTreeSet<SpawnId>,
    pub gameobjects: BTreeSet<SpawnId>,
    pub area_triggers: BTreeSet<SpawnId>,
}

impl CellSpawnGuids {
    pub fn is_empty(&self) -> bool {
        self.creatures.is_empty() && self.gameobjects.is_empty() && self.area_triggers.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpawnMapKey {
    pub map_id: u32,
    pub difficulty: Difficulty,
}

impl SpawnMapKey {
    pub const fn new(map_id: u32, difficulty: Difficulty) -> Self {
        Self { map_id, difficulty }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PersonalSpawnMapKey {
    pub map_id: u32,
    pub difficulty: Difficulty,
    pub phase_id: u32,
}

impl PersonalSpawnMapKey {
    pub const fn new(map_id: u32, difficulty: Difficulty, phase_id: u32) -> Self {
        Self {
            map_id,
            difficulty,
            phase_id,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SpawnStore {
    pub(super) spawns: BTreeMap<(SpawnObjectType, SpawnId), SpawnData>,
    pub(super) object_guids: BTreeMap<SpawnMapKey, BTreeMap<u32, CellSpawnGuids>>,
    pub(super) personal_object_guids: BTreeMap<PersonalSpawnMapKey, BTreeMap<u32, CellSpawnGuids>>,
    pub(super) spawn_groups_by_map: BTreeMap<u32, BTreeSet<u32>>,
    pub(super) spawn_group_members: BTreeMap<u32, BTreeSet<SpawnGroupMember>>,
}

impl SpawnStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// C++ `ObjectMgr::LoadSpawnGroups`: apply `spawn_group` rows to loaded spawn metadata.
    ///
    /// This is intentionally pure/in-memory. DB loading and map runtime activation stay outside
    /// `wow-map` until the ObjectMgr/world-server wiring slice.
    pub fn apply_spawn_groups_like_cpp(
        &mut self,
        templates: &mut BTreeMap<u32, SpawnGroupTemplateData>,
        rows: impl IntoIterator<Item = SpawnGroupMemberRow>,
    ) -> SpawnGroupApplyReport {
        let mut report = SpawnGroupApplyReport::default();

        for row in rows {
            let Some(object_type) = SpawnObjectType::from_raw(row.spawn_type) else {
                report.push(SpawnGroupApplyIssue {
                    kind: SpawnGroupApplyIssueKind::InvalidType,
                    group_id: row.group_id,
                    spawn_type: row.spawn_type,
                    spawn_id: row.spawn_id,
                    existing_group_id: None,
                    group_map_id: None,
                    spawn_map_id: None,
                });
                continue;
            };

            let key = (object_type, row.spawn_id);
            let Some(spawn) = self.spawns.get_mut(&key) else {
                report.push(SpawnGroupApplyIssue {
                    kind: SpawnGroupApplyIssueKind::MissingSpawn,
                    group_id: row.group_id,
                    spawn_type: row.spawn_type,
                    spawn_id: row.spawn_id,
                    existing_group_id: None,
                    group_map_id: None,
                    spawn_map_id: None,
                });
                continue;
            };

            if spawn.spawn_group.group_id != 0 {
                report.push(SpawnGroupApplyIssue {
                    kind: SpawnGroupApplyIssueKind::DuplicateSpawnGroup,
                    group_id: row.group_id,
                    spawn_type: row.spawn_type,
                    spawn_id: row.spawn_id,
                    existing_group_id: Some(spawn.spawn_group.group_id),
                    group_map_id: None,
                    spawn_map_id: Some(spawn.map_id),
                });
                continue;
            }

            let Some(group_template) = templates.get_mut(&row.group_id) else {
                report.push(SpawnGroupApplyIssue {
                    kind: SpawnGroupApplyIssueKind::MissingGroup,
                    group_id: row.group_id,
                    spawn_type: row.spawn_type,
                    spawn_id: row.spawn_id,
                    existing_group_id: None,
                    group_map_id: None,
                    spawn_map_id: Some(spawn.map_id),
                });
                continue;
            };

            if group_template.map_id == SPAWNGROUP_MAP_UNSET {
                group_template.map_id = spawn.map_id;
                self.spawn_groups_by_map
                    .entry(spawn.map_id)
                    .or_default()
                    .insert(row.group_id);
            } else if group_template.map_id != spawn.map_id && !group_template.is_system() {
                report.push(SpawnGroupApplyIssue {
                    kind: SpawnGroupApplyIssueKind::MapMismatch,
                    group_id: row.group_id,
                    spawn_type: row.spawn_type,
                    spawn_id: row.spawn_id,
                    existing_group_id: None,
                    group_map_id: Some(group_template.map_id),
                    spawn_map_id: Some(spawn.map_id),
                });
                continue;
            }

            spawn.spawn_group = group_template.clone();
            if !group_template.is_system() {
                self.spawn_group_members
                    .entry(row.group_id)
                    .or_default()
                    .insert(SpawnGroupMember {
                        object_type,
                        spawn_id: row.spawn_id,
                    });
            }
            report.assigned += 1;
        }

        report
    }

    /// C++ `_spawnGroupsByMap`: groups whose template map was first resolved by a spawn row.
    pub fn spawn_group_ids_by_map(&self, map_id: u32) -> Option<&BTreeSet<u32>> {
        self.spawn_groups_by_map.get(&map_id)
    }

    /// C++ `_spawnGroupMapStore`: non-system spawn members indexed by group id.
    pub fn spawn_group_members(&self, group_id: u32) -> Option<&BTreeSet<SpawnGroupMember>> {
        self.spawn_group_members.get(&group_id)
    }

    /// C++ `Map::GetSpawnGroupData` map filter shape for future runtime consumers.
    pub fn spawn_group_template_for_map<'a>(
        templates: &'a BTreeMap<u32, SpawnGroupTemplateData>,
        group_id: u32,
        map_id: u32,
    ) -> Option<&'a SpawnGroupTemplateData> {
        let data = templates.get(&group_id)?;
        if data.is_system() || data.map_id == map_id {
            Some(data)
        } else {
            None
        }
    }

    /// Inserts canonical spawn metadata without touching grid indexes.
    ///
    /// C++ `ObjectMgr::LoadCreatures` / `LoadGameObjects` always populate
    /// `_creatureDataStore` / `_gameObjectDataStore` before the `gameEvent`
    /// branch. The event branch only gates `AddCreatureToGrid` /
    /// `AddGameobjectToGrid`; it does not discard metadata.
    pub fn insert_spawn_metadata_like_cpp(&mut self, data: &SpawnData) {
        self.spawns
            .insert((data.object_type, data.spawn_id), data.clone());
    }

    /// C++ `ObjectMgr::AddSpawnDataToGrid` for creature/gameobject spawns.
    pub fn add_object_spawn<F>(&mut self, data: &SpawnData, is_personal_phase: F)
    where
        F: Fn(u32) -> bool,
    {
        match data.object_type {
            SpawnObjectType::Creature | SpawnObjectType::GameObject => {}
            SpawnObjectType::AreaTrigger => {
                self.add_area_trigger_spawn(data);
                return;
            }
        }

        self.insert_spawn_metadata_like_cpp(data);

        let cell_id = data.cell_id();
        if is_personal_phase(data.phase_id) {
            for difficulty in data.spawn_difficulties.iter().copied() {
                let key = PersonalSpawnMapKey::new(data.map_id, difficulty, data.phase_id);
                let cell = self
                    .personal_object_guids
                    .entry(key)
                    .or_default()
                    .entry(cell_id)
                    .or_default();
                insert_spawn(cell, data.object_type, data.spawn_id);
            }
        } else {
            for difficulty in data.spawn_difficulties.iter().copied() {
                let key = SpawnMapKey::new(data.map_id, difficulty);
                let cell = self
                    .object_guids
                    .entry(key)
                    .or_default()
                    .entry(cell_id)
                    .or_default();
                insert_spawn(cell, data.object_type, data.spawn_id);
            }
        }
    }

    /// C++ `AreaTriggerDataStore::LoadAreaTriggerSpawns` indexes static area
    /// triggers by map/difficulty/cell only; it does not use ObjectMgr's
    /// personal-phase store.
    pub fn add_area_trigger_spawn(&mut self, data: &SpawnData) {
        debug_assert_eq!(data.object_type, SpawnObjectType::AreaTrigger);
        self.insert_spawn_metadata_like_cpp(data);
        let cell_id = data.cell_id();
        for difficulty in data.spawn_difficulties.iter().copied() {
            let key = SpawnMapKey::new(data.map_id, difficulty);
            self.object_guids
                .entry(key)
                .or_default()
                .entry(cell_id)
                .or_default()
                .area_triggers
                .insert(data.spawn_id);
        }
    }

    pub fn remove_object_spawn<F>(&mut self, data: &SpawnData, is_personal_phase: F)
    where
        F: Fn(u32) -> bool,
    {
        match data.object_type {
            SpawnObjectType::Creature | SpawnObjectType::GameObject => {}
            SpawnObjectType::AreaTrigger => {
                self.remove_area_trigger_spawn(data);
                return;
            }
        }

        self.spawns.remove(&(data.object_type, data.spawn_id));
        let cell_id = data.cell_id();
        if is_personal_phase(data.phase_id) {
            for difficulty in data.spawn_difficulties.iter().copied() {
                let key = PersonalSpawnMapKey::new(data.map_id, difficulty, data.phase_id);
                if let Some(cells) = self.personal_object_guids.get_mut(&key) {
                    remove_spawn_from_cells(cells, cell_id, data.object_type, data.spawn_id);
                }
            }
        } else {
            for difficulty in data.spawn_difficulties.iter().copied() {
                let key = SpawnMapKey::new(data.map_id, difficulty);
                if let Some(cells) = self.object_guids.get_mut(&key) {
                    remove_spawn_from_cells(cells, cell_id, data.object_type, data.spawn_id);
                }
            }
        }
    }

    pub fn remove_area_trigger_spawn(&mut self, data: &SpawnData) {
        debug_assert_eq!(data.object_type, SpawnObjectType::AreaTrigger);
        self.spawns
            .remove(&(SpawnObjectType::AreaTrigger, data.spawn_id));
        let cell_id = data.cell_id();
        for difficulty in data.spawn_difficulties.iter().copied() {
            let key = SpawnMapKey::new(data.map_id, difficulty);
            if let Some(cells) = self.object_guids.get_mut(&key) {
                remove_spawn_from_cells(
                    cells,
                    cell_id,
                    SpawnObjectType::AreaTrigger,
                    data.spawn_id,
                );
            }
        }
    }

    pub fn cell_object_guids(
        &self,
        map_id: u32,
        difficulty: Difficulty,
        cell_id: u32,
    ) -> Option<&CellSpawnGuids> {
        self.object_guids
            .get(&SpawnMapKey::new(map_id, difficulty))?
            .get(&cell_id)
    }

    pub fn cell_personal_object_guids(
        &self,
        map_id: u32,
        difficulty: Difficulty,
        phase_id: u32,
        cell_id: u32,
    ) -> Option<&CellSpawnGuids> {
        self.personal_object_guids
            .get(&PersonalSpawnMapKey::new(map_id, difficulty, phase_id))?
            .get(&cell_id)
    }

    pub fn has_personal_spawns(&self, map_id: u32, difficulty: Difficulty, phase_id: u32) -> bool {
        self.personal_object_guids
            .contains_key(&PersonalSpawnMapKey::new(map_id, difficulty, phase_id))
    }

    pub fn spawn_data(
        &self,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    ) -> Option<&SpawnData> {
        self.spawns.get(&(object_type, spawn_id))
    }
}

pub(super) fn insert_spawn(
    cell: &mut CellSpawnGuids,
    object_type: SpawnObjectType,
    spawn_id: SpawnId,
) {
    match object_type {
        SpawnObjectType::Creature => {
            cell.creatures.insert(spawn_id);
        }
        SpawnObjectType::GameObject => {
            cell.gameobjects.insert(spawn_id);
        }
        SpawnObjectType::AreaTrigger => {
            cell.area_triggers.insert(spawn_id);
        }
    }
}

pub(super) fn remove_spawn_from_cells(
    cells: &mut BTreeMap<u32, CellSpawnGuids>,
    cell_id: u32,
    object_type: SpawnObjectType,
    spawn_id: SpawnId,
) {
    if let Some(cell) = cells.get_mut(&cell_id) {
        match object_type {
            SpawnObjectType::Creature => {
                cell.creatures.remove(&spawn_id);
            }
            SpawnObjectType::GameObject => {
                cell.gameobjects.remove(&spawn_id);
            }
            SpawnObjectType::AreaTrigger => {
                cell.area_triggers.remove(&spawn_id);
            }
        }
        if cell.is_empty() {
            cells.remove(&cell_id);
        }
    }
}

/// C++ `Map::RespawnInfo` equivalent owned by the map respawn store.
///
/// This is a dependency slice for future live `Map::ProcessRespawns` wiring: it
/// stores and plans respawn timers, but intentionally does not execute PoolMgr,
/// `DoRespawn`, DB persistence/delete, linked-respawn checks, or entity loading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RespawnInfoLikeCpp {
    pub object_type: SpawnObjectType,
    pub spawn_id: SpawnId,
    pub entry: u32,
    pub respawn_time: i64,
    pub grid_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddRespawnInfoOutcomeLikeCpp {
    Inserted,
    ReplacedExisting,
    RejectedZeroSpawnId,
    RejectedUnsupportedType,
    RejectedExistingSoonerOrEqual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckRespawnOutcomeLikeCpp {
    Allowed,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckRespawnSpawnGroupGuardOutcomeLikeCpp {
    Allowed,
    InactiveSpawnGroupDeletedTimer,
    MissingSpawnData,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessRespawnActionLikeCpp {
    UpdatePool {
        pool_id: u32,
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    },
    DoRespawn {
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
        grid_id: u32,
    },
    DeleteRespawn {
        object_type: SpawnObjectType,
        spawn_id: SpawnId,
    },
    RescheduleAndSave {
        info: RespawnInfoLikeCpp,
    },
    InvalidRescheduleNotFuture {
        info: RespawnInfoLikeCpp,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct RespawnQueueKey {
    pub(super) respawn_time: i64,
    pub(super) spawn_id: SpawnId,
    pub(super) object_type: SpawnObjectType,
}

impl RespawnQueueKey {
    pub(super) const fn from_info(info: &RespawnInfoLikeCpp) -> Self {
        Self {
            respawn_time: info.respawn_time,
            spawn_id: info.spawn_id,
            object_type: info.object_type,
        }
    }
}
