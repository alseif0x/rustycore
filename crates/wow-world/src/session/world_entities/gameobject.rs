//! Represented gameobject state owned by the Session boundary.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn mutate_canonical_gameobject_by_guid_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        f: impl FnOnce(&mut wow_entities::GameObject) -> R,
    ) -> Option<R> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let gameobject = managed.map_mut().get_typed_game_object_mut(guid)?;
        Some(f(gameobject))
    }
    pub(crate) fn canonical_gameobject_linked_trap_guid_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        if guid.is_empty() || !guid.is_game_object() {
            return None;
        }
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let manager = self.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?;
        let gameobject = map.map().get_typed_game_object(guid)?;
        let linked_trap_guid = gameobject.linked_trap_guid_like_cpp();
        (!linked_trap_guid.is_empty()).then_some(linked_trap_guid)
    }
    pub(crate) fn canonical_gameobject_access_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        if guid.is_empty() || !guid.is_game_object() {
            return None;
        }
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let manager = self.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?;
        let game_object = map.map().get_typed_game_object(guid)?;
        Some(RepresentedGameObjectAccessLikeCpp {
            entry: game_object.world().object().entry(),
            position: game_object.world().position(),
        })
    }
    pub(crate) fn represented_gameobject_dynamic_flags_for_player_like_cpp(
        &self,
        gameobject_entry: u32,
        state: &RepresentedGameObjectUseState,
    ) -> u32 {
        let mut dyn_flags = 0_u32;
        let path_progress = (state.dynamic_flags >> 16) & 0xFFFF;
        let activate_to_quest =
            self.represented_gameobject_activate_to_quest_like_cpp(gameobject_entry, state);

        match state.go_type.map(u32::from) {
            Some(wow_entities::GAMEOBJECT_TYPE_QUESTGIVER) => {
                if activate_to_quest {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_ACTIVATE;
                }
            }
            Some(wow_entities::GAMEOBJECT_TYPE_CHEST) => {
                if activate_to_quest {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_ACTIVATE
                        | wow_entities::GO_DYNFLAG_LO_SPARKLE
                        | wow_entities::GO_DYNFLAG_LO_HIGHLIGHT;
                } else if self.player_is_game_master_like_cpp() == Some(true) {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_ACTIVATE;
                }
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GOOBER) => {
                if activate_to_quest {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_HIGHLIGHT;
                    let state_for_player = self
                        .represented_gameobject_go_state_for_viewer_like_cpp(state, Instant::now());
                    if state_for_player != wow_entities::GoState::Active {
                        dyn_flags |= wow_entities::GO_DYNFLAG_LO_ACTIVATE;
                    }
                } else if self.player_is_game_master_like_cpp() == Some(true) {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_ACTIVATE;
                }
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GENERIC) => {
                if activate_to_quest {
                    dyn_flags |=
                        wow_entities::GO_DYNFLAG_LO_SPARKLE | wow_entities::GO_DYNFLAG_LO_HIGHLIGHT;
                }
            }
            Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE) => {
                if activate_to_quest {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_ACTIVATE
                        | wow_entities::GO_DYNFLAG_LO_SPARKLE
                        | wow_entities::GO_DYNFLAG_LO_HIGHLIGHT;
                }
                let state_for_player =
                    self.represented_gameobject_go_state_for_viewer_like_cpp(state, Instant::now());
                if state_for_player == wow_entities::GoState::Active {
                    dyn_flags |= wow_entities::GO_DYNFLAG_LO_DEPLETED;
                }
            }
            _ => {
                dyn_flags = state.dynamic_flags & 0xFFFF;
            }
        }

        if state
            .condition_id1
            .is_some_and(|id| !self.represented_meets_player_condition_id_like_cpp(id))
        {
            dyn_flags |= wow_entities::GO_DYNFLAG_LO_NO_INTERACT;
        }

        (path_progress << 16) | dyn_flags
    }
    pub(in crate::session) fn represented_gameobject_dynamic_flags_update_like_cpp(
        guid: ObjectGuid,
        map_id: u16,
        dynamic_flags: u32,
    ) -> Option<wow_packet::packets::update::UpdateObject> {
        let mut mask = wow_entities::UpdateMask::new(wow_entities::OBJECT_DATA_BITS);
        mask.set(wow_entities::OBJECT_DATA_PARENT_BIT);
        mask.set(wow_entities::OBJECT_DATA_DYNAMIC_FLAGS_BIT);
        let values_update = wow_entities::GameObjectValuesUpdate {
            changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
            object_data: Some(wow_entities::ObjectDataUpdate {
                mask,
                values: wow_entities::ObjectDataValues {
                    entry_id: 0,
                    dynamic_flags,
                    scale: 0.0,
                },
            }),
            game_object_data: None,
        };
        game_object_values_update_to_update_object(guid, map_id, &values_update)
    }
    pub(in crate::session) fn set_canonical_gameobject_spell_id_like_cpp(
        &mut self,
        guid: ObjectGuid,
        spell_id: u32,
    ) {
        let Some(map_key) =
            self.canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))
        else {
            return;
        };
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        let Some(map) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return;
        };
        if let Some(game_object) = map.map_mut().get_typed_game_object_mut(guid) {
            game_object.set_spell_id(spell_id);
        }
    }
    pub(in crate::session) fn represented_or_canonical_gameobject_owner_guid_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        let map_key =
            self.canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()));
        let canonical_owner = map_key
            .and_then(|map_key| {
                self.canonical_map_manager
                    .as_ref()
                    .and_then(|manager| manager.lock().ok())
                    .and_then(|manager| {
                        manager
                            .find_map(map_key.map_id, map_key.instance_id)
                            .and_then(|map| map.map().get_typed_game_object(guid))
                            .map(|game_object| game_object.owner_guid())
                    })
            })
            .filter(|owner_guid| !owner_guid.is_empty());
        canonical_owner.or_else(|| {
            self.represented_gameobject_use_states
                .get(&guid)
                .and_then(|state| state.owner_guid)
        })
    }
    pub(in crate::session) fn upsert_canonical_gameobject_map_object_like_cpp(
        &mut self,
        map_id: u16,
        guid: ObjectGuid,
        entry: u32,
        position: wow_core::Position,
    ) {
        let owner_guid = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.owner_guid);
        let Some(map_key) = self.canonical_object_lookup_map_key_like_cpp(u32::from(map_id)) else {
            return;
        };
        // A represented object from a stale client/map context must never be
        // materialized beside the player in a different map.
        if map_key.map_id != u32::from(map_id) {
            return;
        }
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        let Some(map) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return;
        };
        if map.map().get_game_object(guid).is_some() {
            let _ = map.map_mut().relocate_map_object_like_cpp(guid, position);
            if let Some(owner_guid) = owner_guid
                && let Some(game_object) = map.map_mut().get_typed_game_object_mut(guid)
            {
                game_object.set_created_by(owner_guid);
            }
            return;
        }

        let mut game_object = GameObject::new();
        game_object.world_mut().object_mut().create(guid);
        game_object.world_mut().object_mut().set_entry(entry);
        if let Some(owner_guid) = owner_guid {
            game_object.set_created_by(owner_guid);
        }
        if game_object
            .world_mut()
            .set_map(map_key.map_id, map_key.instance_id)
            .is_err()
        {
            return;
        }
        game_object.world_mut().relocate(position);
        let _ = map
            .map_mut()
            .add_to_map_like_cpp(AccessorObjectKind::GameObject, game_object.world().clone());
        game_object.world_mut().object_mut().add_to_world();
        let Ok(record) = wow_entities::MapObjectRecord::new_game_object(game_object) else {
            return;
        };
        let _ = map.map_mut().insert_map_object_record(record);
    }
    pub(in crate::session) fn gameobject_create_data_from_canonical_like_cpp(
        &self,
        guid: ObjectGuid,
        gameobject: &wow_entities::GameObject,
    ) -> Option<wow_packet::packets::update::GameObjectCreateData> {
        let object = gameobject.world();
        let data = gameobject.data();
        let display_id = u32::try_from(data.display_id).ok()?;
        if display_id == 0 {
            return None;
        }
        let go_type = u8::try_from(data.type_id).ok()?;
        let go_state = match data.state {
            0 => Some(wow_entities::GoState::Active),
            1 => Some(wow_entities::GoState::Ready),
            2 => Some(wow_entities::GoState::Destroyed),
            24 => Some(wow_entities::GoState::TransportActive),
            25 => Some(wow_entities::GoState::TransportStopped),
            _ => None,
        };

        Some(wow_packet::packets::update::GameObjectCreateData {
            guid,
            entry: object.object().entry(),
            dynamic_flags: self.represented_gameobject_dynamic_flags_for_player_like_cpp(
                object.object().entry(),
                &RepresentedGameObjectUseState {
                    go_type: Some(go_type),
                    go_state,
                    dynamic_flags: object.object().dynamic_flags(),
                    ..Default::default()
                },
            ),
            display_id,
            go_type,
            position: object.position(),
            rotation: gameobject.local_rotation_like_cpp(),
            anim_progress: gameobject.go_anim_progress_like_cpp(),
            state: data.state,
            art_kit: data.art_kit,
            created_by: data.created_by,
            faction_template: data.faction_template,
            gameobject_flags: data.flags,
            world_effect_id: 0,
            scale: object.object().scale(),
            level: 0, // non-transport GameObject: Level unused (period via AnimationData)
            // Canonical GameObject entity does not yet carry per-spawn ParentRotation;
            // identity until the entity stores it (#NEXT.R8.ENTITIES.1216 follow-up).
            parent_rotation: [0.0, 0.0, 0.0, 1.0],
        })
    }
    pub(crate) async fn kill_credit_gameobject_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        gameobject_entry: u32,
        gameobject_guid: wow_core::ObjectGuid,
    ) {
        // C++ QUEST_OBJECTIVE_GAMEOBJECT.
        self.update_represented_storing_value_quest_objective_progress_like_cpp(
            item_guid_generator,
            2,
            gameobject_entry as i32,
            1,
            gameobject_guid,
        )
        .await;
    }
    #[cfg(test)]
    pub(crate) async fn kill_credit_gameobject_like_cpp(
        &mut self,
        gameobject_entry: u32,
        gameobject_guid: wow_core::ObjectGuid,
    ) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.kill_credit_gameobject_with_generator_like_cpp(
            generator.as_ref(),
            gameobject_entry,
            gameobject_guid,
        )
        .await;
    }
    pub(in crate::session) fn represented_gameobject_is_friendly_to_player_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<bool> {
        let gameobject_faction = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.faction_template)?;
        let player_faction = self.player_faction_template_id_like_cpp()?;
        let store = self.faction_template_store.as_ref()?;
        let gameobject_entry = store.get(gameobject_faction)?;
        let player_entry = store.get(player_faction)?;
        Some(gameobject_entry.is_friendly_to_like_cpp(player_entry))
    }
    pub(crate) fn apply_represented_gameobject_cooldown_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        cooldown_secs: u32,
    ) -> bool {
        if cooldown_secs == 0 {
            return true;
        }

        let now = Instant::now();
        let state = self
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        if state
            .cooldown_until
            .is_some_and(|cooldown_until| cooldown_until > now)
        {
            self.represented_gameobject_use_effects
                .push(RepresentedGameObjectUseEffect::CooldownRejected { gameobject_guid });
            return false;
        }

        state.cooldown_until =
            Some(now + Duration::from_millis(u64::from(cooldown_secs).saturating_mul(1000)));
        self.represented_gameobject_use_effects.push(
            RepresentedGameObjectUseEffect::CooldownStarted {
                gameobject_guid,
                cooldown_secs,
            },
        );
        true
    }
    pub(in crate::session) fn tick_represented_gameobject_update_like_cpp(&mut self) {
        let now = Instant::now();
        let current_player_guid = self.player_guid();
        let current_player_position = self.player_position_like_cpp();
        let current_map_id = self.player_map_id_like_cpp();
        let expired_per_player_states = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let despawn_expired = state
                    .per_player_despawn_until
                    .is_some_and(|until| until <= now);
                let go_state_expired = state
                    .per_player_go_state_until
                    .is_some_and(|until| until <= now);
                if !despawn_expired && !go_state_expired {
                    return None;
                }
                let current_go_state = state.go_state.unwrap_or(wow_entities::GoState::Ready);
                Some((
                    guid,
                    state
                        .per_player_state_player_guid
                        .unwrap_or(wow_core::ObjectGuid::EMPTY),
                    despawn_expired,
                    state
                        .per_player_go_state
                        .is_some_and(|go_state| go_state != current_go_state),
                ))
            })
            .collect::<Vec<_>>();
        let expired_despawn_delay_guids = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                state
                    .despawn_delay_until
                    .is_some_and(|despawn_until| despawn_until <= now)
                    .then_some(guid)
            })
            .collect::<Vec<_>>();
        let expired_respawn_guids = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                state
                    .respawn_until
                    .is_some_and(|respawn_until| respawn_until <= now)
                    .then_some(guid)
            })
            .collect::<Vec<_>>();
        let expired_door_or_button_guids = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let is_door_or_button = matches!(
                    state.go_type.map(u32::from),
                    Some(wow_entities::GAMEOBJECT_TYPE_DOOR | wow_entities::GAMEOBJECT_TYPE_BUTTON)
                );
                let is_activated = state.loot_state == Some(wow_entities::LootState::Activated);
                let cooldown_expired = state
                    .cooldown_until
                    .is_some_and(|cooldown_until| cooldown_until <= now);
                (is_door_or_button && is_activated && cooldown_expired).then_some(guid)
            })
            .collect::<Vec<_>>();
        let expired_goober_guids = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let is_goober =
                    state.go_type.map(u32::from) == Some(wow_entities::GAMEOBJECT_TYPE_GOOBER);
                let is_activated = state.loot_state == Some(wow_entities::LootState::Activated);
                let cooldown_expired = state
                    .cooldown_until
                    .is_some_and(|cooldown_until| cooldown_until <= now);
                (is_goober && is_activated && cooldown_expired).then_some(guid)
            })
            .collect::<Vec<_>>();
        let just_deactivated_goobers = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let is_goober =
                    state.go_type.map(u32::from) == Some(wow_entities::GAMEOBJECT_TYPE_GOOBER);
                let is_just_deactivated =
                    state.loot_state == Some(wow_entities::LootState::JustDeactivated);
                (is_goober && is_just_deactivated)
                    .then(|| state.goober_use_source.map(|source| (guid, source)))
                    .flatten()
            })
            .collect::<Vec<_>>();
        let mut generic_just_deactivated_gameobjects = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let is_goober =
                    state.go_type.map(u32::from) == Some(wow_entities::GAMEOBJECT_TYPE_GOOBER);
                let is_just_deactivated =
                    state.loot_state == Some(wow_entities::LootState::JustDeactivated);
                (is_just_deactivated && !is_goober).then_some((
                    guid,
                    state.go_type.map(u32::from),
                    state.owner_guid.is_some(),
                    state.linked_trap_entry,
                    state.linked_trap_guid,
                    state.despawn_at_action,
                    state.go_anim_progress,
                    state.chest_restock_time_secs,
                ))
            })
            .collect::<Vec<_>>();
        generic_just_deactivated_gameobjects
            .sort_by_key(|(_, _, delete_after_clear, _, _, _, _, _)| (*delete_after_clear,));
        let charge_depleted_guids = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let depletes_by_charges = matches!(
                    state.go_type.map(u32::from),
                    Some(
                        wow_entities::GAMEOBJECT_TYPE_SPELLCASTER
                            | wow_entities::GAMEOBJECT_TYPE_GUARDPOST
                    )
                );
                let max_charges = state.max_charges?;
                (depletes_by_charges && state.use_count >= max_charges)
                    .then_some((guid, max_charges))
            })
            .collect::<Vec<_>>();
        let not_ready_bomb_trap_guids = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let is_bomb_trap = state.go_type.map(u32::from)
                    == Some(wow_entities::GAMEOBJECT_TYPE_TRAP)
                    && state
                        .trap_use_source
                        .is_some_and(|source| source.charges == 2);
                let is_not_ready = state.loot_state == Some(wow_entities::LootState::NotReady);
                (is_bomb_trap && is_not_ready).then_some(guid)
            })
            .collect::<Vec<_>>();
        let not_ready_non_bomb_traps = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let source = state.trap_use_source?;
                let is_non_bomb_trap = state.go_type.map(u32::from)
                    == Some(wow_entities::GAMEOBJECT_TYPE_TRAP)
                    && source.charges != 2;
                let is_not_ready = state.loot_state == Some(wow_entities::LootState::NotReady);
                (is_non_bomb_trap && is_not_ready).then_some((
                    guid,
                    state.owner_guid.is_some() && state.owner_in_combat.unwrap_or(false),
                    source.start_delay_secs,
                ))
            })
            .collect::<Vec<_>>();
        let default_not_ready_gameobjects = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let Some(go_type) = state.go_type.map(u32::from) else {
                    return None;
                };
                let has_special_not_ready_branch = matches!(
                    go_type,
                    wow_entities::GAMEOBJECT_TYPE_TRAP
                        | wow_entities::GAMEOBJECT_TYPE_FISHING_NODE
                        | wow_entities::GAMEOBJECT_TYPE_CHEST
                );
                let is_not_ready = state.loot_state == Some(wow_entities::LootState::NotReady);
                (is_not_ready && !has_special_not_ready_branch).then_some(guid)
            })
            .collect::<Vec<_>>();
        let ready_fishing_bobbers = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let is_fishing_bobber = state.go_type.map(u32::from)
                    == Some(wow_entities::GAMEOBJECT_TYPE_FISHING_NODE);
                let is_not_ready = state.loot_state == Some(wow_entities::LootState::NotReady);
                let ready = state
                    .fishing_bobber_ready_at
                    .is_some_and(|ready_at| ready_at <= now);
                (is_fishing_bobber && is_not_ready && ready).then_some((
                    guid,
                    state.owner_guid.unwrap_or(wow_core::ObjectGuid::EMPTY),
                ))
            })
            .collect::<Vec<_>>();
        let restocked_chests = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let is_chest =
                    state.go_type.map(u32::from) == Some(wow_entities::GAMEOBJECT_TYPE_CHEST);
                let restock_expired = state
                    .chest_restock_until
                    .is_some_and(|restock_until| restock_until <= now);
                let can_restock_from_state = matches!(
                    state.loot_state,
                    Some(wow_entities::LootState::Activated | wow_entities::LootState::NotReady)
                );
                (is_chest && restock_expired && can_restock_from_state).then_some(guid)
            })
            .collect::<Vec<_>>();
        let ready_bomb_trap_guids = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let is_bomb_trap = state.go_type.map(u32::from)
                    == Some(wow_entities::GAMEOBJECT_TYPE_TRAP)
                    && state
                        .trap_use_source
                        .is_some_and(|source| source.charges == 2);
                let is_ready = state.loot_state == Some(wow_entities::LootState::Ready);
                let cooldown_expired = state
                    .cooldown_until
                    .is_none_or(|cooldown_until| cooldown_until <= now);
                (is_bomb_trap && is_ready && cooldown_expired).then_some(guid)
            })
            .collect::<Vec<_>>();
        let ready_non_bomb_traps = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let source = state.trap_use_source?;
                let is_non_bomb_trap = state.go_type.map(u32::from)
                    == Some(wow_entities::GAMEOBJECT_TYPE_TRAP)
                    && source.charges != 2;
                let is_ready = state.loot_state == Some(wow_entities::LootState::Ready);
                let cooldown_expired = state
                    .cooldown_until
                    .is_none_or(|cooldown_until| cooldown_until <= now);
                if !is_non_bomb_trap || !is_ready || !cooldown_expired || source.radius == 0 {
                    return None;
                }

                let target_guid = if state.owner_guid.is_some() || source.check_all_units {
                    state.trap_target_guid
                } else {
                    let player_guid = current_player_guid?;
                    let trap_position = state.position?;
                    let player_position = current_player_position?;
                    let same_map = state.map_id.is_none_or(|map_id| map_id == current_map_id);
                    let activation_radius = source.radius as f32 / 2.0;
                    (same_map && trap_position.is_within_dist(&player_position, activation_radius))
                        .then_some(player_guid)
                }?;

                Some((guid, target_guid))
            })
            .collect::<Vec<_>>();
        let activated_bomb_traps = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let source = state.trap_use_source?;
                let is_bomb_trap = state.go_type.map(u32::from)
                    == Some(wow_entities::GAMEOBJECT_TYPE_TRAP)
                    && source.charges == 2;
                let is_activated = state.loot_state == Some(wow_entities::LootState::Activated);
                (is_bomb_trap && is_activated).then_some((guid, source))
            })
            .collect::<Vec<_>>();
        let activated_non_bomb_traps = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let source = state.trap_use_source?;
                let is_non_bomb_trap = state.go_type.map(u32::from)
                    == Some(wow_entities::GAMEOBJECT_TYPE_TRAP)
                    && source.charges != 2;
                let is_activated = state.loot_state == Some(wow_entities::LootState::Activated);
                let target_guid = (!state.loot_state_unit_guid.is_empty())
                    .then_some(state.loot_state_unit_guid)?;
                (is_non_bomb_trap && is_activated).then_some((
                    guid,
                    source,
                    target_guid,
                    state.owner_guid.unwrap_or(wow_core::ObjectGuid::EMPTY),
                ))
            })
            .collect::<Vec<_>>();
        let expired_capture_points = self
            .represented_gameobject_use_states
            .iter()
            .filter_map(|(&guid, state)| {
                let source = state.capture_point_source?;
                let capture_team = match state.capture_point_state {
                    Some(RepresentedCapturePointStateLikeCpp::ContestedHorde) => Team::Horde,
                    Some(RepresentedCapturePointStateLikeCpp::ContestedAlliance) => Team::Alliance,
                    _ => return None,
                };
                let expired = state
                    .capture_point_assault_until
                    .is_some_and(|until| until <= now);
                expired.then_some((guid, capture_team, source))
            })
            .collect::<Vec<_>>();

        for (guid, player_guid, despawned, needs_state_update) in expired_per_player_states {
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                if despawned {
                    state.per_player_despawn_until = None;
                    state.per_player_despawn_secs = None;
                }
                state.per_player_go_state = None;
                state.per_player_go_state_until = None;
                if state.per_player_despawn_until.is_none()
                    && state.per_player_go_state_until.is_none()
                {
                    state.per_player_state_player_guid = None;
                }
            }
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::GameObjectPerPlayerStateExpired {
                    gameobject_guid: guid,
                    player_guid,
                    despawned,
                    needs_state_update,
                },
            );
        }
        for guid in expired_despawn_delay_guids {
            let linked_trap_guid = self
                .represented_gameobject_use_states
                .get(&guid)
                .and_then(|state| state.linked_trap_guid);
            if let Some(trap_guid) = linked_trap_guid.filter(|trap_guid| *trap_guid != guid) {
                self.despawn_represented_linked_trap_by_guid_like_cpp(trap_guid);
            }
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                state.despawn_delay_until = None;
                state.loot_state = Some(wow_entities::LootState::NotReady);
                state.loot_state_unit_guid = wow_core::ObjectGuid::EMPTY;
                if state.go_type.map(u32::from) != Some(wow_entities::GAMEOBJECT_TYPE_TRANSPORT) {
                    state.go_state = Some(wow_entities::GoState::Ready);
                }
            }
            self.client_visible_guids_like_cpp.remove(&guid);
            self.loot_table.remove(&guid);
            self.send_represented_gameobject_delete_packets_like_cpp(guid);
        }
        for guid in expired_respawn_guids {
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                state.respawn_until = None;
                state.loot_state = Some(wow_entities::LootState::Ready);
                state.go_state = Some(wow_entities::GoState::Ready);
            }
            self.client_visible_guids_like_cpp.insert(guid);
        }
        for guid in expired_door_or_button_guids {
            self.reset_represented_gameobject_door_or_button_like_cpp(guid);
        }
        for (guid, max_charges) in charge_depleted_guids {
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                state.use_count = 0;
                state.loot_state = Some(wow_entities::LootState::JustDeactivated);
            }
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::GameObjectChargesDepleted {
                    gameobject_guid: guid,
                    max_charges,
                    loot_state: wow_entities::LootState::JustDeactivated,
                },
            );
        }
        for guid in not_ready_bomb_trap_guids {
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                state.cooldown_until = Some(now + Duration::from_secs(10));
                state.loot_state = Some(wow_entities::LootState::Ready);
            }
        }
        for (guid, owner_in_combat, start_delay_secs) in not_ready_non_bomb_traps {
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                state.cooldown_until = owner_in_combat
                    .then_some(now + Duration::from_secs(u64::from(start_delay_secs)));
                state.loot_state = Some(wow_entities::LootState::Ready);
            }
        }
        for guid in default_not_ready_gameobjects {
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                state.loot_state = Some(wow_entities::LootState::Ready);
            }
        }
        for (guid, owner_guid) in ready_fishing_bobbers {
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                state.fishing_bobber_ready_at = None;
                state.loot_state = Some(wow_entities::LootState::Ready);
            }
            if !owner_guid.is_empty() {
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::FishingBobberReady {
                        gameobject_guid: guid,
                        owner_guid,
                    },
                );
            }
        }
        for guid in restocked_chests {
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                state.chest_restock_until = None;
                state.loot_state = Some(wow_entities::LootState::Ready);
                state.loot_state_unit_guid = wow_core::ObjectGuid::EMPTY;
            }
            self.loot_table.remove(&guid);
            let _ = self.queue_chest_gameobject_state_refresh_for_same_map_like_cpp(guid);
        }
        for guid in ready_bomb_trap_guids {
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                state.loot_state = Some(wow_entities::LootState::Activated);
            }
        }
        for (guid, target_guid) in ready_non_bomb_traps {
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                state.loot_state = Some(wow_entities::LootState::Activated);
                state.loot_state_unit_guid = target_guid;
            }
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::TrapTargetActivated {
                    gameobject_guid: guid,
                    target_guid,
                },
            );
        }
        for (guid, source) in activated_bomb_traps {
            if source.spell_id != 0 {
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::TrapBombSpellCast {
                        gameobject_guid: guid,
                        spell_id: source.spell_id,
                    },
                );
            }
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                state.loot_state = Some(wow_entities::LootState::JustDeactivated);
            }
        }
        for (guid, source, target_guid, original_caster_guid) in activated_non_bomb_traps {
            if source.spell_id != 0 {
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::TrapTargetSpellCast {
                        gameobject_guid: guid,
                        target_guid,
                        spell_id: source.spell_id,
                        original_caster_guid,
                    },
                );
            }
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                let cooldown_secs = if source.cooldown_secs != 0 {
                    source.cooldown_secs
                } else {
                    4
                };
                state.cooldown_until = Some(now + Duration::from_secs(u64::from(cooldown_secs)));
                if source.charges == 1 {
                    state.loot_state = Some(wow_entities::LootState::JustDeactivated);
                } else if source.charges == 0 {
                    state.loot_state = Some(wow_entities::LootState::Ready);
                }
            }
        }
        for (guid, capture_team, source) in expired_capture_points {
            let (state, broadcast_text_id, event_id) = match capture_team {
                Team::Horde => (
                    RepresentedCapturePointStateLikeCpp::HordeCaptured,
                    source.capture_broadcast_horde,
                    source.capture_event_horde,
                ),
                Team::Alliance => (
                    RepresentedCapturePointStateLikeCpp::AllianceCaptured,
                    source.capture_broadcast_alliance,
                    source.capture_event_alliance,
                ),
                Team::Other => continue,
            };
            if let Some(go_state) = self.represented_gameobject_use_states.get_mut(&guid) {
                go_state.capture_point_state = Some(state);
                go_state.capture_point_last_team_capture = capture_team;
                go_state.capture_point_assault_until = None;
            }
            self.record_represented_capture_point_update_like_cpp(
                guid,
                source,
                state,
                broadcast_text_id,
                event_id,
                0,
            );
        }
        for guid in expired_goober_guids {
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                state.gameobject_flags &= !wow_entities::GO_FLAG_IN_USE;
                state.loot_state = Some(wow_entities::LootState::JustDeactivated);
                state.cooldown_until = None;
            }
        }
        for (guid, source) in just_deactivated_goobers {
            self.apply_represented_gameobject_goober_just_deactivated_like_cpp(guid, source);
            if !source.consumable {
                let _ = self.queue_goober_gameobject_state_refresh_for_same_map_like_cpp(guid);
            }
        }
        for (
            guid,
            go_type,
            delete_after_clear,
            linked_trap_entry,
            linked_trap_guid,
            is_despawn_at_action,
            go_anim_progress,
            chest_restock_time_secs,
        ) in generic_just_deactivated_gameobjects
        {
            if let Some(trap_entry) = linked_trap_entry.filter(|trap_entry| *trap_entry != 0) {
                if let Some(trap_guid) = linked_trap_guid {
                    self.despawn_represented_linked_trap_by_guid_like_cpp(trap_guid);
                }
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::GameObjectLinkedTrapDespawn {
                        gameobject_guid: guid,
                        trap_entry,
                    },
                );
            }
            self.loot_table.remove(&guid);
            let mut delete_after_clear = delete_after_clear;
            let mut schedule_respawn = false;
            let is_chest = go_type == Some(wow_entities::GAMEOBJECT_TYPE_CHEST);
            if is_chest && !is_despawn_at_action && !delete_after_clear {
                if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                    state.loot_state_unit_guid = wow_core::ObjectGuid::EMPTY;
                    state.use_count = 0;
                    if chest_restock_time_secs.is_some_and(|restock_secs| restock_secs != 0) {
                        let restock_secs = chest_restock_time_secs.unwrap_or_default();
                        state.chest_restock_until =
                            Some(now + Duration::from_secs(u64::from(restock_secs)));
                        state.loot_state = Some(wow_entities::LootState::NotReady);
                    } else {
                        state.loot_state = Some(wow_entities::LootState::Ready);
                    }
                }
                self.represented_gameobject_use_effects.push(
                    RepresentedGameObjectUseEffect::GameObjectJustDeactivatedCleared {
                        gameobject_guid: guid,
                        deleted: false,
                    },
                );
                continue;
            }
            if let Some(state) = self.represented_gameobject_use_states.get_mut(&guid) {
                state.loot_state = Some(wow_entities::LootState::NotReady);
                state.loot_state_unit_guid = wow_core::ObjectGuid::EMPTY;
                state.use_count = 0;
                if !delete_after_clear
                    && state
                        .respawn_delay_secs
                        .is_some_and(|respawn_delay| respawn_delay != 0)
                {
                    if state.spawned_by_default == Some(false) {
                        delete_after_clear = true;
                    } else {
                        let respawn_delay = state.respawn_delay_secs.unwrap_or_default();
                        state.respawn_until =
                            Some(now + Duration::from_secs(u64::from(respawn_delay)));
                        schedule_respawn = true;
                    }
                }
            }
            let send_despawn_at_action = is_despawn_at_action || go_anim_progress > 0;
            if send_despawn_at_action && !delete_after_clear && !schedule_respawn {
                self.send_represented_gameobject_despawn_to_visible_set_like_cpp(guid);
                self.restore_represented_gameobject_override_flags_like_cpp(guid);
            }
            if delete_after_clear || schedule_respawn {
                self.client_visible_guids_like_cpp.remove(&guid);
                self.send_represented_gameobject_delete_packets_like_cpp(guid);
            }
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::GameObjectJustDeactivatedCleared {
                    gameobject_guid: guid,
                    deleted: delete_after_clear,
                },
            );
        }
    }
    pub(crate) fn represented_gameobject_area_id_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<u32> {
        self.represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.area_id)
            .or_else(|| self.player_zone_area_like_cpp().map(|(_, area_id)| area_id))
    }
    pub(in crate::session) fn represented_gameobject_spell_lookup_difficulty_id_like_cpp(
        &self,
    ) -> u8 {
        self.current_map_difficulty_id_like_cpp()
    }
}
