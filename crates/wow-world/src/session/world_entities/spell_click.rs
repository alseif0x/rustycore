//! Represented spell-click interaction with observed creatures.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_spell_click_creature_snapshot_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedSpellClickCreatureSnapshotLikeCpp> {
        self.world_entities
            .represented_spell_click_creature_snapshot_like_cpp(
                &self.core.quest_objective_access_like_cpp(),
                guid,
            )
    }
    pub(crate) fn represented_can_see_spell_click_on_creature_like_cpp(
        &self,
        creature_guid: ObjectGuid,
    ) -> RepresentedCanSeeSpellClickOutcomeLikeCpp {
        let owner = self.core.quest_objective_access_like_cpp();
        let npc_access = self.core.npc_interaction_access_with_selected_refs_like_cpp(
            self.catalogs.factions.store.as_deref(),
            self.catalogs.factions.template_store.as_deref(),
            self.catalogs.friendship_rep_reaction_store.as_deref(),
            #[cfg(any(test, feature = "test-fixtures"))]
            wow_world_core::session::NpcInteractionFixtureRefsLikeCpp::new(
                &self.fixtures.movement.player_position,
                &self.fixtures.identity.player_faction_template_like_cpp,
                &self.fixtures.identity.player_race,
                &self.fixtures.identity.player_class,
                &self.fixtures.combat.player_health_like_cpp,
                &self.fixtures.combat.player_max_health_like_cpp,
                &self.fixtures.combat.player_alive_like_cpp,
                &self.fixtures.progression.reputation_state_like_cpp,
                &self.fixtures.vehicles.taxi_destinations_like_cpp,
                &self.fixtures.vehicles.taxi_flight_state_like_cpp,
                &self.fixtures.vehicles.taxi_unit_flags_like_cpp,
                &self.fixtures.vehicles.taxi_mounted_like_cpp,
            ),
        );
        let player_access = self
            .core
            .player_condition_access_with_selected_fixture_refs_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))]
                wow_world_core::session::PlayerConditionFixtureRefsLikeCpp::new(
                    &self.fixtures.identity.player_race,
                    &self.fixtures.identity.player_class,
                    &self.fixtures.identity.player_level,
                    &self.fixtures.identity.player_gender,
                    &self
                        .fixtures
                        .progression
                        .represented_primary_specialization_id_like_cpp,
                    &self.fixtures.movement.player_position,
                    &self.fixtures.identity.player_zone_id_like_cpp,
                    &self.fixtures.identity.player_area_id_like_cpp,
                    &self
                        .fixtures
                        .identity
                        .player_zone_area_authority_complete_like_cpp,
                    &self.fixtures.combat.player_pvp_hostile_like_cpp,
                    &self.fixtures.combat.player_pvp_end_timer_like_cpp,
                    &self.fixtures.combat.player_contested_pvp_timer_like_cpp,
                    &self.fixtures.identity.represented_is_outdoors_like_cpp,
                    &self.fixtures.combat.player_health_like_cpp,
                    &self.fixtures.combat.player_max_health_like_cpp,
                    &self.fixtures.combat.player_alive_like_cpp,
                    &self.fixtures.vehicles.taxi_destinations_like_cpp,
                    &self.fixtures.vehicles.taxi_flight_state_like_cpp,
                    &self.fixtures.vehicles.taxi_unit_flags_like_cpp,
                    &self.fixtures.vehicles.taxi_mounted_like_cpp,
                    &self.fixtures.auras.visible_auras,
                    &self.fixtures.auras.player_aura_authority_complete_like_cpp,
                    &self
                        .fixtures
                        .auras
                        .player_spell_hit_aura_authority_tombstoned_like_cpp,
                    &self.fixtures.auras.canonical_threat_aura_snapshots_like_cpp,
                    &self
                        .fixtures
                        .progression
                        .player_skill_test_fixture_like_cpp
                        .player_skill_records_like_cpp,
                ),
            );
        let condition_projection = self.player_condition_projection_cx_like_cpp();
        wow_world_application::represented_can_see_spell_click_on_like_cpp(
            &self.world_entities,
            &owner,
            &npc_access,
            &player_access,
            &condition_projection,
            &self.social,
            self.catalogs.chr.specialization_store.as_deref(),
            creature_guid,
            self.catalogs.spell_catalogs.npc_spell_click_store.as_deref(),
            self.catalogs.condition_store.as_deref(),
            self.catalogs.player_condition_store.as_ref(),
            self.catalogs.area_table_store.as_ref(),
            cfg!(test),
        )
    }
    pub(in crate::session) async fn apply_represented_spell_click_creature_damage_to_clicker_like_cpp(
        &mut self,
        spell_id: i32,
        player_guid: ObjectGuid,
        damage_amount: u32,
    ) -> Result<(), &'static str> {
        if self.player_guid() != Some(player_guid) {
            return Err("Target player not current session");
        }
        let Some((original_health, health_after, _, applied_damage, _)) = crate::session::hub_mut(
            self,
        )
        .apply_owned_player_damage_like_cpp(damage_amount, wow_constants::DeathState::Corpse) else {
            return Err("Target player owner not available");
        };
        if damage_amount > 0 && applied_damage == 0 {
            debug!(
                account = self.core.account_id,
                player = ?player_guid,
                spell_id,
                damage = damage_amount,
                "Skipping spellclick creature-caster damage because C++ EffectSchoolDMG requires alive target"
            );
            return Ok(());
        }
        self.sync_player_registry_state_like_cpp();
        if health_after != original_health {
            self.core
                .send_player_health_update_like_cpp(player_guid, u64::from(health_after));
        }

        Ok(())
    }
    pub(in crate::session) async fn apply_represented_spell_click_creature_damage_to_clickee_like_cpp(
        &mut self,
        spell_id: i32,
        creature_guid: ObjectGuid,
        damage_amount: u32,
    ) -> Result<(), &'static str> {
        let account_id = self.core.account_id;
        let values_update = self.core.mutate_world_creature(creature_guid, |creature| {
                if !creature.is_alive() {
                    debug!(
                        account = account_id,
                        creature = ?creature_guid,
                        spell_id,
                        damage = damage_amount,
                        "Skipping spellclick creature-caster damage because C++ EffectSchoolDMG requires alive target"
                    );
                    return None;
                }
                let _died = creature.take_damage_before_death_state_like_cpp(damage_amount);
                Some(creature.creature.unit().values_update())
            })
            .ok_or("Target creature not found")?;
        let Some(values_update) = values_update else {
            return Ok(());
        };

        if self
            .core
            .client_visible_guids_like_cpp
            .contains(&creature_guid)
            && let Some(update) = self.represented_unit_values_update_to_update_object_like_cpp(
                creature_guid,
                self.core.player_map_id_like_cpp(),
                &values_update,
            )
        {
            self.send_packet(&update);
        }

        Ok(())
    }
}
