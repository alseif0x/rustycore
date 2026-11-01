// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! #1241 F3 test shims: WorldSession entry points kept only for unit_tests callers.

#[allow(unused_imports)]
use super::*;

impl crate::session::WorldSession {
    pub(crate) fn account_heirloom_save_rows_like_cpp(
        &self,
    ) -> Option<Vec<AccountHeirloomSaveRowLikeCpp>> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_heirloom_save_rows_like_cpp(hub)
    }
    pub(crate) fn account_toy_save_rows_like_cpp(&self) -> Option<Vec<AccountToySaveRowLikeCpp>> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_toy_save_rows_like_cpp(hub)
    }
    pub(crate) fn account_mount_save_rows_like_cpp(
        &self,
    ) -> Option<Vec<AccountMountSaveRowLikeCpp>> {
        let (state, hub) = crate::session::split_lifecycle_ref(self);
        state.account_mount_save_rows_like_cpp(hub)
    }
}

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
