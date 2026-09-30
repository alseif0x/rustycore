//! Feature-only Character world entry operation forwards.

use super::*;

impl WorldSession {
    pub fn character_begin_worldport_post_add_for_test(
        &mut self,
        map_id: u32,
        position: Position,
    ) -> bool {
        self.begin_worldport_post_add_like_cpp(map_id, position)
    }

    pub fn character_canonical_player_has_player_flag_for_test(
        &self,
        guid: ObjectGuid,
        flag: u32,
    ) -> Option<bool> {
        self.canonical_player_has_player_flag_like_cpp(guid, flag)
    }

    pub fn character_creature_spawn_catalogs_for_test_for_test(
        &self,
    ) -> CreatureSpawnCatalogsLikeCpp {
        self.creature_spawn_catalogs_for_test_like_cpp()
    }

    pub fn character_current_canonical_player_map_key_for_test(
        &self,
    ) -> Option<wow_map::MapKey> {
        self.current_canonical_player_map_key_like_cpp()
    }

    pub fn character_ensure_canonical_world_map_for_current_player_for_test(
        &mut self,
    ) -> Option<wow_map::CreateMapDecision> {
        self.ensure_canonical_world_map_for_current_player_like_cpp()
    }

    pub fn character_has_represented_battle_pet_journal_lock_for_test(
        &self,
    ) -> bool {
        self.has_represented_battle_pet_journal_lock_like_cpp()
    }

    pub fn character_pending_teleport_for_test(
        &self,
    ) -> Option<(u32, Position)> {
        self.pending_teleport_like_cpp()
    }

    pub fn character_register_in_player_registry_for_test(
        &self,
    ) {
        self.register_in_player_registry()
    }

    pub fn character_release_character_login_claim_for_test(
        &mut self,
    ) {
        self.release_character_login_claim_like_cpp()
    }

    pub fn character_remove_current_player_from_canonical_current_map_for_test(
        &mut self,
    ) -> bool {
        self.remove_current_player_from_canonical_current_map_like_cpp()
    }

    pub fn character_represented_delayed_resurrection_after_teleport_for_test(
        &self,
    ) -> Option<PlayerResurrectionRequestLikeCpp> {
        self.player_resurrection_state_snapshot_like_cpp()
            .and_then(|state| state.delayed_after_teleport)
    }

    pub fn character_represented_far_teleport_pending_for_test(
        &self,
    ) -> bool {
        self.represented_far_teleport_pending_like_cpp()
    }

    pub fn character_schedule_represented_resurrection_after_teleport_for_test(
        &mut self,
        request: PlayerResurrectionRequestLikeCpp,
    ) -> bool {
        self.schedule_represented_resurrection_after_teleport_like_cpp(request)
    }

    pub fn character_set_pending_teleport_for_test(
        &mut self,
        destination: Option<(u32, Position)>,
    ) -> bool {
        self.set_pending_teleport_like_cpp(destination)
    }

    pub fn character_set_represented_far_teleport_pending_for_test(
        &mut self,
        pending: bool,
    ) -> bool {
        self.set_represented_far_teleport_pending_like_cpp(pending)
    }

    pub fn character_temporary_pet_resummon_requests_for_test(
        &self,
    ) -> u32 {
        self.temporary_pet_resummon_requests_like_cpp()
    }

    pub fn character_try_claim_character_login_for_test(
        &mut self,
        guid: ObjectGuid,
    ) -> bool {
        self.try_claim_character_login_like_cpp(guid)
    }
}
