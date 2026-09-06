//! Private Core Wasm adapter: one Store owns one HostCore and all guest instances.
//! Native and guest callbacks share scoped operations and cumulative root budgets.
//! No WASI, raw store access, alternate gameplay owner, or native sandbox promise.

mod dispatch;
mod imports;
mod lifecycle;
mod probes;
mod registration;
mod validation;

use crate::{EntitySnapshot, HostCore, Observables};
use conformance_contract::{
    FUEL, Fault, Handle, MAX_MODULES, MEMORY_BYTES, Module, Result, Snapshot,
};
use std::collections::BTreeMap;
use std::time::Instant;
use wasmtime::{
    AsContextMut, Config, Engine, Instance, Store, StoreContextMut, StoreLimits,
    StoreLimitsBuilder, Trap, TypedFunc,
};

#[derive(Clone)]
struct Guest {
    instance: Instance,
    invoke: TypedFunc<(u32, i64), i64>,
    validate: TypedFunc<u32, i32>,
}

struct Data {
    core: HostCore,
    guests: BTreeMap<u64, Guest>,
    limits: StoreLimits,
    validation: Option<validation::Phase>,
    #[cfg(test)]
    refill_nested_fuel: bool,
}

pub struct WasmRuntime {
    engine: Engine,
    store: Store<Data>,
    engine_creation_ns: u128,
    cold_timings: Vec<WasmColdTiming>,
}

/// Instrumentation of attempted guest loads, including rejected attempts. None
/// means a phase was not reached, not that compilation/instantiation was free.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WasmColdTiming {
    pub module_id: u64,
    pub compile_ns: Option<u128>,
    pub instantiate_ns: Option<u128>,
    pub metadata_ns: Option<u128>,
    pub fault: Option<Fault>,
}

fn execution_fault(error: wasmtime::Error) -> Fault {
    match error.downcast_ref::<Trap>() {
        Some(Trap::OutOfFuel) => Fault::Limit,
        _ => Fault::Trap,
    }
}

fn decode_result(value: i64) -> Result<i64> {
    if value < 0 {
        Err(Fault::from_code(value))
    } else {
        Ok(value)
    }
}

impl WasmRuntime {
    pub fn new(core: HostCore) -> Result<Self> {
        core.require_idle()?;
        // Existing native registrations are supported. An opaque registration
        // without its executable must never become a silently skipped callback.
        for manifest in core.registered() {
            core.native_invoker(manifest.id)?;
        }
        let mut config = Config::new();
        config.consume_fuel(true);
        // The cap is per instance, not merely per declared/exported memory.
        config.wasm_multi_memory(false);
        let started = Instant::now();
        let engine = Engine::new(&config).map_err(execution_fault)?;
        let engine_creation_ns = started.elapsed().as_nanos();
        let limits = StoreLimitsBuilder::new()
            .memory_size(MEMORY_BYTES)
            .memories(MAX_MODULES)
            .instances(MAX_MODULES)
            .tables(MAX_MODULES)
            .table_elements(256)
            .build();
        let mut store = Store::new(
            &engine,
            Data {
                core,
                guests: BTreeMap::new(),
                limits,
                validation: None,
                #[cfg(test)]
                refill_nested_fuel: false,
            },
        );
        store.limiter(|data| &mut data.limits);
        store.set_fuel(FUEL).map_err(execution_fault)?;
        Ok(Self {
            engine,
            store,
            engine_creation_ns,
            cold_timings: Vec::new(),
        })
    }

    pub fn core(&self) -> &HostCore {
        &self.store.data().core
    }

    pub fn engine_creation_ns(&self) -> u128 {
        self.engine_creation_ns
    }

    pub fn timings(&self) -> &[WasmColdTiming] {
        &self.cold_timings
    }

    pub fn register_native<M: Module>(&mut self) -> Result<()> {
        self.store.data_mut().core.register::<M>()
    }

    /// One transition budget, including every nested native/Wasm callback. Fuel
    /// does not interrupt arbitrary host/native CPU; callbacks must use bounded work.
    fn root<T>(&mut self, run: impl FnOnce(StoreContextMut<'_, Data>) -> Result<T>) -> Result<T> {
        self.store.data_mut().core.begin_root()?;
        self.store.set_fuel(FUEL).map_err(execution_fault)?;
        let result = run(self.store.as_context_mut());
        self.store.data().core.end_root();
        result
    }

    pub fn dispatch(
        &mut self,
        handle: Handle,
        event: u32,
        argument: i64,
    ) -> Result<Vec<(u64, i64)>> {
        self.root(|context| dispatch::all(context, handle, event, argument))
    }

    pub fn dispatch_one(
        &mut self,
        handle: Handle,
        module: u64,
        event: u32,
        argument: i64,
    ) -> Result<i64> {
        self.root(|context| dispatch::one(context, handle, module, event, argument))
    }

    /// Like HostCore::spawn, creation alone does not implicitly invoke ATTACHED.
    pub fn spawn(&mut self, guid: u64, map: u8) -> Result<Handle> {
        self.store.data_mut().core.spawn(guid, map)
    }

    pub fn spawn_with_modules(&mut self, guid: u64, map: u8, modules: &[u64]) -> Result<Handle> {
        self.store
            .data_mut()
            .core
            .spawn_with_modules(guid, map, modules)
    }

    pub fn state(&self, handle: Handle, module: u64) -> Result<Snapshot> {
        self.core().state(handle, module)
    }

    pub fn observables(&self, handle: Handle) -> Result<Observables> {
        self.core().observables(handle)
    }

    pub fn snapshot(&self, handle: Handle) -> Result<EntitySnapshot> {
        self.core().snapshot(handle)
    }

    /// Same-incarnation, versioned in-memory replay; not SQL durability.
    pub fn replay(&mut self, handle: Handle, snapshot: &EntitySnapshot) -> Result<()> {
        self.root(|mut context| {
            let mut plan = context.data().core.prepare_replay(handle, snapshot)?;
            while let Some(record) = plan.next_record().cloned() {
                if record.executor == crate::Executor::Wasm {
                    validation::registered(context.as_context_mut(), record.id, &record.bytes)?;
                }
                context.data().core.stage_replay_record(&mut plan)?;
            }
            context.data_mut().core.commit_replay(plan)
        })
    }

    pub fn switch_executor(&mut self, module: u64, executor: crate::Executor) -> Result<()> {
        self.store.data_mut().core.switch_executor(module, executor)
    }

    pub fn fuel_remaining(&self) -> Result<u64> {
        self.store.get_fuel().map_err(execution_fault)
    }
}

#[cfg(test)]
mod tests;
