//! transitions for the existing appearance owner.

use super::*;

impl WorldSession {
    #[cfg(test)]
    pub(crate) fn represented_has_item_appearance_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> bool {
        self.represented_item_appearances_like_cpp
            .contains(&item_modified_appearance_id)
    }
    /// C++ `CollectionMgr::HasItemAppearance`.
    pub fn has_item_appearance_like_cpp(&self, item_modified_appearance_id: u32) -> (bool, bool) {
        let Some(collections) = self.player_collection_state_snapshot_like_cpp() else {
            return (false, false);
        };
        collections.has_appearance(item_modified_appearance_id)
    }
    /// C++ `CollectionMgr::SetAppearanceIsFavorite`.
    pub fn set_appearance_is_favorite_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
        apply: bool,
    ) -> bool {
        let changed = self
            .mutate_player_collection_state_like_cpp(|collections| {
                collections.set_appearance_favorite(item_modified_appearance_id, apply)
            })
            .unwrap_or(false);

        if changed {
            if !account_transmog_update_opcode_resolved_like_cpp() {
                warn!(
                    "Skipping AccountTransmogUpdate favorite delta: legacy C++ opcode is unresolved 0xBADD for 54261"
                );
                return changed;
            }

            self.send_packet(
                &wow_packet::packets::collection::AccountTransmogUpdate::favorite_delta(
                    item_modified_appearance_id,
                    apply,
                ),
            );
        }

        changed
    }
    #[cfg(test)]
    pub(crate) fn represented_favorite_item_appearance_state_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> Option<FavoriteAppearanceStateLikeCpp> {
        self.represented_favorite_item_appearances_like_cpp
            .get(&item_modified_appearance_id)
            .copied()
    }
    /// C++ `CollectionMgr::AddTemporaryAppearance`.
    pub fn add_temporary_item_appearance_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let was_empty = self.mutate_player_collection_state_like_cpp(|collections| {
            collections
                .add_temporary_item_appearance_like_cpp(item_modified_appearance_id, item_guid)
        })?;

        was_empty.then_some(())?;
        self.mutate_canonical_player_like_cpp(|player| {
            wow_entities::PlayerCollectionStateLikeCpp::apply_temporary_appearance_fields(
                player,
                item_modified_appearance_id,
            )
        })
    }
    /// C++ `CollectionMgr::RemoveTemporaryAppearance`.
    pub fn remove_temporary_item_appearance_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
        item_guid: ObjectGuid,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let removed_last = self.mutate_player_collection_state_like_cpp(|collections| {
            collections
                .remove_temporary_item_appearance_like_cpp(item_modified_appearance_id, item_guid)
                .then_some(())
        })??;
        let _ = removed_last;
        self.mutate_canonical_player_like_cpp(|player| {
            wow_entities::PlayerCollectionStateLikeCpp::remove_temporary_appearance_fields(
                player,
                item_modified_appearance_id,
            )
        })
    }
    /// C++ `CollectionMgr::GetItemsProvidingTemporaryAppearance`.
    pub fn items_providing_temporary_appearance_like_cpp(
        &self,
        item_modified_appearance_id: u32,
    ) -> HashSet<ObjectGuid> {
        self.player_collection_state_snapshot_like_cpp()
            .and_then(|collections| {
                Some(collections.temporary_appearance_providers(item_modified_appearance_id))
            })
            .unwrap_or_default()
    }
}
