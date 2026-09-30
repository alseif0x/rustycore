//! Represented item appearance: enchantment, transmogrification and illusion state.
//!
//! Moved out of the Session root under #597. Behaviour is preserved; the
//! canonical Player remains the single owner of this state.

use super::*;

fn account_transmog_update_opcode_resolved_like_cpp() -> bool {
    <wow_packet::packets::collection::AccountTransmogUpdate as wow_packet::ServerPacket>::OPCODE
        != ServerOpcodes::UpdateCapturePoint
}

mod persistence;
mod catalog;
mod acquisition;
mod admission;
mod transitions;
mod publication;
mod criteria;
#[cfg(test)]
mod tests;

impl WorldSession {
    #[cfg_attr(not(test), allow(unused_variables))]
    pub(crate) fn record_represented_alter_appearance_like_cpp(
        &mut self,
        request: RepresentedAlterAppearanceLikeCpp,
    ) {
        #[cfg(test)]
        {
            self.represented_alter_appearance_requests_like_cpp
                .push(request);
        }
    }
    #[cfg(test)]
    pub(crate) fn represented_alter_appearance_requests_like_cpp(
        &self,
    ) -> &[RepresentedAlterAppearanceLikeCpp] {
        &self.represented_alter_appearance_requests_like_cpp
    }
}

#[cfg(any(test, feature = "test-fixtures"))]
impl WorldSession {
    pub(crate) fn appearance_owner_handle_for_test(&self) -> Option<wow_map::PlayerHandle> {
        self.player_handle_like_cpp
    }

    pub(crate) fn appearance_item_spec_class_mask_for_test(&self, item_id: u32) -> Option<u32> {
        self.item_spec_class_mask_from_overrides_like_cpp(item_id)
    }

    pub(crate) fn enable_appearance_criteria_diagnostics_for_test(&mut self) {
        self.player_item_test_fixture_like_cpp.enable_appearance_criteria_diagnostics_for_test();
    }

    pub(crate) fn appearance_criteria_events_for_test(&self) -> &[RepresentedTransmogCriteriaEvent] {
        #[cfg(test)]
        {
            return &self.represented_transmog_criteria_events;
        }
        #[cfg(not(test))]
        self.player_item_test_fixture_like_cpp.appearance_criteria_events_for_test()
    }

    pub(crate) fn clear_appearance_criteria_events_for_test(&mut self) {
        #[cfg(test)]
        self.represented_transmog_criteria_events.clear();
        #[cfg(not(test))]
        self.player_item_test_fixture_like_cpp.clear_appearance_criteria_events_for_test();
    }
}
