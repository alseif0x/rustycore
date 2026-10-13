//! The World session as the host of the talent bodies moved to
//! `wow_world_spell` (#1363 slice 9b). Each operation names the inherent
//! `WorldSession` method by path.

use std::future::Future;

use super::*;

impl wow_world_spell::TalentHostLikeCpp for WorldSession {
    type CommittedTalentResetLikeCpp = CommittedRepresentedTalentResetLikeCpp;
    fn refresh_represented_talent_points_like_cpp(&mut self) {
        WorldSession::refresh_represented_talent_points_like_cpp(self)
    }
    fn resolved_represented_at_login_flags_like_cpp(&self) -> Option<u16> {
        WorldSession::resolved_represented_at_login_flags_like_cpp(self)
    }
    fn remove_represented_at_login_flag_like_cpp(&mut self, flags: u16, persist: bool) -> bool {
        WorldSession::remove_represented_at_login_flag_like_cpp(self, flags, persist)
    }
    fn represented_talents_loaded_like_cpp(&self) -> bool {
        WorldSession::represented_talents_loaded_like_cpp(self)
    }
    fn trinity_string_like_cpp(&self, entry: u32) -> &str {
        WorldSession::trinity_string_like_cpp(self, entry)
    }
    fn remove_represented_feign_death_if_needed_like_cpp(&mut self) -> bool {
        WorldSession::remove_represented_feign_death_if_needed_like_cpp(self)
    }
    fn commit_represented_talent_reset_like_cpp(
        &mut self,
        no_reset_talent_cost: bool,
    ) -> impl Future<Output = Option<CommittedRepresentedTalentResetLikeCpp>> + Send {
        WorldSession::commit_represented_talent_reset_like_cpp(self, no_reset_talent_cost)
    }
    fn publish_committed_represented_talent_reset_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        committed: CommittedRepresentedTalentResetLikeCpp,
        request: RepresentedConfirmRespecWipeLikeCpp,
        visual_spell_id: u32,
    ) -> impl Future<Output = ()> + Send {
        WorldSession::publish_committed_represented_talent_reset_like_cpp(
            self,
            item_guid_generator,
            committed,
            request,
            visual_spell_id,
        )
    }
}
