// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::{LootReleaseAccessLikeCpp, LootReleaseOwnerAccessLikeCpp};
use wow_core::ObjectGuid;
use wow_loot::{OwnedLootAuthority, OwnedLootAuthorityStamp};
use wow_map::MapKey;

/// Typed result of resolving one owner's object-owned loot authority.
///
/// The compatibility `Option` wrappers collapse [`Self::Absent`] and
/// [`Self::Unavailable`], because both fail closed for them. They are
/// different facts: `Absent` means no object authority was readable for this
/// owner, while `Unavailable` means the dual-store reconciliation did not
/// converge inside its bounded rounds (F6-7 R4), which is not absent loot and
/// must not be answered like it.
#[derive(Debug, Clone)]
pub enum OwnedLootAuthorityLookupOutcomeLikeCpp {
    Found(OwnedLootAuthority),
    Absent,
    Unavailable,
}

impl OwnedLootAuthorityLookupOutcomeLikeCpp {
    /// The compatibility collapse for the untouched `Option` consumers:
    /// `Absent` and `Unavailable` both fail closed. This is the one body that
    /// answers the historical `Option<OwnedLootAuthority>` shape, so the
    /// `_like_cpp` wrappers in this crate, in `wow-world-application` and in
    /// `wow-world` are delegations rather than copies.
    pub fn into_option_like_cpp(self) -> Option<OwnedLootAuthority> {
        match self {
            Self::Found(authority) => Some(authority),
            Self::Absent | Self::Unavailable => None,
        }
    }
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

    /// Typed counterpart of [`Self::represented_owned_loot_authority_like_cpp`].
    /// Every non-converging exit of the creature reconciliation loop is
    /// reported as `Unavailable`, not as absent loot.
    pub fn represented_owned_loot_authority_outcome_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> OwnedLootAuthorityLookupOutcomeLikeCpp {
        if owner_guid.is_creature_or_vehicle() {
            // The legacy and canonical maps deliberately use separate locks.
            // Reconcile optimistically with object-local compare/exchange;
            // blind rebinding can otherwise clobber a newer respawn between
            // the read and write phases.
            for _ in 0..8 {
                let canonical_player_map_key =
                    self.core.current_canonical_player_map_key_like_cpp();
                let map_key = canonical_player_map_key
                    .or_else(|| {
                        self.core
                            .canonical_object_lookup_map_key_like_cpp(u32::from(
                                self.core.player_map_id_like_cpp(),
                            ))
                    })
                    .unwrap_or_else(|| {
                        let (map_id, instance_id) =
                            self.core.current_legacy_runtime_map_key_like_cpp();
                        wow_map::MapKey::new(u32::from(map_id), instance_id)
                    });
                let map_key_still_valid = |session: &Self| {
                    session
                        .transitions_like_cpp()
                        .loot_reconciliation_map_key_still_valid_like_cpp(
                            map_key,
                            canonical_player_map_key.is_some(),
                        )
                };
                let legacy = self
                    .transitions_like_cpp()
                    .read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key);
                let canonical = self
                    .transitions_like_cpp()
                    .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key);
                let (legacy, canonical) = match (legacy, canonical) {
                    (Some(legacy), Some(canonical)) => (legacy, canonical),
                    (None, None) => return OwnedLootAuthorityLookupOutcomeLikeCpp::Absent,
                    (Some(authority), None) | (None, Some(authority)) => {
                        if !map_key_still_valid(self) {
                            continue;
                        }
                        return OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority);
                    }
                };
                if !map_key_still_valid(self) {
                    continue;
                }

                let legacy_stamp = legacy.stamp_like_cpp();
                let canonical_stamp = canonical.stamp_like_cpp();
                let selected = crate::session::reconcile_creature_loot_authority_mirrors_like_cpp(
                    &canonical,
                    canonical_stamp,
                    &legacy,
                    legacy_stamp,
                );
                if !map_key_still_valid(self) {
                    continue;
                }
                if self
                    .transitions_like_cpp()
                    .rebind_canonical_creature_loot_authority_on_map_like_cpp(
                        owner_guid,
                        map_key,
                        &canonical,
                        canonical_stamp,
                        selected.clone(),
                    )
                    .is_none()
                {
                    continue;
                }
                if !map_key_still_valid(self) {
                    continue;
                }
                if self
                    .core
                    .rebind_legacy_creature_loot_authority_on_map_like_cpp(
                        owner_guid,
                        map_key,
                        &legacy,
                        legacy_stamp,
                        selected.clone(),
                    )
                    .is_none()
                {
                    continue;
                }

                if !map_key_still_valid(self) {
                    continue;
                }
                let converged_legacy = self
                    .transitions_like_cpp()
                    .read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
                    .is_some_and(|authority| authority.shares_storage_like_cpp(&selected));
                let converged_canonical = self
                    .transitions_like_cpp()
                    .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
                    .is_some_and(|authority| authority.shares_storage_like_cpp(&selected));
                if converged_legacy && converged_canonical {
                    return OwnedLootAuthorityLookupOutcomeLikeCpp::Found(selected);
                }
            }

            // Continuous concurrent replacement is safer as a failed request
            // than as an overwrite of the newest mirror. Exhaustion is the
            // third fact (F6-7 R4), not absence: the caller must decide what
            // to do with an unproven answer instead of reading it as no loot.
            return OwnedLootAuthorityLookupOutcomeLikeCpp::Unavailable;
        }

        if owner_guid.is_game_object() {
            let canonical_player_map_key = self.core.current_canonical_player_map_key_like_cpp();
            let Some(map_key) = canonical_player_map_key.or_else(|| {
                self.core
                    .canonical_object_lookup_map_key_like_cpp(u32::from(
                        self.core.player_map_id_like_cpp(),
                    ))
            }) else {
                return OwnedLootAuthorityLookupOutcomeLikeCpp::Absent;
            };
            let Some(authority) = self
                .transitions_like_cpp()
                .read_canonical_gameobject_loot_authority_on_map_like_cpp(owner_guid, map_key)
            else {
                return OwnedLootAuthorityLookupOutcomeLikeCpp::Absent;
            };
            let still_valid = self
                .transitions_like_cpp()
                .loot_reconciliation_map_key_still_valid_like_cpp(
                    map_key,
                    canonical_player_map_key.is_some(),
                );
            // The gameobject branch has no reconciliation loop and therefore
            // no exhausted state: an invalid map key stays `Absent`, exactly
            // as the fail-closed `None` it maps to today.
            return if still_valid {
                OwnedLootAuthorityLookupOutcomeLikeCpp::Found(authority)
            } else {
                OwnedLootAuthorityLookupOutcomeLikeCpp::Absent
            };
        }

        OwnedLootAuthorityLookupOutcomeLikeCpp::Absent
    }

    /// Compatibility wrapper: unchanged signature and behaviour for the
    /// consumers that only ask "is there an authority for this owner".
    /// `Unavailable` remains a fail-closed `None` here, so no untouched
    /// consumer changes.
    pub fn represented_owned_loot_authority_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        self.represented_owned_loot_authority_outcome_like_cpp(owner_guid)
            .into_option_like_cpp()
    }
}

impl LootReleaseAccessLikeCpp<'_> {
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

    pub fn read_legacy_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let map_id = u16::try_from(map_key.map_id).ok()?;
        let manager = self.core.map_manager.as_ref()?;
        manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .find_creature(map_id, map_key.instance_id, guid)
            .map(|world_creature| world_creature.creature.loot_authority_like_cpp().clone())
    }

    pub fn read_canonical_creature_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let manager = self.core.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        manager
            .find_map(map_key.map_id, map_key.instance_id)?
            .map()
            .with_creature_like_cpp(guid, |creature| creature.loot_authority_like_cpp().clone())
    }

    pub fn read_canonical_gameobject_loot_authority_on_map_like_cpp(
        &self,
        guid: ObjectGuid,
        map_key: wow_map::MapKey,
    ) -> Option<OwnedLootAuthority> {
        let manager = self.core.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        manager
            .find_map(map_key.map_id, map_key.instance_id)?
            .map()
            .get_typed_game_object(guid)
            .map(|gameobject| gameobject.loot_authority_like_cpp().clone())
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

    pub fn loot_reconciliation_map_key_still_valid_like_cpp(
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
}
