// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_entities::PlayerInventoryRuntime;

use crate::session::SessionCore;

/// Borrowed access to the session's canonical inventory owner.
pub struct OwnedInventoryAccessLikeCpp<'a> {
    core: &'a SessionCore,
}

impl SessionCore {
    /// Build a borrowed capability for the canonical inventory owner.
    pub fn owned_inventory_access_like_cpp(&self) -> OwnedInventoryAccessLikeCpp<'_> {
        OwnedInventoryAccessLikeCpp { core: self }
    }
}

impl OwnedInventoryAccessLikeCpp<'_> {
    /// Read canonical player money without applying an Inventory fixture fallback.
    pub fn player_money_like_cpp(&self) -> Option<u64> {
        self.core.with_owned_player_like_cpp(|player| player.money())
    }

    /// Set canonical player money and report whether the owned player resolved.
    pub fn set_player_money_like_cpp(&self, gold: u64) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| player.set_money(gold))
            .is_some()
    }

    /// Read canonical bank-bag slot count without applying an Inventory fixture fallback.
    pub fn player_bank_bag_slot_count_like_cpp(&self) -> Option<u8> {
        self.core
            .with_owned_player_like_cpp(|player| player.bank_bag_slot_count())
    }

    /// Set canonical bank-bag slot count and report whether the owned player resolved.
    pub fn set_player_bank_bag_slot_count_like_cpp(&self, count: u8) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| player.set_bank_bag_slot_count(count))
            .is_some()
    }

    /// Read one canonical bank-bag slot flag; an absent Player or slot is `None`.
    pub fn player_bank_bag_slot_flag_like_cpp(&self, slot: usize) -> Option<u32> {
        self.core
            .with_owned_player_like_cpp(|player| player.bank_bag_slot_flag_value_like_cpp(slot))
            .flatten()
    }

    /// Set one canonical bank-bag slot flag and preserve the Player setter's bounds result.
    pub fn set_player_bank_bag_slot_flag_like_cpp(&self, slot: usize, value: u32) -> bool {
        self.core
            .with_owned_player_mut_like_cpp(|player| {
                player.set_bank_bag_slot_flag_value_like_cpp(slot, value)
            })
            .unwrap_or(false)
    }

    /// Read canonical inventory slot count without applying an Inventory fixture fallback.
    pub fn player_inventory_slot_count_like_cpp(&self) -> Option<u8> {
        self.core
            .with_owned_player_like_cpp(|player| player.inventory_slot_count())
    }

    /// Clone the current canonical inventory runtime while its owner is held.
    pub fn inventory_runtime_snapshot_like_cpp(&self) -> Option<PlayerInventoryRuntime> {
        self.core
            .with_owned_player_like_cpp(|player| player.inventory_runtime_like_cpp().clone())
    }

    /// Invalidate aura authority through the existing GUID/map mutation path.
    /// A missing manager or matching Player is ignored; the manager guard is
    /// released before this call returns.
    pub fn invalidate_spell_hit_aura_authority_for_inventory_mutation_like_cpp(&self) {
        self.core
            .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
    }

    /// Mutate inventory only through this session's generation-checked Player
    /// handle. A missing or stale owner returns `None` without GUID/map fallback.
    /// The synchronous callback runs under the manager guard, which is released
    /// before this method returns.
    pub fn with_inventory_runtime_mut_like_cpp<R>(
        &self,
        update: impl FnOnce(&mut PlayerInventoryRuntime) -> R,
    ) -> Option<R> {
        self.core.with_owned_player_mut_like_cpp(|player| {
            update(player.inventory_runtime_mut_like_cpp())
        })
    }

    /// Whether this fixture session has no canonical Player handle installed.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn owner_handle_absent_like_cpp(&self) -> bool {
        self.core.player_handle_like_cpp.is_none()
    }
}
