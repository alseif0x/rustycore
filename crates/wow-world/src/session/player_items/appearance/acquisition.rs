//! acquisition for the existing appearance owner.

use super::*;

impl WorldSession {
    /// Bounded C++ `CollectionMgr::AddItemAppearance`.
    pub fn add_item_appearance_like_cpp(
        &mut self,
        item_modified_appearance_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let (block_index, flag) =
            wow_entities::PlayerCollectionStateLikeCpp::permanent_appearance_flag(
                item_modified_appearance_id,
            )?;
        let had_temporary = self
            .player_collection_state_snapshot_like_cpp()?
            .has_temporary_item_appearance_like_cpp(item_modified_appearance_id);

        let result = self.mutate_canonical_player_like_cpp(|player| {
            wow_entities::PlayerCollectionStateLikeCpp::apply_permanent_appearance_fields(
                player,
                item_modified_appearance_id,
                had_temporary,
                block_index,
                flag,
            )
        })??;

        self.mutate_player_collection_state_like_cpp(|collections| {
            collections.add_item_appearance_like_cpp(item_modified_appearance_id);
        })?;
        self.update_represented_transmog_criteria_like_cpp(item_modified_appearance_id);
        Some(result)
    }
    /// Bounded C++ `CollectionMgr::AddItemAppearance(uint32, uint32)`.
    ///
    /// This resolves `ItemModifiedAppearance` by `(item_id, appearance_mod_id)` like
    /// `sDB2Manager.GetItemModifiedAppearance` and runs the represented
    /// `CanAddAppearance` item-template/proficiency/quality gate, including the
    /// learned `Player::GetWeaponProficiency` mask.
    pub fn add_item_appearance_for_item_like_cpp(
        &mut self,
        item_id: u32,
        appearance_mod_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let item_modified_appearance_id =
            self.item_modified_appearance_for_item(item_id, appearance_mod_id)?;
        if !self.can_add_item_appearance_represented_like_cpp(item_modified_appearance_id) {
            return None;
        }
        self.add_item_appearance_like_cpp(item_modified_appearance_id)
    }
    /// Bounded C++ `CollectionMgr::AddItemAppearance(Item*)`.
    pub(crate) fn add_item_appearance_for_runtime_item_like_cpp(
        &mut self,
        item: &wow_entities::Item,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        use wow_entities::{PlayerCollectionStateLikeCpp, RuntimeAppearanceRoute};

        let route = PlayerCollectionStateLikeCpp::runtime_appearance_route(
            item.is_soul_bound(),
            || {
                let item_id = item.object().entry();
                let appearance_mod_id = item.visible_appearance_mod_id(0, |appearance_id| {
                    self.item_modified_appearance_ref(appearance_id)
                });
                self.item_modified_appearance_for_item(item_id, u32::from(appearance_mod_id))
            },
            |appearance_id| self.can_add_item_appearance_represented_like_cpp(appearance_id),
            || item.is_bop_tradeable() || item.is_refundable(),
        )?;
        match route {
            RuntimeAppearanceRoute::Temporary(appearance_id) => self
                .add_temporary_item_appearance_like_cpp(appearance_id, item.object().guid()),
            RuntimeAppearanceRoute::Permanent(appearance_id) => {
                self.add_item_appearance_like_cpp(appearance_id)
            }
        }
    }
    /// Bounded represented C++ `Player::_LoadQuestStatusRewarded` item-appearance replay.
    ///
    /// This covers direct reward items plus the `GetQuestPackageItems` branch that uses
    /// `ItemTemplate::ItemSpecClassMask & Player::GetClassMask`. The mask is currently represented
    /// only through `ItemSpecOverride.db2`, matching C++'s primary `ObjectMgr::LoadItemTemplates`
    /// branch. The C++ fallback that derives `ItemSpecClassMask` from `ItemSpecStats` is intentionally
    /// not guessed here because Rust does not yet represent the sparse stat-modifier bonus fields.
    pub fn replay_rewarded_quest_item_appearances_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let last_update = self.replay_rewarded_quest_direct_item_appearances_like_cpp(quest);
        let player_class_mask =
            player_class_mask_for_transmog_like_cpp(self.player_class_like_cpp());
        let package_item_ids = self
            .quests
            .package_item_store
            .as_ref()
            .map(|store| {
                store
                    .quest_package_items_like_cpp(quest.quest_package_id)
                    .filter_map(|item| u32::try_from(item.item_id).ok())
                    .filter(|item_id| *item_id != 0)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        wow_entities::PlayerCollectionStateLikeCpp::replay_package_appearances(
            self,
            package_item_ids,
            player_class_mask,
            last_update,
        )
    }
    /// Bounded direct-item part of C++ `Player::_LoadQuestStatusRewarded`.
    ///
    /// C++ replays reward choice item ids and fixed reward item ids through
    /// `CollectionMgr::AddItemAppearance(itemId)`. Use
    /// `replay_rewarded_quest_item_appearances_like_cpp` for the combined direct + represented
    /// quest-package replay.
    pub fn replay_rewarded_quest_direct_item_appearances_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let reward_item_ids =
            wow_entities::PlayerCollectionStateLikeCpp::reward_appearance_item_ids(
                quest.reward_choice_items.iter().map(|(item_id, _quantity)| *item_id),
                quest.reward_items.iter().copied(),
            );
        wow_entities::PlayerCollectionStateLikeCpp::collect_item_appearance_updates(
            self,
            reward_item_ids,
        )
    }
    /// C++ `CollectionMgr::IsSetCompleted`.
    pub fn is_transmog_set_completed_like_cpp(&self, transmog_set_id: u32) -> bool {
        wow_entities::PlayerCollectionStateLikeCpp::transmog_set_complete(
            self.transmog_set_items_like_cpp(transmog_set_id).map(|items| {
                items.iter().map(|item| item.item_modified_appearance_id)
            }),
            |appearance_id| {
                self.items.modified_appearance_store.as_ref()
                    .and_then(|store| store.get(appearance_id))
                    .map(|appearance| appearance.item_id)
            },
            |item_id| self.items.store.as_ref()
                .and_then(|store| store.inventory_type(item_id)),
            |appearance_id| self.has_item_appearance_like_cpp(appearance_id),
        )
    }
    /// Bounded C++ `CollectionMgr::AddTransmogSet`.
    pub fn add_transmog_set_like_cpp(
        &mut self,
        transmog_set_id: u32,
    ) -> Option<wow_entities::PlayerValuesUpdate> {
        let appearance_ids = self
            .transmog_set_item_modified_appearances_like_cpp(transmog_set_id)
            .into_iter()
            .map(|appearance| appearance.id)
            .collect::<Vec<_>>();

        wow_entities::PlayerCollectionStateLikeCpp::collect_appearance_updates(
            self,
            appearance_ids,
        )
    }
}

impl wow_entities::AppearanceAcquisitionSource for WorldSession {
    type Update = wow_entities::PlayerValuesUpdate;

    fn item_specialization_class_mask(&self, item_id: u32) -> Option<u32> {
        self.item_spec_class_mask_from_overrides_like_cpp(item_id)
    }

    fn acquire_item_appearance(&mut self, item_id: u32) -> Option<Self::Update> {
        self.add_item_appearance_for_item_like_cpp(item_id, 0)
    }

    fn acquire_permanent_appearance(&mut self, appearance_id: u32) -> Option<Self::Update> {
        self.add_item_appearance_like_cpp(appearance_id)
    }
}
