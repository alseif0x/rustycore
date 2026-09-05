use super::{WasmRuntime, dispatch};
use crate::Residence;
use conformance_contract::{Handle, Result, event};
use wasmtime::AsContextMut;

impl WasmRuntime {
    /// Storage transfer precedes DETACHED notification. Callback failure preserves
    /// the detached owner; it does not implicitly move the entity back.
    pub fn detach(&mut self, handle: Handle) -> Result<()> {
        self.root(|mut context| {
            dispatch::preflight(context.data(), handle)?;
            context.data_mut().core.detach_storage(handle)?;
            dispatch::all(context.as_context_mut(), handle, event::DETACHED, 0).map(drop)
        })
    }

    /// Failed destination admission preserves the detached allocation and state.
    /// Failure of ATTACHED after admission leaves the new active residence visible.
    pub fn attach(&mut self, handle: Handle, map: u8) -> Result<()> {
        self.root(|mut context| {
            dispatch::preflight(context.data(), handle)?;
            context.data_mut().core.attach_storage(handle, map)?;
            dispatch::all(
                context.as_context_mut(),
                handle,
                event::ATTACHED,
                i64::from(map),
            )
            .map(drop)
        })
    }

    pub fn reset(&mut self, handle: Handle, module: u64) -> Result<()> {
        self.root(|mut context| {
            dispatch::preflight(context.data(), handle)?;
            dispatch::preflight_module(context.data(), module)?;
            context.data_mut().core.reset_storage(handle, module)?;
            dispatch::one(context.as_context_mut(), handle, module, event::RESET, 0).map(drop)
        })
    }

    pub fn retire(&mut self, handle: Handle) -> Result<()> {
        self.root(|mut context| {
            dispatch::preflight(context.data(), handle)?;
            dispatch::all(context.as_context_mut(), handle, event::REMOVING, 0)?;
            context.data_mut().core.retire_storage(handle)
        })
    }

    /// Prepare fallible destination/default state before REMOVING. A later callback
    /// failure is partial: inspect core.handle(guid)/residence, not the old handle
    /// alone, to determine whether replacement was already installed.
    pub fn replace(&mut self, handle: Handle, map: u8) -> Result<Handle> {
        self.root(|mut context| {
            dispatch::preflight(context.data(), handle)?;
            let prepared = context.data_mut().core.prepare_replace(handle, map)?;
            dispatch::all(context.as_context_mut(), handle, event::REMOVING, 0)?;
            let replacement = context.data_mut().core.commit_replace(prepared)?;
            dispatch::all(
                context.as_context_mut(),
                replacement,
                event::ATTACHED,
                i64::from(map),
            )?;
            Ok(replacement)
        })
    }

    pub fn unload_module(&mut self, module: u64) -> Result<()> {
        self.root(|mut context| {
            dispatch::preflight_module(context.data(), module)?;
            let handles: Vec<_> = context
                .data()
                .core
                .owners
                .values()
                .filter(|owner| owner.modules.contains(&module))
                .map(|owner| owner.handle)
                .collect();
            for handle in &handles {
                dispatch::preflight(context.data(), *handle)?;
            }
            for handle in handles {
                dispatch::one(context.as_context_mut(), handle, module, event::REMOVING, 0)?;
            }
            context.data_mut().core.remove_module_storage(module)?;
            // Wasmtime allocations live until Store drop; dropping this index
            // disables execution without pretending an individual instance unload.
            context.data_mut().guests.remove(&module);
            Ok(())
        })
    }

    pub fn add_module_state(&mut self, handle: Handle, module: u64) -> Result<()> {
        self.root(|mut context| {
            dispatch::preflight(context.data(), handle)?;
            dispatch::preflight_module(context.data(), module)?;
            context
                .data_mut()
                .core
                .add_module_state_storage(handle, module)?;
            let (event, argument) = match context.data().core.residence(handle)? {
                Residence::Detached => (event::DETACHED, 0),
                Residence::Active(map) => (event::ATTACHED, i64::from(map)),
            };
            dispatch::one(context.as_context_mut(), handle, module, event, argument).map(drop)
        })
    }

    pub fn remove_module_state(&mut self, handle: Handle, module: u64) -> Result<()> {
        self.root(|mut context| {
            dispatch::preflight(context.data(), handle)?;
            dispatch::preflight_module(context.data(), module)?;
            context.data().core.state(handle, module)?;
            dispatch::one(context.as_context_mut(), handle, module, event::REMOVING, 0)?;
            context
                .data_mut()
                .core
                .remove_module_state_storage(handle, module)
        })
    }

    pub fn unload_map(&mut self, map: u8) -> Result<()> {
        self.root(|mut context| {
            let handles = context.data().core.handles_in_map(map)?;
            for handle in &handles {
                dispatch::preflight(context.data(), *handle)?;
            }
            for handle in handles {
                context.data_mut().core.detach_storage(handle)?;
                dispatch::all(context.as_context_mut(), handle, event::DETACHED, 0)?;
            }
            context.data_mut().core.mark_map_unloaded(map)
        })
    }

    pub fn load_map(&mut self, map: u8) -> Result<()> {
        self.store.data_mut().core.load_map(map)
    }
}
