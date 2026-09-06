//! Experimental native host. No production runtime, database, or sandbox guarantee.
//! Components and hecs handles stay private; dispatch carries only owned projections.

mod checkpoint;
mod dispatch;
mod lifecycle;
mod registry;
mod storage;
mod types;
#[cfg(feature = "wasm")]
mod wasm;

pub use registry::NativeInvoke;
pub use types::*;
#[cfg(feature = "wasm")]
pub use wasm::{WasmColdTiming, WasmRuntime};

use conformance_contract::{Fault, Handle, Result};
use hecs::World;
use registry::Registration;
use std::collections::BTreeMap;
use storage::Owner;

/// Shared ABI reserves negative integers for Fault; successful signed data is unsupported.
/// Preserve explicit errors rather than reinterpreting a native Ok(-N) as an arbitrary Fault.
pub(crate) fn portable_result(result: Result<i64>) -> Result<i64> {
    match result {
        Ok(value) if value < 0 => Err(Fault::Invalid),
        other => other,
    }
}

/// Sole owner of entity residence, typed module state, contributions and revisions.
/// Native code is trusted: these limits do not bound arbitrary native CPU or allocation.
pub struct HostCore {
    worlds: [World; 2],
    loaded: [bool; 2],
    owners: BTreeMap<u64, Owner>,
    generations: BTreeMap<u64, u64>,
    modules: BTreeMap<u64, Registration>,
    revision_clock: u64,
    limits: Limits,
    frames: Vec<Invocation>,
    calls: usize,
    trace: Vec<Trace>,
}

impl Default for HostCore {
    fn default() -> Self {
        Self::new(Limits::default())
    }
}

impl HostCore {
    pub fn new(limits: Limits) -> Self {
        Self {
            worlds: [World::new(), World::new()],
            loaded: [true, true],
            owners: BTreeMap::new(),
            generations: BTreeMap::new(),
            modules: BTreeMap::new(),
            revision_clock: 0,
            limits,
            frames: Vec::new(),
            calls: 0,
            trace: Vec::new(),
        }
    }

    pub fn limits(&self) -> Limits {
        self.limits
    }

    pub fn trace(&self) -> &[Trace] {
        &self.trace
    }

    pub fn clear_trace(&mut self) -> Result<()> {
        self.require_idle()?;
        self.trace.clear();
        Ok(())
    }

    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    pub fn calls(&self) -> usize {
        self.calls
    }

    pub fn handle(&self, guid: u64) -> Option<Handle> {
        self.owners.get(&guid).map(|owner| owner.handle)
    }

    fn require_idle(&self) -> Result<()> {
        if self.frames.is_empty() {
            Ok(())
        } else {
            Err(Fault::Conflict)
        }
    }

    fn next_revision(&mut self) -> Result<u64> {
        self.revision_clock = self.revision_clock.checked_add(1).ok_or(Fault::Overflow)?;
        Ok(self.revision_clock)
    }

    fn push_trace(&mut self, trace: Trace) -> Result<()> {
        self.ensure_trace_capacity(1)?;
        self.trace.push(trace);
        Ok(())
    }

    /// Every live frame reserves a Leave slot. New effects cannot consume that reserve.
    fn ensure_trace_capacity(&self, additional: usize) -> Result<()> {
        let needed = self
            .trace
            .len()
            .checked_add(self.frames.len())
            .and_then(|reserved| reserved.checked_add(additional))
            .ok_or(Fault::Limit)?;
        if needed <= self.limits.trace {
            Ok(())
        } else {
            Err(Fault::Limit)
        }
    }

    fn charge(&mut self) -> Result<()> {
        if self.calls >= self.limits.calls {
            return Err(Fault::Limit);
        }
        self.calls += 1;
        Ok(())
    }
}

#[cfg(test)]
mod codec_tests;
#[cfg(test)]
mod tests;
