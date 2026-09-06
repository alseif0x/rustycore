use crate::storage::OpaqueStates;
use crate::{Executor, HostCore};
use conformance_contract::{
    ABI_VERSION, Fault, Handle, Host, Manifest, Module, Result, Snapshot, State, capability,
    decode_canonical,
};
use hecs::{Entity, EntityBuilder, World};
use std::any::TypeId;
use std::marker::PhantomData;

pub type NativeInvoke = fn(&mut dyn Host, u32, i64) -> Result<i64>;

/// Namespace is M, not M::State: two modules can use the same Rust state type.
struct ModuleState<M: Module> {
    value: M::State,
    revision: u64,
    marker: PhantomData<fn() -> M>,
}

#[derive(Clone)]
pub(crate) struct Registration {
    pub manifest: Manifest,
    pub type_id: Option<TypeId>,
    pub executor: Executor,
    pub storage: Storage,
}

#[derive(Clone)]
pub(crate) enum Storage {
    Native(NativeFns),
    Opaque { initial: Vec<u8> },
}

#[derive(Clone, Copy)]
pub(crate) struct NativeFns {
    pub invoke: NativeInvoke,
    pub default: fn(u64) -> Result<EntityBuilder>,
    pub decode: fn(u64, &[u8]) -> Result<EntityBuilder>,
    pub read: fn(&HostCore, Handle) -> Result<Snapshot>,
    pub invalidate: fn(&mut HostCore, Handle, u64) -> Result<()>,
    pub remove: fn(&mut World, Entity) -> Result<()>,
}

pub(crate) enum PreparedState {
    Native(EntityBuilder),
    Opaque { module: u64, snapshot: Snapshot },
}

impl PreparedState {
    pub(crate) fn add_to(self, builder: &mut EntityBuilder) {
        match self {
            Self::Native(mut component) => {
                builder.add_bundle(component.build());
            }
            Self::Opaque { module, snapshot } => {
                if !builder.has::<OpaqueStates>() {
                    builder.add(OpaqueStates::default());
                }
                builder
                    .get_mut::<&mut OpaqueStates>()
                    .expect("installed opaque state component")
                    .0
                    .insert(module, snapshot);
            }
        }
    }
}

impl Registration {
    pub(crate) fn initial(&self, revision: u64) -> Result<PreparedState> {
        match &self.storage {
            Storage::Native(functions) => (functions.default)(revision).map(PreparedState::Native),
            Storage::Opaque { initial } => Ok(PreparedState::Opaque {
                module: self.manifest.id,
                snapshot: Snapshot {
                    revision,
                    bytes: initial.clone(),
                },
            }),
        }
    }

    pub(crate) fn decode(&self, revision: u64, bytes: &[u8]) -> Result<PreparedState> {
        match &self.storage {
            Storage::Native(functions) => {
                (functions.decode)(revision, bytes).map(PreparedState::Native)
            }
            Storage::Opaque { .. } => Ok(PreparedState::Opaque {
                module: self.manifest.id,
                snapshot: Snapshot {
                    revision,
                    bytes: bytes.to_vec(),
                },
            }),
        }
    }

    pub(crate) fn read(&self, core: &HostCore, handle: Handle) -> Result<Snapshot> {
        match &self.storage {
            Storage::Native(functions) => (functions.read)(core, handle),
            Storage::Opaque { .. } => core.with_component::<OpaqueStates, _>(handle, |states| {
                states
                    .0
                    .get(&self.manifest.id)
                    .cloned()
                    .ok_or(Fault::Missing)
            })?,
        }
    }

    pub(crate) fn remove(&self, world: &mut World, entity: Entity) -> Result<()> {
        match &self.storage {
            Storage::Native(functions) => (functions.remove)(world, entity),
            Storage::Opaque { .. } => {
                world
                    .get::<&mut OpaqueStates>(entity)
                    .map_err(|_| Fault::Missing)?
                    .0
                    .remove(&self.manifest.id)
                    .ok_or(Fault::Missing)?;
                Ok(())
            }
        }
    }

    pub(crate) fn invalidate(
        &self,
        core: &mut HostCore,
        handle: Handle,
        revision: u64,
    ) -> Result<()> {
        match &self.storage {
            Storage::Native(functions) => (functions.invalidate)(core, handle, revision),
            Storage::Opaque { .. } => {
                core.with_component_mut::<OpaqueStates, _>(handle, |states| {
                    states
                        .0
                        .get_mut(&self.manifest.id)
                        .ok_or(Fault::Missing)?
                        .revision = revision;
                    Ok(())
                })?
            }
        }
    }
}

fn bundle<M: Module>(value: M::State, revision: u64) -> EntityBuilder {
    let mut builder = EntityBuilder::new();
    builder.add(ModuleState::<M> {
        value,
        revision,
        marker: PhantomData,
    });
    builder
}

fn read<M: Module>(core: &HostCore, handle: Handle) -> Result<Snapshot> {
    core.with_component::<ModuleState<M>, _>(handle, |state| Snapshot {
        revision: state.revision,
        bytes: state.value.encode(),
    })
}

impl HostCore {
    pub fn register<M: Module>(&mut self) -> Result<()> {
        self.require_idle()?;
        let manifest = M::manifest();
        let type_id = TypeId::of::<M>();
        self.preflight_registration(manifest, Some(type_id))?;
        if manifest.schema != M::State::SCHEMA {
            return Err(Fault::Version);
        }
        let default = M::State::default().encode();
        self.check_bytes(manifest, &default)?;
        decode_canonical::<M::State>(&default, manifest.state_limit)?;
        let registration = Registration {
            manifest,
            type_id: Some(type_id),
            executor: Executor::Native,
            storage: Storage::Native(NativeFns {
                invoke: M::invoke,
                default: |revision| {
                    let bytes = M::State::default().encode();
                    let value = decode_canonical::<M::State>(&bytes, M::manifest().state_limit)?;
                    Ok(bundle::<M>(value, revision))
                },
                decode: |revision, bytes| {
                    let state = decode_canonical::<M::State>(bytes, M::manifest().state_limit)?;
                    Ok(bundle::<M>(state, revision))
                },
                read: read::<M>,
                invalidate: |core, handle, revision| {
                    core.with_component_mut::<ModuleState<M>, _>(handle, |state| {
                        state.revision = revision
                    })
                },
                remove: |world, entity| {
                    world
                        .remove_one::<ModuleState<M>>(entity)
                        .map(drop)
                        .map_err(|_| Fault::Missing)
                },
            }),
        };
        // Registration and per-entity membership are separate. Existing entities stay unchanged.
        self.modules.insert(manifest.id, registration);
        Ok(())
    }

    fn preflight_registration(&self, manifest: Manifest, type_id: Option<TypeId>) -> Result<()> {
        if manifest.id == 0 || manifest.name.is_empty() {
            return Err(Fault::Invalid);
        }
        if manifest.abi != ABI_VERSION || manifest.schema == 0 {
            return Err(Fault::Version);
        }
        if manifest.capabilities & !capability::ALL != 0 {
            return Err(Fault::Capability);
        }
        if manifest.state_limit > self.limits.state_bytes
            || self.modules.len() >= self.limits.modules
        {
            return Err(Fault::Limit);
        }
        if self.modules.values().any(|entry| {
            entry.manifest.id == manifest.id
                || (type_id.is_some() && entry.type_id == type_id)
                || entry.manifest.name == manifest.name
                || (manifest.exclusive.is_some() && entry.manifest.exclusive == manifest.exclusive)
        }) {
            return Err(Fault::Conflict);
        }
        Ok(())
    }

    pub(crate) fn preflight_opaque_registration(&self, manifest: Manifest) -> Result<()> {
        self.require_idle()?;
        self.preflight_registration(manifest, None)
    }

    /// Adapter-only installation after validating initial bytes with the guest codec.
    /// This is not a public way to bypass codec validation.
    pub(crate) fn register_opaque(&mut self, manifest: Manifest, initial: &[u8]) -> Result<()> {
        self.preflight_opaque_registration(manifest)?;
        self.check_bytes(manifest, initial)?;
        self.modules.insert(
            manifest.id,
            Registration {
                manifest,
                type_id: None,
                executor: Executor::Wasm,
                storage: Storage::Opaque {
                    initial: initial.to_vec(),
                },
            },
        );
        Ok(())
    }

    pub fn registered(&self) -> Vec<Manifest> {
        let mut manifests: Vec<_> = self.modules.values().map(|entry| entry.manifest).collect();
        manifests.sort_by_key(|manifest| (manifest.order, manifest.id));
        manifests
    }

    pub fn entity_modules(&self, handle: Handle) -> Result<Vec<u64>> {
        let owner = self.owner(handle)?;
        Ok(self
            .registered()
            .into_iter()
            .filter_map(|manifest| owner.modules.contains(&manifest.id).then_some(manifest.id))
            .collect())
    }

    pub fn executor(&self, module: u64) -> Result<Executor> {
        Ok(self.modules.get(&module).ok_or(Fault::Missing)?.executor)
    }

    /// This native-only stage rejects unsupported switches before any state changes.
    /// A real adapter must supply and validate its executor before widening this contract.
    pub fn switch_executor(&mut self, module: u64, executor: Executor) -> Result<()> {
        self.require_idle()?;
        let registered = self.modules.get(&module).ok_or(Fault::Missing)?;
        if registered.executor != executor {
            return Err(Fault::Version);
        }
        Ok(())
    }

    pub fn native_invoker(&self, module: u64) -> Result<NativeInvoke> {
        let registered = self.modules.get(&module).ok_or(Fault::Missing)?;
        if registered.executor != Executor::Native {
            return Err(Fault::Version);
        }
        match &registered.storage {
            Storage::Native(functions) => Ok(functions.invoke),
            Storage::Opaque { .. } => Err(Fault::Version),
        }
    }

    pub fn state(&self, handle: Handle, module: u64) -> Result<Snapshot> {
        self.owner(handle)?;
        let registered = self.modules.get(&module).ok_or(Fault::Missing)?;
        let snapshot = registered.read(self, handle)?;
        self.check_bytes(registered.manifest, &snapshot.bytes)?;
        Ok(snapshot)
    }

    pub(crate) fn check_bytes(&self, manifest: Manifest, bytes: &[u8]) -> Result<()> {
        if bytes.len() > manifest.state_limit || bytes.len() > self.limits.state_bytes {
            Err(Fault::Limit)
        } else {
            Ok(())
        }
    }

    pub(crate) fn preflight_write_state(
        &self,
        handle: Handle,
        module: u64,
        expected: u64,
        bytes: &[u8],
    ) -> Result<()> {
        let registered = self.modules.get(&module).ok_or(Fault::Missing)?;
        let current = self.state(handle, module)?;
        if current.revision != expected {
            return Err(Fault::Revision);
        }
        self.check_bytes(registered.manifest, bytes)?;
        self.ensure_trace_capacity(1)?;
        self.revision_clock.checked_add(1).ok_or(Fault::Overflow)?;
        Ok(())
    }

    pub(crate) fn write_state(
        &mut self,
        handle: Handle,
        module: u64,
        expected: u64,
        bytes: &[u8],
    ) -> Result<()> {
        self.preflight_write_state(handle, module, expected, bytes)?;
        let registered = self.modules.get(&module).ok_or(Fault::Missing)?;
        // A rejected codec must not consume a clock value: native and opaque validation
        // take place at different adapter layers but share the same successful sequence.
        let revision = self.revision_clock.checked_add(1).ok_or(Fault::Overflow)?;
        let component = registered.decode(revision, bytes)?;
        self.install_state(handle, component)?;
        self.revision_clock = revision;
        self.push_trace(crate::Trace::Write {
            handle,
            module,
            revision,
            bytes: bytes.to_vec(),
        })
    }

    pub(crate) fn install_state(&mut self, handle: Handle, state: PreparedState) -> Result<()> {
        match state {
            PreparedState::Native(component) => self.insert_bundle(handle, component),
            PreparedState::Opaque { module, snapshot } => self
                .with_component_mut::<OpaqueStates, _>(handle, |states| {
                    states.0.insert(module, snapshot);
                }),
        }
    }
}
