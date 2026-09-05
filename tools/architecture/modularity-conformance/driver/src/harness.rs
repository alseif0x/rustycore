//! Frozen mode/executor harness. New modules may not change this adapter.

use conformance_contract::{Fault, Handle, Manifest, Module, Result};
use conformance_host::{EntitySnapshot, Executor, HostCore, Limits};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mode {
    Native,
    RustWasm,
    CWasm,
    Mixed,
}

impl Mode {
    pub const ALL: [Self; 4] = [Self::Native, Self::RustWasm, Self::CWasm, Self::Mixed];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::RustWasm => "rust-wasm",
            Self::CWasm => "c-wasm",
            Self::Mixed => "mixed",
        }
    }
}

impl FromStr for Mode {
    type Err = String;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|mode| mode.as_str() == value)
            .ok_or_else(|| format!("unsupported mode {value:?}"))
    }
}

/// Dependency/registration descriptors, not module-specific storage or lifecycle machinery.
pub struct Descriptor {
    pub manifest: fn() -> Manifest,
    pub register_native: fn(&mut HostCore) -> Result<()>,
    #[cfg(feature = "wasm")]
    pub register_native_mixed: fn(&mut conformance_host::WasmRuntime) -> Result<()>,
    pub rust_artifact: &'static str,
    pub c_artifact: &'static str,
}

pub fn descriptor<M: Module>(rust_artifact: &'static str, c_artifact: &'static str) -> Descriptor {
    Descriptor {
        manifest: M::manifest,
        register_native: HostCore::register::<M>,
        #[cfg(feature = "wasm")]
        register_native_mixed: conformance_host::WasmRuntime::register_native::<M>,
        rust_artifact,
        c_artifact,
    }
}

enum Inner {
    Native(HostCore),
    #[cfg(feature = "wasm")]
    Wasm(conformance_host::WasmRuntime),
}

pub struct Harness {
    inner: Inner,
}

macro_rules! delegate {
    ($self:ident, $method:ident $(, $argument:expr)*) => {
        match &mut $self.inner {
            Inner::Native(core) => core.$method($($argument),*),
            #[cfg(feature = "wasm")]
            Inner::Wasm(runtime) => runtime.$method($($argument),*),
        }
    };
}

impl Harness {
    pub fn cold_metrics(&self) -> serde_json::Value {
        let expected_wasm_modules: Vec<_> = self
            .core()
            .registered()
            .iter()
            .filter(|m| self.core().executor(m.id) == Ok(Executor::Wasm))
            .map(|m| m.id)
            .collect();
        let mut value = match &self.inner {
            Inner::Native(_) => serde_json::json!({
                "engine_creation_ns": null, "artifacts": [],
                "boundary": "native build cost is not measured by runtime construction",
            }),
            #[cfg(feature = "wasm")]
            Inner::Wasm(runtime) => serde_json::json!({
                "engine_creation_ns": runtime.engine_creation_ns(),
                "artifacts": runtime.timings().iter().map(|timing| serde_json::json!({
                    "module_id": timing.module_id,
                    "compile_ns": timing.compile_ns,
                    "instantiate_ns": timing.instantiate_ns,
                    "metadata_ns": timing.metadata_ns,
                    "fault": timing.fault.map(|fault| format!("{fault:?}")),
                })).collect::<Vec<_>>(),
            }),
        };
        value["expected_wasm_modules"] = serde_json::json!(expected_wasm_modules);
        value
    }

    pub fn core(&self) -> &HostCore {
        match &self.inner {
            Inner::Native(core) => core,
            #[cfg(feature = "wasm")]
            Inner::Wasm(runtime) => runtime.core(),
        }
    }

    pub fn spawn(&mut self, guid: u64, map: u8) -> Result<Handle> {
        delegate!(self, spawn, guid, map)
    }

    pub fn spawn_with_modules(&mut self, guid: u64, map: u8, modules: &[u64]) -> Result<Handle> {
        delegate!(self, spawn_with_modules, guid, map, modules)
    }

    pub fn dispatch(
        &mut self,
        handle: Handle,
        event: u32,
        argument: i64,
    ) -> Result<Vec<(u64, i64)>> {
        delegate!(self, dispatch, handle, event, argument)
    }

    pub fn dispatch_one(
        &mut self,
        handle: Handle,
        module: u64,
        event: u32,
        argument: i64,
    ) -> Result<i64> {
        delegate!(self, dispatch_one, handle, module, event, argument)
    }

    pub fn detach(&mut self, handle: Handle) -> Result<()> {
        delegate!(self, detach, handle)
    }

    pub fn attach(&mut self, handle: Handle, map: u8) -> Result<()> {
        delegate!(self, attach, handle, map)
    }

    pub fn reset(&mut self, handle: Handle, module: u64) -> Result<()> {
        delegate!(self, reset, handle, module)
    }

    pub fn retire(&mut self, handle: Handle) -> Result<()> {
        delegate!(self, retire, handle)
    }

    pub fn replace(&mut self, handle: Handle, map: u8) -> Result<Handle> {
        delegate!(self, replace, handle, map)
    }

    pub fn unload_module(&mut self, module: u64) -> Result<()> {
        delegate!(self, unload_module, module)
    }

    pub fn add_module_state(&mut self, handle: Handle, module: u64) -> Result<()> {
        delegate!(self, add_module_state, handle, module)
    }

    pub fn remove_module_state(&mut self, handle: Handle, module: u64) -> Result<()> {
        delegate!(self, remove_module_state, handle, module)
    }

    pub fn unload_map(&mut self, map: u8) -> Result<()> {
        delegate!(self, unload_map, map)
    }

    pub fn load_map(&mut self, map: u8) -> Result<()> {
        delegate!(self, load_map, map)
    }

    pub fn replay(&mut self, handle: Handle, snapshot: &EntitySnapshot) -> Result<()> {
        delegate!(self, replay, handle, snapshot)
    }

    pub fn switch_executor(&mut self, module: u64, executor: Executor) -> Result<()> {
        delegate!(self, switch_executor, module, executor)
    }

    #[cfg(feature = "wasm")]
    pub fn wasm(&mut self) -> Result<&mut conformance_host::WasmRuntime> {
        match &mut self.inner {
            Inner::Native(_) => Err(Fault::Version),
            Inner::Wasm(runtime) => Ok(runtime),
        }
    }
}

// Only immutable convenience access is delegated: mutation must use the admitted executor.
impl std::ops::Deref for Harness {
    type Target = HostCore;

    fn deref(&self) -> &Self::Target {
        self.core()
    }
}

pub fn empty(mode: Mode) -> Result<Harness> {
    empty_with_limits(mode, Limits::default())
}

pub fn empty_with_limits(mode: Mode, limits: Limits) -> Result<Harness> {
    let core = HostCore::new(limits);
    let inner = match mode {
        Mode::Native => Inner::Native(core),
        _ => {
            #[cfg(feature = "wasm")]
            {
                Inner::Wasm(conformance_host::WasmRuntime::new(core)?)
            }
            #[cfg(not(feature = "wasm"))]
            {
                return Err(Fault::Version);
            }
        }
    };
    Ok(Harness { inner })
}

pub fn compose(mode: Mode, limits: Limits, modules: &[Descriptor]) -> Result<Harness> {
    let mut host = empty_with_limits(mode, limits)?;
    for (index, module) in modules.iter().enumerate() {
        match &mut host.inner {
            Inner::Native(core) => (module.register_native)(core)?,
            #[cfg(feature = "wasm")]
            Inner::Wasm(runtime) => {
                if mode == Mode::Mixed && index.is_multiple_of(3) {
                    (module.register_native_mixed)(runtime)?;
                } else {
                    let use_c = mode == Mode::CWasm || (mode == Mode::Mixed && index % 3 == 1);
                    let path = if use_c {
                        c_artifacts().join(module.c_artifact)
                    } else {
                        rust_artifacts().join(module.rust_artifact)
                    };
                    let bytes = std::fs::read(&path).map_err(|_| Fault::Missing)?;
                    runtime.register_wasm((module.manifest)(), &bytes)?;
                }
            }
        }
        #[cfg(not(feature = "wasm"))]
        let _ = index;
    }
    Ok(host)
}

pub fn lab_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("driver in laboratory")
        .to_owned()
}

pub fn rust_artifacts() -> PathBuf {
    lab_root().join("target/wasm32-unknown-unknown/release")
}

pub fn c_artifacts() -> PathBuf {
    lab_root().join("../../../target/modularity-conformance/artifacts")
}
