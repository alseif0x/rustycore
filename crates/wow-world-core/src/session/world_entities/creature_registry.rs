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
    /// roots); the incarnation is still re-inserted unchanged.
    ///
    /// #1263 F6-8D3b-3: the incarnation always returns to its map. A panic in
    /// `mutation` is caught, the incarnation is put back into its record and
    /// cell, the canonical guard is released (so the mutex the map loop needs
    /// is not poisoned), and the panic resumes; a guard also returns it if
    /// anything else unwinds. A mutation that rebinds the creature's map is
    /// refused loudly and the binding it was lent with restored. A position the
    /// mutation changed is applied as C++ `Unit::UpdatePosition` →
    /// `Map::CreatureRelocation`, so the grid cell follows (vehicle, possess and
    /// mind-control movers write it through `set_ai_position`). Lock order is
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
        let Some((creature, origin)) = map.lend_creature_like_cpp(guid) else {
            return unavailable(
                CreatureRepresentationAdmissionClauseLikeCpp::IncarnationAbsent,
                "incarnation_absent",
            );
        };
        // The incarnation is its own representation: the predicate evaluated
        // against itself is the admitted decision the capture records. Since
        // D3b-2 this is tautological (always `Admitted`); the record is kept
        // for the C1 capture stream until E retires it.
        let decision = creature_representation_admission_decision_like_cpp(&creature, &creature);

        // A legacy copy exists only under the test-fixture owners. Production
        // has none, so it pays neither the projection clone nor the legacy
        // write guard; the read guard here is released before the mutation.
        let legacy_create_data = legacy_manager.as_ref().and_then(|legacy| {
            legacy
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .find_creature(map_id, instance_id, guid)
                .map(|legacy| legacy.create_data.clone())
        });
        let legacy_copy = legacy_create_data.is_some();
        let create_data = legacy_create_data.unwrap_or_else(|| {
            crate::map_manager::WorldCreature::create_data_from_canonical_like_cpp(&creature)
        });
        let mut lent = LentIncarnationLikeCpp {
            map,
            origin,
            old_threat_guids: Some(old_threat_guids),
            incarnation: Some(crate::map_manager::WorldCreature {
                creature,
                create_data,
            }),
        };
        let result = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            mutation(lent.incarnation.as_mut().expect("lent until returned"))
        })) {
            Ok(result) => result,
            Err(panic) => {
                // The guard returns the incarnation to its record and cell, and
                // the canonical guard is released normally before the panic
                // resumes: the creature is not lost and the canonical mutex the
                // map loop needs is not poisoned.
                drop(lent);
                drop(canonical_manager);
                std::panic::resume_unwind(panic);
            }
        };
        let crate::map_manager::WorldCreature {
            creature,
            create_data,
        } = lent.incarnation.take().expect("lent until returned");
        let projection = (legacy_copy && result.is_some()).then(|| creature.clone());
        let returned = lent.return_like_cpp(creature);
        if returned != LentIncarnationReturnLikeCpp::Clean {
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
        drop(lent);
        let Some(result) = result else {
            return (
                None,
                Some(capture(decision, true, "mutation_root_refused", false)),
            );
        };
        if let (Some(legacy), Some(projection)) = (legacy_manager.as_ref(), projection) {
            let mut legacy = legacy
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(represented) = legacy.find_creature_mut(map_id, instance_id, guid) {
                represented.creature = projection;
                represented.create_data = create_data;
            }
        }
        (Some(result), Some(capture(decision, true, "applied", true)))
    }
}

/// How a lent incarnation went back to its map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LentIncarnationReturnLikeCpp {
    /// Back in its record, relocated to where the mutation left it.
    Clean,
    /// Back in its record, but only after the binding it was lent with was put
    /// back: the mutation rebound or unbound its map, which is refused.
    BindingRestored,
    /// The record could not be rebuilt and the incarnation is gone.
    Lost,
}

/// One canonical incarnation lent out of its map record by the mutation root.
///
/// #1263 F6-8D3b-3: it goes back to the map however the lending ends. The
/// root returns it explicitly; if anything unwinds first, `Drop` returns it,
/// still under the canonical guard, so the creature is never left out of
/// `entity_world` with its GUID in a grid cell.
struct LentIncarnationLikeCpp<'m> {
    map: &'m mut wow_map::ManagedMapInnerLikeCpp,
    origin: wow_map::LentCreatureOriginLikeCpp,
    old_threat_guids: Option<Vec<ObjectGuid>>,
    incarnation: Option<crate::map_manager::WorldCreature>,
}

impl LentIncarnationLikeCpp<'_> {
    /// Put `creature` back (C++ `Map::CreatureRelocation` for a position the
    /// mutation changed), then bring the other units' `threatened by me`
    /// references in line with its threat list.
    fn return_like_cpp(
        &mut self,
        creature: wow_entities::Creature,
    ) -> LentIncarnationReturnLikeCpp {
        let guid = self.origin.guid;
        let returned = match self
            .map
            .return_lent_creature_like_cpp(creature, self.origin)
        {
            wow_map::LentCreatureReturnLikeCpp::Returned {
                binding_restored,
                relocation,
            } => {
                match relocation {
                    Some(Ok(outcome)) if !outcome.relocated => tracing::debug!(
                        ?guid,
                        ?outcome,
                        "creature mutation moved the creature into an unloaded grid; it stays at its origin like C++ Map::CreatureRelocation"
                    ),
                    Some(Err(error)) => tracing::warn!(
                        ?guid,
                        ?error,
                        "creature mutation left the creature at a position the map refused; it stays at its origin"
                    ),
                    _ => {}
                }
                if binding_restored {
                    tracing::error!(
                        ?guid,
                        map_id = self.origin.map_id,
                        instance_id = self.origin.instance_id,
                        "creature mutation rebound the creature's map; the binding it was lent with was restored and the mutation is refused"
                    );
                    LentIncarnationReturnLikeCpp::BindingRestored
                } else {
                    LentIncarnationReturnLikeCpp::Clean
                }
            }
            wow_map::LentCreatureReturnLikeCpp::Lost { error } => {
                tracing::error!(
                    ?guid,
                    ?error,
                    "creature mutation left an incarnation that cannot return to its map record; the creature is lost"
                );
                LentIncarnationReturnLikeCpp::Lost
            }
        };
        if returned != LentIncarnationReturnLikeCpp::Lost
            && let Some(old_threat_guids) = self.old_threat_guids.take()
        {
            reconcile_creature_threat_references_on_map_like_cpp(self.map, guid, old_threat_guids);
        }
        returned
    }
}

impl Drop for LentIncarnationLikeCpp<'_> {
    fn drop(&mut self) {
        if let Some(incarnation) = self.incarnation.take() {
            tracing::error!(
                guid = ?self.origin.guid,
                "creature mutation unwound; returning the lent incarnation to its map"
            );
            let _ = self.return_like_cpp(incarnation.creature);
        }
    }
}
