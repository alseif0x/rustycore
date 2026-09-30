//! Heirloom calculations from borrowed DB2 columns and lazy row selectors.
//!
//! C++ a5f8da2e, CollectionMgr.cpp:174–214, :243–329.
//! The application retains its original catalog lookup, snapshot, active-field
//! writes and subsequent collection replacement. Cyclic catalog chains remain
//! the existing gap; this extraction adds no cycle handling.

use super::PlayerCollectionStateLikeCpp;

impl PlayerCollectionStateLikeCpp {
    pub fn heirloom_bonus_for_flags(
        upgrade_item_id: &[i32; 6],
        upgrade_item_bonus_list_id: &[u16; 6],
        flags: u32,
    ) -> u32 {
        for upgrade_level in (0..upgrade_item_id.len()).rev() {
            if flags & (1_u32 << upgrade_level) != 0 {
                return u32::from(upgrade_item_bonus_list_id[upgrade_level]);
            }
        }

        0
    }

    pub fn heirloom_upgrade(
        upgrade_item_id: &[i32; 6],
        upgrade_item_bonus_list_id: &[u16; 6],
        current_flags: u32,
        cast_item: i32,
    ) -> (u32, u32) {
        let mut flags = current_flags;
        let mut bonus_id = 0_u32;
        for (upgrade_level, &upgrade_item_id) in upgrade_item_id.iter().enumerate() {
            if upgrade_item_id == cast_item {
                flags |= 1_u32 << upgrade_level;
                bonus_id = u32::from(upgrade_item_bonus_list_id[upgrade_level]);
            }
        }
        (flags, bonus_id)
    }

    pub fn static_heirloom_upgrade<'a, E: 'a>(
        static_upgraded_item_id: i32,
        mut lookup: impl FnMut(u32) -> Option<&'a E>,
        item_id: impl Fn(&E) -> i32,
        static_upgrade: impl Fn(&E) -> i32,
        mut contains_item: impl FnMut(u32) -> bool,
    ) -> Option<u32> {
        let mut heirloom_item_id = u32::try_from(static_upgraded_item_id).ok()?;
        let mut new_item_id = 0_u32;
        while let Some(heirloom_diff) = lookup(heirloom_item_id) {
            let diff_item_id = u32::try_from(item_id(heirloom_diff)).ok()?;
            if contains_item(diff_item_id) {
                new_item_id = diff_item_id;
            }

            let Some(heirloom_sub_item_id) = u32::try_from(static_upgrade(heirloom_diff))
                .ok()
                .and_then(|static_item_id| {
                    lookup(static_item_id)
                        .and_then(|heirloom_sub| u32::try_from(item_id(heirloom_sub)).ok())
                })
            else {
                break;
            };
            heirloom_item_id = heirloom_sub_item_id;
        }

        if new_item_id == 0 {
            return None;
        }
        Some(new_item_id)
    }
}

#[cfg(test)]
mod tests;
