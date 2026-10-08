use crate::session::creature_canonical_adapter::{
    CanonicalCreatureEntityApplicationLikeCpp, apply_canonical_creature_entity_on_map_like_cpp,
    creature_representation_is_admitted_like_cpp, sync_canonical_creature_entity_on_map_like_cpp,
};
use crate::session::state::SessionCore;
use wow_core::ObjectGuid;

impl SessionCore {
    pub fn sync_canonical_creature_entity_like_cpp(&mut self, creature: wow_entities::Creature) {
        let guid = creature.guid();
        let expected_legacy_authority = creature.loot_authority_like_cpp().clone();
        let expected_legacy_stamp = expected_legacy_authority.stamp_like_cpp();
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return;
        };
        let authority = sync_canonical_creature_entity_on_map_like_cpp(
            manager,
            u32::from(map_id),
            instance_id,
            creature,
        );
        if let Some(authority) = authority {
            let _ = self.rebind_legacy_creature_loot_authority_like_cpp(
                guid,
                &expected_legacy_authority,
                expected_legacy_stamp,
                authority,
            );
        }
    }

    /// Read one value from the session's represented creature *without*
    /// mutating it, synchronizing it or publishing anything.
    ///
    /// F6-7 R7a. Read-only consumers used to call [`Self::mutate_world_creature`]
    /// and simply return a field they read, which coupled a pure observation to
    /// the mutation root: the observation was refused whenever the legacy
    /// representation had no admitted canonical incarnation, and it ran the
    /// mutation root's canonical snapshot synchronization as an incidental side
    /// effect. This accessor is the read-only path those consumers use instead.
    ///
    /// Representation: `f` observes the session's **legacy per-session
    /// compatibility mirror** for `guid` (`MapManager`'s [`WorldCreature`] at the
    /// session's current legacy runtime map key) — exactly the object
    /// [`Self::mutate_world_creature`] mutates, including its `create_data`, its
    /// runtime bridges and its loot-authority alias. This is not a direct view of
    /// the canonical map's incarnation and it is not a second authority: the
    /// canonical map manager is neither locked nor consulted, so this accessor
    /// cannot observe, refresh, replace or reconcile canonical state.
    ///
    /// Freshness: the mirror is read exactly as it stands. This accessor neither
    /// pulls the canonical entity state into the mirror nor pushes the mirror
    /// back to the canonical incarnation, does not rebind a loot authority and
    /// does not run the mutation root's admission guard. A value observed here is
    /// therefore "what the legacy mirror holds now", not "what the canonical
    /// incarnation holds". `None` means only that no such mirror exists (no
    /// legacy map manager, no map at the current legacy runtime map key, or no
    /// creature with `guid` in it); it carries no statement about admission.
    ///
    /// Guard: one **shared** read guard on the legacy map manager. Concurrent
    /// reads do not exclude each other, no write guard is taken, and no canonical
    /// map-manager lock is taken, so this accessor cannot participate in the
    /// canonical→legacy lock order. `f` runs inside that guard and only ever
    /// receives `&WorldCreature`; it must stay a pure read, and it must not
    /// perform I/O, delivery or an await.
    ///
    /// [`Self::mutate_world_creature`]'s doc records the observable consequence
    /// of moving a consumer from the mutation root to this path: the incidental
    /// canonical snapshot synchronization that used to accompany such a read no
    /// longer happens.
    pub fn read_world_creature_like_cpp<F, R>(&self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&crate::map_manager::WorldCreature) -> R,
    {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        let manager = self.map_manager.as_ref()?;
        let manager = manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let creature = manager.find_creature(map_id, instance_id, guid)?;
        Some(f(creature))
    }

    /// Execute one admitted creature mutation against the current canonical
    /// incarnation, then synchronize the existing legacy representation.
    ///
    /// F6-7 R7 (reviewer signature §5.3.1 R7, repair order §5.3.2). The mutation
    /// runs at most once, inside one critical section that holds the canonical
    /// owner first and the legacy representation second, so the two stores can
    /// never be locked in opposite orders. Ownership is admitted *before* the
    /// mutation is invoked: a missing canonical map or object, a representation
    /// from another health timeline, a stale revision (including an ABA cycle)
    /// and a competing used loot allocation are all refused without touching
    /// either representation, and without publishing success. The mutated
    /// representation replaces the canonical entity state before the caller sees
    /// anything, so a successful mutation is canonical instead of surviving only
    /// in the legacy mirror. No I/O, delivery, await or manager re-entry happens
    /// inside these guards.
    ///
    /// A session with no canonical map manager at all owns no canonical
    /// incarnation store to diverge from, so it keeps the legacy-only path —
    /// the same compatibility contract
    /// [`Self::sync_canonical_creature_entity_like_cpp`] already has.
    ///
    /// This root is the *mutation* contract, so callers that only observe a
    /// creature belong on [`Self::read_world_creature_like_cpp`] instead.
    /// Because this root synchronizes the representation into the canonical
    /// incarnation and refuses to expose a result the incarnation did not
    /// apply, moving a read-only caller onto the read path also removes the
    /// snapshot synchronization this root used to perform as a side effect of
    /// that read. That is an observable change, not a pure move.
    pub fn mutate_world_creature<F, R>(&mut self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut crate::map_manager::WorldCreature) -> R,
    {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        let legacy_manager = self.map_manager.as_ref().cloned()?;
        let Some(canonical_manager) = self.canonical_map_manager.as_ref().cloned() else {
            let mut legacy_manager = legacy_manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let creature = legacy_manager.find_creature_mut(map_id, instance_id, guid)?;
            return Some(f(creature));
        };

        let mut canonical_manager = canonical_manager.lock().ok()?;
        let managed = canonical_manager.find_map_mut(u32::from(map_id), instance_id)?;
        let mut legacy_manager = legacy_manager
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let creature = legacy_manager.find_creature_mut(map_id, instance_id, guid)?;
        let admitted = managed
            .map()
            .with_creature_like_cpp(guid, |current| {
                creature_representation_is_admitted_like_cpp(current, &creature.creature)
            })
            .unwrap_or(false);
        if !admitted {
            return None;
        }

        let result = f(creature);
        let expected_legacy_authority = creature.creature.loot_authority_like_cpp().clone();
        let expected_legacy_stamp = expected_legacy_authority.stamp_like_cpp();
        let authority = match apply_canonical_creature_entity_on_map_like_cpp(
            managed.map_mut(),
            creature.creature.clone(),
        ) {
            CanonicalCreatureEntityApplicationLikeCpp::Applied(authority) => authority,
            // The canonical incarnation may not publish a representation it did
            // not apply. Admission above is the same predicate the application
            // re-evaluates, so an admitted representation cannot reach these
            // branches unless the incarnation changed under the guards.
            CanonicalCreatureEntityApplicationLikeCpp::Rejected(_)
            | CanonicalCreatureEntityApplicationLikeCpp::Refused => return None,
        };
        let _ = creature.creature.rebind_loot_authority_if_current_like_cpp(
            &expected_legacy_authority,
            expected_legacy_stamp,
            authority,
        );
        Some(result)
    }
}
