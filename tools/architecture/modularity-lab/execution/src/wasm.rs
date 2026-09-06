use crate::{logic, model::Aggregate};
use std::{path::Path, time::Instant};
use wasmtime::{
    Caller, Config, Engine, Instance, Linker, Module, Result, Store, StoreLimits,
    StoreLimitsBuilder, TypedFunc, bail, ensure,
};

pub const ENGINE_VERSION: &str = "47.0.3";
pub const FUEL: u64 = 1_000_000;
pub const MEMORY_LIMIT: usize = 3 * 1024 * 1024;
pub const PAYLOAD_LIMIT: usize = 1024;
pub const DEPTH_LIMIT: usize = 4;

pub struct Data {
    pub aggregate: Aggregate,
    limits: StoreLimits,
    #[cfg(test)]
    pub refill_reentry_fuel: bool,
}

pub struct Compiled {
    pub engine: Engine,
    pub module: Module,
    pub cold_compile_ns: u128,
}

impl Compiled {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        ensure!(
            bytes.starts_with(b"\0asm"),
            "guest must be actual compiled core Wasm, not WAT"
        );
        let start = Instant::now();
        let mut config = Config::new();
        config.consume_fuel(true);
        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, &bytes)?;
        Ok(Self {
            engine,
            module,
            cold_compile_ns: start.elapsed().as_nanos(),
        })
    }
}

pub struct Wasm {
    pub store: Store<Data>,
    pub instance: Instance,
    invoke: TypedFunc<(u32, i64), i64>,
    pub instantiate_ns: u128,
}

impl Wasm {
    pub fn new(compiled: &Compiled) -> Result<Self> {
        Self::with_memory_limit(compiled, MEMORY_LIMIT)
    }

    #[cfg(test)]
    pub fn without_memory_cap(compiled: &Compiled) -> Result<Self> {
        Self::with_memory_limit(compiled, usize::MAX)
    }

    fn with_memory_limit(compiled: &Compiled, memory_limit: usize) -> Result<Self> {
        let start = Instant::now();
        let mut linker = Linker::<Data>::new(&compiled.engine);
        linker.func_wrap(
            "lab",
            "action",
            |mut caller: Caller<'_, Data>, op: u32, handle: u64, argument: i64| -> Result<i64> {
                let (value, reenter) = caller.data_mut().aggregate.action(op, handle, argument)?;
                if reenter
                    || op == logic::BURN_PROBE
                    || (op == logic::RECURSE_PROBE && argument > 0)
                {
                    ensure!(
                        caller.data().aggregate.depth < DEPTH_LIMIT,
                        "callback depth exhausted"
                    );
                    caller.data_mut().aggregate.depth += 1;
                    // Never retain caller.data_mut(), a memory slice or a guest-state reference
                    // while calling back into this SAME core-Wasm instance. Production never
                    // refills fuel; the cfg(test) branch deliberately injects that bug.
                    let result = (|| -> Result<i64> {
                        #[cfg(test)]
                        if caller.data().refill_reentry_fuel {
                            caller.set_fuel(FUEL)?;
                        }
                        if op == logic::BURN_PROBE {
                            let function = caller
                                .get_export("probe_burn")
                                .and_then(|e| e.into_func())
                                .ok_or_else(|| {
                                    wasmtime::format_err!("missing finite burn probe")
                                })?;
                            function
                                .typed::<u32, u64>(&caller)?
                                .call(&mut caller, argument as u32)
                                .map(|v| v as i64)
                        } else if op == logic::RECURSE_PROBE {
                            let function = caller
                                .get_export("probe_recurse")
                                .and_then(|e| e.into_func())
                                .ok_or_else(|| wasmtime::format_err!("missing recursion probe"))?;
                            function
                                .typed::<u32, i64>(&caller)?
                                .call(&mut caller, (argument - 1) as u32)
                        } else {
                            let function = caller
                                .get_export("invoke")
                                .and_then(|e| e.into_func())
                                .ok_or_else(|| wasmtime::format_err!("missing reentry export"))?;
                            function
                                .typed::<(u32, i64), i64>(&caller)?
                                .call(&mut caller, (logic::CALLBACK, 0))
                        }
                    })();
                    caller.data_mut().aggregate.depth -= 1;
                    let result = result?;
                    caller.data_mut().aggregate.callback_finished(result)?;
                    if op == logic::BURN_PROBE {
                        return Ok(result);
                    }
                }
                Ok(value)
            },
        )?;
        linker.func_wrap(
            "lab",
            "payload",
            |mut caller: Caller<'_, Data>, pointer: u32, length: u32| -> Result<i64> {
                let aggregate = &mut caller.data_mut().aggregate;
                aggregate.hostcall_attempts += 1;
                ensure!(aggregate.remaining_calls > 0, "host-call budget exhausted");
                aggregate.remaining_calls -= 1;
                ensure!(length as usize <= PAYLOAD_LIMIT, "oversize payload");
                let memory = caller
                    .get_export("memory")
                    .and_then(|e| e.into_memory())
                    .ok_or_else(|| wasmtime::format_err!("missing guest memory"))?;
                let end = (pointer as usize)
                    .checked_add(length as usize)
                    .ok_or_else(|| wasmtime::format_err!("payload range overflow"))?;
                ensure!(
                    end <= memory.data_size(&caller),
                    "payload outside guest memory"
                );
                // Bounded borrowed read, no allocation and no nested call while it exists.
                Ok(memory.data(&caller)[pointer as usize..end]
                    .iter()
                    .map(|b| i64::from(*b))
                    .sum())
            },
        )?;
        let limits = StoreLimitsBuilder::new()
            .memory_size(memory_limit)
            .memories(1)
            .instances(1)
            .tables(1)
            .table_elements(128)
            .build();
        let mut store = Store::new(
            &compiled.engine,
            Data {
                aggregate: Aggregate::default(),
                limits,
                #[cfg(test)]
                refill_reentry_fuel: false,
            },
        );
        store.limiter(|data| &mut data.limits);
        store.set_fuel(FUEL)?;
        let instance = linker.instantiate(&mut store, &compiled.module)?;
        let abi = instance
            .get_typed_func::<(), u32>(&mut store, "abi_version")?
            .call(&mut store, ())?;
        ensure!(abi == logic::ABI_VERSION, "unsupported guest ABI {abi}");
        let invoke = instance.get_typed_func(&mut store, "invoke")?;
        Ok(Self {
            store,
            instance,
            invoke,
            instantiate_ns: start.elapsed().as_nanos(),
        })
    }

    pub fn reset_budget(&mut self) -> Result<()> {
        self.store.data_mut().aggregate.begin_transition();
        self.store.set_fuel(FUEL)
    }

    pub fn invoke(&mut self, event: u32, argument: i64) -> Result<i64> {
        self.reset_budget()?;
        self.invoke.call(&mut self.store, (event, argument))
    }

    pub fn snapshot(&mut self) -> Result<u64> {
        self.instance
            .get_typed_func::<(), u64>(&mut self.store, "snapshot")?
            .call(&mut self.store, ())
    }

    pub fn restore(&mut self, schema: u32, state: u64) -> Result<i32> {
        self.instance
            .get_typed_func::<(u32, u64), i32>(&mut self.store, "restore")?
            .call(&mut self.store, (schema, state))
    }

    pub fn configure(&mut self, revision: u32, percent: u32) -> Result<()> {
        let result = self
            .instance
            .get_typed_func::<(u32, u32), i32>(&mut self.store, "configure")?
            .call(&mut self.store, (revision, percent))?;
        if result != 0 {
            bail!("configuration rejected");
        }
        Ok(())
    }
}
