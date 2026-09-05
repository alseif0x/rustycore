//! Battle.net collection load/save projections with the existing independent transaction boundaries.
//! Mechanical relocation from lib.rs in #578; public crate-root paths are retained.

use crate::LogicalDatabaseLikeCpp;

/// One account-collection read requested by the Player login lifecycle.
///
/// C++ prepares these Login-database reads in `AccountInfoQueryHolder` and
/// passes their rows to `CollectionMgr`. The request names the business
/// collection only; statement identity and row decoding remain adapter work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountCollectionLoadRequestLikeCpp {
    Mounts { bnet_account_id: u32 },
    Toys { bnet_account_id: u32 },
    Heirlooms { bnet_account_id: u32 },
    ItemAppearances { bnet_account_id: u32 },
    TransmogIllusions { bnet_account_id: u32 },
}

impl AccountCollectionLoadRequestLikeCpp {
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Login
    }
}

/// Raw semantic rows returned by one account-collection read.
///
/// Signed identifiers deliberately stay signed here. Existing gameplay owns
/// the C++-faithful validation and must be able to distinguish malformed rows
/// rather than receiving a value fabricated by the database adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountCollectionLoadedLikeCpp {
    Mounts(Vec<AccountMountLoadRowLikeCpp>),
    Toys(Vec<AccountToyLoadRowLikeCpp>),
    Heirlooms(Vec<AccountHeirloomLoadRowLikeCpp>),
    ItemAppearances {
        appearance_blocks: AccountCollectionRowsLikeCpp<Vec<AccountMaskBlockLikeCpp>>,
        favorite_appearance_ids: AccountCollectionRowsLikeCpp<Vec<u32>>,
    },
    TransmogIllusions {
        illusion_blocks: Vec<AccountMaskBlockLikeCpp>,
    },
}

/// Result of one physical read inside a semantic collection load. Item
/// appearances use two independent C++ queries and preserve partial success.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountCollectionRowsLikeCpp<T> {
    Loaded(T),
    Failed { reason: String },
}

/// A read has no indeterminate COMMIT state: it either produced typed rows or
/// failed. Callers preserve their existing fail-closed publication behavior.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountCollectionLoadOutcomeLikeCpp {
    Loaded(AccountCollectionLoadedLikeCpp),
    Failed { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountMountLoadRowLikeCpp {
    pub mount_spell_id: i32,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountToyLoadRowLikeCpp {
    pub item_id: i32,
    pub is_favorite: bool,
    pub has_fanfare: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountHeirloomLoadRowLikeCpp {
    pub item_id: i32,
    pub flags: u32,
}

/// One row of an account-wide collection, ready to persist.
///
/// These are Battle.net account collections, not character state: C++ writes
/// them to the Login database during logout, each collection in its own
/// transaction. The five-transaction shape is preserved deliberately — #187
/// records that C++ appends them to one transaction and Rust does not, and
/// changing that is a behaviour fix with its own evidence, not something to
/// fold into an architecture move.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountCollectionSaveLikeCpp {
    Mounts(Vec<AccountMountRowLikeCpp>),
    Toys(Vec<AccountToyRowLikeCpp>),
    Heirlooms(Vec<AccountHeirloomRowLikeCpp>),
    /// Appearances are stored as packed masks per block, with the favourite
    /// list maintained by explicit inserts and deletes. Insert order before
    /// delete order is preserved: they share one transaction and a delete that
    /// overtook its insert would drop a favourite the client still shows.
    ItemAppearances {
        bnet_account_id: u32,
        appearance_blocks: Vec<AccountMaskBlockLikeCpp>,
        favorite_inserts: Vec<u32>,
        favorite_deletes: Vec<u32>,
    },
    TransmogIllusions {
        bnet_account_id: u32,
        illusion_blocks: Vec<AccountMaskBlockLikeCpp>,
    },
}

/// One packed bitmask block of an account-wide collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountMaskBlockLikeCpp {
    pub block_index: u32,
    pub mask: u32,
}

impl AccountCollectionSaveLikeCpp {
    /// True when there is nothing to write. The caller skips the transaction
    /// rather than opening an empty one.
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Mounts(rows) => rows.is_empty(),
            Self::Toys(rows) => rows.is_empty(),
            Self::Heirlooms(rows) => rows.is_empty(),
            Self::ItemAppearances {
                appearance_blocks,
                favorite_inserts,
                favorite_deletes,
                ..
            } => {
                appearance_blocks.is_empty()
                    && favorite_inserts.is_empty()
                    && favorite_deletes.is_empty()
            }
            Self::TransmogIllusions {
                illusion_blocks, ..
            } => illusion_blocks.is_empty(),
        }
    }

    /// Account collections live in the Login database.
    pub fn logical_database(&self) -> LogicalDatabaseLikeCpp {
        LogicalDatabaseLikeCpp::Login
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountMountRowLikeCpp {
    pub bnet_account_id: u32,
    pub mount_spell_id: u32,
    pub flags: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountToyRowLikeCpp {
    pub bnet_account_id: u32,
    pub item_id: u32,
    pub is_favorite: bool,
    pub has_fanfare: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccountHeirloomRowLikeCpp {
    pub bnet_account_id: u32,
    pub item_id: u32,
    pub flags: u32,
}
