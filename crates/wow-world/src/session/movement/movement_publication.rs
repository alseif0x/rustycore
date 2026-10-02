//! Movement packets and updates published to the client and observers.
//!
//! Moved out of the Session root under #603. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn send_represented_capture_point_removed_from_last_update_like_cpp(
        &mut self,
    ) -> usize {
        let Some((map_id, instance_id, update_generation, removable_guids)) = self
            .visible_gameobject_guids_from_last_update_summary_like_cpp(|summary| {
                summary
                    .generic_capture_point_removed_guids
                    .as_slice()
                    .to_vec()
            })
        else {
            return 0;
        };

        let mut seen = std::collections::HashSet::new();
        let mut sent = 0;
        for guid in removable_guids {
            if !seen.insert(guid) || !self.core.client_visible_guids_like_cpp.contains(&guid) {
                continue;
            }
            if !self
                .visibility
                .represented_capture_point_removed_delivered_like_cpp
                .insert((map_id, instance_id, update_generation, guid))
            {
                continue;
            }
            crate::session::hub_mut(self).send_represented_capture_point_removed_like_cpp(guid);
            sent += 1;
        }
        sent
    }
}
