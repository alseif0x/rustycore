//! #1263 F6-8C: which creature objects the canonical designated owner holds.
//!
//! The legacy creature phases (aggro, spell, melee and player melee) used to
//! decide from whatever `WorldCreature` the legacy runtime store happened to
//! hold, with no canonical owner involved at all. The canonical designated
//! owner of a creature object is the **canonical map incarnation** whenever a
//! canonical store is configured, and the legacy representation only in the
//! legitimate legacy-only configuration. That is the same designated-owner rule
//! F6-7 R2 gave the loot-authority lookup and F6-8A/B gave respawn and movement
//! publication, so a surviving legacy copy decides nothing by itself.
//!
//! The roster is read **once per phase, before any legacy guard is taken**, so
//! the established canonical→legacy lock order is preserved and no phase ever
//! reaches for the canonical owner while holding the legacy write guard.
//! A poisoned canonical owner is not proof of absence: it refuses every
//! decision instead of falling back to the legacy copy (F6-7 R4 semantics).

use super::*;

/// The canonical designated owner, resolved for one phase's residences.
#[derive(Debug, Clone)]
pub(in crate::session) enum CanonicalCreatureOwnershipLikeCpp {
    /// No canonical store is configured: the legacy representation is the
    /// designated owner and keeps deciding exactly as before.
    LegacyOnlyConfiguration,
    /// A canonical store is configured. Only the listed `(map, instance, guid)`
    /// triples have an incarnation; every other creature has no canonical owner.
    CanonicalConfigured(std::collections::HashSet<(u16, u32, ObjectGuid)>),
}

impl CanonicalCreatureOwnershipLikeCpp {
    /// Whether the canonical owner exists for this creature at this residence.
    ///
    /// `true` in the legacy-only configuration (the legacy representation *is*
    /// the designated owner there) and for a listed incarnation.
    pub(in crate::session) fn decides_like_cpp(
        &self,
        map_id: u16,
        instance_id: u32,
        guid: ObjectGuid,
    ) -> bool {
        match self {
            Self::LegacyOnlyConfiguration => true,
            Self::CanonicalConfigured(incarnations) => {
                incarnations.contains(&(map_id, instance_id, guid))
            }
        }
    }

    /// Whether a canonical store is configured at all. A phase refuses without
    /// touching the legacy copy in that configuration.
    pub(in crate::session) fn is_canonical_configured_like_cpp(&self) -> bool {
        matches!(self, Self::CanonicalConfigured(_))
    }

    /// Number of creature incarnations the canonical owner actually holds.
    pub(in crate::session) fn incarnations_seen_like_cpp(&self) -> usize {
        match self {
            Self::LegacyOnlyConfiguration => 0,
            Self::CanonicalConfigured(incarnations) => incarnations.len(),
        }
    }
}

/// Read the canonical owner's incarnations for the residences the legacy store
/// currently holds, one exact-GUID probe per `(map, instance, guid)`.
///
/// Lock scope and order: the canonical owner is locked **first** and the legacy
/// store second, which is the established canonical→legacy order; both guards
/// are released before the caller takes the legacy write guard. No I/O, await,
/// delivery or manager re-entry happens inside them.
///
/// The probe is exact per GUID (`Map::with_creature_like_cpp`), not the map's
/// grid roster: the canonical owner also holds records that are not placed in a
/// loaded grid cell yet, and an existing but unplaced incarnation still owns
/// its object.
pub(in crate::session) fn canonical_creature_ownership_like_cpp(
    canonical_map_manager: Option<&SharedCanonicalMapManager>,
    legacy_map_manager: &crate::map_manager::SharedMapManager,
) -> CanonicalCreatureOwnershipLikeCpp {
    let Some(canonical_map_manager) = canonical_map_manager else {
        return CanonicalCreatureOwnershipLikeCpp::LegacyOnlyConfiguration;
    };
    let mut incarnations = std::collections::HashSet::new();
    let Ok(canonical) = canonical_map_manager.lock() else {
        // An unreadable canonical owner is not proof that the object has no
        // owner: refuse every decision rather than decide from the legacy copy.
        return CanonicalCreatureOwnershipLikeCpp::CanonicalConfigured(incarnations);
    };
    let legacy = legacy_map_manager
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for (map_id, instance_id) in legacy.active_map_keys() {
        let Some(managed) = canonical.find_map(u32::from(map_id), instance_id) else {
            continue;
        };
        for guid in legacy.creature_guids(map_id, instance_id) {
            if managed.map().with_creature_like_cpp(guid, |_| ()).is_some() {
                incarnations.insert((map_id, instance_id, guid));
            }
        }
    }
    CanonicalCreatureOwnershipLikeCpp::CanonicalConfigured(incarnations)
}
