use super::{Data, decode_result, execution_fault};
use crate::{Executor, Invocation, registry::NativeInvoke};
use conformance_contract::{Action, Fault, Handle, Host, Query, Result, Snapshot};
use wasmtime::{AsContextMut, StoreContextMut, TypedFunc};

enum Target {
    Native(NativeInvoke),
    Wasm(TypedFunc<(u32, i64), i64>),
}

fn target(data: &Data, module: u64) -> Result<Target> {
    match data.core.executor(module)? {
        Executor::Native => Ok(Target::Native(data.core.native_invoker(module)?)),
        Executor::Wasm => data
            .guests
            .get(&module)
            .map(|guest| Target::Wasm(guest.invoke.clone()))
            .ok_or(Fault::Missing),
    }
}

pub(super) fn preflight_module(data: &Data, module: u64) -> Result<()> {
    target(data, module).map(drop)
}

pub(super) fn preflight(data: &Data, handle: Handle) -> Result<Vec<u64>> {
    let modules = data.core.entity_modules(handle)?;
    for module in &modules {
        target(data, *module)?;
    }
    Ok(modules)
}

pub(super) fn all(
    mut context: StoreContextMut<'_, Data>,
    handle: Handle,
    event: u32,
    argument: i64,
) -> Result<Vec<(u64, i64)>> {
    let modules = preflight(context.data(), handle)?;
    let mut results = Vec::with_capacity(modules.len());
    for module in modules {
        results.push((
            module,
            one(context.as_context_mut(), handle, module, event, argument)?,
        ));
    }
    Ok(results)
}

pub(super) fn one(
    mut context: StoreContextMut<'_, Data>,
    handle: Handle,
    module: u64,
    event: u32,
    argument: i64,
) -> Result<i64> {
    // Only copied executable metadata survives dispatch, never a registry/state borrow.
    let target = target(context.data(), module)?;
    let frame = Invocation {
        handle,
        module,
        event,
        argument,
    };
    context.data_mut().core.enter_call(frame)?;
    let result = match target {
        Target::Native(invoke) => crate::portable_result(invoke(
            &mut NativeHost {
                context: context.as_context_mut(),
            },
            event,
            argument,
        )),
        Target::Wasm(invoke) => invoke
            .call(context.as_context_mut(), (event, argument))
            .map_err(execution_fault)
            .and_then(decode_result),
    };
    // Every Result path, including a Wasm trap after prior effects, leaves its frame.
    // Trusted native panic containment is deliberately not claimed.
    context.data_mut().core.leave_call(frame, result);
    result
}

struct NativeHost<'a> {
    context: StoreContextMut<'a, Data>,
}

impl Host for NativeHost<'_> {
    fn read(&mut self) -> Result<Snapshot> {
        self.context.data_mut().core.read_scoped()
    }
    fn write(&mut self, revision: u64, bytes: &[u8]) -> Result<()> {
        self.context.data_mut().core.write_scoped(revision, bytes)
    }
    fn query(&mut self, query: Query) -> Result<i64> {
        self.context.data_mut().core.query_scoped(query)
    }
    fn action(&mut self, action: Action, argument: i64) -> Result<i64> {
        let outcome = self
            .context
            .data_mut()
            .core
            .action_scoped(action, argument)?;
        if let Some(callback) = outcome.callback {
            all(
                self.context.as_context_mut(),
                callback.handle,
                callback.event,
                callback.argument,
            )?;
        }
        Ok(outcome.value)
    }
}
