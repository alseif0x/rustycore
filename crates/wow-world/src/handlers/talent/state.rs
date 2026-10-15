//! Talent handlers state definitions, part 1 of 1.
//!
//! Separated from the talent.rs root under #662. Behaviour is preserved.
//! CMSG_CONFIRM_RESPEC_WIPE moved to `wow_world_application::session_lifecycle`
//! with its constants (#1363 slice 13b).

#[cfg(test)]
use super::*;

#[cfg(test)]
pub(super) use wow_world_application::{
    AT_LOGIN_RESET_TALENTS_LIKE_CPP, UNTALENT_VISUAL_EFFECT_SPELL_ID_LIKE_CPP,
};

#[cfg(test)]
pub(super) const AT_LOGIN_RENAME_LIKE_CPP: u16 = 0x001;

#[cfg(test)]
pub(super) const AT_LOGIN_RESET_SPELLS_LIKE_CPP: u16 = 0x002;

#[cfg(test)]
impl WorldSession {
    pub(crate) async fn handle_confirm_respec_wipe(&mut self, packet: WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_confirm_respec_wipe_with_generator_like_cpp(
            generators.item.as_ref(),
            false,
            packet,
        )
        .await;
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/handlers/talent/state/test_only.rs"]
mod test_only;
