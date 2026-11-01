use crate::session::creature_canonical_adapter::{
    CreatureMutationRootCaptureLikeCpp, creature_threat_guids_on_map_like_cpp,
    emit_creature_mutation_root_capture_like_cpp,
    reconcile_creature_threat_references_on_map_like_cpp,
};
use crate::session::state::SessionCore;
use wow_core::ObjectGuid;
use wow_loot::OwnedLootAuthority;

impl SessionCore {
    /// Read one value from the session's represented creature *without*
    /// mutating it or publishing anything.
    ///
    /// `f` observes a clone of the canonical incarnation at the session's
    /// current map key, taken under the canonical execution lock and released
    /// before `f` runs, so `f` holds no guard. A caller that needs the
    /// `CreatureCreateData` packet projection computes it on demand
    /// (`creature_projection::creature_create_data_like_cpp`). #1263 F6-8E: the canonical map
    /// is the only creature store.
    ///
    /// `None` means the canonical map does not hold the creature at the
    /// current map key.
    pub fn read_world_creature_like_cpp<F, R>(&self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&wow_entities::Creature) -> R,
    {
        let (map_id, instance_id) = self.current_runtime_map_key_like_cpp();
        let creature = {
            let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
            manager
                .find_map(u32::from(map_id), instance_id)?
                .map()
                .get_typed_creature(guid)?
                .clone()
        };
        Some(f(&creature))
    }

    /// Run one pure read against the canonical incarnation at the session's
    /// current map key **inside** the canonical execution lock, without
    /// cloning it. `f` must not take another manager lock, perform I/O,
    /// deliver or await.
    pub fn with_represented_creature_like_cpp<F, R>(&self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&wow_entities::Creature) -> R,
    {
        let (map_id, instance_id) = self.current_runtime_map_key_like_cpp();
        let manager = self.canonical_map_manager.as_ref()?.lock().ok()?;
        let creature = manager
            .find_map(u32::from(map_id), instance_id)?
            .map()
            .get_typed_creature(guid)?;
        Some(f(creature))
    }

    /// Execute one creature mutation against the current canonical
    /// incarnation.
    ///
    /// The mutation runs at most once, on the incarnation the canonical map
    /// holds at the session's current map key, inside the canonical execution
    /// lock. A missing canonical map or incarnation refuses without invoking
    /// it. No I/O, delivery, await or manager re-entry happens inside the
    /// guard. Callers that only observe a creature belong on
    /// [`Self::read_world_creature_like_cpp`] instead.
    pub fn mutate_world_creature<F, R>(&mut self, guid: ObjectGuid, f: F) -> Option<R>
    where
        F: FnOnce(&mut wow_entities::Creature) -> R,
    {
        self.with_admitted_world_creature_like_cpp(guid, |creature| Some(f(creature)))
    }

    /// Execute one admitted mutation guarded by a fully-looted lifecycle
    /// observation of the owner's loot authority.
    ///
    /// F6-7 R7b-2a. This is the viewed-release sibling of the gated mutation
    /// root: `SessionCore::with_admitted_world_creature_like_cpp` lends the
    /// canonical incarnation under its lock, and the authority's own
    /// observation supplies the loot admission — `authority` must still be the
    /// incarnation's allocation,
    /// and the whole-owner generation, lifecycle revision and fully-looted pool
    /// topology must still be the observed ones. The observation and the
    /// mutation are one critical section, so a pool that reopened between the
    /// release and the mutation refuses the mutation instead of letting it land.
    ///
    /// `f` is invoked only once both admissions passed; a refusal returns `None`
    /// with the callback unexecuted and nothing written.
    pub fn mutate_world_creature_if_fully_looted_observation_like_cpp<F, R>(
        &mut self,
        guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut wow_entities::Creature) -> R,
    {
        self.with_admitted_world_creature_like_cpp(guid, |creature| {
            if !creature
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
        F: FnOnce(&mut wow_entities::Creature) -> R,
    {
        self.with_admitted_world_creature_like_cpp(guid, |creature| {
            if !creature
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
    /// C++ has one live `Creature` per GUID and every session mutation
    /// (`Unit::DealDamage`, `Creature::setDeathState`, loot release, threat,
    /// spell effects) writes that object. The canonical map's incarnation is
    /// that object here (#1263 F6-8D3b-2; F6-8E deleted the legacy copy): it is
    /// moved out of its map record under the canonical execution lock, lent to
    /// `mutation` as `&mut Creature` (moved out, no clone), and returned
    /// to the same map before the lock is released, whatever `mutation`
    /// returned. The other units' `threatened by me` references are then
    /// brought in line with its threat list.
    ///
    /// `mutation` refuses by returning `None` (the loot-lifecycle observation
    /// roots); the incarnation is still returned unchanged.
    ///
    /// #1263 F6-8D3b-3: the incarnation always returns to its map. A panic in
    /// `mutation` is caught, the incarnation is put back into its record and
    /// cell, the canonical guard is released (so the mutex the map loop needs
    /// is not poisoned), and the panic resumes; a guard also returns it if
    /// anything else unwinds. A mutation that rebinds the creature's map is
    /// refused loudly and the binding it was lent with restored. A position the
    /// mutation changed is applied as C++ `Unit::UpdatePosition` →
    /// `Map::CreatureRelocation`, so the grid cell follows (vehicle, possess and
    /// mind-control movers write it through `set_ai_position`). No I/O,
    /// delivery, await or manager re-entry happens inside the guard, and the
    /// authority lock a caller's `mutation` may take stays innermost.
    ///
    /// #1263 F6-8E-2: the transient `WorldCreature` view (the creature plus a
    /// `CreatureCreateData` projection computed on the spot) is gone; the
    /// mutation receives the canonical creature itself.
    fn with_admitted_world_creature_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        mutation: impl FnOnce(&mut wow_entities::Creature) -> Option<R>,
    ) -> Option<R> {
        // #1263 C1: the guarded body copies one outcome record and returns it;
        // the event is emitted here, after the canonical guard was released.
        let (result, capture) = self.with_admitted_world_creature_captured_like_cpp(guid, mutation);
        if let Some(capture) = capture {
            emit_creature_mutation_root_capture_like_cpp(&capture);
        }
        result
    }

    /// The guarded body of [`Self::with_admitted_world_creature_like_cpp`],
    /// with the #1263 C1 capture record returned instead of emitted so the
    /// caller can emit it after the canonical guard is released.
    fn with_admitted_world_creature_captured_like_cpp<R>(
        &mut self,
        guid: ObjectGuid,
        mutation: impl FnOnce(&mut wow_entities::Creature) -> Option<R>,
    ) -> (Option<R>, Option<CreatureMutationRootCaptureLikeCpp>) {
        const ROOT: &str = "SessionCore::with_admitted_world_creature_like_cpp";
        let (map_id, instance_id) = self.current_runtime_map_key_like_cpp();
        let Some(canonical_manager) = self.canonical_map_manager.as_ref().cloned() else {
            return (None, None);
        };
        let capture = |mutation_invoked: bool, application: &'static str, applied: bool| {
            Some(CreatureMutationRootCaptureLikeCpp {
                root: ROOT,
                owner_guid: guid,
                map_id,
                instance_id,
                mutation_invoked,
                application,
                applied,
            })
        };

        let mut canonical_manager = match canonical_manager.lock() {
            Ok(guard) => guard,
            Err(_) => return (None, capture(false, "owner_unavailable", false)),
        };
        let Some(managed) = canonical_manager.find_map_mut(u32::from(map_id), instance_id) else {
            return (None, capture(false, "owner_unavailable", false));
        };
        let map = managed.map_mut();
        if map.get_typed_creature(guid).is_none() {
            return (None, capture(false, "incarnation_absent", false));
        }
        let old_threat_guids = creature_threat_guids_on_map_like_cpp(map, guid);
        let Some((creature, origin)) = map.lend_creature_like_cpp(guid) else {
            return (None, capture(false, "incarnation_absent", false));
        };
        let mut lent = LentIncarnationLikeCpp {
            map,
            origin,
            old_threat_guids: Some(old_threat_guids),
            incarnation: Some(creature),
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
        let incarnation = lent.incarnation.take().expect("lent until returned");
        let returned = lent.return_like_cpp(incarnation);
        drop(lent);
        if returned != LentIncarnationReturnLikeCpp::Clean {
            return (None, capture(true, "canonical_application_refused", false));
        }
        match result {
            Some(result) => (Some(result), capture(true, "applied", true)),
            None => (None, capture(true, "mutation_root_refused", false)),
        }
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
    incarnation: Option<wow_entities::Creature>,
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
            let _ = self.return_like_cpp(incarnation);
        }
    }
}
