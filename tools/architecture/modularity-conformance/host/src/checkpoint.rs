//! Versioned in-memory replay only. No DB, receipt, crash safety or cross-process proof.

use crate::registry::PreparedState;
use crate::storage::CoreComponent;
use crate::{EntitySnapshot, Executor, HostCore, SavedModule};
use conformance_contract::{Fault, Handle, Result, capability};
use std::collections::BTreeSet;

const SNAPSHOT_FORMAT: u32 = 1;

/// All storage and revisions remain unchanged until every ordered codec accepts.
/// The Wasm adapter owns this plan between calls into a guest validator.
pub(crate) struct ReplayPlan {
    snapshot: EntitySnapshot,
    clock: u64,
    final_revision: u64,
    staged: Vec<PreparedState>,
}

impl ReplayPlan {
    pub(crate) fn next_record(&self) -> Option<&SavedModule> {
        self.snapshot.modules.get(self.staged.len())
    }
}

impl HostCore {
    pub fn snapshot(&self, handle: Handle) -> Result<EntitySnapshot> {
        let mut modules = Vec::new();
        for module in self.entity_modules(handle)? {
            let registered = self.modules.get(&module).ok_or(Fault::Missing)?;
            let snapshot = self.state(handle, module)?;
            modules.push(SavedModule {
                id: module,
                abi: registered.manifest.abi,
                schema: registered.manifest.schema,
                executor: registered.executor,
                revision: snapshot.revision,
                bytes: snapshot.bytes,
            });
        }
        self.with_component::<CoreComponent, _>(handle, |core| EntitySnapshot {
            format: SNAPSHOT_FORMAT,
            handle,
            core_revision: core.revision,
            modules,
            contributions: core
                .contributions
                .iter()
                .map(|(id, value)| (*id, *value))
                .collect(),
        })
    }

    /// Restore a validated same-incarnation snapshot only if neither module state nor core
    /// effects have advanced. The new revisions make the consumed snapshot stale immediately.
    pub fn replay(&mut self, handle: Handle, snapshot: &EntitySnapshot) -> Result<()> {
        self.native_root(|core| core.replay_storage(handle, snapshot))
    }

    pub(crate) fn replay_storage(
        &mut self,
        handle: Handle,
        snapshot: &EntitySnapshot,
    ) -> Result<()> {
        let mut plan = self.prepare_replay(handle, snapshot)?;
        // HostCore has no guest executor and therefore cannot validate opaque codecs.
        if plan
            .snapshot
            .modules
            .iter()
            .any(|record| record.executor != Executor::Native)
        {
            return Err(Fault::Version);
        }
        while plan.next_record().is_some() {
            self.stage_replay_record(&mut plan)?;
        }
        self.commit_replay(plan)
    }

    pub(crate) fn prepare_replay(
        &self,
        handle: Handle,
        snapshot: &EntitySnapshot,
    ) -> Result<ReplayPlan> {
        let ordered = self.preflight_replay(handle, snapshot)?;
        let count = u64::try_from(ordered.len()).map_err(|_| Fault::Overflow)?;
        let final_revision = self
            .revision_clock
            .checked_add(count)
            .and_then(|value| value.checked_add(1))
            .ok_or(Fault::Overflow)?;
        let mut snapshot = snapshot.clone();
        snapshot.modules = ordered;
        Ok(ReplayPlan {
            snapshot,
            clock: self.revision_clock,
            final_revision,
            staged: Vec::new(),
        })
    }

    fn preflight_replay(
        &self,
        handle: Handle,
        snapshot: &EntitySnapshot,
    ) -> Result<Vec<SavedModule>> {
        self.require_idle()?;
        self.owner(handle)?;
        if snapshot.format != SNAPSHOT_FORMAT {
            return Err(Fault::Version);
        }
        if handle != snapshot.handle {
            return Err(Fault::Stale);
        }
        let ordered_ids = self.entity_modules(handle)?;
        let present: BTreeSet<_> = ordered_ids.iter().copied().collect();
        let supplied: BTreeSet<_> = snapshot.modules.iter().map(|module| module.id).collect();
        if supplied.len() != snapshot.modules.len() || supplied != present {
            return Err(Fault::Conflict);
        }
        let core_revision =
            self.with_component::<CoreComponent, _>(handle, |core| core.revision)?;
        if core_revision != snapshot.core_revision {
            return Err(Fault::Revision);
        }
        let contribution_ids: BTreeSet<_> =
            snapshot.contributions.iter().map(|(id, _)| *id).collect();
        if contribution_ids.len() != snapshot.contributions.len()
            || !contribution_ids.is_subset(&present)
        {
            return Err(Fault::Conflict);
        }
        let mut total_summons = 0u64;
        for (id, contribution) in &snapshot.contributions {
            if !(0..=1000).contains(&contribution.amount) {
                return Err(Fault::Invalid);
            }
            let manifest = self.modules.get(id).ok_or(Fault::Missing)?.manifest;
            let required = if contribution.shield {
                capability::SHIELD
            } else {
                0
            } | if contribution.summons != 0 {
                capability::SUMMON
            } else {
                0
            } | if contribution.amount != 0 {
                capability::CONTRIBUTION
            } else {
                0
            };
            if manifest.capabilities & required != required {
                return Err(Fault::Capability);
            }
            total_summons = total_summons
                .checked_add(contribution.summons)
                .ok_or(Fault::Overflow)?;
        }
        i64::try_from(total_summons).map_err(|_| Fault::Overflow)?;
        // Canonical order also fixes which decoder runs first if supplied records were shuffled.
        let mut ordered = Vec::with_capacity(ordered_ids.len());
        for id in ordered_ids {
            let module = snapshot
                .modules
                .iter()
                .find(|record| record.id == id)
                .ok_or(Fault::Conflict)?;
            let registered = self
                .modules
                .get(&module.id)
                .cloned()
                .ok_or(Fault::Missing)?;
            if module.abi != registered.manifest.abi
                || module.schema != registered.manifest.schema
                || module.executor != registered.executor
            {
                return Err(Fault::Version);
            }
            if self.state(handle, module.id)?.revision != module.revision {
                return Err(Fault::Revision);
            }
            self.check_bytes(registered.manifest, &module.bytes)?;
            ordered.push(module.clone());
        }
        Ok(ordered)
    }

    /// Call the guest codec immediately before this phase for an opaque record.
    /// Native decoding happens here, in that same order, without publishing revisions.
    pub(crate) fn stage_replay_record(&self, plan: &mut ReplayPlan) -> Result<()> {
        if self.revision_clock != plan.clock {
            return Err(Fault::Revision);
        }
        let record = plan.next_record().ok_or(Fault::Conflict)?;
        let registered = self.modules.get(&record.id).ok_or(Fault::Missing)?;
        let offset = u64::try_from(plan.staged.len()).map_err(|_| Fault::Overflow)?;
        let revision = plan
            .clock
            .checked_add(offset)
            .and_then(|value| value.checked_add(1))
            .ok_or(Fault::Overflow)?;
        let component = registered.decode(revision, &record.bytes)?;
        plan.staged.push(component);
        Ok(())
    }

    pub(crate) fn commit_replay(&mut self, plan: ReplayPlan) -> Result<()> {
        if self.revision_clock != plan.clock {
            return Err(Fault::Revision);
        }
        if plan.staged.len() != plan.snapshot.modules.len() {
            return Err(Fault::Conflict);
        }
        self.preflight_replay(plan.snapshot.handle, &plan.snapshot)?;
        // No callback or foreign code runs in this phase; the validated live components
        // cannot disappear between the preflight and replacement.
        for component in plan.staged {
            self.install_state(plan.snapshot.handle, component)?;
        }
        self.with_component_mut::<CoreComponent, _>(plan.snapshot.handle, |core| {
            core.contributions = plan.snapshot.contributions.iter().copied().collect();
            core.revision = plan.final_revision;
        })?;
        self.revision_clock = plan.final_revision;
        Ok(())
    }
}
