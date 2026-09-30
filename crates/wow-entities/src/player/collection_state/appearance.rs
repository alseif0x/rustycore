//! Appearance ownership queries and Player update-field operations.

use super::*;
use crate::{Player, PlayerValuesUpdate};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeAppearanceRoute {
    Permanent(u32),
    Temporary(u32),
}

impl PlayerCollectionStateLikeCpp {
    pub fn runtime_appearance_route(
        soul_bound: bool,
        resolve: impl FnOnce() -> Option<u32>,
        admit: impl FnOnce(u32) -> bool,
        temporary: impl FnOnce() -> bool,
    ) -> Option<RuntimeAppearanceRoute> {
        if !soul_bound {
            return None;
        }
        let appearance_id = resolve()?;
        if !admit(appearance_id) {
            return None;
        }
        Some(if temporary() {
            RuntimeAppearanceRoute::Temporary(appearance_id)
        } else {
            RuntimeAppearanceRoute::Permanent(appearance_id)
        })
    }

    pub fn permanent_appearance_flag(appearance_id: u32) -> Option<(usize, u32)> {
        let block_index = usize::try_from(appearance_id / 32).ok()?;
        let bit_index = appearance_id % 32;
        let flag = 1_u32.checked_shl(bit_index)?;
        Some((block_index, flag))
    }

    pub fn has_appearance(&self, item_modified_appearance_id: u32) -> (bool, bool) {
        if self
            .item_appearances_like_cpp()
            .contains(&item_modified_appearance_id)
        {
            return (true, false);
        }
        if self.has_temporary_item_appearance_like_cpp(item_modified_appearance_id) {
            return (true, true);
        }
        (false, false)
    }

    pub fn temporary_appearance_providers(
        &self,
        item_modified_appearance_id: u32,
    ) -> HashSet<ObjectGuid> {
        self.temporary_item_appearances_like_cpp()
            .get(&item_modified_appearance_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn apply_permanent_appearance_fields(
        player: &mut Player,
        item_modified_appearance_id: u32,
        had_temporary: bool,
        block_index: usize,
        flag: u32,
    ) -> Option<PlayerValuesUpdate> {
        while player.transmog_blocks_like_cpp().len() <= block_index {
            player.add_transmog_block_like_cpp(0);
        }
        let added_flag = player.add_transmog_flag_like_cpp(block_index, flag);
        if had_temporary {
            player.remove_conditional_transmog_like_cpp(item_modified_appearance_id);
        }
        added_flag.then(|| player.values_update(true))
    }

    pub fn apply_temporary_appearance_fields(
        player: &mut Player,
        appearance_id: u32,
    ) -> PlayerValuesUpdate {
        player.add_conditional_transmog_like_cpp(appearance_id);
        player.values_update(true)
    }

    pub fn remove_temporary_appearance_fields(
        player: &mut Player,
        appearance_id: u32,
    ) -> PlayerValuesUpdate {
        player.remove_conditional_transmog_like_cpp(appearance_id);
        player.values_update(true)
    }
}
