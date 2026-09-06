use crate::{Contribution, HostCore, Observables, Residence};
use conformance_contract::{Fault, Handle, Result};
use hecs::{Component, Entity, EntityBuilder};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct Owner {
    pub handle: Handle,
    pub slot: Slot,
    pub modules: BTreeSet<u64>,
}

pub(crate) enum Slot {
    Active { map: u8, entity: Entity },
    Detached(EntityBuilder),
}

/// Deliberately non-Clone payload: transfers must move the real allocation.
pub(crate) struct Payload {
    pub sentinel: u64,
}

pub(crate) struct CoreComponent {
    pub payload: Box<Payload>,
    pub contributions: BTreeMap<u64, Contribution>,
    pub revision: u64,
}

/// Wasm has no Rust type identity; its owned wire state is namespaced by checked module ID.
/// This component is canonical, not a mirror of guest memory or the typed native state.
#[derive(Default)]
pub(crate) struct OpaqueStates(pub BTreeMap<u64, conformance_contract::Snapshot>);

impl HostCore {
    pub(crate) fn owner(&self, handle: Handle) -> Result<&Owner> {
        match self.owners.get(&handle.guid) {
            Some(owner) if owner.handle == handle => Ok(owner),
            _ => Err(Fault::Stale),
        }
    }

    pub(crate) fn with_component<T: Component, R>(
        &self,
        handle: Handle,
        read: impl FnOnce(&T) -> R,
    ) -> Result<R> {
        match &self.owner(handle)?.slot {
            Slot::Active { map, entity } => {
                let value = self.worlds[usize::from(*map)]
                    .get::<&T>(*entity)
                    .map_err(|_| Fault::Missing)?;
                Ok(read(&value))
            }
            Slot::Detached(builder) => Ok(read(builder.get::<&T>().ok_or(Fault::Missing)?)),
        }
    }

    pub(crate) fn with_component_mut<T: Component, R>(
        &mut self,
        handle: Handle,
        write: impl FnOnce(&mut T) -> R,
    ) -> Result<R> {
        self.owner(handle)?;
        let owner = self.owners.get_mut(&handle.guid).ok_or(Fault::Stale)?;
        match &mut owner.slot {
            Slot::Active { map, entity } => {
                let mut value = self.worlds[usize::from(*map)]
                    .get::<&mut T>(*entity)
                    .map_err(|_| Fault::Missing)?;
                Ok(write(&mut value))
            }
            Slot::Detached(builder) => {
                Ok(write(builder.get_mut::<&mut T>().ok_or(Fault::Missing)?))
            }
        }
    }

    /// Owns all components on return; no borrowed bundle escapes a source World.
    pub(crate) fn take_slot(&mut self, slot: Slot) -> EntityBuilder {
        match slot {
            Slot::Detached(builder) => builder,
            Slot::Active { map, entity } => {
                let mut builder = EntityBuilder::new();
                builder.add_bundle(
                    self.worlds[usize::from(map)]
                        .take(entity)
                        .expect("private owner graph"),
                );
                builder
            }
        }
    }

    pub(crate) fn insert_bundle(
        &mut self,
        handle: Handle,
        mut bundle: EntityBuilder,
    ) -> Result<()> {
        self.owner(handle)?;
        let owner = self.owners.get_mut(&handle.guid).ok_or(Fault::Stale)?;
        match &mut owner.slot {
            Slot::Active { map, entity } => self.worlds[usize::from(*map)]
                .insert(*entity, bundle.build())
                .map_err(|_| Fault::Stale),
            Slot::Detached(builder) => {
                builder.add_bundle(bundle.build());
                Ok(())
            }
        }
    }

    pub fn residence(&self, handle: Handle) -> Result<Residence> {
        Ok(match self.owner(handle)?.slot {
            Slot::Active { map, .. } => Residence::Active(map),
            Slot::Detached(_) => Residence::Detached,
        })
    }

    pub fn observables(&self, handle: Handle) -> Result<Observables> {
        let residence = self.residence(handle)?;
        self.with_component::<CoreComponent, _>(handle, |core| Observables {
            handle,
            residence,
            payload_sentinel: core.payload.sentinel,
            shield: core.contributions.values().any(|value| value.shield),
            summons: core.contributions.values().map(|value| value.summons).sum(),
            contribution: core.contributions.values().map(|value| value.amount).sum(),
            by_module: core
                .contributions
                .iter()
                .map(|(id, value)| (*id, *value))
                .collect(),
        })
    }

    /// Diagnostic only: proves the non-Clone Box allocation survives transfer.
    pub fn payload_identity(&self, handle: Handle) -> Result<usize> {
        self.with_component::<CoreComponent, _>(handle, |core| {
            (&*core.payload as *const Payload) as usize
        })
    }
}
