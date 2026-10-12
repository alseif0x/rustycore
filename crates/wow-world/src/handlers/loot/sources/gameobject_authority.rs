use super::*;

impl WorldSession {
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
    pub(super) fn represented_gameobject_exists_for_loot_like_cpp(&self, guid: ObjectGuid) -> bool {
        if !guid.is_game_object() {
            return false;
        }

        let canonical_position = self.loot.canonical_map_object_position_for_loot_like_cpp(
            self.hub,
            guid,
            &[
                AccessorObjectKind::GameObject,
                AccessorObjectKind::Transport,
            ],
        );
        let represented_state = self
            .world_entities
            .represented_gameobject_use_state_like_cpp(guid);
        canonical_position.is_some()
            || represented_state.and_then(|state| state.position).is_some()
            || self.hub.core.client_visible_guids_like_cpp.contains(&guid)
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
