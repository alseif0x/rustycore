//! F6-7 R7b-2a fixtures shared by `r7b2a_guarded_loot_release`.
//!
//! The slice also gated the legacy → canonical creature entity synchronization
//! root; #1263 F6-8E deleted that root with the legacy store, and with it the
//! regressions this module held for it. What remains are the fixtures the
//! guarded loot-release module shares: the complete lifecycle observable, the
//! allocation rebind, the fully looted pool and observation builders, and the
//! one-incarnation assertion.

use super::r7a_canonical_mutation::*;
use super::*;

/// The complete observable set of one representation for a corpse-lifecycle
/// mutation. Comparing it before and after is what makes "a refused operation
/// mutated nothing" checkable, including whether the callback ran at all.
#[derive(Debug, PartialEq)]
pub(super) struct LootLifecycleStateLikeCpp {
    lootable_dynamic_flag: bool,
    changed_fields: u8,
    health: u64,
    max_health: u64,
    death_state: wow_constants::unit::DeathState,
    loot_stamp: wow_loot::OwnedLootAuthorityStamp,
}

pub(super) fn loot_lifecycle_state_like_cpp(
    creature: &wow_entities::Creature,
) -> LootLifecycleStateLikeCpp {
    let object = creature.unit().world().object();
    LootLifecycleStateLikeCpp {
        lootable_dynamic_flag: object.has_dynamic_flag(UnitDynFlags::Lootable as u32),
        changed_fields: object.changed_fields().bits(),
        health: creature.unit().data().health,
        max_health: creature.unit().data().max_health,
        death_state: creature.unit().death_state(),
        loot_stamp: creature.loot_authority_like_cpp().stamp_like_cpp(),
    }
}

pub(super) fn session_loot_lifecycle_state_like_cpp(
    manager: &SharedCanonicalMapManager,
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) -> (LootLifecycleStateLikeCpp, LootLifecycleStateLikeCpp) {
    (
        loot_lifecycle_state_like_cpp(
            &legacy_creature_like_cpp(manager, guid).expect("legacy representation"),
        ),
        loot_lifecycle_state_like_cpp(
            &canonical_creature_like_cpp(canonical, guid).expect("canonical incarnation"),
        ),
    )
}

/// One incarnation: one loot allocation and one health-state revision timeline.
pub(super) fn assert_one_incarnation_like_cpp(
    manager: &SharedCanonicalMapManager,
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
) {
    let legacy = legacy_creature_like_cpp(manager, guid).expect("legacy representation");
    let owner = canonical_creature_like_cpp(canonical, guid).expect("canonical incarnation");
    assert!(
        legacy
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(owner.loot_authority_like_cpp()),
        "the legacy representation shares the incarnation's loot allocation"
    );
    assert!(
        legacy
            .unit()
            .shares_health_state_revision_authority_like_cpp(
                &owner.unit().health_state_revision_authority_like_cpp()
            ),
        "the legacy representation shares the incarnation's health timeline"
    );
}

pub(super) fn has_lootable_flag_like_cpp(creature: &wow_entities::Creature) -> bool {
    creature
        .unit()
        .world()
        .object()
        .has_dynamic_flag(UnitDynFlags::Lootable as u32)
}

/// Replace the allocation the **canonical incarnation** holds.
pub(super) fn rebind_canonical_authority_like_cpp(
    canonical: &SharedCanonicalMapManager,
    guid: ObjectGuid,
    authority: wow_loot::OwnedLootAuthority,
) {
    canonical
        .lock()
        .unwrap()
        .find_map_mut(0, 0)
        .expect("canonical map instance")
        .map_mut()
        .with_creature_mut_like_cpp(guid, |creature| {
            creature.rebind_loot_authority_like_cpp(authority)
        })
        .expect("canonical incarnation");
}

/// A pool with no coins and no items: `active_loot_pools_fully_looted_like_cpp`
/// is true for it, which is what a completed release observes.
pub(super) fn fully_looted_pool_like_cpp(owner: ObjectGuid, viewer: ObjectGuid) -> CreatureLoot {
    CreatureLoot {
        loot_guid: owner,
        coins: 0,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![viewer],
        items: Vec::new(),
        looted_by_player: false,
    }
}

/// An allocation that already owns a fully looted lifetime, plus the exact
/// viewed release observation the caller would have captured for it.
pub(super) fn observed_fully_looted_authority_like_cpp(
    guid: ObjectGuid,
    viewer: ObjectGuid,
) -> (wow_loot::OwnedLootAuthority, u64, u64) {
    let authority = wow_loot::OwnedLootAuthority::new();
    assert_ne!(
        authority.replace_like_cpp(
            Some(fully_looted_pool_like_cpp(guid, viewer)),
            HashMap::new()
        ),
        0,
        "the fully looted pool is installed on a pristine allocation"
    );
    let open = authority
        .add_viewer_like_cpp(viewer)
        .expect("the viewer opens the fully looted pool");
    let close = authority
        .close_viewer_if_generation_like_cpp(open.generation, viewer)
        .expect("the viewer closes the observed pool");
    assert!(
        close.whole_object_fully_looted,
        "the closed pool leaves the owner fully looted"
    );
    (authority, close.object_generation, close.lifecycle_revision)
}

/// An independently constructed admission candidate, matching what production
/// registration sets for a creature recreated under the same GUID.
pub(super) fn admission_candidate_like_cpp(guid: ObjectGuid) -> wow_entities::Creature {
    let mut creature = wow_entities::Creature::new(false);
    creature.unit_mut().world_mut().object_mut().create(guid);
    creature
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(9_400);
    let _ = creature.unit_mut().world_mut().set_map(0, 0);
    creature.unit_mut().world_mut().relocate(Position::ZERO);
    creature.unit_mut().set_level(1);
    creature.unit_mut().set_max_health(100);
    creature.unit_mut().set_health(100);
    creature
}
