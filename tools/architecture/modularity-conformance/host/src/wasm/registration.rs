use super::{Guest, WasmColdTiming, WasmRuntime, execution_fault, imports, validation};
use conformance_contract::{FUEL, Fault, MAX_STATE_BYTES, Manifest, Result};
use std::time::Instant;
use wasmtime::{AsContextMut, Instance, Module};

const MAX_GUEST_BYTES: usize = 4 * 1024 * 1024;

impl WasmRuntime {
    /// Validate metadata before activating the executable or its canonical state.
    /// Store allocations from rejected instances remain bounded but are not reclaimed
    /// individually; replacing the runtime reclaims them. This is not a hot-reload API.
    pub fn register_wasm(&mut self, manifest: Manifest, bytes: &[u8]) -> Result<()> {
        let mut timing = WasmColdTiming {
            module_id: manifest.id,
            compile_ns: None,
            instantiate_ns: None,
            metadata_ns: None,
            fault: None,
        };
        let result = self.load(manifest, bytes, &mut timing);
        timing.fault = result.as_ref().err().copied();
        self.cold_timings.push(timing);
        result
    }

    fn load(
        &mut self,
        manifest: Manifest,
        bytes: &[u8],
        timing: &mut WasmColdTiming,
    ) -> Result<()> {
        self.store.data().core.require_idle()?;
        self.store
            .data()
            .core
            .preflight_opaque_registration(manifest)?;
        if bytes.len() > MAX_GUEST_BYTES {
            return Err(Fault::Limit);
        }
        if !bytes.starts_with(b"\0asm") {
            return Err(Fault::Invalid);
        }
        let started = Instant::now();
        let compiled = Module::new(&self.engine, bytes);
        timing.compile_ns = Some(started.elapsed().as_nanos());
        let module = compiled.map_err(execution_fault)?;
        let linker = imports::linker(&self.engine)?;
        // Metadata and initial-state evaluation share one finite registration budget.
        // With no invocation frame, even a hostile start/metadata function cannot
        // obtain authority to mutate an entity through our imports.
        self.store.data_mut().core.begin_root()?;
        self.store.set_fuel(FUEL).map_err(execution_fault)?;
        let result = (|| {
            let started = Instant::now();
            let instantiated = linker.instantiate(&mut self.store, &module);
            timing.instantiate_ns = Some(started.elapsed().as_nanos());
            let instance = instantiated.map_err(execution_fault)?;
            let started = Instant::now();
            let result = self.validate_and_register(manifest, instance);
            timing.metadata_ns = Some(started.elapsed().as_nanos());
            result
        })();
        self.store.data().core.end_root();
        result
    }

    fn validate_and_register(&mut self, manifest: Manifest, instance: Instance) -> Result<()> {
        let memory = instance
            .get_memory(&mut self.store, "memory")
            .ok_or(Fault::Version)?;
        let memory_type = memory.ty(&self.store);
        if memory_type.is_64() || memory_type.is_shared() {
            return Err(Fault::Version);
        }
        macro_rules! metadata {
            ($name:literal, $ty:ty) => {
                instance
                    .get_typed_func::<(), $ty>(&mut self.store, $name)
                    .map_err(|_| Fault::Version)?
                    .call(&mut self.store, ())
                    .map_err(execution_fault)?
            };
        }
        let abi = metadata!("abi_version", u32);
        let id = metadata!("module_id", u64);
        let schema = metadata!("state_schema", u32);
        let capabilities = metadata!("capabilities", u64);
        let state_limit = metadata!("state_limit", u32);
        let order = metadata!("module_order", i32);
        if abi != manifest.abi
            || id != manifest.id
            || schema != manifest.schema
            || capabilities != manifest.capabilities
            || state_limit as usize != manifest.state_limit
            || order != manifest.order
        {
            return Err(Fault::Version);
        }
        let invoke = instance
            .get_typed_func::<(u32, i64), i64>(&mut self.store, "invoke")
            .map_err(|_| Fault::Version)?;
        let length = metadata!("initial_state_len", i32);
        if length < 0 {
            return Err(Fault::from_code(i64::from(length)));
        }
        let length = length as usize;
        if length > MAX_STATE_BYTES || length > manifest.state_limit {
            return Err(Fault::Limit);
        }
        let byte = instance
            .get_typed_func::<u32, i32>(&mut self.store, "initial_state_byte")
            .map_err(|_| Fault::Version)?;
        let mut initial = Vec::with_capacity(length);
        for index in 0..length {
            let value = byte
                .call(&mut self.store, index as u32)
                .map_err(execution_fault)?;
            initial.push(u8::try_from(value).map_err(|_| Fault::Invalid)?);
        }
        let validate = instance
            .get_typed_func::<u32, i32>(&mut self.store, "validate_state")
            .map_err(|_| Fault::Version)?;
        validation::candidate(
            self.store.as_context_mut(),
            manifest.id,
            validate.clone(),
            &initial,
        )?;
        self.store
            .data_mut()
            .core
            .register_opaque(manifest, &initial)?;
        self.store.data_mut().guests.insert(
            manifest.id,
            Guest {
                instance,
                invoke,
                validate,
            },
        );
        Ok(())
    }
}
