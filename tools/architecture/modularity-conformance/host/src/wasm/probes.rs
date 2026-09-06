//! Diagnostic guest exports, not production gameplay capabilities. The progress
//! marker is guest-local instrumentation only, never canonical module state.

use super::{WasmRuntime, decode_result, execution_fault};
use crate::Invocation;
use conformance_contract::{Fault, Handle, Result, event};
use wasmtime::{AsContextMut, WasmParams, WasmResults};

impl WasmRuntime {
    pub(super) fn probe<P: WasmParams, R: WasmResults>(
        &mut self,
        handle: Handle,
        module: u64,
        name: &str,
        argument: P,
    ) -> Result<R> {
        self.root(|mut context| {
            let instance = context
                .data()
                .guests
                .get(&module)
                .ok_or(Fault::Missing)?
                .instance;
            let function = instance
                .get_typed_func::<P, R>(context.as_context_mut(), name)
                .map_err(|_| Fault::Missing)?;
            let frame = Invocation {
                handle,
                module,
                event: event::CUSTOM + 100,
                argument: 0,
            };
            context.data_mut().core.enter_call(frame)?;
            let result = function
                .call(context.as_context_mut(), argument)
                .map_err(execution_fault);
            // Generic probe output is asserted by the caller; this diagnostic
            // frame records completion/failure. Real invoke returns are exact.
            let completion = result.as_ref().map(|_| 0).map_err(|fault| *fault);
            context.data_mut().core.leave_call(frame, completion);
            result
        })
    }

    /// A denied memory.grow is the normal Wasm result -1, not a host trap.
    pub fn probe_grow(&mut self, handle: Handle, module: u64, pages: u32) -> Result<i32> {
        self.probe(handle, module, "probe_grow", pages)
    }

    pub fn probe_spin(&mut self, handle: Handle, module: u64) -> Result<()> {
        self.probe(handle, module, "probe_spin", ())
    }

    pub fn probe_burn(&mut self, handle: Handle, module: u64, iterations: u32) -> Result<u64> {
        self.probe(handle, module, "probe_burn", iterations)
    }

    /// Finite guest work, a synchronous nested callback, then more finite work.
    /// Neither the host import nor the callback replenishes the root fuel budget.
    pub fn probe_nested(&mut self, handle: Handle, module: u64, iterations: u32) -> Result<i64> {
        self.probe(handle, module, "probe_nested", iterations)
            .and_then(decode_result)
    }

    /// This is a separate idle diagnostic root, so it can read instrumentation
    /// after exhausted fuel. With no invocation frame its imports have no authority.
    pub fn probe_stage(&mut self, module: u64) -> Result<u32> {
        self.root(|mut context| {
            let instance = context
                .data()
                .guests
                .get(&module)
                .ok_or(Fault::Missing)?
                .instance;
            instance
                .get_typed_func::<(), u32>(context.as_context_mut(), "probe_stage")
                .map_err(|_| Fault::Missing)?
                .call(context.as_context_mut(), ())
                .map_err(execution_fault)
        })
    }
}
