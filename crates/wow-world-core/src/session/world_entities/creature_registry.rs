use crate::session::creature_canonical_adapter::{
    CreatureRepresentationAdmissionClauseLikeCpp, CreatureRepresentationAdmissionDecisionLikeCpp,
    CreatureRepresentationCaptureLikeCpp, creature_representation_admission_decision_like_cpp,
    creature_threat_guids_on_map_like_cpp, emit_creature_representation_capture_like_cpp,
    reconcile_creature_threat_references_on_map_like_cpp,
};
use crate::session::state::SessionCore;
use wow_core::ObjectGuid;
use wow_loot::OwnedLootAuthority;

impl SessionCore {
    /// Read one value from the session's represented creature *without*
    /// mutating it, synchronizing it or publishing anything.
    ///
    /// F6-7 R7a introduced this read-only path; #1263 F6-8D3b-2 makes it
    /// **canonical first**, like the mutation root: with a canonical map
    /// manager, `f` observes a projection of the canonical incarnation at the
    /// session's current map key (a clone of the incarnation and its
    /// `CreatureCreateData` projection, taken under the canonical execution
    /// lock and released before `f` runs, so `f` holds no guard). Only a
    /// creature the canonical map does not hold — a session with no canonical
    /// map manager, or a legacy-only registration no canonical instance
    /// admitted — is read from the legacy compatibility representation, under
    /// one shared read guard.
    ///
    /// `f` must stay a pure read: nothing it observes is written back. `None`
    /// means neither store holds the creature at the current map key.
    pub fn read_world_creature_like_cpp<F, R>(&self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&crate::map_manager::WorldCreature) -> R,
    {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        let canonical = self.canonical_map_manager.as_ref().and_then(|manager| {
            let manager = manager.lock().ok()?;
            let creature = manager
                .find_map(u32::from(map_id), instance_id)?
                .map()
                .get_typed_creature(guid)?
                .clone();
            Some(creature)
        });
        if let Some(creature) = canonical {
            let create_data =
                crate::map_manager::WorldCreature::create_data_from_canonical_like_cpp(&creature);
            return Some(f(&crate::map_manager::WorldCreature {
                creature,
                create_data,
            }));
        }
        let manager = self.map_manager.as_ref()?;
        let manager = manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let creature = manager.find_creature(map_id, instance_id, guid)?;
        Some(f(creature))
    }

    /// Run one pure read against the session's represented creature **inside**
    /// the owning guard, without cloning it.
    ///
    /// #1263 F6-8D3b-2: the canonical incarnation at the session's current map
    /// key when the canonical map holds it, otherwise the legacy compatibility
    /// representation (same selection as [`Self::read_world_creature_like_cpp`]).
    /// `f` runs under the canonical execution lock or a shared legacy read
    /// guard: it must not take another manager lock, perform I/O, deliver or
    /// await.
    pub fn with_represented_creature_like_cpp<F, R>(&self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&wow_entities::Creature) -> R,
    {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        let mut f = Some(f);
        if let Some(manager) = self.canonical_map_manager.as_ref()
            && let Ok(manager) = manager.lock()
            && let Some(creature) = manager
                .find_map(u32::from(map_id), instance_id)
                .and_then(|map| map.map().get_typed_creature(guid))
        {
            return f.take().map(|f| f(creature));
        }
        let manager = self.map_manager.as_ref()?;
        let manager = manager
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let creature = manager.find_creature(map_id, instance_id, guid)?;
        f.take().map(|f| f(&creature.creature))
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
    /// the same compatibility split the loot-authority lookup keeps for that
    /// configuration.
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
        self.with_admitted_world_creature_like_cpp(guid, |creature| Some(f(creature)))
    }

    /// Execute one admitted mutation guarded by a fully-looted lifecycle
    /// observation of the owner's loot authority.
    ///
    /// F6-7 R7b-2a. This is the viewed-release sibling of the gated mutation
    /// root: `SessionCore::with_admitted_world_creature_like_cpp` supplies the
    /// incarnation admission, the canonical→legacy lock order and the canonical
    /// application, and the authority's own observation supplies the loot
    /// admission — `authority` must still be the representation's allocation,
    /// and the whole-owner generation, lifecycle revision and fully-looted pool
    /// topology must still be the observed ones. The observation and the
    /// mutation are one critical section, so a pool that reopened between the
    /// release and the mutation refuses the mutation instead of letting it land.
    ///
    /// `f` is invoked only once both admissions passed; a refusal returns `None`
    /// with the callback unexecuted and nothing written to either store.
    pub fn mutate_world_creature_if_fully_looted_observation_like_cpp<F, R>(
        &mut self,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut crate::map_manager::WorldCreature) -> R,
    {
        self.with_admitted_world_creature_like_cpp(guid, |creature| {
            if !creature
                .creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(authority)
            {
                return None;
            }
            authority.with_fully_looted_lifecycle_observation_like_cpp(
                object_generation,
                lifecycle_revision,
                || f(creature),
            )
        })
    }

    /// Detached durable-claim completion variant of
    /// [`Self::mutate_world_creature_if_fully_looted_observation_like_cpp`].
    /// It additionally requires every authoritative loot pool to remain
    /// unviewed through the map mutation.
    pub fn mutate_world_creature_if_unviewed_fully_looted_observation_like_cpp<F, R>(
        &mut self,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut crate::map_manager::WorldCreature) -> R,
    {
        self.with_admitted_world_creature_like_cpp(guid, |creature| {
            if !creature
                .creature
                .loot_authority_like_cpp()
                .shares_storage_like_cpp(authority)
            {
                return None;
            }
            authority.with_unviewed_fully_looted_lifecycle_observation_like_cpp(
                object_generation,
                lifecycle_revision,
                || f(creature),
            )
        })
    }

    /// The one creature-mutation sequence all creature mutation roots share.
    ///
    /// #1263 F6-8D3b-2: **canonical first**. C++ has one live `Creature` per
    /// GUID and every session mutation (`Unit::DealDamage`,
    /// `Creature::setDeathState`, loot release, threat, spell effects) writes
    /// that object. The canonical map's incarnation is that object here:
    ///
    /// * with a canonical map manager, the incarnation is moved out of its map
    ///   record under the canonical execution lock, lent to `mutation` as a
    ///   [`WorldCreature`] by value (no clone), and inserted back into the same
    ///   map before the lock is released, whatever `mutation` returned. The
    ///   other units' `threatened by me` references are then brought in line
    ///   with its threat list. There is nothing to admit any more: the root no
    ///   longer transports a second representation into the incarnation, so
    ///   the former health-timeline, revision and loot-allocation clauses have
    ///   no subject (the D3b-1 failure mode — a stale legacy copy refused on
    ///   `Revision` after any executor health change, or overwriting the state
    ///   the executor advanced — cannot occur);
    /// * a legacy compatibility representation, where one still exists, is a
    ///   **projection** of the incarnation: after an applied mutation it is
    ///   replaced by a clone of the incarnation (canonical → legacy only; the
    ///   legacy copy is never read back);
    /// * with no canonical map manager (isolated legacy-only fixtures) the
    ///   mutation runs on the legacy representation alone, as before.
    ///
    /// `mutation` refuses by returning `None` (the loot-lifecycle observation
    /// roots); the incarnation is still re-inserted unchanged. Lock order is
    /// canonical then legacy; no I/O, delivery, await or manager re-entry
    /// happens inside these guards, and the authority lock a caller's
    /// `mutation` may take stays innermost.
    ///
    /// [`WorldCreature`]: crate::map_manager::WorldCreature
    fn with_admitted_world_creature_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        mutation: impl FnOnce(&mut crate::map_manager::WorldCreature) -> Option<R>,
    ) -> Option<R> {
        // #1263 C1: the guarded body copies one decision record and returns it;
        // the event is emitted here, after both guards have been released.
        let (result, capture) = self.with_admitted_world_creature_captured_like_cpp(guid, mutation);
        if let Some(capture) = capture {
            emit_creature_representation_capture_like_cpp(&capture);
        }
        result
    }

    /// The guarded body of [`Self::with_admitted_world_creature_like_cpp`],
    /// with the #1263 C1 capture record returned instead of emitted so the
    /// caller can emit it after the canonical and legacy guards are released.
    fn with_admitted_world_creature_captured_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        mutation: impl FnOnce(&mut crate::map_manager::WorldCreature) -> Option<R>,
    ) -> (Option<R>, Option<CreatureRepresentationCaptureLikeCpp>) {
        const ROOT: &str = "SessionCore::with_admitted_world_creature_like_cpp";
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        let legacy_manager = self.map_manager.as_ref().cloned();
        let Some(canonical_manager) = self.canonical_map_manager.as_ref().cloned() else {
            let Some(legacy_manager) = legacy_manager else {
                return (None, None);
            };
            let mut legacy_manager = legacy_manager
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(creature) = legacy_manager.find_creature_mut(map_id, instance_id, guid) else {
                return (None, None);
            };
            return (mutation(creature), None);
        };

        let capture = |decision: CreatureRepresentationAdmissionDecisionLikeCpp,
                       mutation_invoked: bool,
                       application: &'static str,
                       applied: bool| CreatureRepresentationCaptureLikeCpp {
            root: ROOT,
            map_id,
            instance_id,
            decision,
            mutation_invoked,
            application,
            applied,
        };
        let unavailable = |clause: CreatureRepresentationAdmissionClauseLikeCpp,
                           application: &'static str| {
            (
                None,
                Some(capture(
                    CreatureRepresentationAdmissionDecisionLikeCpp::unevaluated_like_cpp(
                        guid, clause,
                    ),
                    false,
                    application,
                    false,
                )),
            )
        };

        let mut canonical_manager = match canonical_manager.lock() {
            Ok(guard) => guard,
            Err(_) => {
                return unavailable(
                    CreatureRepresentationAdmissionClauseLikeCpp::OwnerUnavailable,
                    "owner_unavailable",
                );
            }
        };
        let Some(managed) = canonical_manager.find_map_mut(u32::from(map_id), instance_id) else {
            return unavailable(
                CreatureRepresentationAdmissionClauseLikeCpp::OwnerUnavailable,
                "owner_unavailable",
            );
        };
        let map = managed.map_mut();
        if map.get_typed_creature(guid).is_none() {
            return unavailable(
                CreatureRepresentationAdmissionClauseLikeCpp::IncarnationAbsent,
                "incarnation_absent",
            );
        }
        let old_threat_guids = creature_threat_guids_on_map_like_cpp(map, guid);
        let Some(record) = map.remove_map_object(guid) else {
            return unavailable(
                CreatureRepresentationAdmissionClauseLikeCpp::IncarnationAbsent,
                "incarnation_absent",
            );
        };
        let creature = match record.into_creature() {
            Ok(creature) => *creature,
            Err(record) => {
                let _ = map.insert_map_object_record(record);
                return unavailable(
                    CreatureRepresentationAdmissionClauseLikeCpp::IncarnationAbsent,
                    "incarnation_absent",
                );
            }
        };
        // The incarnation is its own representation: the predicate evaluated
        // against itself is the admitted decision the capture records.
        let decision = creature_representation_admission_decision_like_cpp(&creature, &creature);

        let mut legacy_guard = legacy_manager.as_ref().map(|legacy| {
            legacy
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        });
        let create_data = legacy_guard
            .as_mut()
            .and_then(|legacy| legacy.find_creature(map_id, instance_id, guid))
            .map(|legacy| legacy.create_data.clone())
            .unwrap_or_else(|| {
                crate::map_manager::WorldCreature::create_data_from_canonical_like_cpp(&creature)
            });
        let mut incarnation = crate::map_manager::WorldCreature {
            creature,
            create_data,
        };
        let result = mutation(&mut incarnation);
        let crate::map_manager::WorldCreature {
            creature,
            create_data,
        } = incarnation;
        let projection = result.is_some().then(|| creature.clone());
        let reinserted = wow_entities::MapObjectRecord::new_creature(creature)
            .ok()
            .is_some_and(|record| map.insert_map_object_record(record).is_ok());
        if !reinserted {
            return (
                None,
                Some(capture(
                    decision,
                    true,
                    "canonical_application_refused",
                    false,
                )),
            );
        }
        reconcile_creature_threat_references_on_map_like_cpp(map, guid, old_threat_guids);
        let Some(result) = result else {
            return (
                None,
                Some(capture(decision, true, "mutation_root_refused", false)),
            );
        };
        if let (Some(legacy), Some(projection)) = (legacy_guard.as_mut(), projection)
            && let Some(represented) = legacy.find_creature_mut(map_id, instance_id, guid)
        {
            represented.creature = projection;
            represented.create_data = create_data;
        }
        (Some(result), Some(capture(decision, true, "applied", true)))
    }
}
