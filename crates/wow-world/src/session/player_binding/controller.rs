//! Controller attachment and its explicitly selected detached-fixture route.

use super::*;

impl WorldSession {
    pub(crate) fn attach_player_controller_like_cpp(
        &mut self,
        controller: SessionPlayerController,
    ) {
        self.attach_player_controller_with_fixture_mode(controller, cfg!(test));
    }

    /// Keep the historical unit-fixture writes and delayed canonical installation.
    /// Normal production attachment, including test-fixtures builds, does not use this route.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) fn attach_player_controller_for_fixture(
        &mut self,
        controller: SessionPlayerController,
    ) {
        self.attach_player_controller_with_fixture_mode(controller, true);
    }

    fn attach_player_controller_with_fixture_mode(
        &mut self,
        controller: SessionPlayerController,
        hydrate_fixture: bool,
    ) {
        let controller_position = controller.position();
        self.set_player_guid(Some(controller.guid()));
        self.player_identity_bootstrap_like_cpp = Some(PlayerIdentityBootstrapLikeCpp {
            name: Some(controller.name().to_string()),
            race: controller.race(),
            class: controller.class(),
            level: controller.level(),
            gender: controller.gender(),
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if hydrate_fixture {
            self.player_name = Some(controller.name().to_string());
            self.player_position = Some(controller_position);
        }
        self.current_map_id = controller.map_id();
        #[cfg(any(test, feature = "test-fixtures"))]
        if hydrate_fixture {
            self.player_race = controller.race();
            self.player_class = controller.class();
            self.player_level = controller.level();
            self.player_gender = controller.gender();
        }
        #[cfg(any(test, feature = "test-fixtures"))]
        if hydrate_fixture {
            self.visibility_test_fixture_like_cpp
                .represented_seer_guid_like_cpp = Some(controller.guid());
        }
        self.visibility_publication.last_observed_farsight_object_like_cpp = wow_core::ObjectGuid::EMPTY;
        #[cfg(any(test, feature = "test-fixtures"))]
        if hydrate_fixture {
            self.player_bootstrap_attached_like_cpp = true;
        }
        self.initialize_reputation_mgr_like_cpp();
        self.set_fall_information_like_cpp(0, controller_position.z);
        // Production receives MapManager at session construction, so consume
        // the login bootstrap immediately. Unit fixtures historically inject
        // or replace their synthetic manager after attachment; they exercise
        // the same ownership transition through
        // `ensure_canonical_player_owner_for_map_like_cpp` instead.
        if !hydrate_fixture {
            let _ = self.install_detached_canonical_player_from_session_like_cpp(controller_position);
        }
        self.set_player_moved_unit_guid_like_cpp(controller.guid());
    }
}
