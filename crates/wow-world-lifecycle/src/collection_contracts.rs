#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountMountSaveRowLikeCpp {
    pub bnet_account_id: u32,
    pub mount_spell_id: u32,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountToySaveRowLikeCpp {
    pub bnet_account_id: u32,
    pub item_id: u32,
    pub is_favorite: bool,
    pub has_fanfare: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountHeirloomSaveRowLikeCpp {
    pub bnet_account_id: u32,
    pub item_id: u32,
    pub flags: u32,
}
