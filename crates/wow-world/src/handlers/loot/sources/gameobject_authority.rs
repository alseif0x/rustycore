use super::*;

impl WorldSession {
    pub(in crate::handlers::loot) fn sync_represented_gameobject_loot_to_canonical_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> Option<()> {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(gameobject_guid)
        else {
            return (represented_local_loot_fixture_allowed_like_cpp()
                && self.loot_table.contains_key(&gameobject_guid))
            .then_some(());
        };
        let loot = self.loot_table.get(&gameobject_guid)?.clone();
        let is_personal = self
            .represented_personal_loot_owners
            .contains(&gameobject_guid);
        let (shared, personal) = self.represented_loot_authority_pools_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            is_personal,
        )?;
        let installed = authority
            .initialize_pristine_like_cpp(shared, personal)
            .installed();
        if !installed
            && authority
                .snapshot_for_player_like_cpp(player_guid)
                .is_none()
        {
            self.loot_table.remove(&gameobject_guid);
            self.represented_loot_cache_generations_like_cpp
                .remove(&gameobject_guid);
            return None;
        }
        self.refresh_owned_loot_summary_like_cpp(gameobject_guid);
        let _ = self.reconcile_represented_loot_cache_like_cpp(gameobject_guid, player_guid);
        Some(())
    }

    pub(in crate::handlers::loot) fn upsert_represented_personal_gameobject_loot_authority_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
    ) -> Option<()> {
        let observation =
            self.represented_gameobject_loot_install_observation_like_cpp(gameobject_guid)?;
        self.upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            replace,
            &observation,
        )
    }

    pub(in crate::handlers::loot) fn represented_gameobject_loot_install_observation_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectLootInstallObservationLikeCpp> {
        self.represented_gameobject_loot_install_observation_result_like_cpp(gameobject_guid)?
    }

    /// Preserves the distinction between a missing canonical owner (`None`)
    /// and an owner whose current lifecycle rejects generation (`Some(None)`).
    /// Test-only packet fixtures may fall back only for the former.
    pub(super) fn represented_gameobject_loot_install_observation_result_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
    ) -> Option<Option<RepresentedGameObjectLootInstallObservationLikeCpp>> {
        self.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, |gameobject| {
            (gameobject.loot_state() != LootState::JustDeactivated).then(|| {
                let authority = gameobject.loot_authority_like_cpp().clone();
                RepresentedGameObjectLootInstallObservationLikeCpp {
                    object_generation: authority.generation_like_cpp(),
                    authority,
                    loot_lifecycle_revision: gameobject.loot_lifecycle_revision_like_cpp(),
                }
            })
        })
    }

    pub(in crate::handlers::loot) fn upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
        observation: &RepresentedGameObjectLootInstallObservationLikeCpp,
    ) -> Option<()> {
        self.upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            replace,
            false,
            observation,
        )
    }

    pub(in crate::handlers::loot) fn upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
        discard_empty_pool: bool,
        observation: &RepresentedGameObjectLootInstallObservationLikeCpp,
    ) -> Option<()> {
        let (_, mut personal) = self.represented_loot_authority_pools_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            true,
        )?;
        let pool = personal.remove(&player_guid)?;
        if discard_empty_pool && loot_is_looted_like_cpp(&pool) {
            self.discard_represented_personal_loot_cache_for_player_like_cpp(
                gameobject_guid,
                player_guid,
            );
            return None;
        }
        let installed =
            self.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, move |gameobject| {
                gameobject.install_personal_loot_if_lifecycle_like_cpp(
                    &observation.authority,
                    observation.object_generation,
                    observation.loot_lifecycle_revision,
                    player_guid,
                    pool,
                    replace,
                )
            });
        if installed != Some(true) {
            self.discard_represented_personal_loot_cache_for_player_like_cpp(
                gameobject_guid,
                player_guid,
            );
            return None;
        }
        if !self.reconcile_represented_loot_cache_like_cpp(gameobject_guid, player_guid) {
            return None;
        }
        Some(())
    }

    pub(in crate::handlers::loot) fn canonical_gameobject_fully_looted_after_represented_sync_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        fallback_fully_looted: bool,
    ) -> bool {
        if self
            .sync_represented_gameobject_loot_to_canonical_like_cpp(gameobject_guid, player_guid)
            .is_some()
        {
            return self
                .canonical_gameobject_is_fully_looted_like_cpp(gameobject_guid)
                .unwrap_or(fallback_fully_looted);
        }

        fallback_fully_looted
    }

    fn canonical_gameobject_owner_for_loot_like_cpp(&self, guid: ObjectGuid) -> Option<ObjectGuid> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?.map();
        let owner_guid = map.get_typed_game_object(guid)?.owner_guid();
        (!owner_guid.is_empty()).then_some(owner_guid)
    }

    fn represented_gameobject_loot_state_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectLootStateLikeCpp> {
        if !guid.is_game_object() {
            return None;
        }

        let canonical_position = self.canonical_map_object_position_for_loot_like_cpp(
            guid,
            &[
                AccessorObjectKind::GameObject,
                AccessorObjectKind::Transport,
            ],
        );
        let canonical_owner = self.canonical_gameobject_owner_for_loot_like_cpp(guid);
        let represented_state = self.represented_gameobject_use_states.get(&guid);
        if canonical_position.is_none()
            && represented_state.and_then(|state| state.position).is_none()
            && !self.client_visible_guids_like_cpp.contains(&guid)
        {
            return None;
        }

        Some(RepresentedGameObjectLootStateLikeCpp {
            position: canonical_position
                .or_else(|| represented_state.and_then(|state| state.position)),
            display_id: represented_state.and_then(|state| state.display_id),
            scale: represented_state.map(|state| state.scale).unwrap_or(1.0),
            rotation: represented_state
                .map(|state| state.rotation)
                .unwrap_or([0.0, 0.0, 0.0, 1.0]),
            go_type: represented_state.and_then(|state| state.go_type),
            interact_radius_override: represented_state
                .and_then(|state| state.interact_radius_override),
            lock_id: represented_state.and_then(|state| state.lock_id),
            owner_guid: canonical_owner
                .or_else(|| represented_state.and_then(|state| state.owner_guid)),
        })
    }

    pub(super) fn represented_gameobject_exists_for_loot_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.represented_gameobject_loot_state_like_cpp(guid)
            .is_some()
    }

    fn represented_gameobject_spell_lock_range_like_cpp(
        &self,
        lock_id: Option<u32>,
    ) -> Option<f32> {
        let lock_id = lock_id?;
        let lock = self.lock_store()?.get(lock_id)?;
        for i in 0..wow_data::lock::MAX_LOCK_CASE {
            let lock_type = lock.lock_type[i];
            if lock_type == 0 {
                continue;
            }

            if lock_type == LOCK_KEY_SPELL_LIKE_CPP {
                if let Some(range) = self.represented_spell_max_range_like_cpp(lock.index[i]) {
                    return Some(range);
                }
            }

            if lock_type != LOCK_KEY_SKILL_LIKE_CPP {
                break;
            }

            for spell_id in self.known_spells_like_cpp() {
                let Some(spell) = self.spell_store().and_then(|store| store.get(spell_id)) else {
                    continue;
                };
                let can_open_lock = spell.effects().iter().any(|effect| {
                    effect.effect == SPELL_EFFECT_OPEN_LOCK_LIKE_CPP
                        && effect.effect_misc_value_1 == lock.index[i]
                        && effect.effect_base_points >= i32::from(lock.skill[i])
                });
                if can_open_lock {
                    if let Some(range) = self.represented_spell_max_range_like_cpp(spell_id) {
                        return Some(range);
                    }
                }
            }
        }

        None
    }

    pub(in crate::handlers::loot) fn represented_gameobject_can_autostore_loot_item_like_cpp(
        &self,
        guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(state) = self.represented_gameobject_loot_state_like_cpp(guid) else {
            return false;
        };

        // C++ ref: LootHandler.cpp HandleAutostoreLootItemOpcode skips distance
        // for owned GameObjects and GAMEOBJECT_TYPE_FISHINGHOLE. DB spawns do
        // not carry CreatedBy; apply the owner exception only when runtime GO
        // state explicitly recorded GetOwnerGUID.
        if state.owner_guid == Some(player_guid)
            || state.go_type == Some(GAMEOBJECT_TYPE_FISHING_HOLE as u8)
        {
            return true;
        }

        match (self.player_position_like_cpp(), state.position) {
            (Some(player), Some(position)) => {
                let radius = represented_gameobject_interaction_distance_like_cpp(
                    state.go_type,
                    state.interact_radius_override,
                );
                let radius = self
                    .represented_gameobject_spell_lock_range_like_cpp(state.lock_id)
                    .unwrap_or(radius);
                if let Some(display_info) = self.gameobject_display_info_store().and_then(|store| {
                    state
                        .display_id
                        .and_then(|display_id| store.get(display_id))
                }) {
                    represented_gameobject_display_box_contains_like_cpp(
                        position,
                        player,
                        display_info,
                        state.scale,
                        state.rotation,
                        radius,
                    )
                } else {
                    player.is_within_dist(&position, radius)
                }
            }
            _ => true,
        }
    }
}
