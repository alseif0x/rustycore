// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{LootReleaseAccessLikeCpp, LootReleaseOwnerAccessLikeCpp};
use wow_core::ObjectGuid;
use wow_loot::{OwnedLootAuthority, OwnedLootAuthorityLifecycle, OwnedLootAuthorityStamp};
use wow_map::MapKey;

/// Typed result of resolving one owner's object-owned loot authority.
///
/// F6-7 R2: the lookup resolves through the **one designated owner** of the
/// GUID and reports three distinct facts. None of them is inferred from an
/// unreadable store, and no `Option` collapse exists any more:
///
/// * `Found` — the designated, validated incarnation supplied this allocation.
///   A surviving mirror in the other store never confers authority.
/// * `Absent` — the designated owner was addressed and **proved** the object
///   absent, and no surviving representation of it exists in the other store.
/// * `Unavailable` — the designated owner could not be read or validated: no
///   manager, a failed lock, an unresolved residence, a missing map instance,
///   an unadmitted representation, an allocation belonging to another
///   incarnation, or a quarantined/detached allocation. This is not absent loot
///   and must not be answered like it (F6-7 R4).
#[derive(Debug, Clone)]
pub enum OwnedLootAuthorityLookupOutcomeLikeCpp {
    Found(OwnedLootAuthority),
    Absent,
    Unavailable,
}

/// Outcome of addressing one designated loot store for one owner GUID.
///
/// `Absent` means the object is **not in that store**, which is only ever
/// reported after the store's own addressing was resolved: an existing manager
/// was locked and, when the store is the canonical one, its map instance was
/// found. `Unreadable` is the fact that must never be read as absence: a failed
/// lock, an unresolved residence, or a configured canonical store that has no
/// map instance for the resolved key.
///
/// A store that is **not configured at all** is neither: there is no other
/// owner source that could hold an allocation, so the object-owned authority
/// model has no authority for the GUID and the R4a-signed "genuine absence"
/// fact applies. That is the represented-fixture/bootstrap configuration.
enum DesignatedLootStoreReadLikeCpp {
    /// The store was addressed and holds the object with its allocation.
    Present(OwnedLootAuthority),
    /// The store holds no allocation for the object (or is not configured).
    Absent,
    /// A configured store could not be read.
    Unreadable,
}

impl LootReleaseOwnerAccessLikeCpp<'_> {
    pub fn next_canonical_loot_object_guid_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        (|| {
            let owner_map_id = u32::from(owner_guid.map_id());
            let key = self
                .core
                .canonical_object_lookup_map_key_like_cpp(owner_map_id)?;
            if key.map_id != owner_map_id {
                return None;
            }
            let manager = self.core.canonical_map_manager.as_ref()?;
            let mut manager = manager.lock().ok()?;
            let map = manager.find_map_mut(key.map_id, key.instance_id)?.map_mut();
            let counter = map
                .generate_low_guid_like_cpp(wow_core::guid::HighGuid::LootObject)
                .ok()?;
            let map_id = u16::try_from(key.map_id).ok()?;
            Some(ObjectGuid::create_world_object(
                wow_core::guid::HighGuid::LootObject,
                0,
                self.core.realm_id(),
                map_id,
                0,
                0,
                counter,
            ))
        })()
    }

    /// Resolve one owner's object-owned loot authority through its **one
    /// designated owner** with an explicit outcome (F6-7 R2).
    ///
    /// Designated owner: the canonical store whenever a canonical map manager
    /// is configured, and the legacy store in the legitimate legacy-only
    /// configuration. The other store may keep a temporary alias, but its
    /// survival never confers authority and is never read as absence.
    pub fn represented_owned_loot_authority_outcome_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> OwnedLootAuthorityLookupOutcomeLikeCpp {
        if owner_guid.is_creature_or_vehicle() {
            return self.designated_creature_loot_authority_like_cpp(owner_guid);
        }
        if owner_guid.is_game_object() {
            return self.designated_game_object_loot_authority_like_cpp(owner_guid);
        }
        // No object-owned loot kind is designated for this GUID, so no owner
        // exists at all. That is not an unreadable store.
        OwnedLootAuthorityLookupOutcomeLikeCpp::Absent
    }

    /// Creature lookup under the R2 designated-owner contract.
    fn designated_creature_loot_authority_like_cpp(
        &self,
        owner_guid: ObjectGuid,
    ) -> OwnedLootAuthorityLookupOutcomeLikeCpp {
        if self.core.canonical_map_manager.is_none() {
            // Legitimate legacy-only configuration: no canonical incarnation
            // exists for this session, so the legacy store is the designated
            // owner and its authority is the object's authority. The creature
            // path keeps working exactly as before.
            let (map_id, instance_id) = self.core.current_legacy_runtime_map_key_like_cpp();
            return match self
                .transitions_like_cpp()
                .read_designated_legacy_creature_authority_like_cpp(owner_guid, map_id, instance_id)
            {
                DesignatedLootStoreReadLikeCpp::Present(authority) => {
                    designated_creature_authority_outcome_like_cpp(authority)
                }
                DesignatedLootStoreReadLikeCpp::Absent => {
                    OwnedLootAuthorityLookupOutcomeLikeCpp::Absent
                }
                DesignatedLootStoreReadLikeCpp::Unreadable => {
                    OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable
                }
            };
        }

        // Canonical ownership controls whenever a canonical manager exists.
        let canonical_player_map_key = self.core.current_canonical_player_map_key_like_cpp();
        let Some(map_key) = canonical_player_map_key.or_else(|| {
            self.core
                .canonical_object_lookup_map_key_like_cpp(u32::from(
                    self.core.player_map_id_like_cpp(),
                ))
        }) else {
            // Unresolved residence: the designated store cannot be addressed,
            // so neither absence nor a found authority can be reported.
            return OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable;
        };
        let authority = match self
            .transitions_like_cpp()
            .read_designated_canonical_creature_authority_like_cpp(owner_guid, map_key)
        {
            DesignatedLootStoreReadLikeCpp::Present(authority) => authority,
            DesignatedLootStoreReadLikeCpp::Unreadable => {
                return OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable;
            }
            DesignatedLootStoreReadLikeCpp::Absent => {
                // The canonical incarnation does not exist. A representation
                // that was never admitted into an incarnation must not
                // confer authority (its survival is not a found authority)
                // and must not be read as proof that the object is gone
                // either.
                let (map_id, instance_id) = self.core.current_legacy_runtime_map_key_like_cpp();
                return match self
                    .transitions_like_cpp()
                    .read_designated_legacy_creature_authority_like_cpp(
                        owner_guid,
                        map_id,
                        instance_id,
                    ) {
                    DesignatedLootStoreReadLikeCpp::Absent => {
                        OwnedLootAuthorityLookupOutcomeLikeCpp::Absent
                    }
                    DesignatedLootStoreReadLikeCpp::Present(_)
                    | DesignatedLootStoreReadLikeCpp::Unreadable => {
                        OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable
                    }
                };
            }
        };
        designated_creature_authority_outcome_like_cpp(authority)
    }

    /// GameObject lookup under the R2 designated-owner contract: canonical
    /// ownership only, with the signed validity gate R8 accepted.
    fn designated_game_object_loot_authority_like_cpp(
        &self,
        owner_guid: ObjectGuid,
    ) -> OwnedLootAuthorityLookupOutcomeLikeCpp {
        if self.core.canonical_map_manager.is_none() {
            // Canonical-only ownership with no canonical store configured:
            // there is no owner source at all, which is the absence fact the
            // signed R8 canonical-only read always produced. It is not a store
            // that failed to be read.
            return OwnedLootAuthorityLookupOutcomeLikeCpp::Absent;
        }
        let canonical_player_map_key = self.core.current_canonical_player_map_key_like_cpp();
        let Some(map_key) = canonical_player_map_key.or_else(|| {
            self.core
                .canonical_object_lookup_map_key_like_cpp(u32::from(
                    self.core.player_map_id_like_cpp(),
                ))
        }) else {
            return OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable;
        };
        let authority = match self
            .transitions_like_cpp()
            .read_designated_canonical_game_object_authority_like_cpp(owner_guid, map_key)
        {
            DesignatedLootStoreReadLikeCpp::Present(authority) => authority,
            DesignatedLootStoreReadLikeCpp::Absent => {
                return OwnedLootAuthorityLookupOutcomeLikeCpp::Absent;
            }
            DesignatedLootStoreReadLikeCpp::Unreadable => {
                return OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable;
            }
        };
        // The signed validity gate stays: a map key that stopped being the key
        // this session resolves may not answer with an authority. It is no
        // longer a reconciliation recheck, and an invalid key is not proven
        // absence.
        if !self
            .transitions_like_cpp()
            .canonical_object_map_key_is_current_like_cpp(
                map_key,
                canonical_player_map_key.is_some(),
            )
        {
            return OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable;
        }
        designated_creature_authority_outcome_like_cpp(authority)
    }
}

/// Validate one designated owner's allocation before it may confer authority.
///
/// A quarantined allocation is a terminal fail-closed tombstone and a detached
/// allocation has lost its owning entity, so neither belongs to the incarnation
/// that owns the GUID. Both are unreadable ownership, not absence. A retired
/// but attached allocation is a real readable lifetime tombstone (respawn,
/// restock, consumed corpse) and is reported as `Found` unchanged.
fn designated_creature_authority_outcome_like_cpp(
    authority: OwnedLootAuthority,
) -> OwnedLootAuthorityLookupOutcomeLikeCpp {
    match authority.lifecycle_like_cpp() {
        OwnedLootAuthorityLifecycle::Detached | OwnedLootAuthorityLifecycle::Quarantined => {
            OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable
        }
        _ => OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority),
    }
}

impl LootReleaseAccessLikeCpp<'_> {
    fn read_designated_canonical_creature_authority_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: MapKey,
    ) -> DesignatedLootStoreReadLikeCpp {
        let Some(manager) = self.core.canonical_map_manager.as_ref() else {
            return DesignatedLootStoreReadLikeCpp::Unreadable;
        };
        let Ok(manager) = manager.lock() else {
            return DesignatedLootStoreReadLikeCpp::Unreadable;
        };
        let Some(managed) = manager.find_map(map_key.map_id, map_key.instance_id) else {
            return DesignatedLootStoreReadLikeCpp::Unreadable;
        };
        match managed
            .map()
            .with_creature_like_cpp(guid, |creature| creature.loot_authority_like_cpp().clone())
        {
            Some(authority) => DesignatedLootStoreReadLikeCpp::Present(authority),
            None => DesignatedLootStoreReadLikeCpp::Absent,
        }
    }

    fn read_designated_canonical_game_object_authority_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: MapKey,
    ) -> DesignatedLootStoreReadLikeCpp {
        let Some(manager) = self.core.canonical_map_manager.as_ref() else {
            return DesignatedLootStoreReadLikeCpp::Unreadable;
        };
        let Ok(manager) = manager.lock() else {
            return DesignatedLootStoreReadLikeCpp::Unreadable;
        };
        let Some(managed) = manager.find_map(map_key.map_id, map_key.instance_id) else {
            return DesignatedLootStoreReadLikeCpp::Unreadable;
        };
        match managed
            .map()
            .get_typed_game_object(guid)
            .map(|gameobject| gameobject.loot_authority_like_cpp().clone())
        {
            Some(authority) => DesignatedLootStoreReadLikeCpp::Present(authority),
            None => DesignatedLootStoreReadLikeCpp::Absent,
        }
    }

    fn read_designated_legacy_creature_authority_like_cpp(
        &self,
        guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
    ) -> DesignatedLootStoreReadLikeCpp {
        let Some(manager) = self.core.map_manager.as_ref() else {
            // The legacy store is not configured: no owner source holds this
            // object, so its authority is genuinely absent rather than an
            // unreadable fact. This is the represented-fixture configuration
            // R4a signed as "genuine absence".
            return DesignatedLootStoreReadLikeCpp::Absent;
        };
        let Ok(manager) = manager.read() else {
            return DesignatedLootStoreReadLikeCpp::Unreadable;
        };
        match manager.find_creature(map_id, instance_id, guid) {
            Some(world_creature) => DesignatedLootStoreReadLikeCpp::Present(
                world_creature.creature.loot_authority_like_cpp().clone(),
            ),
            // No map instance for the key, or no creature in it: the legacy
            // store holds no allocation either way.
            None => DesignatedLootStoreReadLikeCpp::Absent,
        }
    }

    /// Whether the map key a lookup resolved under is still the key this
    /// session resolves for the object. Single-shot validity gate retained for
    /// the canonical-only GameObject read that R8 signed.
    fn canonical_object_map_key_is_current_like_cpp(
        &self,
        map_key: MapKey,
        canonical_player_was_present: bool,
    ) -> bool {
        if canonical_player_was_present {
            return self.core.current_canonical_player_map_key_like_cpp() == Some(map_key);
        }
        if self.core.canonical_map_manager.is_some() {
            return self
                .core
                .canonical_object_lookup_map_key_like_cpp(map_key.map_id)
                == Some(map_key);
        }
        let (map_id, instance_id) = self.core.current_legacy_runtime_map_key_like_cpp();
        u32::from(map_id) == map_key.map_id && instance_id == map_key.instance_id
    }

    pub fn rebind_canonical_gameobject_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let map_key = self
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                self.core.player_map_id_like_cpp(),
            ))?;
        let manager = self.core.canonical_map_manager.as_ref()?;
        let mut manager = manager.lock().ok()?;
        manager
            .find_map_mut(map_key.map_id, map_key.instance_id)?
            .map_mut()
            .get_typed_game_object_mut(guid)
            .and_then(|gameobject| {
                gameobject.rebind_loot_authority_if_current_like_cpp(
                    expected,
                    expected_stamp,
                    authority,
                )
            })
    }

    pub fn rebind_canonical_creature_loot_authority_like_cpp(
        &self,
        guid: ObjectGuid,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let map_key = self
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                self.core.player_map_id_like_cpp(),
            ))?;
        self.rebind_canonical_creature_loot_authority_on_map_like_cpp(
            guid,
            map_key,
            expected,
            expected_stamp,
            authority,
        )
    }

    /// Read one store's allocation without deciding ownership. The designated
    /// owner decides in
    /// [`Self::represented_owned_loot_authority_outcome_like_cpp`]; this and its
    /// two siblings are the untyped single-store probes that fixtures and
    /// callers use to observe one store at a time.
    pub fn read_legacy_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let map_id = u16::try_from(map_key.map_id).ok()?;
        match self.read_designated_legacy_creature_authority_like_cpp(
            guid,
            map_id,
            map_key.instance_id,
        ) {
            DesignatedLootStoreReadLikeCpp::Present(authority) => Some(authority),
            DesignatedLootStoreReadLikeCpp::Absent | DesignatedLootStoreReadLikeCpp::Unreadable => {
                None
            }
        }
    }

    pub fn read_canonical_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        match self.read_designated_canonical_creature_authority_like_cpp(guid, map_key) {
            DesignatedLootStoreReadLikeCpp::Present(authority) => Some(authority),
            DesignatedLootStoreReadLikeCpp::Absent | DesignatedLootStoreReadLikeCpp::Unreadable => {
                None
            }
        }
    }

    pub fn read_canonical_gameobject_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        match self.read_designated_canonical_game_object_authority_like_cpp(guid, map_key) {
            DesignatedLootStoreReadLikeCpp::Present(authority) => Some(authority),
            DesignatedLootStoreReadLikeCpp::Absent | DesignatedLootStoreReadLikeCpp::Unreadable => {
                None
            }
        }
    }

    pub fn rebind_canonical_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
        expected: &OwnedLootAuthority,
        expected_stamp: OwnedLootAuthorityStamp,
        authority: OwnedLootAuthority,
    ) -> Option<bool> {
        let manager = self.core.canonical_map_manager.as_ref()?;
        let mut manager = manager.lock().ok()?;
        manager
            .find_map_mut(map_key.map_id, map_key.instance_id)?
            .map_mut()
            .get_typed_creature_mut(guid)
            .and_then(|creature| {
                creature.rebind_loot_authority_if_current_like_cpp(
                    expected,
                    expected_stamp,
                    authority,
                )
            })
    }
}
