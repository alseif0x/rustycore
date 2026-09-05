use super::{Data, dispatch, execution_fault, validation};
use conformance_contract::{Action, Fault, MAX_STATE_BYTES, Query, Result};
use std::ops::Range;
use wasmtime::{AsContextMut, Caller, Engine, Linker, Memory};

fn range(pointer: u32, length: u32, memory_size: usize) -> Result<Range<usize>> {
    let start = pointer as usize;
    let end = start.checked_add(length as usize).ok_or(Fault::Invalid)?;
    if end > memory_size {
        return Err(Fault::Invalid);
    }
    Ok(start..end)
}

fn memory(caller: &mut Caller<'_, Data>) -> Result<Memory> {
    caller
        .get_export("memory")
        .and_then(|export| export.into_memory())
        .ok_or(Fault::Invalid)
}

/// Malformed imports consume the same call budget as valid scoped operations.
fn rejected(caller: &mut Caller<'_, Data>, fault: Fault) -> Fault {
    caller.data_mut().core.charge().err().unwrap_or(fault)
}

fn read(mut caller: Caller<'_, Data>, pointer: u32, capacity: u32, revision_pointer: u32) -> i32 {
    if let Some(fault) = validation::reject_semantic(&mut caller) {
        return fault.code() as i32;
    }
    let validated = (|| -> Result<_> {
        if capacity as usize > MAX_STATE_BYTES {
            return Err(Fault::Limit);
        }
        let memory = memory(&mut caller)?;
        let size = memory.data_size(&caller);
        let bytes = range(pointer, capacity, size)?;
        let revision = range(revision_pointer, 8, size)?;
        if !bytes.is_empty() && bytes.start < revision.end && revision.start < bytes.end {
            return Err(Fault::Invalid);
        }
        Ok((memory, bytes, revision))
    })();
    let (memory, bytes, revision) = match validated {
        Ok(value) => value,
        Err(fault) => return rejected(&mut caller, fault).code() as i32,
    };
    let snapshot = match caller.data_mut().core.read_scoped() {
        Ok(snapshot) => snapshot,
        Err(fault) => return fault.code() as i32,
    };
    if snapshot.bytes.len() > bytes.len() {
        return Fault::Limit.code() as i32;
    }
    // Both complete ranges were checked before either write. No callback can grow
    // memory between validation and these writes, and no slice escapes the import.
    if memory
        .write(&mut caller, bytes.start, &snapshot.bytes)
        .is_err()
        || memory
            .write(
                &mut caller,
                revision.start,
                &snapshot.revision.to_le_bytes(),
            )
            .is_err()
    {
        return Fault::Invalid.code() as i32;
    }
    snapshot.bytes.len() as i32
}

fn write(mut caller: Caller<'_, Data>, pointer: u32, length: u32, revision: u64) -> i32 {
    if let Some(fault) = validation::reject_semantic(&mut caller) {
        return fault.code() as i32;
    }
    // Do not allocate/copy another payload once the cumulative call budget is gone.
    if caller.data().core.calls() >= caller.data().core.limits().calls {
        return Fault::Limit.code() as i32;
    }
    let copied = (|| -> Result<Vec<u8>> {
        if length as usize > MAX_STATE_BYTES {
            return Err(Fault::Limit);
        }
        let memory = memory(&mut caller)?;
        let bytes = range(pointer, length, memory.data_size(&caller))?;
        Ok(memory.data(&caller)[bytes].to_vec())
    })();
    let bytes = match copied {
        Ok(bytes) => bytes,
        Err(fault) => return rejected(&mut caller, fault).code() as i32,
    };
    let ticket = match caller
        .data_mut()
        .core
        .preflight_write_scoped(revision, &bytes)
    {
        Ok(ticket) => ticket,
        Err(fault) => return fault.code() as i32,
    };
    // The ticket is owned, not a storage borrow. Validation cannot execute any
    // semantic Host import using the outer invocation's authority.
    if let Err(fault) = validation::registered(
        caller.as_context_mut(),
        ticket.invocation.module,
        &ticket.bytes,
    ) {
        return fault.code() as i32;
    }
    caller
        .data_mut()
        .core
        .commit_write_scoped(ticket)
        .map_or_else(|fault| fault.code() as i32, |()| 0)
}

fn query(mut caller: Caller<'_, Data>, operation: u32) -> i64 {
    if let Some(fault) = validation::reject_semantic(&mut caller) {
        return fault.code();
    }
    let query = match operation {
        1 => Query::Shield,
        2 => Query::Summons,
        3 => Query::Contribution,
        4 => Query::Residence,
        _ => return rejected(&mut caller, Fault::Invalid).code(),
    };
    caller
        .data_mut()
        .core
        .query_scoped(query)
        .unwrap_or_else(Fault::code)
}

fn action(mut caller: Caller<'_, Data>, operation: u32, argument: i64) -> i64 {
    if let Some(fault) = validation::reject_semantic(&mut caller) {
        return fault.code();
    }
    let action = match operation {
        1 => Action::Shield,
        2 => Action::Summon,
        3 => Action::Contribution,
        4 => Action::Reenter,
        5 => Action::Fail,
        _ => return rejected(&mut caller, Fault::Invalid).code(),
    };
    let outcome = match caller.data_mut().core.action_scoped(action, argument) {
        Ok(outcome) => outcome,
        Err(fault) => return fault.code(),
    };
    if let Some(callback) = outcome.callback {
        #[cfg(test)]
        if caller.data().refill_nested_fuel {
            if caller.set_fuel(conformance_contract::FUEL).is_err() {
                return Fault::Trap.code();
            }
        }
        // The action above has already applied. Clone metadata in dispatch and
        // reborrow this SAME Store; no data_mut(), state guard or memory slice lives here.
        if let Err(fault) = dispatch::all(
            caller.as_context_mut(),
            callback.handle,
            callback.event,
            callback.argument,
        ) {
            return fault.code();
        }
    }
    outcome.value
}

pub(super) fn linker(engine: &Engine) -> Result<Linker<Data>> {
    let mut linker = Linker::new(engine);
    linker
        .func_wrap("conformance", "read", read)
        .map_err(execution_fault)?;
    linker
        .func_wrap("conformance", "write", write)
        .map_err(execution_fault)?;
    linker
        .func_wrap("conformance", "query", query)
        .map_err(execution_fault)?;
    linker
        .func_wrap("conformance", "action", action)
        .map_err(execution_fault)?;
    linker
        .func_wrap("conformance", "validation_read", validation::read)
        .map_err(execution_fault)?;
    Ok(linker)
}
