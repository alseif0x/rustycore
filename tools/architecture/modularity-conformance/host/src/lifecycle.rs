use crate::storage::{CoreComponent, OpaqueStates, Owner, Payload, Slot};
use crate::{HostCore, Residence};
use conformance_contract::{Fault, Handle, Result, event};
use hecs::{EntityBuilder, World};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct PreparedReplacement {
    original: Handle,
    replacement: Handle,
    map: u8,
    builder: EntityBuilder,
    modules: BTreeSet<u64>,
}

impl HostCore {
    fn check_map(&self, map: u8) -> Result<()> {
        if self.loaded.get(usize::from(map)).copied() == Some(true) {
            Ok(())
        } else {
            Err(Fault::Invalid)
        }
    }

    fn next_handle(&self, guid: u64) -> Result<Handle> {
        if guid == 0 {
            return Err(Fault::Invalid);
        }
        let previous = self.generations.get(&guid).copied().unwrap_or(0);
        Ok(Handle {
            guid,
            generation: previous.checked_add(1).ok_or(Fault::Overflow)?,
        })
    }

    fn initial_bundle(&mut self, handle: Handle, modules: &BTreeSet<u64>) -> Result<EntityBuilder> {
        let mut builder = EntityBuilder::new();
        builder.add(CoreComponent {
            payload: Box::new(Payload {
                sentinel: handle.guid,
            }),
            contributions: BTreeMap::new(),
            revision: self.next_revision()?,
        });
        builder.add(OpaqueStates::default());
        let registrations: Vec<_> = modules
            .iter()
            .map(|id| self.modules.get(id).cloned().ok_or(Fault::Missing))
            .collect::<Result<_>>()?;
        for registered in registrations {
            let revision = self.next_revision()?;
            registered.initial(revision)?.add_to(&mut builder);
        }
        Ok(builder)
    }

    /// Create an entity without implicit module execution. A harness can dispatch ATTACHED.
    pub fn spawn(&mut self, guid: u64, map: u8) -> Result<Handle> {
        let modules: Vec<_> = self.modules.keys().copied().collect();
        self.spawn_with_modules(guid, map, &modules)
    }

    pub fn spawn_with_modules(&mut self, guid: u64, map: u8, modules: &[u64]) -> Result<Handle> {
        self.require_idle()?;
        self.check_map(map)?;
        if self.owners.contains_key(&guid) {
            return Err(Fault::Conflict);
        }
        let selected: BTreeSet<_> = modules.iter().copied().collect();
        if selected.len() != modules.len() {
            return Err(Fault::Conflict);
        }
        if selected
            .iter()
            .any(|module| !self.modules.contains_key(module))
        {
            return Err(Fault::Missing);
        }
        let handle = self.next_handle(guid)?;
        let mut builder = self.initial_bundle(handle, &selected)?;
        let entity = self.worlds[usize::from(map)].spawn(builder.build());
        self.generations.insert(guid, handle.generation);
        self.owners.insert(
            guid,
            Owner {
                handle,
                slot: Slot::Active { map, entity },
                modules: selected,
            },
        );
        Ok(handle)
    }

    pub fn detach(&mut self, handle: Handle) -> Result<()> {
        self.native_root(|core| {
            core.preflight_native_lifecycle(handle)?;
            core.detach_storage(handle)?;
            core.dispatch_nested(handle, event::DETACHED, 0).map(drop)
        })
    }

    pub(crate) fn detach_storage(&mut self, handle: Handle) -> Result<()> {
        self.require_idle()?;
        self.owner(handle)?;
        if self.residence(handle)? == Residence::Detached {
            return Err(Fault::NotActive);
        }
        // All fallible preflight precedes moving the unique bundle out of the owner.
        let owner = self.owners.remove(&handle.guid).expect("validated owner");
        let builder = self.take_slot(owner.slot);
        self.owners.insert(
            handle.guid,
            Owner {
                handle,
                slot: Slot::Detached(builder),
                modules: owner.modules,
            },
        );
        Ok(())
    }

    pub fn attach(&mut self, handle: Handle, map: u8) -> Result<()> {
        self.native_root(|core| {
            core.preflight_native_lifecycle(handle)?;
            core.attach_storage(handle, map)?;
            core.dispatch_nested(handle, event::ATTACHED, i64::from(map))
                .map(drop)
        })
    }

    pub(crate) fn attach_storage(&mut self, handle: Handle, map: u8) -> Result<()> {
        self.require_idle()?;
        self.owner(handle)?;
        self.check_map(map)?;
        if self.residence(handle)? != Residence::Detached {
            return Err(Fault::Conflict);
        }
        let owner = self.owners.remove(&handle.guid).expect("validated owner");
        let mut builder = self.take_slot(owner.slot);
        let entity = self.worlds[usize::from(map)].spawn(builder.build());
        self.owners.insert(
            handle.guid,
            Owner {
                handle,
                slot: Slot::Active { map, entity },
                modules: owner.modules,
            },
        );
        Ok(())
    }

    pub fn reset(&mut self, handle: Handle, module: u64) -> Result<()> {
        self.native_root(|core| {
            core.preflight_native_lifecycle(handle)?;
            core.native_invoker(module)?;
            core.reset_storage(handle, module)?;
            core.invoke_native(handle, module, event::RESET, 0)
                .map(drop)
        })
    }

    pub(crate) fn reset_storage(&mut self, handle: Handle, module: u64) -> Result<()> {
        self.require_idle()?;
        self.owner(handle)?;
        self.state(handle, module)?;
        let registered = self.modules.get(&module).cloned().ok_or(Fault::Missing)?;
        let revision = self.next_revision()?;
        let core_revision = self.next_revision()?;
        // Reset is a module-defined transition, not a host-imposed Default. Preserve the
        // current typed/opaque value and revoke stale projections before the callback.
        registered.invalidate(self, handle, revision)?;
        self.with_component_mut::<CoreComponent, _>(handle, |core| {
            core.contributions.remove(&module);
            core.revision = core_revision;
        })
    }

    pub fn retire(&mut self, handle: Handle) -> Result<()> {
        self.native_root(|core| {
            core.preflight_native_lifecycle(handle)?;
            core.dispatch_nested(handle, event::REMOVING, 0)?;
            core.retire_storage(handle)
        })
    }

    pub(crate) fn retire_storage(&mut self, handle: Handle) -> Result<()> {
        self.require_idle()?;
        self.owner(handle)?;
        let owner = self.owners.remove(&handle.guid).expect("validated owner");
        drop(self.take_slot(owner.slot));
        // Keep generations tombstone; reusing a GUID must not revive an old Handle.
        Ok(())
    }

    pub fn replace(&mut self, handle: Handle, map: u8) -> Result<Handle> {
        self.native_root(|core| {
            core.preflight_native_lifecycle(handle)?;
            let prepared = core.prepare_replace(handle, map)?;
            core.dispatch_nested(handle, event::REMOVING, 0)?;
            let replacement = core.commit_replace(prepared)?;
            core.dispatch_nested(replacement, event::ATTACHED, i64::from(map))?;
            Ok(replacement)
        })
    }

    pub(crate) fn prepare_replace(
        &mut self,
        handle: Handle,
        map: u8,
    ) -> Result<PreparedReplacement> {
        self.require_idle()?;
        self.owner(handle)?;
        self.check_map(map)?;
        let replacement = self.next_handle(handle.guid)?;
        let modules = self.owner(handle)?.modules.clone();
        let builder = self.initial_bundle(replacement, &modules)?;
        Ok(PreparedReplacement {
            original: handle,
            replacement,
            map,
            builder,
            modules,
        })
    }

    /// Prepared bundle is owned: adapter builds it before executing REMOVING callbacks.
    pub(crate) fn commit_replace(&mut self, prepared: PreparedReplacement) -> Result<Handle> {
        self.require_idle()?;
        let PreparedReplacement {
            original: handle,
            replacement,
            map,
            mut builder,
            modules,
        } = prepared;
        let owner = self.owner(handle)?;
        self.check_map(map)?;
        if owner.modules != modules || self.next_handle(handle.guid)? != replacement {
            return Err(Fault::Conflict);
        }
        self.retire_storage(handle)?;
        let entity = self.worlds[usize::from(map)].spawn(builder.build());
        self.generations
            .insert(replacement.guid, replacement.generation);
        self.owners.insert(
            replacement.guid,
            Owner {
                handle: replacement,
                slot: Slot::Active { map, entity },
                modules,
            },
        );
        Ok(replacement)
    }

    pub fn unload_module(&mut self, module: u64) -> Result<()> {
        self.native_root(|core| {
            core.native_invoker(module)?;
            let handles: Vec<_> = core
                .owners
                .values()
                .filter(|owner| owner.modules.contains(&module))
                .map(|owner| owner.handle)
                .collect();
            for handle in &handles {
                core.preflight_native_lifecycle(*handle)?;
            }
            for handle in handles {
                core.invoke_native(handle, module, event::REMOVING, 0)?;
            }
            core.remove_module_storage(module)
        })
    }

    pub fn add_module_state(&mut self, handle: Handle, module: u64) -> Result<()> {
        self.native_root(|core| {
            core.preflight_native_lifecycle(handle)?;
            core.native_invoker(module)?;
            core.add_module_state_storage(handle, module)?;
            let (event, argument) = match core.residence(handle)? {
                Residence::Detached => (event::DETACHED, 0),
                Residence::Active(map) => (event::ATTACHED, i64::from(map)),
            };
            core.invoke_native(handle, module, event, argument)
                .map(drop)
        })
    }

    pub(crate) fn add_module_state_storage(&mut self, handle: Handle, module: u64) -> Result<()> {
        self.require_idle()?;
        if self.owner(handle)?.modules.contains(&module) {
            return Err(Fault::Conflict);
        }
        let registered = self.modules.get(&module).cloned().ok_or(Fault::Missing)?;
        let revision = self.next_revision()?;
        let component = registered.initial(revision)?;
        self.install_state(handle, component)?;
        self.owners
            .get_mut(&handle.guid)
            .expect("validated owner")
            .modules
            .insert(module);
        Ok(())
    }

    pub fn remove_module_state(&mut self, handle: Handle, module: u64) -> Result<()> {
        self.native_root(|core| {
            core.preflight_native_lifecycle(handle)?;
            core.state(handle, module)?;
            core.native_invoker(module)?;
            core.invoke_native(handle, module, event::REMOVING, 0)?;
            core.remove_module_state_storage(handle, module)
        })
    }

    pub(crate) fn remove_module_state_storage(
        &mut self,
        handle: Handle,
        module: u64,
    ) -> Result<()> {
        self.require_idle()?;
        self.state(handle, module)?;
        let registered = self.modules.get(&module).cloned().ok_or(Fault::Missing)?;
        let core_revision = self.next_revision()?;
        let owner = self.owners.get_mut(&handle.guid).expect("validated owner");
        match &mut owner.slot {
            Slot::Active { map, entity } => {
                registered.remove(&mut self.worlds[usize::from(*map)], *entity)?;
            }
            Slot::Detached(builder) => {
                // No clone or serialization: temporary storage provides selective removal.
                let mut temporary = World::new();
                let entity = temporary.spawn(builder.build());
                let result = registered.remove(&mut temporary, entity);
                builder.add_bundle(temporary.take(entity).expect("temporary entity exists"));
                result?;
            }
        }
        self.with_component_mut::<CoreComponent, _>(handle, |core| {
            core.contributions.remove(&module);
            core.revision = core_revision;
        })?;
        self.owners
            .get_mut(&handle.guid)
            .expect("validated owner")
            .modules
            .remove(&module);
        Ok(())
    }

    pub(crate) fn remove_module_storage(&mut self, module: u64) -> Result<()> {
        self.require_idle()?;
        self.modules.get(&module).ok_or(Fault::Missing)?;
        let handles: Vec<_> = self
            .owners
            .values()
            .filter(|owner| owner.modules.contains(&module))
            .map(|owner| owner.handle)
            .collect();
        // Validate every slot before removing any component.
        for handle in &handles {
            self.state(*handle, module)?;
        }
        for handle in handles {
            self.remove_module_state_storage(handle, module)?;
        }
        self.modules.remove(&module);
        Ok(())
    }

    pub fn unload_map(&mut self, map: u8) -> Result<()> {
        self.native_root(|core| {
            let handles = core.handles_in_map(map)?;
            for handle in &handles {
                core.preflight_native_lifecycle(*handle)?;
            }
            for handle in handles {
                core.detach_storage(handle)?;
                core.dispatch_nested(handle, event::DETACHED, 0)?;
            }
            core.mark_map_unloaded(map)
        })
    }

    pub(crate) fn handles_in_map(&self, map: u8) -> Result<Vec<Handle>> {
        self.check_map(map)?;
        Ok(self
            .owners
            .values()
            .filter(
                |owner| matches!(owner.slot, Slot::Active { map: current, .. } if current == map),
            )
            .map(|owner| owner.handle)
            .collect())
    }

    pub(crate) fn mark_map_unloaded(&mut self, map: u8) -> Result<()> {
        self.require_idle()?;
        if !self.handles_in_map(map)?.is_empty() {
            return Err(Fault::Conflict);
        }
        self.loaded[usize::from(map)] = false;
        Ok(())
    }

    pub fn load_map(&mut self, map: u8) -> Result<()> {
        self.require_idle()?;
        *self
            .loaded
            .get_mut(usize::from(map))
            .ok_or(Fault::Invalid)? = true;
        Ok(())
    }

    fn preflight_native_lifecycle(&self, handle: Handle) -> Result<()> {
        self.require_idle()?;
        self.owner(handle)?;
        for module in self.entity_modules(handle)? {
            self.native_invoker(module)?;
        }
        Ok(())
    }
}
