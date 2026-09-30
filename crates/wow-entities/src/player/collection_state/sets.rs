//! Ordered transmog-set completion and acquisition folds.

use super::*;
use wow_constants::InventoryType;
use crate::{EQUIPMENT_SLOT_END, item_transmogrification_slot_like_cpp};

pub trait AppearanceAcquisitionSource {
    type Update;
    fn item_specialization_class_mask(&self, item_id: u32) -> Option<u32>;
    fn acquire_item_appearance(&mut self, item_id: u32) -> Option<Self::Update>;
    fn acquire_permanent_appearance(&mut self, appearance_id: u32) -> Option<Self::Update>;
}

impl PlayerCollectionStateLikeCpp {
    pub fn transmog_set_complete<I: IntoIterator<Item = u32>>(
        transmog_set_items: Option<I>,
        modified_item: impl Fn(u32) -> Option<i32>,
        inventory_type: impl Fn(u32) -> Option<InventoryType>,
        has_appearance: impl Fn(u32) -> (bool, bool),
    ) -> bool {
        let Some(transmog_set_items) = transmog_set_items else {
            return false;
        };
        let mut known_pieces = [-1_i8; EQUIPMENT_SLOT_END as usize];
        for item_modified_appearance_id in transmog_set_items {
            let Some(item_id) = modified_item(item_modified_appearance_id) else {
                continue;
            };
            let Some(item_id) = u32::try_from(item_id).ok() else {
                continue;
            };
            let Some(inventory_type) = inventory_type(item_id) else {
                continue;
            };
            let Some(transmog_slot) = item_transmogrification_slot_like_cpp(inventory_type) else {
                continue;
            };
            if known_pieces[transmog_slot] == 1 {
                continue;
            }
            let (has_appearance, is_temporary) = has_appearance(item_modified_appearance_id);
            known_pieces[transmog_slot] = if has_appearance && !is_temporary { 1 } else { 0 };
        }
        !known_pieces.contains(&0)
    }

    pub fn collect_appearance_updates<S: AppearanceAcquisitionSource>(
        source: &mut S,
        appearance_ids: impl IntoIterator<Item = u32>,
    ) -> Option<S::Update> {
        let mut last_update = None;
        for appearance_id in appearance_ids {
            if let Some(update) = source.acquire_permanent_appearance(appearance_id) {
                last_update = Some(update);
            }
        }
        last_update
    }

    pub fn collect_item_appearance_updates<S: AppearanceAcquisitionSource>(
        source: &mut S,
        item_ids: impl IntoIterator<Item = u32>,
    ) -> Option<S::Update> {
        let mut last_update = None;
        for item_id in item_ids {
            if let Some(update) = source.acquire_item_appearance(item_id) {
                last_update = Some(update);
            }
        }
        last_update
    }

    pub fn reward_appearance_item_ids(
        choice_items: impl Iterator<Item = u32>,
        fixed_items: impl Iterator<Item = u32>,
    ) -> Vec<u32> {
        choice_items
            .chain(fixed_items)
            .filter(|item_id| *item_id != 0)
            .collect::<Vec<_>>()
    }

    pub fn replay_package_appearances<S: AppearanceAcquisitionSource>(
        source: &mut S,
        package_item_ids: impl IntoIterator<Item = u32>,
        player_class_mask: u32,
        mut last_update: Option<S::Update>,
    ) -> Option<S::Update> {
        for item_id in package_item_ids {
            let Some(item_spec_class_mask) = source.item_specialization_class_mask(item_id) else {
                continue;
            };
            if (item_spec_class_mask & player_class_mask) == 0 {
                continue;
            }
            if let Some(update) = source.acquire_item_appearance(item_id) {
                last_update = Some(update);
            }
        }
        last_update
    }
}
