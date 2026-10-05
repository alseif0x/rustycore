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
                && self
                    .loot
                    .cached_loot_contains_owner_like_cpp(gameobject_guid))
            .then_some(());
        };
        let loot = self
            .loot
            .cached_loot_for_owner_like_cpp(gameobject_guid)?
            .clone();
        let is_personal = self.loot.is_personal_loot_owner_like_cpp(gameobject_guid);
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
            self.loot
                .remove_cached_loot_for_owner_like_cpp(gameobject_guid);
            self.loot
                .remove_cached_loot_generation_like_cpp(gameobject_guid);
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
        let observation = crate::session::cx_loot(self)
            .represented_gameobject_loot_install_observation_like_cpp(gameobject_guid)?;
        self.upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            replace,
            &observation,
        )
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
            self.loot
                .discard_represented_personal_loot_cache_for_player_like_cpp(
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
            self.loot
                .discard_represented_personal_loot_cache_for_player_like_cpp(
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

    /// World facade over the single App authority for the autostore distance,
    /// display-box and spell-lock gates.
    pub(in crate::handlers::loot) fn represented_gameobject_can_autostore_loot_item_like_cpp(
        &self,
        guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        wow_world_application::represented_gameobject_can_autostore_loot_item_like_cpp(
            &self.loot,
            &self.world_entities,
            crate::session::hub_ref(self),
            &self.known_spells_like_cpp(),
            guid,
            player_guid,
        )
    }
}

impl crate::session::LootCxRef<'_> {
    fn represented_gameobject_loot_state_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectLootStateLikeCpp> {
        if !guid.is_game_object() {
            return None;
        }

        let canonical_position = self.loot.canonical_map_object_position_for_loot_like_cpp(
            self.hub,
            guid,
            &[
                AccessorObjectKind::GameObject,
                AccessorObjectKind::Transport,
            ],
        );
        let canonical_owner = self
            .loot
            .canonical_gameobject_owner_for_loot_like_cpp(self.hub, guid);
        let represented_state = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(guid);
        if canonical_position.is_none()
            && represented_state.and_then(|state| state.position).is_none()
            && !self.hub.core.client_visible_guids_like_cpp.contains(&guid)
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
}

impl crate::session::LootCx<'_> {
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
        self.world_entities
            .mutate_canonical_gameobject_by_guid_like_cpp(
                &mut self.hub,
                gameobject_guid,
                |gameobject| {
                    (gameobject.loot_state() != LootState::JustDeactivated).then(|| {
                        let authority = gameobject.loot_authority_like_cpp().clone();
                        RepresentedGameObjectLootInstallObservationLikeCpp {
                            object_generation: authority.generation_like_cpp(),
                            authority,
                            loot_lifecycle_revision: gameobject.loot_lifecycle_revision_like_cpp(),
                        }
                    })
                },
            )
    }
}

#[cfg(test)]
#[path = "../../../../unit_tests/handlers/loot/sources/gameobject_authority/f3_shims.rs"]
mod f3_shims;
