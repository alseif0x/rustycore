//! Ordered account appearance and illusion loading and save projections.

use super::*;
use crate::Player;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountItemAppearanceSavePlanLikeCpp {
    pub appearance_blocks: Vec<(u32, u32)>,
    pub favorite_inserts: Vec<u32>,
    pub favorite_deletes: Vec<u32>,
}

impl AccountItemAppearanceSavePlanLikeCpp {
    pub fn is_empty(&self) -> bool {
        self.appearance_blocks.is_empty()
            && self.favorite_inserts.is_empty()
            && self.favorite_deletes.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountTransmogIllusionSavePlanLikeCpp {
    pub illusion_blocks: Vec<(u32, u32)>,
}

impl AccountTransmogIllusionSavePlanLikeCpp {
    pub fn is_empty(&self) -> bool {
        self.illusion_blocks.is_empty()
    }
}

pub const DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP: [u32; 7] = [
    3,  // Lifestealing
    13, // Crusader
    22, // Striking
    23, // Agility
    34, // Hide Weapon Enchant
    43, // Beastslayer
    44, // Titanguard
];

impl PlayerCollectionStateLikeCpp {
    pub fn prepare_appearance_blocks(
        known_appearance_blocks: impl IntoIterator<Item = (u32, u32)>,
    ) -> (BTreeMap<u32, u32>, HashSet<u32>, Vec<u32>, Option<u32>) {
        let mut blocks = BTreeMap::new();
        for (block_index, appearance_mask) in known_appearance_blocks {
            if appearance_mask != 0 {
                blocks.insert(block_index, appearance_mask);
            }
        }
        let mut item_appearances = HashSet::new();
        for (&block_index, &appearance_mask) in &blocks {
            for bit_index in 0..32 {
                if (appearance_mask & (1_u32 << bit_index)) != 0 {
                    item_appearances.insert(block_index * 32 + bit_index);
                }
            }
        }
        let mut item_appearance_blocks = Vec::new();
        let highest_block = blocks.iter().next_back().map(|(&index, _)| index);
        if let Some(highest_block) = highest_block {
            item_appearance_blocks = vec![0; highest_block as usize + 1];
            for (&block_index, &appearance_mask) in &blocks {
                item_appearance_blocks[block_index as usize] = appearance_mask;
            }
        }
        (
            blocks,
            item_appearances,
            item_appearance_blocks,
            highest_block,
        )
    }

    pub fn apply_loaded_appearance_fields(
        player: &mut Player,
        highest_block: u32,
        blocks: &BTreeMap<u32, u32>,
    ) {
        while player.transmog_blocks_like_cpp().len() <= highest_block as usize {
            player.add_transmog_block_like_cpp(0);
        }
        for (&block_index, &appearance_mask) in blocks {
            if appearance_mask != 0 {
                player.add_transmog_flag_like_cpp(block_index as usize, appearance_mask);
            }
        }
    }

    pub fn loaded_appearance_favorites(
        favorite_appearances: impl IntoIterator<Item = u32>,
    ) -> HashMap<u32, PlayerFavoriteAppearanceStateLikeCpp> {
        favorite_appearances
            .into_iter()
            .map(|appearance| (appearance, PlayerFavoriteAppearanceStateLikeCpp::Unchanged))
            .collect()
    }

    pub fn active_appearance_blocks(
        &self,
        canonical_blocks: impl FnOnce() -> Option<Vec<u32>>,
    ) -> Vec<u32> {
        if !self.item_appearance_blocks_like_cpp().is_empty() {
            return self.item_appearance_blocks_snapshot_like_cpp();
        }
        if let Some(blocks) = canonical_blocks() {
            return blocks;
        }
        let Some(highest_appearance) = self.item_appearances_like_cpp().iter().max() else {
            return Vec::new();
        };
        let mut blocks = vec![0_u32; (highest_appearance / 32 + 1) as usize];
        for &item_modified_appearance_id in self.item_appearances_like_cpp() {
            let block_index = (item_modified_appearance_id / 32) as usize;
            let bit_index = item_modified_appearance_id % 32;
            if let Some(flag) = 1_u32.checked_shl(bit_index) {
                blocks[block_index] |= flag;
            }
        }
        blocks
    }

    pub fn appearance_save_plan(&mut self) -> AccountItemAppearanceSavePlanLikeCpp {
        let mut blocks = BTreeMap::<u32, u32>::new();
        for &item_modified_appearance_id in self.item_appearances_like_cpp() {
            let block_index = item_modified_appearance_id / 32;
            let bit_index = item_modified_appearance_id % 32;
            if let Some(flag) = 1_u32.checked_shl(bit_index) {
                *blocks.entry(block_index).or_default() |= flag;
            }
        }
        let (favorite_inserts, favorite_deletes) =
            self.settle_favorite_item_appearance_saves_like_cpp();
        AccountItemAppearanceSavePlanLikeCpp {
            appearance_blocks: blocks
                .into_iter()
                .filter(|(_, appearance_mask)| *appearance_mask != 0)
                .collect(),
            favorite_inserts,
            favorite_deletes,
        }
    }

    pub fn loaded_illusion_ids(
        known_illusion_blocks: impl IntoIterator<Item = (u32, u32)>,
    ) -> HashSet<u32> {
        let mut illusions = HashSet::new();
        for (block_index, illusion_mask) in known_illusion_blocks {
            for bit_index in 0..32 {
                if (illusion_mask & (1_u32 << bit_index)) != 0 {
                    illusions.insert(block_index * 32 + bit_index);
                }
            }
        }
        for illusion_id in DEFAULT_TRANSMOG_ILLUSIONS_LIKE_CPP {
            illusions.insert(illusion_id);
        }
        illusions
    }

    pub fn has_transmog_illusion(&self, transmog_illusion_id: u32) -> bool {
        self.transmog_illusions_like_cpp()
            .contains(&transmog_illusion_id)
    }

    pub fn illusion_save_plan(&self) -> AccountTransmogIllusionSavePlanLikeCpp {
        let mut blocks = BTreeMap::<u32, u32>::new();
        for &illusion_id in self.transmog_illusions_like_cpp() {
            let block_index = illusion_id / 32;
            let bit_index = illusion_id % 32;
            if let Some(flag) = 1_u32.checked_shl(bit_index) {
                *blocks.entry(block_index).or_default() |= flag;
            }
        }
        AccountTransmogIllusionSavePlanLikeCpp {
            illusion_blocks: blocks
                .into_iter()
                .filter(|(_, illusion_mask)| *illusion_mask != 0)
                .collect(),
        }
    }
}
