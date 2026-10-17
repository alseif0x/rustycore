//! Represented load and hydration at the Session boundary.
//!
//! Moved out of the Session root under #609. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) async fn load_instance_time_restrictions_like_cpp(&mut self) {
        crate::session::cx_lifecycle(self)
            .load_instance_time_restrictions_like_cpp()
            .await
    }
    /// Set the player loading GUID (ConnectTo flow).
    pub fn set_player_loading(&mut self, guid: Option<ObjectGuid>) {
        self.lifecycle.set_player_loading(guid);
        wow_world_entities::sync_current_player_session_visibility_detection_like_cpp(self);
    }
    pub async fn load_tutorials_data_like_cpp(&mut self) {
        self.lifecycle
            .load_tutorials_data_like_cpp(self.core.account_id)
            .await
    }
    pub async fn load_global_account_data_like_cpp(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state
            .load_account_data_like_cpp(&mut hub, ObjectGuid::EMPTY, GLOBAL_CACHE_MASK_LIKE_CPP)
            .await;
    }
}

impl crate::session::LifecycleCx<'_> {
    #[cfg(test)]
    pub(in crate::session) fn load_instance_time_restriction_rows_like_cpp(
        &mut self,
        rows: impl IntoIterator<Item = (u32, u64)>,
    ) {
        self.instances
            .replace_represented_instance_reset_times_for_test_like_cpp(rows);
    }

    pub(crate) async fn load_instance_time_restrictions_like_cpp(&mut self) {
        let _ = self
            .instances
            .replace_instance_reset_times_like_cpp(&mut self.hub, []);

        let Some(port) = self
            .lifecycle
            .player_lifecycle_port_like_cpp()
            .map(Arc::clone)
        else {
            warn!(
                account = self.hub.core.account_id,
                "LoadInstanceTimeRestrictions skipped: Player lifecycle port unavailable"
            );
            return;
        };

        let rows = match port
            .load_login_auxiliary_like_cpp(
                wow_persistence::PlayerLoginAuxiliaryLoadRequestLikeCpp::InstanceTimeRestrictions {
                    account_id: self.hub.core.account_id,
                },
            )
            .await
        {
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(
                wow_persistence::PlayerLoginAuxiliaryLoadedLikeCpp::InstanceTimeRestrictions(rows),
            ) => rows,
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Failed { reason } => {
                warn!(
                    account = self.hub.core.account_id,
                    "LoadInstanceTimeRestrictions query failed: {reason}"
                );
                return;
            }
            wow_persistence::PlayerLoginAuxiliaryLoadOutcomeLikeCpp::Loaded(_) => {
                warn!(
                    account = self.hub.core.account_id,
                    "Player lifecycle port returned the wrong auxiliary login data for instance time restrictions"
                );
                return;
            }
        };

        let _ = self.instances.replace_instance_reset_times_like_cpp(
            &mut self.hub,
            rows.into_iter()
                .map(|row| (row.instance_id, row.release_time)),
        );
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/persistence/load/f3_shims.rs"]
mod f3_shims;
