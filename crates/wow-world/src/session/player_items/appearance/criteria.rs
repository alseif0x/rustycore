//! criteria for the existing appearance owner.

use super::*;

impl WorldSession {
    /// C++ `CollectionMgr::AddItemAppearance` criteria side effects.
    pub(super) fn update_represented_transmog_criteria_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
    ) {
        let item_id = self
            .items
            .modified_appearance_store
            .as_ref()
            .and_then(|store| store.get(item_modified_appearance_id))
            .and_then(|appearance| u32::try_from(appearance.item_id).ok());

        if let Some(_transmog_slot) = item_id
            .and_then(|item_id| {
                self.items
                    .store
                    .as_ref()
                    .and_then(|store| store.inventory_type(item_id))
            })
            .and_then(wow_entities::item_transmogrification_slot_like_cpp)
        {
            #[cfg(test)]
            self.represented_transmog_criteria_events.push(
                RepresentedTransmogCriteriaEvent::LearnAnyTransmogInSlot {
                    equipment_slot: _transmog_slot as u32,
                    item_modified_appearance_id,
                },
            );
            #[cfg(all(not(test), feature = "test-fixtures"))]
            self.player_item_test_fixture_like_cpp.record_appearance_criteria_for_test(
                RepresentedTransmogCriteriaEvent::LearnAnyTransmogInSlot {
                    equipment_slot: _transmog_slot as u32,
                    item_modified_appearance_id,
                },
            );
        }

        let transmog_sets = self
            .transmog_sets_for_item_modified_appearance_like_cpp(item_modified_appearance_id)
            .map(|sets| {
                sets.iter()
                    .map(|set| (set.id, set.transmog_set_group_id))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        for (transmog_set_id, _transmog_set_group_id) in transmog_sets {
            if self.is_transmog_set_completed_like_cpp(transmog_set_id) {
                #[cfg(test)]
                self.represented_transmog_criteria_events.push(
                    RepresentedTransmogCriteriaEvent::CollectTransmogSetFromGroup {
                        transmog_set_group_id: _transmog_set_group_id,
                    },
                );
                #[cfg(all(not(test), feature = "test-fixtures"))]
                self.player_item_test_fixture_like_cpp.record_appearance_criteria_for_test(
                    RepresentedTransmogCriteriaEvent::CollectTransmogSetFromGroup {
                        transmog_set_group_id: _transmog_set_group_id,
                    },
                );
            }
        }
    }
}
