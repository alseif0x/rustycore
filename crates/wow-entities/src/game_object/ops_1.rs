//! GameObject template, loot and runtime state operations, part 1 of 2.
//!
//! The inherent `GameObject` impl is divided by responsibility under
//! #636; every method keeps its original body.

use super::*;

impl GameObject {
    pub fn new() -> Self {
        let mut world = WorldObject::new(
            false,
            TypeId::GameObject,
            TypeMask::OBJECT | TypeMask::GAME_OBJECT,
        );
        world
            .object_mut()
            .create_flags_mut()
            .insert(CreateObjectFlags::STATIONARY | CreateObjectFlags::ROTATION);

        Self {
            world,
            data: GameObjectDataValues::default(),
            game_object_data_changes: UpdateMask::new(GAME_OBJECT_DATA_BITS),
            spell_id: 0,
            respawn_time: 0,
            respawn_delay_time: DEFAULT_GAMEOBJECT_RESPAWN_DELAY_SECS,
            despawn_delay: 0,
            despawn_respawn_time: 0,
            restock_time: 0,
            loot_state: LootState::NotReady,
            loot_state_unit_guid: ObjectGuid::EMPTY,
            loot_lifecycle_revision: 0,
            loot_authority: OwnedLootAuthority::new(),
            shared_loot: None,
            personal_loot: HashMap::new(),
            unique_users: HashSet::new(),
            spawned_by_default: true,
            use_times: 0,
            cooldown_time: 0,
            prev_go_state: GoState::Active,
            packed_rotation: 0,
            local_rotation: [0.0, 0.0, 0.0, 1.0],
            spawn_id: 0,
            loot_mode: GAMEOBJECT_LOOT_MODE_DEFAULT,
            respawn_compatibility_mode: false,
            anim_kit_id: 0,
            world_effect_id: 0,
            lifecycle_string_id: String::new(),
            linked_trap_guid: ObjectGuid::EMPTY,
            stationary_position: Position::new(0.0, 0.0, 0.0, 0.0),
            go_anim_progress_like_cpp: 0,
            represented_baseline_flags_like_cpp: None,
            chest_loot_source_like_cpp: None,
            goober_use_source_like_cpp: None,
            spell_focus_use_source_like_cpp: None,
            represented_gameobject_model_like_cpp: false,
            represented_gameobject_model_is_map_object_like_cpp: false,
            represented_gameobject_model_collision_enabled_like_cpp: None,
            represented_gameobject_data_present_like_cpp: false,
            grid_unload_cleanup_before_delete_count: 0,
            grid_unload_delete_requested: false,
            grid_unload_respawn_relocation_requested: false,
        }
    }
    pub fn try_create_from_lifecycle(
        record: GameObjectCreateLifecycleRecord,
    ) -> Result<Self, GameObjectLifecycleError> {
        let mut game_object = Self::new();
        game_object.apply_create_lifecycle(record)?;
        Ok(game_object)
    }
    pub fn try_load_from_db_lifecycle(
        record: GameObjectLoadFromDbLifecycleRecord,
    ) -> Result<Self, GameObjectLifecycleError> {
        let mut game_object = Self::try_create_from_lifecycle(record.create.clone())?;
        game_object.apply_load_from_db_lifecycle(record);
        Ok(game_object)
    }
    pub fn apply_create_lifecycle(
        &mut self,
        record: GameObjectCreateLifecycleRecord,
    ) -> Result<(), GameObjectLifecycleError> {
        Self::validate_create_lifecycle(&record)?;
        let template = &record.template;
        self.world_mut()
            .set_map(record.map_id, record.instance_id)
            .map_err(|source| GameObjectLifecycleError::MapBinding {
                entry: template.entry,
                source,
            })?;
        self.world_mut().relocate(record.position);
        self.stationary_position = record.position;
        self.world_mut().object_mut().create(record.guid);
        self.world_mut().object_mut().set_entry(template.entry);
        self.world_mut().object_mut().set_scale(template.scale);
        self.world_mut().set_name(template.name.clone());

        self.set_respawn_compatibility_mode(!record.dynamic);
        self.set_spawn_id(record.spawn_id);
        self.local_rotation = record.rotation;
        self.packed_rotation = pack_gameobject_local_rotation(record.rotation);
        self.world_effect_id = template.world_effect_id;
        self.anim_kit_id = template.anim_kit_id;

        self.set_display_id(template.display_id);
        self.set_faction(template.faction);
        self.set_flags(template.flags);
        self.represented_baseline_flags_like_cpp = Some(template.flags);
        self.set_go_type(template.go_type as u8);
        self.prev_go_state = record.go_state;
        self.set_go_state(record.go_state);
        self.set_art_kit(record.art_kit);
        self.set_level(template.level);
        self.set_percent_health(template.percent_health);
        self.set_custom_param(template.custom_param);

        // C++ keeps template `data` through `m_goInfo`; Rust carries only the
        // bounded CHEST/GOOBER source needed by represented update branches here. The
        // remaining type-specific implementations, model creation, zone scripts,
        // DB phasing and AddToMap stay external/unrepresented in this entity constructor.
        let template_data = GameObjectTemplateData::new(template.go_type, template.data);
        self.chest_loot_source_like_cpp = template_data.chest_loot_source_like_cpp();
        self.goober_use_source_like_cpp = template_data.goober_use_source_like_cpp();
        self.spell_focus_use_source_like_cpp = template_data.spell_focus_use_source_like_cpp();
        match template.go_type {
            GAMEOBJECT_TYPE_FISHING_HOLE | GAMEOBJECT_TYPE_TRANSPORT => {
                self.set_go_anim_progress_like_cpp(record.anim_progress);
            }
            GAMEOBJECT_TYPE_FISHING_NODE => {
                self.set_level(0);
                self.set_go_anim_progress_like_cpp(u8::MAX);
            }
            GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING => {
                self.set_go_anim_progress_like_cpp(u8::MAX);
            }
            _ => {
                self.set_go_anim_progress_like_cpp(record.anim_progress);
            }
        }

        Ok(())
    }
    pub(super) fn validate_create_lifecycle(
        record: &GameObjectCreateLifecycleRecord,
    ) -> Result<(), GameObjectLifecycleError> {
        if record.template.go_type >= MAX_GAMEOBJECT_TYPE {
            return Err(GameObjectLifecycleError::InvalidGameObjectType {
                entry: record.template.entry,
                go_type: record.template.go_type,
            });
        }
        if record.template.go_type == GAMEOBJECT_TYPE_MAP_OBJ_TRANSPORT {
            return Err(GameObjectLifecycleError::InvalidMapObjectTransportType {
                entry: record.template.entry,
            });
        }
        if !record.position.is_valid_map_coord_like_cpp() {
            return Err(GameObjectLifecycleError::InvalidPosition {
                entry: record.template.entry,
                position: record.position,
            });
        }

        Ok(())
    }
    pub fn apply_load_from_db_lifecycle(&mut self, record: GameObjectLoadFromDbLifecycleRecord) {
        let mut respawn_compatibility_mode = record.respawn_compatibility_mode;
        let (spawned_by_default, respawn_delay_time, respawn_time) = if record.spawntimesecs >= 0 {
            if !record.despawn_possible && !record.despawn_at_action {
                self.set_flags(self.data().flags | GO_FLAG_NODESPAWN);
                (true, 0, 0)
            } else {
                (
                    true,
                    record.spawntimesecs as u32,
                    record.effective_map_respawn_time,
                )
            }
        } else {
            respawn_compatibility_mode = true;
            (false, record.spawntimesecs.unsigned_abs(), 0)
        };

        self.set_respawn_compatibility_mode(respawn_compatibility_mode);
        self.set_spawned_by_default(spawned_by_default);
        self.set_respawn_delay_time(respawn_delay_time);
        self.set_respawn_time(respawn_time);
        self.lifecycle_string_id = record.string_id;
        self.set_represented_gameobject_data_present_like_cpp(true);
    }
    pub const fn world(&self) -> &WorldObject {
        &self.world
    }
    pub fn world_mut(&mut self) -> &mut WorldObject {
        &mut self.world
    }
    pub const fn data(&self) -> &GameObjectDataValues {
        &self.data
    }
    pub fn game_object_data_changes_mask(&self) -> &UpdateMask {
        &self.game_object_data_changes
    }
    /// Returns explicit represented evidence for TrinityCore `GameObject::m_model != nullptr`.
    ///
    /// This is only model-existence evidence for map-owned represented DynamicMapTree
    /// registration. It is not a real `GameObjectModel`, model geometry, `GO_FLAG_MAP_OBJECT`,
    /// `EnableCollision`, or DB/model-store hydration, and it is never inferred from display id,
    /// template, or type.
    pub const fn has_represented_gameobject_model_like_cpp(&self) -> bool {
        self.represented_gameobject_model_like_cpp
    }
    pub const fn has_represented_gameobject_model_map_object_like_cpp(&self) -> bool {
        self.represented_gameobject_model_is_map_object_like_cpp
    }
    pub const fn represented_gameobject_model_collision_enabled_like_cpp(&self) -> Option<bool> {
        self.represented_gameobject_model_collision_enabled_like_cpp
    }
    /// Returns explicit represented evidence for TrinityCore `GameObject::m_goData != nullptr`.
    ///
    /// This only gates `GameObject::SaveRespawnTime()` parity; it does not imply full
    /// ObjectMgr/DB metadata ownership.
    pub const fn has_represented_gameobject_data_like_cpp(&self) -> bool {
        self.represented_gameobject_data_present_like_cpp
    }
    pub fn set_represented_gameobject_data_present_like_cpp(&mut self, present: bool) {
        self.represented_gameobject_data_present_like_cpp = present;
    }
    /// Sets explicit represented evidence for TrinityCore `GameObject::m_model != nullptr`.
    ///
    /// Callers must set this only when they have external evidence that the C++ object would have
    /// a model. The flag is consumed only by map-owned add/remove seams and does not create real
    /// model geometry, collision, or DB/model-store state. Setting this false also mirrors losing
    /// `m_model`: represented map-object evidence, `GO_FLAG_MAP_OBJECT`, and collision evidence
    /// are cleared. Setting this true does not infer map-object or collision state.
    pub fn set_represented_gameobject_model_like_cpp(&mut self, has_model: bool) {
        self.represented_gameobject_model_like_cpp = has_model;
        if !has_model {
            self.represented_gameobject_model_is_map_object_like_cpp = false;
            self.represented_gameobject_model_collision_enabled_like_cpp = None;
            self.set_flags(self.data.flags & !GO_FLAG_MAP_OBJECT);
        }
    }
    /// Applies explicit represented output from TrinityCore `GameObject::CreateModel()`.
    ///
    /// C++ anchor: `GameObject.cpp:4394-4399` assigns `m_model` from
    /// `GameObjectModel::Create(...)` and sets `GO_FLAG_MAP_OBJECT` only when the resulting model
    /// exists and `isMapObject()` is true. Rust does not infer either fact from display id,
    /// template or type.
    pub fn apply_represented_gameobject_model_creation_like_cpp(
        &mut self,
        has_model: bool,
        is_map_object: bool,
    ) {
        self.set_represented_gameobject_model_like_cpp(has_model);
        // C++ `CreateModel()` assigns a fresh `m_model`. Any previous represented
        // `m_model->enableCollision(...)` evidence belongs to the deleted/replaced
        // model and must not leak onto the new one; `UpdateModel()` does not call
        // `EnableCollision()` after recreation.
        self.represented_gameobject_model_collision_enabled_like_cpp = None;
        let represented_map_object = has_model && is_map_object;
        self.represented_gameobject_model_is_map_object_like_cpp = represented_map_object;
        if represented_map_object {
            self.set_flags(self.data.flags | GO_FLAG_MAP_OBJECT);
        } else {
            self.set_flags(self.data.flags & !GO_FLAG_MAP_OBJECT);
        }
    }
    pub fn set_represented_gameobject_model_map_object_like_cpp(&mut self, is_map_object: bool) {
        self.apply_represented_gameobject_model_creation_like_cpp(
            self.represented_gameobject_model_like_cpp,
            is_map_object,
        );
    }
    /// Bounded representation of TrinityCore `GameObject::EnableCollision(bool)`.
    ///
    /// This records only the local `m_model->enableCollision(enable)` evidence. With no represented
    /// model, it mirrors the C++ early return and does not mutate collision state or insert a model.
    pub fn enable_represented_gameobject_collision_like_cpp(
        &mut self,
        enable: bool,
    ) -> GameObjectCollisionOutcomeLikeCpp {
        let previous_collision_enabled =
            self.represented_gameobject_model_collision_enabled_like_cpp;
        if !self.represented_gameobject_model_like_cpp {
            return GameObjectCollisionOutcomeLikeCpp {
                requested_enable: enable,
                represented_model_present: false,
                previous_collision_enabled,
                new_collision_enabled: previous_collision_enabled,
            };
        }

        self.represented_gameobject_model_collision_enabled_like_cpp = Some(enable);
        GameObjectCollisionOutcomeLikeCpp {
            requested_enable: enable,
            represented_model_present: true,
            previous_collision_enabled,
            new_collision_enabled: self.represented_gameobject_model_collision_enabled_like_cpp,
        }
    }
    pub fn clear_game_object_data_changes(&mut self) {
        self.game_object_data_changes.reset_all();
    }
    pub const fn spell_id(&self) -> u32 {
        self.spell_id
    }
    pub fn set_spell_id(&mut self, spell_id: u32) {
        self.spell_id = spell_id;
        self.spawned_by_default = false;
    }
    pub const fn respawn_time(&self) -> i64 {
        self.respawn_time
    }
    pub fn set_respawn_time(&mut self, respawn_time: i64) {
        self.respawn_time = respawn_time;
    }
    pub const fn respawn_delay_time(&self) -> u32 {
        self.respawn_delay_time
    }
    pub fn set_respawn_delay_time(&mut self, delay: u32) {
        self.respawn_delay_time = delay;
    }
    pub const fn despawn_delay(&self) -> u32 {
        self.despawn_delay
    }
    pub const fn despawn_respawn_time(&self) -> u32 {
        self.despawn_respawn_time
    }
    /// Represented subset of TrinityCore `GameObject::DespawnOrUnsummon(delay, forceRespawnTime)`
    /// for delayed scheduling only.
    ///
    /// C++ anchor: `GameObject.cpp:1711-1719` sets `m_despawnDelay` only when
    /// `delay > 0` and either no delay is pending or the new delay is shorter.
    /// The immediate `delay == 0` Delete/AddObjectToRemoveList path belongs to
    /// `wow-map` in this Rust slice.
    pub fn schedule_despawn_or_unsummon_like_cpp(
        &mut self,
        delay_ms: u32,
        force_respawn_time_secs: u32,
    ) -> bool {
        if delay_ms == 0 {
            return false;
        }

        if self.despawn_delay == 0 || self.despawn_delay > delay_ms {
            self.despawn_delay = delay_ms;
            self.despawn_respawn_time = force_respawn_time_secs;
            true
        } else {
            false
        }
    }
    /// Bounded local representation of TrinityCore `GameObject::Update(diff)`
    /// through the `m_despawnDelay` branch only.
    ///
    /// C++ anchors: `GameObject.cpp:1215-1233` for `WorldObject::Update`
    /// plus despawn delay, and `GameObject.cpp:1235-1274`/`1276+` as explicit
    /// gaps. No AI, go-type runtime, per-player state, packets, DB, pool
    /// manager or full loot-state machine executes in `wow-entities`.
    pub fn update_like_cpp(&mut self, diff_ms: u32) -> GameObjectUpdateOutcomeLikeCpp {
        let despawn_delay_before_ms = self.despawn_delay;
        let mut status = GameObjectUpdateStatusLikeCpp::Updated;
        let mut despawn_or_unsummon_requested = false;

        if self.despawn_delay != 0 {
            if self.despawn_delay > diff_ms {
                self.despawn_delay -= diff_ms;
            } else {
                self.despawn_delay = 0;
                status = GameObjectUpdateStatusLikeCpp::DespawnRequested;
                despawn_or_unsummon_requested = true;
            }
        }

        GameObjectUpdateOutcomeLikeCpp {
            diff_ms,
            status,
            despawn_delay_before_ms,
            despawn_delay_after_ms: self.despawn_delay,
            despawn_respawn_time_secs: self.despawn_respawn_time,
            world_update_would_run: true,
            ai_update_not_represented: true,
            go_type_impl_update_not_represented: true,
            despawn_or_unsummon_requested,
        }
    }
    pub const fn restock_time(&self) -> i64 {
        self.restock_time
    }
    pub const fn loot_state(&self) -> LootState {
        self.loot_state
    }
    pub const fn loot_state_unit_guid(&self) -> ObjectGuid {
        self.loot_state_unit_guid
    }
    pub const fn loot_lifecycle_revision_like_cpp(&self) -> u64 {
        self.loot_lifecycle_revision
    }
    pub(super) fn advance_loot_lifecycle_revision_like_cpp(&mut self) -> u64 {
        // A restock identity may stop advancing only at exhaustion; it must
        // never wrap and become equal to a stale async observation.
        self.loot_lifecycle_revision = self.loot_lifecycle_revision.saturating_add(1).max(1);
        self.loot_lifecycle_revision
    }
    pub fn set_loot_state(&mut self, state: LootState, unit: Option<ObjectGuid>) {
        self.loot_state = state;
        self.loot_state_unit_guid = unit.unwrap_or(ObjectGuid::EMPTY);
    }
    /// Represented local setter for TrinityCore `GameObject::SetLootState` restock writes.
    ///
    /// C++ anchor: `GameObject.cpp:3693-3695` assigns `m_restockTime` only after the
    /// map-owned caller has proven chest type, activated loot state, positive restock seconds,
    /// previous zero restock time, and real `Loot::IsChanged()` evidence. This method only writes
    /// the local represented field; it does not infer `GameTime`, template data, or loot changes.
    pub fn set_restock_time_like_cpp(&mut self, restock_time: i64) {
        self.restock_time = restock_time;
    }
    pub const fn spawned_by_default(&self) -> bool {
        self.spawned_by_default
    }
    pub fn set_spawned_by_default(&mut self, spawned: bool) {
        self.spawned_by_default = spawned;
    }
    pub const fn use_times(&self) -> u32 {
        self.use_times
    }
    pub const fn shared_loot_like_cpp(&self) -> Option<&GameObjectOwnedLoot> {
        self.shared_loot.as_ref()
    }
    pub fn set_shared_loot_like_cpp(&mut self, loot: GameObjectOwnedLoot) {
        self.shared_loot = Some(loot);
    }
    pub fn clear_shared_loot_like_cpp(&mut self) {
        self.shared_loot = None;
    }
    pub fn personal_loot_like_cpp(&self, guid: ObjectGuid) -> Option<&GameObjectOwnedLoot> {
        self.personal_loot.get(&guid)
    }
    pub fn loot_for_player_like_cpp(&self, guid: ObjectGuid) -> Option<&GameObjectOwnedLoot> {
        if self.personal_loot.is_empty() {
            return self.shared_loot.as_ref();
        }

        self.personal_loot.get(&guid)
    }
    pub fn set_personal_loot_like_cpp(&mut self, guid: ObjectGuid, loot: GameObjectOwnedLoot) {
        self.personal_loot.insert(guid, loot);
    }
    pub fn clear_personal_loot_like_cpp(&mut self) {
        self.personal_loot.clear();
    }
    pub fn personal_loot_count_like_cpp(&self) -> usize {
        self.personal_loot.len()
    }
    pub fn add_unique_use_like_cpp(&mut self, guid: ObjectGuid) -> bool {
        self.add_use_like_cpp();
        self.unique_users.insert(guid)
    }
    pub fn unique_user_count_like_cpp(&self) -> usize {
        self.unique_users.len()
    }
    pub fn unique_users_snapshot_like_cpp(&self) -> Vec<ObjectGuid> {
        self.unique_users.iter().copied().collect()
    }
    pub fn clear_unique_users_and_reset_use_times_like_cpp(&mut self) {
        self.unique_users.clear();
        self.use_times = 0;
    }
    pub fn represented_chest_loot_source_like_cpp(&self) -> Option<GameObjectLootSource> {
        self.chest_loot_source_like_cpp
    }
    pub fn set_represented_chest_loot_source_like_cpp(
        &mut self,
        source: Option<GameObjectLootSource>,
    ) {
        self.chest_loot_source_like_cpp = source;
    }
    pub fn represented_goober_use_source_like_cpp(&self) -> Option<GooberUseSource> {
        self.goober_use_source_like_cpp
    }
    pub fn set_represented_goober_use_source_like_cpp(&mut self, source: Option<GooberUseSource>) {
        self.goober_use_source_like_cpp = source;
    }
    pub fn represented_spell_focus_use_source_like_cpp(&self) -> Option<SpellFocusUseSource> {
        self.spell_focus_use_source_like_cpp
    }
    pub fn set_represented_spell_focus_use_source_like_cpp(
        &mut self,
        source: Option<SpellFocusUseSource>,
    ) {
        self.spell_focus_use_source_like_cpp = source;
    }
    pub fn add_use_like_cpp(&mut self) {
        self.use_times = self.use_times.saturating_add(1);
    }
    pub fn reset_use_times_like_cpp(&mut self) {
        self.use_times = 0;
    }
    pub const fn loot_authority_like_cpp(&self) -> &OwnedLootAuthority {
        &self.loot_authority
    }
    pub fn rebind_loot_authority_like_cpp(&mut self, authority: OwnedLootAuthority) -> bool {
        if self.loot_authority.shares_storage_like_cpp(&authority) {
            self.sync_loot_summaries_from_authority_like_cpp();
            return false;
        }

        self.loot_authority.detach_like_cpp();
        self.loot_authority = authority;
        self.sync_loot_summaries_from_authority_like_cpp();
        true
    }
    /// Compare/exchange variant for callers that cannot keep the canonical
    /// map lock between observing and rebinding this object.
    pub fn rebind_loot_authority_if_current_like_cpp(
        &mut self,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        if !self.loot_authority.shares_storage_like_cpp(expected) {
            return None;
        }

        if self.loot_authority.stamp_like_cpp() != expected_stamp {
            return None;
        }

        if self.loot_authority.shares_storage_like_cpp(&authority) {
            self.sync_loot_summaries_from_authority_like_cpp();
            return Some(false);
        }

        if !self.loot_authority.detach_if_stamp_like_cpp(expected_stamp) {
            return None;
        }

        self.loot_authority = authority;
        self.sync_loot_summaries_from_authority_like_cpp();
        Some(true)
    }
    /// Non-owning snapshot adoption counterpart to the entity-local CAS.
    pub fn adopt_loot_authority_for_snapshot_like_cpp(&mut self, authority: OwnedLootAuthority) {
        self.loot_authority = authority;
        self.sync_loot_summaries_from_authority_like_cpp();
    }
    pub fn share_loot_authority_like_cpp(&mut self, authority: OwnedLootAuthority) {
        self.rebind_loot_authority_like_cpp(authority);
    }
    pub fn initialize_loot_authority_like_cpp(
        &mut self,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> LootInstallOutcome {
        let outcome = self.loot_authority.initialize_like_cpp(shared, personal);
        self.sync_loot_summaries_from_authority_like_cpp();
        outcome
    }
    pub fn initialize_shared_loot_authority_like_cpp(
        &mut self,
        loot: CreatureLoot,
    ) -> LootInstallOutcome {
        let outcome = self.loot_authority.initialize_shared_like_cpp(loot);
        self.sync_loot_summaries_from_authority_like_cpp();
        outcome
    }
    pub fn upsert_personal_loot_authority_like_cpp(
        &mut self,
        player: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
    ) -> LootInstallOutcome {
        let outcome = self
            .loot_authority
            .upsert_personal_like_cpp(player, loot, replace);
        self.sync_loot_summaries_from_authority_like_cpp();
        outcome
    }
    /// Installs the complete shared/personal pool topology only while this is
    /// still the exact `ClearLoot`/restock lifetime observed before async
    /// template generation.
    ///
    /// C++ creates chest loot synchronously from `GameObject::Use`, on the map
    /// thread (`GameObject.cpp:2559-2575`). Rust releases the map lock for DB
    /// work, so identity, object generation, and the entity-local lifecycle
    /// revision form the equivalent compare/exchange boundary. If another
    /// opener already installed this same lifecycle, its first-writer result is
    /// accepted without replacing it.
    pub fn install_loot_authority_if_lifecycle_like_cpp(
        &mut self,
        expected_authority: &OwnedLootAuthority,
        expected_object_generation: u64,
        expected_lifecycle_revision: u64,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> bool {
        if self.loot_lifecycle_revision != expected_lifecycle_revision
            || self.loot_state == LootState::JustDeactivated
            || !self
                .loot_authority
                .shares_storage_like_cpp(expected_authority)
        {
            return false;
        }

        let stamp = self.loot_authority.stamp_like_cpp();
        let installed = match stamp.lifecycle {
            OwnedLootAuthorityLifecycle::Pristine
                if expected_object_generation == 0 && stamp.object_generation == 0 =>
            {
                self.loot_authority
                    .initialize_pristine_like_cpp(shared, personal)
                    .installed()
            }
            OwnedLootAuthorityLifecycle::Retired
                if stamp.object_generation == expected_object_generation =>
            {
                self.loot_authority
                    .replace_retired_generation_like_cpp(
                        expected_object_generation,
                        shared,
                        personal,
                    )
                    .is_some()
            }
            OwnedLootAuthorityLifecycle::Active
                if stamp.object_generation == expected_object_generation.wrapping_add(1).max(1) =>
            {
                // A concurrent opener won the same map-object lifecycle. C++
                // keeps the first `m_loot`; do not replace its generated pools.
                true
            }
            _ => false,
        };

        if installed {
            self.sync_loot_summaries_from_authority_like_cpp();
        }
        installed
    }
    /// Installs one personal pool only while the object is still in the exact
    /// `ClearLoot`/restock lifetime observed before async template generation.
    ///
    /// A retired non-pristine authority is a valid new generation only when
    /// its object generation still matches the tombstone captured by the
    /// caller. Once another player has started that same lifecycle, additional
    /// personal pools may join the active authority without replacing it.
    pub fn install_personal_loot_if_lifecycle_like_cpp(
        &mut self,
        expected_authority: &OwnedLootAuthority,
        expected_object_generation: u64,
        expected_lifecycle_revision: u64,
        player: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
    ) -> bool {
        if self.loot_lifecycle_revision != expected_lifecycle_revision
            || self.loot_state == LootState::JustDeactivated
            || !self
                .loot_authority
                .shares_storage_like_cpp(expected_authority)
        {
            return false;
        }

        let current_generation = self.loot_authority.generation_like_cpp();
        let installed = if self.loot_authority.is_retired_like_cpp() && current_generation != 0 {
            if current_generation != expected_object_generation {
                return false;
            }
            self.loot_authority
                .replace_retired_generation_like_cpp(
                    expected_object_generation,
                    None,
                    HashMap::from([(player, loot)]),
                )
                .is_some()
        } else {
            if current_generation < expected_object_generation {
                return false;
            }
            let outcome = self
                .loot_authority
                .upsert_personal_like_cpp(player, loot, replace);
            outcome.installed()
                || self
                    .loot_authority
                    .snapshot_for_player_like_cpp(player)
                    .is_some()
        };

        self.sync_loot_summaries_from_authority_like_cpp();
        installed
    }
    pub fn replace_loot_authority_like_cpp(
        &mut self,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> u64 {
        let generation = self.loot_authority.replace_like_cpp(shared, personal);
        self.sync_loot_summaries_from_authority_like_cpp();
        generation
    }
    pub fn sync_loot_summaries_from_authority_like_cpp(&mut self) {
        self.shared_loot = self
            .loot_authority
            .shared_snapshot_like_cpp()
            .map(|snapshot| game_object_owned_loot_from_snapshot(&snapshot));
        self.personal_loot = self
            .loot_authority
            .personal_snapshots_like_cpp()
            .into_iter()
            .map(|(player, snapshot)| (player, game_object_owned_loot_from_snapshot(&snapshot)))
            .collect();
    }
    /// Invalidates every outstanding async claim without eagerly applying
    /// `ClearLoot` side effects to the still map-resident object. C++
    /// `GameObject::Delete` queues physical removal after setting
    /// `GO_NOT_READY`; its loot members remain observable until destruction.
    pub fn retire_loot_authority_like_cpp(&mut self) {
        self.advance_loot_lifecycle_revision_like_cpp();
        self.loot_authority.retire_like_cpp();
    }
    pub fn clear_loot_like_cpp(&mut self) {
        self.advance_loot_lifecycle_revision_like_cpp();
        self.loot_authority.retire_like_cpp();
        self.shared_loot = None;
        self.personal_loot.clear();
        self.unique_users.clear();
        self.use_times = 0;
    }
    pub fn is_fully_looted_like_cpp(&self) -> bool {
        if !self.loot_authority.is_pristine_like_cpp() {
            return self.loot_authority.is_fully_looted_like_cpp();
        }

        if self
            .shared_loot
            .as_ref()
            .is_some_and(|loot| !loot.is_looted_like_cpp())
        {
            return false;
        }

        for loot in self.personal_loot.values() {
            if !loot.is_looted_like_cpp() {
                return false;
            }
        }

        true
    }
    pub const fn cooldown_time(&self) -> i64 {
        self.cooldown_time
    }
    pub fn set_cooldown_time(&mut self, cooldown_time: i64) {
        self.cooldown_time = cooldown_time;
    }
    pub const fn prev_go_state(&self) -> GoState {
        self.prev_go_state
    }
    pub const fn packed_rotation(&self) -> i64 {
        self.packed_rotation
    }
    pub const fn local_rotation_like_cpp(&self) -> [f32; 4] {
        self.local_rotation
    }
    pub const fn spawn_id(&self) -> u64 {
        self.spawn_id
    }
    pub fn set_spawn_id(&mut self, spawn_id: u64) {
        self.spawn_id = spawn_id;
    }
    pub const fn loot_mode(&self) -> u16 {
        self.loot_mode
    }
    pub fn reset_loot_mode(&mut self) {
        self.loot_mode = GAMEOBJECT_LOOT_MODE_DEFAULT;
    }
    pub const fn respawn_compatibility_mode(&self) -> bool {
        self.respawn_compatibility_mode
    }
    pub fn set_respawn_compatibility_mode(&mut self, enabled: bool) {
        self.respawn_compatibility_mode = enabled;
    }
    pub const fn anim_kit_id(&self) -> u16 {
        self.anim_kit_id
    }
    pub const fn world_effect_id(&self) -> u32 {
        self.world_effect_id
    }
    pub fn lifecycle_string_id(&self) -> &str {
        &self.lifecycle_string_id
    }
    pub const fn stationary_position(&self) -> Position {
        self.stationary_position
    }
    pub const fn cleanup_before_delete_count(&self) -> u32 {
        self.grid_unload_cleanup_before_delete_count
    }
    pub const fn grid_unload_delete_requested(&self) -> bool {
        self.grid_unload_delete_requested
    }
    pub const fn grid_unload_respawn_relocation_requested(&self) -> bool {
        self.grid_unload_respawn_relocation_requested
    }
    pub fn set_destroyed_object(&mut self, destroyed: bool) {
        self.world.object_mut().set_destroyed_object(destroyed);
    }
    pub fn request_respawn_relocation_from_grid_unload(&mut self) {
        self.grid_unload_respawn_relocation_requested = true;
    }
    pub fn cleanup_before_delete(&mut self) {
        self.grid_unload_cleanup_before_delete_count = self
            .grid_unload_cleanup_before_delete_count
            .saturating_add(1);
    }
    pub fn request_delete_from_grid_unload(&mut self) {
        self.grid_unload_delete_requested = true;
        self.world.clear_current_cell();
    }
    pub fn set_display_id(&mut self, display_id: u32) {
        self.set_i32_field(GAME_OBJECT_DATA_DISPLAY_ID_BIT, display_id as i32, |data| {
            &mut data.display_id
        });
    }
    pub fn set_faction(&mut self, faction: u32) {
        self.set_i32_field(
            GAME_OBJECT_DATA_FACTION_TEMPLATE_BIT,
            faction as i32,
            |data| &mut data.faction_template,
        );
    }
    pub fn set_go_state(&mut self, state: GoState) {
        self.set_i8_field(GAME_OBJECT_DATA_STATE_BIT, state as i8, |data| {
            &mut data.state
        });
    }
    pub fn set_go_type(&mut self, type_id: u8) {
        self.set_i8_field(GAME_OBJECT_DATA_TYPE_ID_BIT, type_id as i8, |data| {
            &mut data.type_id
        });
    }
    pub fn set_flags(&mut self, flags: u32) {
        self.set_u32_field(GAME_OBJECT_DATA_FLAGS_BIT, flags, |data| &mut data.flags);
    }
    pub fn set_level(&mut self, level: u32) {
        self.set_i32_field(GAME_OBJECT_DATA_LEVEL_BIT, level as i32, |data| {
            &mut data.level
        });
    }
    pub fn set_percent_health(&mut self, percent_health: u8) {
        self.set_u8_field(
            GAME_OBJECT_DATA_PERCENT_HEALTH_BIT,
            percent_health,
            |data| &mut data.percent_health,
        );
    }
    pub fn set_art_kit(&mut self, art_kit: u32) {
        self.set_u32_field(GAME_OBJECT_DATA_ART_KIT_BIT, art_kit, |data| {
            &mut data.art_kit
        });
    }
    pub fn set_custom_param(&mut self, custom_param: u32) {
        self.set_u32_field(GAME_OBJECT_DATA_CUSTOM_PARAM_BIT, custom_param, |data| {
            &mut data.custom_param
        });
    }
    pub fn set_created_by(&mut self, created_by: ObjectGuid) {
        self.set_guid_field(GAME_OBJECT_DATA_CREATED_BY_BIT, created_by, |data| {
            &mut data.created_by
        });
    }
    /// Bounded local representation of TrinityCore `GameObject::SetOwnerGUID`.
    ///
    /// C++ anchor: `GameObject.h:227-237` always sets `m_spawnedByDefault = false`
    /// and writes `GameObjectData::CreatedBy`, including for `ObjectGuid::Empty`.
    /// This does not run `Unit::RemoveGameObject` side effects, owned object slots,
    /// auras, cooldown events, Creature AI callbacks, ObjectAccessor, or packets.
    pub fn set_owner_guid_like_cpp(&mut self, owner_guid: ObjectGuid) {
        self.spawned_by_default = false;
        self.set_created_by(owner_guid);
    }
    pub fn clear_owner_guid_like_cpp(&mut self) {
        self.set_owner_guid_like_cpp(ObjectGuid::EMPTY);
    }
}
