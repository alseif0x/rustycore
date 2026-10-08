//! F6-7 R7b-2a regressions: the creature entity synchronization root.
//!
//! The slice gates `SessionCore::sync_canonical_creature_entity_like_cpp` with
//! the same mechanism the R7a gate uses: the R7a admission predicate
//! (`creature_representation_is_admitted_like_cpp`) evaluated against the
//! current canonical incarnation, canonical→legacy lock order, and the
//! canonical application under its execution lock before any success is
//! exposed. This root used to expose the successful **legacy-authority rebind**
//! for a transported representation the incarnation had *not* applied.
//!
//! This module also owns the fixtures the sibling
//! `r7b2a_guarded_loot_release` module shares: the complete lifecycle
//! observable, the allocation rebinds, the fully looted pool and observation
//! builders, and the one-incarnation assertion. Every fixture is `pub(super)`
//! and every case drives the production roots, not a double.

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
    manager: &crate::map_manager::SharedMapManager,
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
    manager: &crate::map_manager::SharedMapManager,
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

/// Replace the allocation the **legacy** representation holds.
pub(super) fn rebind_legacy_authority_like_cpp(
    manager: &crate::map_manager::SharedMapManager,
    guid: ObjectGuid,
    authority: wow_loot::OwnedLootAuthority,
) {
    let mut guard = manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let creature = guard
        .find_creature_mut(0, 0, guid)
        .expect("legacy representation");
    creature.creature.rebind_loot_authority_like_cpp(authority);
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

// ---------------------------------------------------------------------------
// Root 1: `SessionCore::sync_canonical_creature_entity_like_cpp`
// ---------------------------------------------------------------------------

/// A session whose legacy representation and canonical incarnation are one
/// admitted incarnation, but whose **allocation is split**: the representation
/// carries a pristine candidate while the incarnation owns a used one. That is
/// the shape in which this root's legacy synchronization is observable: the
/// transported pristine candidate is admitted, and the root must move the
/// legacy alias onto the incarnation's allocation.
fn split_allocation_session_like_cpp(
    guid: ObjectGuid,
    player: ObjectGuid,
) -> (
    WorldSession,
    crate::map_manager::SharedMapManager,
    SharedCanonicalMapManager,
    wow_loot::OwnedLootAuthority,
    wow_loot::OwnedLootAuthority,
) {
    let (session, manager, canonical) = mirrored_session_like_cpp(guid, 100);
    let pristine = wow_loot::OwnedLootAuthority::new();
    let incarnation = observed_fully_looted_authority_like_cpp(guid, player).0;
    rebind_canonical_authority_like_cpp(&canonical, guid, incarnation.clone());
    rebind_legacy_authority_like_cpp(&manager, guid, pristine.clone());
    (session, manager, canonical, pristine, incarnation)
}

#[test]
fn fresh_sync_applies_to_the_incarnation_and_synchronizes_the_legacy_alias_like_cpp() {
    let guid = test_creature_guid(92_100);
    let player = ObjectGuid::create_player(1, 92_101);
    let (mut session, manager, canonical, pristine, incarnation) =
        split_allocation_session_like_cpp(guid, player);
    let mut transported = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    assert!(
        transported
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&pristine),
        "the transported representation carries the representation's own allocation"
    );
    transported.ai_ownership_mut().npc_flags = 0x0001_0002;

    let applied = session
        .core
        .sync_canonical_creature_entity_like_cpp(transported);

    assert!(
        applied,
        "a fresh same-incarnation representation is admitted and applied"
    );
    let owner = canonical_creature_like_cpp(&canonical, guid).expect("canonical incarnation");
    assert_eq!(
        owner.ai_ownership().npc_flags,
        0x0001_0002,
        "the canonical incarnation carries the transported state"
    );
    let legacy = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    assert!(
        legacy
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(owner.loot_authority_like_cpp()),
        "the legacy alias is synchronized onto the incarnation's allocation"
    );
    assert!(
        !legacy
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&pristine),
        "the legacy alias no longer holds the displaced candidate"
    );
    assert!(
        owner
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&incarnation),
        "the incarnation keeps the allocation it owns"
    );
    assert_one_incarnation_like_cpp(&manager, &canonical, guid);
}

/// **Negative control (R7b-2a primary).** Executed against the root's pre-slice
/// body it fails: the stale representation is applied to neither entity state,
/// yet the old root exposed success and moved the legacy alias onto the
/// incarnation's allocation. It passes once the gate admits ownership before the
/// application is invoked.
#[test]
fn stale_sync_is_refused_before_application_and_leaves_both_stores_like_cpp() {
    let guid = test_creature_guid(92_102);
    let player = ObjectGuid::create_player(1, 92_103);
    let (mut session, manager, canonical, pristine, _) =
        split_allocation_session_like_cpp(guid, player);
    let transported = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    // The canonical incarnation advances without the representation.
    advance_canonical_max_health_like_cpp(&canonical, guid, 140);
    let before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid);

    let applied = session
        .core
        .sync_canonical_creature_entity_like_cpp(transported);

    assert!(
        !applied,
        "a stale representation must not be reported as a successful synchronization"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid),
        before,
        "a refused synchronization writes neither representation"
    );
    let legacy = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    assert!(
        legacy
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&pristine),
        "a refused synchronization does not move the legacy alias"
    );
    assert!(
        !legacy.loot_authority_like_cpp().shares_storage_like_cpp(
            canonical_creature_like_cpp(&canonical, guid)
                .expect("canonical incarnation")
                .loot_authority_like_cpp()
        ),
        "the legacy alias is not silently pointed at a snapshot the incarnation rejected"
    );
}

#[test]
fn aba_sync_is_refused_before_application_like_cpp() {
    let guid = test_creature_guid(92_104);
    let player = ObjectGuid::create_player(1, 92_105);
    let (mut session, manager, canonical, pristine, _) =
        split_allocation_session_like_cpp(guid, player);
    // Health leaves and returns to the same tuple while the revision advances.
    {
        let mut guard = canonical.lock().unwrap();
        let map = guard.find_map_mut(0, 0).expect("canonical map instance");
        map.map_mut()
            .with_creature_mut_like_cpp(guid, |creature| {
                creature.unit_mut().set_health(60);
                creature.unit_mut().set_health(100);
            })
            .expect("canonical incarnation");
    }
    let transported = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    let before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid);

    assert!(
        !session
            .core
            .sync_canonical_creature_entity_like_cpp(transported),
        "an ABA replay is a stale representation, not a fresh one"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid),
        before
    );
    assert!(
        legacy_creature_like_cpp(&manager, guid)
            .expect("legacy representation")
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&pristine)
    );
}

#[test]
fn different_incarnation_and_competing_allocation_refuse_the_sync_like_cpp() {
    // (a) The canonical object is destroyed and recreated under the same GUID.
    let guid = test_creature_guid(92_106);
    let (mut session, manager, canonical) = mirrored_session_like_cpp(guid, 100);
    let destroyed = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    remove_canonical_creature_map_object_on_map_like_cpp(&canonical, 0, 0, guid);
    let _ = insert_canonical_creature_map_object_on_map_like_cpp(
        &canonical,
        0,
        0,
        admission_candidate_like_cpp(guid),
    )
    .expect("the recreation is admitted");
    let replacement_before = loot_lifecycle_state_like_cpp(
        &canonical_creature_like_cpp(&canonical, guid).expect("replacement"),
    );
    assert!(
        !session
            .core
            .sync_canonical_creature_entity_like_cpp(destroyed),
        "a representation of a destroyed incarnation must not replace the recreation"
    );
    assert_eq!(
        loot_lifecycle_state_like_cpp(
            &canonical_creature_like_cpp(&canonical, guid).expect("replacement")
        ),
        replacement_before,
        "the replacement keeps its own state and lifetime"
    );

    // (b) The representation carries a competing used allocation for the same
    // incarnation: neither store may be reconciled or displaced.
    let guid = test_creature_guid(92_107);
    let player = ObjectGuid::create_player(1, 92_108);
    let (mut session, manager, canonical) = mirrored_session_like_cpp(guid, 100);
    let competing = observed_fully_looted_authority_like_cpp(guid, player).0;
    rebind_legacy_authority_like_cpp(&manager, guid, competing.clone());
    let transported = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    let before = session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid);
    let canonical_authority = canonical_creature_like_cpp(&canonical, guid)
        .expect("canonical incarnation")
        .loot_authority_like_cpp()
        .clone();

    assert!(
        !session
            .core
            .sync_canonical_creature_entity_like_cpp(transported),
        "a competing used allocation must not be reconciled into the incarnation"
    );
    assert_eq!(
        session_loot_lifecycle_state_like_cpp(&manager, &canonical, guid),
        before,
        "the refused representation keeps its own competing allocation"
    );
    assert_eq!(
        competing.lifecycle_like_cpp(),
        wow_loot::OwnedLootAuthorityLifecycle::Active,
        "the competing pool is neither quarantined nor retired"
    );
    assert!(
        canonical_creature_like_cpp(&canonical, guid)
            .expect("canonical incarnation")
            .loot_authority_like_cpp()
            .shares_storage_like_cpp(&canonical_authority),
        "the canonical incarnation keeps its own allocation"
    );
}

#[test]
fn unavailable_sync_ownership_is_refused_before_application_like_cpp() {
    let guid = test_creature_guid(92_109);
    let manager = shared_map_manager();
    let (mut session, _, _) = make_session();
    register_test_creature(&mut session, manager.clone(), guid, 100);
    let transported = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    let before = loot_lifecycle_state_like_cpp(
        &legacy_creature_like_cpp(&manager, guid).expect("legacy representation"),
    );

    // No canonical manager at all: there is no incarnation to synchronize into.
    assert!(
        !session
            .core
            .sync_canonical_creature_entity_like_cpp(transported.clone()),
        "a session without a canonical store has no incarnation to synchronize"
    );

    // A bound canonical manager whose map instance does not exist.
    let canonical = shared_canonical_map_manager();
    session.set_canonical_map_manager(Arc::clone(&canonical));
    assert!(
        !session
            .core
            .sync_canonical_creature_entity_like_cpp(transported.clone()),
        "a missing canonical map instance is unavailable ownership"
    );

    // A canonical map instance without the incarnation for this GUID.
    canonical.lock().unwrap().create_world_map(0, 0);
    assert!(
        !session
            .core
            .sync_canonical_creature_entity_like_cpp(transported.clone()),
        "a missing canonical object is unavailable ownership"
    );
    assert!(
        canonical_creature_like_cpp(&canonical, guid).is_none(),
        "the refused synchronization created no incarnation"
    );
    assert_eq!(
        loot_lifecycle_state_like_cpp(
            &legacy_creature_like_cpp(&manager, guid).expect("legacy representation")
        ),
        before,
        "every refusal left the representation untouched"
    );
}

/// Documented boundary: this root's ownership is the **canonical incarnation**,
/// so the absence of a legacy mirror does not refuse the canonical application —
/// the legacy synchronization is an expected-authority compare-and-exchange that
/// simply has no representation to update.
#[test]
fn sync_without_a_legacy_representation_still_applies_to_the_incarnation_like_cpp() {
    let guid = test_creature_guid(92_110);
    let (mut session, manager, canonical) = mirrored_session_like_cpp(guid, 100);
    let mut transported = legacy_creature_like_cpp(&manager, guid).expect("legacy representation");
    transported.ai_ownership_mut().npc_flags = 0x0001_0003;
    manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove_creature_any(0, 0, guid)
        .expect("the fixture registered the legacy representation");

    assert!(
        session
            .core
            .sync_canonical_creature_entity_like_cpp(transported),
        "the canonical incarnation is the ownership this root resolves"
    );
    assert_eq!(
        canonical_creature_like_cpp(&canonical, guid)
            .expect("canonical incarnation")
            .ai_ownership()
            .npc_flags,
        0x0001_0003
    );
}
