//! Void storage operations for the QA bot.
//!
//! Moved out of main.rs under #630. Behaviour is preserved.

use super::*;

pub(crate) const CMSG_UNLOCK_VOID_STORAGE: u16 = 0x31A2;
pub(crate) const CMSG_QUERY_VOID_STORAGE: u16 = 0x31A3;
pub(crate) const CMSG_VOID_STORAGE_TRANSFER: u16 = 0x31A4;
pub(crate) const CMSG_SWAP_VOID_ITEM: u16 = 0x31A5;
pub(crate) const SMSG_VOID_STORAGE_FAILED: u16 = 0x2DA0;
pub(crate) const SMSG_VOID_STORAGE_CONTENTS: u16 = 0x2DA1;
pub(crate) const SMSG_VOID_STORAGE_TRANSFER_CHANGES: u16 = 0x2DA2;
pub(crate) const SMSG_VOID_TRANSFER_RESULT: u16 = 0x2DA3;
pub(crate) const SMSG_VOID_ITEM_SWAP_RESPONSE: u16 = 0x2DA4;
pub(crate) const DEFAULT_VOID_STORAGE_SMOKE_ITEM_ENTRY: u32 = 2589;
pub(crate) const PLAYER_FLAGS_VOID_UNLOCKED: u32 = 0x2000_0000;
pub(crate) const VOID_STORAGE_UNLOCK_COST: u64 = 1_000_000;
pub(crate) const VOID_STORAGE_STORE_ITEM_COST: u64 = 100_000;
pub(crate) fn void_storage_login_target_ready(
    discover_runtime_guid: bool,
    target_seen: bool,
) -> bool {
    !discover_runtime_guid || target_seen
}
