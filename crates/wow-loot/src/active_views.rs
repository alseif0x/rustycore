//! Connection-resident loot views, independent of packets and persistence.
//!
//! Source anchors: Player::SendLoot/GetLootByWorldObjectGUID and
//! WorldSession::DoLootRelease in the target C++ core. Object-owned loot
//! remains in OwnedLootAuthority; these bindings identify the opened lifetime.

use crate::{LootClaimLease, OwnedLootAuthority};
use std::collections::{HashMap, HashSet};
use wow_core::ObjectGuid;

/// Open loot views and their exact backing allocations and generations.
pub struct LootViews {
    /// Mirrors C++ PlayerData::LootTargetGUID for guards that compare active loot by GUID.
    primary_guid: ObjectGuid,
    /// Represented owner GUIDs currently visible through C++ `Player::m_AELootView`.
    owners: HashSet<ObjectGuid>,
    /// Object-owned generation that was actually opened for each active loot view.
    ///
    /// GUIDs are reused across creature respawns and gameobject restocks. A delayed
    /// packet from an older window must therefore not be authorized merely because
    /// the replacement lifetime has the same owner/loot GUID and player eligibility.
    generations: HashMap<ObjectGuid, u64>,
    /// Exact backing allocation opened for each view. Scope epochs restart at
    /// one in a newly allocated authority, so the generation map alone cannot
    /// prevent ABA when a creature GUID is recreated.
    authorities: HashMap<ObjectGuid, OwnedLootAuthority>,
}

impl Default for LootViews {
    fn default() -> Self {
        Self {
            primary_guid: ObjectGuid::EMPTY,
            owners: HashSet::new(),
            generations: HashMap::new(),
            authorities: HashMap::new(),
        }
    }
}

impl LootViews {
    pub fn primary_guid(&self) -> ObjectGuid {
        self.primary_guid
    }

    pub fn set_primary(&mut self, guid: ObjectGuid) {
        self.primary_guid = ObjectGuid::EMPTY;
        self.owners.clear();
        self.generations.clear();
        self.authorities.clear();
        self.add_owner(guid);
    }

    pub fn has_views(&self) -> bool {
        !self.primary_guid.is_empty() || !self.owners.is_empty()
    }

    pub fn add_owner(&mut self, guid: ObjectGuid) {
        if guid.is_empty() {
            return;
        }
        if self.primary_guid.is_empty() {
            self.primary_guid = guid;
        }
        self.owners.insert(guid);
    }

    pub fn record_generation(&mut self, guid: ObjectGuid, generation: u64) {
        self.generations.insert(guid, generation);
    }

    pub fn remove_owner(&mut self, guid: ObjectGuid) {
        self.owners.remove(&guid);
        self.generations.remove(&guid);
        self.authorities.remove(&guid);
        if self.primary_guid == guid {
            self.primary_guid = ObjectGuid::EMPTY;
        }
    }

    pub fn is_primary(&self, guid: ObjectGuid) -> bool {
        !guid.is_empty() && self.primary_guid == guid
    }

    pub fn contains_owner(&self, guid: &ObjectGuid) -> bool {
        self.owners.contains(guid)
    }

    pub fn has_owners(&self) -> bool {
        !self.owners.is_empty()
    }

    pub fn owner_count(&self) -> usize {
        self.owners.len()
    }

    pub fn owners(&self) -> impl Iterator<Item = &ObjectGuid> {
        self.owners.iter()
    }

    pub fn owners_snapshot(&self) -> HashSet<ObjectGuid> {
        self.owners.clone()
    }

    /// Retains HashSet iteration order and the primary fallback, including EMPTY.
    pub fn owner_selection(&self) -> Vec<ObjectGuid> {
        if self.owners.is_empty() {
            vec![self.primary_guid]
        } else {
            self.owners.iter().copied().collect()
        }
    }

    pub fn has_non_item_views(&self) -> bool {
        (!self.primary_guid.is_empty() && !self.primary_guid.is_item())
            || self.owners.iter().any(|guid| !guid.is_item())
    }

    pub fn generation(&self, guid: &ObjectGuid) -> Option<u64> {
        self.generations.get(guid).copied()
    }

    pub fn authority(&self, guid: &ObjectGuid) -> Option<&OwnedLootAuthority> {
        self.authorities.get(guid)
    }

    pub fn authority_bindings(&self) -> impl Iterator<Item = (&ObjectGuid, &OwnedLootAuthority)> {
        self.authorities.iter()
    }

    pub fn bound_authorities(&self) -> impl Iterator<Item = &OwnedLootAuthority> {
        self.authorities.values()
    }

    /// Called only at the original publication point after response admission.
    pub fn bind_opened(
        &mut self,
        guid: ObjectGuid,
        generation: u64,
        authority: &OwnedLootAuthority,
    ) {
        self.generations.insert(guid, generation);
        self.authorities.insert(guid, authority.clone());
    }

    pub fn bootstrap_binding(
        &mut self,
        guid: ObjectGuid,
        generation: u64,
        authority: &OwnedLootAuthority,
    ) {
        self.generations.entry(guid).or_insert(generation);
        self.authorities
            .entry(guid)
            .or_insert_with(|| authority.clone());
    }

    pub fn bind_authority_if_changed(&mut self, guid: ObjectGuid, authority: &OwnedLootAuthority) {
        if !self
            .authorities
            .get(&guid)
            .is_some_and(|opened| opened.shares_storage_like_cpp(authority))
        {
            self.authorities.insert(guid, authority.clone());
        }
    }

    /// The caller samples current generation before invoking this comparison.
    /// No authority state is queried here, including during publication.
    pub fn matches_authority(
        &self,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        current_generation: Option<u64>,
    ) -> bool {
        self.authorities
            .get(&guid)
            .is_some_and(|opened| opened.shares_storage_like_cpp(authority))
            && self
                .generations
                .get(&guid)
                .is_some_and(|opened| Some(*opened) == current_generation)
    }

    pub fn matches_claim(&self, guid: ObjectGuid, claim: &LootClaimLease) -> bool {
        self.authorities
            .get(&guid)
            .is_some_and(|opened| claim.shares_authority_like_cpp(opened))
            && self
                .generations
                .get(&guid)
                .is_some_and(|opened| *opened == claim.generation_like_cpp())
    }
}

#[cfg(test)]
mod tests;
