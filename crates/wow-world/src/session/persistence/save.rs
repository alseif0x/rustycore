//! Represented Session-side save operations.
//!
//! Moved out of the Session root under #609. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn current_player_save_to_db_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerSaveToDbSnapshotLikeCpp> {
        #[cfg(test)]
        if self.core.player_handle_like_cpp.is_none() {
            return self.fixture_player_save_to_db_snapshot_like_cpp();
        }
        wow_world_core::session::PlayerSaveOwnerAccessLikeCpp::header_like_cpp(&self.core)
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_player_save_to_db_snapshot_like_cpp(
        &self,
    ) -> Option<PlayerSaveToDbSnapshotLikeCpp> {
        let guid = self.player_guid()?;
        // C++ saves through this session's exact `Player*`. Resolve the raw
        // power array through the generation-checked owner before any spatial
        // lookup so a replacement with the same GUID cannot be persisted by a
        // stale session incarnation.
        let powers = self.resolved_player_power_snapshot_like_cpp()?;
        let xp = crate::session::hub_ref(self).resolved_player_xp_like_cpp()?;
        let money = self.resolved_player_money_like_cpp()?;
        // Resolve every session-owned input before taking the manager lock:
        // `player_level_like_cpp` re-enters it and would self-deadlock.
        let level = crate::session::hub_ref(self).player_level_like_cpp();
        let pending_teleport_destination = self.pending_teleport_save_destination_like_cpp();
        if let Some(snapshot) =
            wow_world_core::session::PlayerSaveOwnerAccessLikeCpp::fixture_canonical_header_like_cpp(
                &self.core,
                guid,
                level,
                xp,
                money,
                powers,
                pending_teleport_destination,
            )
        {
            return Some(snapshot);
        }

        let (map_id, instance_id, position) =
            if let Some((map_id, position)) = pending_teleport_destination {
                (map_id, 0, position)
            } else {
                (
                    self.core.player_map_id_like_cpp(),
                    self.core
                        .current_canonical_player_map_key_like_cpp()
                        .map(|key| key.instance_id)
                        .unwrap_or(0),
                    crate::session::hub_ref(self).player_position_like_cpp()?,
                )
            };

        let (health, max_health, _) =
            crate::session::hub_ref(self).resolved_player_vitals_like_cpp()?;
        Some(PlayerSaveToDbSnapshotLikeCpp {
            guid,
            map_id,
            instance_id,
            position,
            level: crate::session::hub_ref(self).player_level_like_cpp(),
            xp,
            money,
            health,
            max_health,
            powers,
        })
    }
    pub(crate) async fn persist_standalone_player_currency_save_like_cpp(
        &mut self,
        character_guid: u64,
        pre_save_snapshot: HashMap<u32, PlayerCurrency>,
    ) -> Result<(), wow_persistence::PersistenceOutcomeLikeCpp> {
        let Some(port) = self
            .lifecycle
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        else {
            return Ok(());
        };
        let Some(mut currencies) = self.player_currencies_like_cpp() else {
            return Err(wow_persistence::PersistenceOutcomeLikeCpp::Failed {
                reason: "canonical Player currency owner is unavailable".to_string(),
            });
        };
        let request = self
            .catalogs
            .plan_player_currency_save_like_cpp(character_guid, &mut currencies);
        if !self.set_player_currencies_like_cpp(currencies) {
            return Err(wow_persistence::PersistenceOutcomeLikeCpp::Failed {
                reason: "canonical Player currency owner became unavailable".to_string(),
            });
        }
        let outcome = port.persist_currency_save_like_cpp(request).await;
        if matches!(
            outcome,
            wow_persistence::PersistenceOutcomeLikeCpp::Applied { .. }
        ) {
            Ok(())
        } else {
            self.set_player_currencies_like_cpp(pre_save_snapshot);
            Err(outcome)
        }
    }
    pub fn set_player_save_interval_ms_like_cpp(&mut self, interval_ms: u32) {
        self.lifecycle
            .set_player_save_interval_ms_like_cpp(interval_ms);
    }
    pub(in crate::session) fn resolved_player_flags_for_rest_state_save_like_cpp(
        &self,
    ) -> Option<u32> {
        wow_world_application::QuestRewardCx::resolved_player_flags_for_rest_state_save_from_access_like_cpp(
            &self
                .core
                .xp_gain_access_like_cpp(&self.catalogs, &self.config),
            &self.lifecycle,
            cfg!(test),
            #[cfg(any(test, feature = "test-fixtures"))]
            &self.fixtures.progression.rest_mgr_test_fixture_like_cpp,
        )
    }
    #[cfg(test)]
    pub(in crate::session) fn represented_player_flags_for_rest_state_save_like_cpp(&self) -> u32 {
        self.resolved_player_flags_for_rest_state_save_like_cpp()
            .unwrap_or_else(|| {
                self.lifecycle
                    .represented_loaded_player_flags_for_test_like_cpp()
                    .unwrap_or(0)
            })
    }
    pub(in crate::session) async fn process_pending_periodic_player_save_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) {
        if !self.lifecycle.pending_periodic_player_save_like_cpp()
            || self.core.state != SessionState::LoggedIn
        {
            return;
        }
        if self.pending_teleport_save_destination_like_cpp().is_some() {
            return;
        }

        self.save_current_player_to_db_with_generator_like_cpp(item_guid_generator)
            .await;
    }
    #[cfg(test)]
    pub(in crate::session) async fn process_pending_periodic_player_save_like_cpp(&mut self) {
        let generators = self.id_generators_for_test_like_cpp();
        self.process_pending_periodic_player_save_with_generator_like_cpp(generators.item.as_ref())
            .await;
    }
    #[cfg(test)]
    pub(in crate::session) fn fixture_mark_player_skills_saved_like_cpp(&mut self) {
        let Some(mut records) =
            crate::session::hub_ref(self).resolved_player_skill_records_like_cpp()
        else {
            return;
        };
        let Some(mut tombstones) =
            crate::session::hub_ref(self).resolved_player_skill_non_durable_tombstones_like_cpp()
        else {
            return;
        };
        for skill in records.values_mut() {
            if skill.state == RepresentedPlayerSkillStateLikeCpp::Deleted {
                tombstones.insert(skill.skill_id);
            }
            skill.state = RepresentedPlayerSkillStateLikeCpp::Unchanged;
        }
        let occupied =
            crate::session::hub_ref(self).complete_player_skill_occupied_slots_like_cpp();
        let _ = self.replace_player_skill_runtime_exact_like_cpp(
            records,
            true,
            occupied.is_some(),
            occupied,
            tombstones,
        );
        self.sync_player_registry_state_like_cpp();
    }
    pub(in crate::session) fn has_complete_player_skill_save_authority_like_cpp(&self) -> bool {
        crate::session::hub_ref(self)
            .complete_player_skill_records_like_cpp()
            .zip(crate::session::hub_ref(self).complete_player_skill_occupied_slots_like_cpp())
            .is_some_and(|(skills, occupied_slots)| skills.len() == usize::from(occupied_slots))
    }
    pub(crate) fn represented_save_cuf_profiles_like_cpp(
        &mut self,
        profiles: Vec<wow_packet::packets::misc::CufProfile>,
    ) -> bool {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.represented_save_cuf_profiles_like_cpp(&mut hub, profiles)
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/persistence/save/f3_shims.rs"]
mod f3_shims;
