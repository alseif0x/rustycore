use crate::storage::CoreComponent;
use crate::{ActionOutcome, CallbackPlan, HostCore, Invocation, Residence, Trace};
use conformance_contract::{
    Action, Fault, Handle, Host, Query, Result, Snapshot, capability, event,
};

struct NativeHost<'a> {
    core: &'a mut HostCore,
}

/// Owned request: guest codec validation cannot retain a core/component borrow.
pub(crate) struct WriteTicket {
    pub invocation: Invocation,
    pub bytes: Vec<u8>,
    expected: u64,
}

impl Host for NativeHost<'_> {
    fn read(&mut self) -> Result<Snapshot> {
        self.core.read_scoped()
    }

    fn write(&mut self, revision: u64, bytes: &[u8]) -> Result<()> {
        self.core.write_scoped(revision, bytes)
    }

    fn query(&mut self, query: Query) -> Result<i64> {
        self.core.query_scoped(query)
    }

    fn action(&mut self, action: Action, argument: i64) -> Result<i64> {
        let outcome = self.core.action_scoped(action, argument)?;
        if let Some(callback) = outcome.callback {
            self.core
                .dispatch_nested(callback.handle, callback.event, callback.argument)?;
        }
        Ok(outcome.value)
    }
}

impl HostCore {
    pub fn dispatch(
        &mut self,
        handle: Handle,
        event: u32,
        argument: i64,
    ) -> Result<Vec<(u64, i64)>> {
        self.begin_root()?;
        let result = self.dispatch_nested(handle, event, argument);
        self.end_root();
        result
    }

    pub fn dispatch_one(
        &mut self,
        handle: Handle,
        module: u64,
        event: u32,
        argument: i64,
    ) -> Result<i64> {
        self.begin_root()?;
        let result = self.invoke_native(handle, module, event, argument);
        self.end_root();
        result
    }

    pub(crate) fn begin_root(&mut self) -> Result<()> {
        self.require_idle()?;
        self.calls = 0;
        self.trace.clear();
        Ok(())
    }

    pub(crate) fn end_root(&self) {
        debug_assert!(
            self.frames.is_empty(),
            "all Result paths must leave their invocation"
        );
    }

    pub(crate) fn dispatch_nested(
        &mut self,
        handle: Handle,
        event: u32,
        argument: i64,
    ) -> Result<Vec<(u64, i64)>> {
        self.owner(handle)?;
        let modules = self.entity_modules(handle)?;
        // Fail a native-only dispatch before the first callback if any executor is unsupported.
        for module in &modules {
            self.native_invoker(*module)?;
        }
        let mut results = Vec::new();
        for module in modules {
            let value = self.invoke_native(handle, module, event, argument)?;
            results.push((module, value));
        }
        Ok(results)
    }

    pub(crate) fn invoke_native(
        &mut self,
        handle: Handle,
        module: u64,
        event: u32,
        argument: i64,
    ) -> Result<i64> {
        let invoke = self.native_invoker(module)?;
        let frame = Invocation {
            handle,
            module,
            event,
            argument,
        };
        self.enter_call(frame)?;
        let result =
            crate::portable_result(invoke(&mut NativeHost { core: self }, event, argument));
        self.leave_call(frame, result);
        result
    }

    pub(crate) fn native_root<T>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        self.begin_root()?;
        let result = operation(self);
        self.end_root();
        result
    }

    pub(crate) fn enter_call(&mut self, frame: Invocation) -> Result<()> {
        self.owner(frame.handle)?;
        self.state(frame.handle, frame.module)?;
        if self.frames.len() >= self.limits.depth {
            return Err(Fault::Limit);
        }
        self.charge()?;
        // Enter plus its eventual Leave must fit before the frame is admitted.
        self.ensure_trace_capacity(2)?;
        self.push_trace(Trace::Enter(frame))?;
        self.frames.push(frame);
        Ok(())
    }

    pub(crate) fn leave_call(&mut self, frame: Invocation, result: Result<i64>) {
        assert_eq!(
            self.frames.pop(),
            Some(frame),
            "executor must leave exactly its frame"
        );
        self.push_trace(Trace::Leave {
            invocation: frame,
            result,
        })
        .expect("admitted frame reserved its Leave trace capacity");
    }

    fn current(&self) -> Result<Invocation> {
        self.frames.last().copied().ok_or(Fault::Invalid)
    }

    fn require_capability(&self, module: u64, capability: u64) -> Result<()> {
        let entry = self.modules.get(&module).ok_or(Fault::Missing)?;
        if entry.manifest.capabilities & capability == capability {
            Ok(())
        } else {
            Err(Fault::Capability)
        }
    }

    pub(crate) fn read_scoped(&mut self) -> Result<Snapshot> {
        self.charge()?;
        let frame = self.current()?;
        self.state(frame.handle, frame.module)
    }

    pub(crate) fn write_scoped(&mut self, revision: u64, bytes: &[u8]) -> Result<()> {
        let ticket = self.preflight_write_scoped(revision, bytes)?;
        self.commit_write_scoped(ticket)
    }

    pub(crate) fn preflight_write_scoped(
        &mut self,
        revision: u64,
        bytes: &[u8],
    ) -> Result<WriteTicket> {
        self.charge()?;
        let frame = self.current()?;
        self.preflight_write_state(frame.handle, frame.module, revision, bytes)?;
        Ok(WriteTicket {
            invocation: frame,
            bytes: bytes.to_vec(),
            expected: revision,
        })
    }

    /// The adapter validates opaque bytes before calling this phase. Neither phase
    /// refills root budgets; the second phase rechecks admission without charging twice.
    pub(crate) fn commit_write_scoped(&mut self, ticket: WriteTicket) -> Result<()> {
        if self.current()? != ticket.invocation {
            return Err(Fault::Conflict);
        }
        self.write_state(
            ticket.invocation.handle,
            ticket.invocation.module,
            ticket.expected,
            &ticket.bytes,
        )
    }

    pub(crate) fn query_scoped(&mut self, query: Query) -> Result<i64> {
        self.charge()?;
        let frame = self.current()?;
        self.require_capability(frame.module, capability::QUERY)?;
        let observed = self.observables(frame.handle)?;
        match query {
            Query::Shield => Ok(i64::from(observed.shield)),
            Query::Summons => i64::try_from(observed.summons).map_err(|_| Fault::Overflow),
            Query::Contribution => Ok(observed.contribution),
            Query::Residence => Ok(match observed.residence {
                Residence::Active(map) => i64::from(map) + 1,
                Residence::Detached => 0,
            }),
        }
    }

    /// Commit this action's bounded effect, then return an owned callback request.
    /// Caller must execute it synchronously, propagate failure, and not roll back prior effects.
    pub(crate) fn action_scoped(&mut self, action: Action, argument: i64) -> Result<ActionOutcome> {
        self.charge()?;
        let frame = self.current()?;
        let required = match action {
            Action::Shield => capability::SHIELD,
            Action::Summon => capability::SUMMON,
            Action::Contribution => capability::CONTRIBUTION,
            Action::Reenter | Action::Fail => capability::REENTRY_PROBE,
        };
        self.require_capability(frame.module, required)?;
        let residence = self.residence(frame.handle)?;
        let cleanup = matches!(action, Action::Shield | Action::Contribution) && argument == 0;
        if residence == Residence::Detached && !cleanup && action != Action::Reenter {
            return Err(Fault::NotActive);
        }
        if action == Action::Fail {
            return Err(Fault::ActionFailed);
        }
        match action {
            Action::Shield | Action::Summon if !(0..=1).contains(&argument) => {
                return Err(Fault::Invalid);
            }
            Action::Contribution if !(0..=1000).contains(&argument) => return Err(Fault::Invalid),
            // Negative i64 results are fault codes in the shared Core Wasm ABI.
            Action::Reenter if argument < 0 => return Err(Fault::Invalid),
            _ => {}
        }
        // Reserve trace capacity before mutating; a full trace cannot silently drop evidence.
        self.ensure_trace_capacity(1)?;
        if action == Action::Summon && argument == 1 {
            let total = self.observables(frame.handle)?.summons;
            i64::try_from(total.checked_add(1).ok_or(Fault::Overflow)?)
                .map_err(|_| Fault::Overflow)?;
        }
        let mut value = argument;
        let mutates = matches!(action, Action::Shield | Action::Contribution)
            || (action == Action::Summon && argument == 1);
        let revision = if mutates {
            Some(self.next_revision()?)
        } else {
            None
        };
        self.with_component_mut::<CoreComponent, _>(frame.handle, |core| -> Result<()> {
            match action {
                Action::Shield => {
                    core.contributions.entry(frame.module).or_default().shield = argument != 0
                }
                Action::Contribution => {
                    core.contributions.entry(frame.module).or_default().amount = argument
                }
                Action::Summon if argument == 1 => {
                    let contribution = core.contributions.entry(frame.module).or_default();
                    let next = contribution.summons.checked_add(1).ok_or(Fault::Overflow)?;
                    value = i64::try_from(next).map_err(|_| Fault::Overflow)?;
                    contribution.summons = next;
                }
                _ => {}
            }
            if let Some(revision) = revision {
                core.revision = revision;
            }
            Ok(())
        })??;
        self.push_trace(Trace::Action {
            handle: frame.handle,
            module: frame.module,
            action,
            argument,
            value,
        })?;
        let callback = if action == Action::Reenter || (action == Action::Summon && argument == 1) {
            Some(CallbackPlan {
                handle: frame.handle,
                event: event::CALLBACK,
                argument: if action == Action::Summon {
                    0
                } else {
                    argument
                },
            })
        } else {
            None
        };
        Ok(ActionOutcome { value, callback })
    }
}
