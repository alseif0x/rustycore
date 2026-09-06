use crate::logic::{self, Host, State};
use serde::Serialize;
use std::collections::BTreeSet;
use wasmtime::{Result, bail};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct DurableMock {
    pub receipts: BTreeSet<i64>,
    pub money: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Entry(pub u32, pub u64, pub i64);

pub struct Aggregate {
    pub shield: bool,
    pub summons: i64,
    pub durable: DurableMock,
    pub trace: Vec<Entry>,
    pub checksum: u64,
    pub record_trace: bool,
    pub remaining_calls: usize,
    pub remaining_outputs: usize,
    pub depth: usize,
    pub hostcall_attempts: u64,
}

impl Default for Aggregate {
    fn default() -> Self {
        Self {
            shield: false,
            summons: 0,
            durable: DurableMock::default(),
            trace: Vec::new(),
            checksum: 0xcbf29ce484222325,
            record_trace: true,
            remaining_calls: 64,
            remaining_outputs: 64,
            depth: 0,
            hostcall_attempts: 0,
        }
    }
}

impl Aggregate {
    pub fn record(&mut self, op: u32, handle: u64, argument: i64) -> Result<()> {
        if self.remaining_outputs == 0 {
            bail!("output budget exhausted");
        }
        self.remaining_outputs -= 1;
        for value in [u64::from(op), handle, argument as u64] {
            self.checksum = (self.checksum ^ value).wrapping_mul(0x100000001b3);
        }
        if self.record_trace {
            self.trace.push(Entry(op, handle, argument));
        }
        Ok(())
    }

    pub fn action(&mut self, op: u32, handle: u64, argument: i64) -> Result<(i64, bool)> {
        self.hostcall_attempts += 1;
        if self.remaining_calls == 0 {
            bail!("host-call budget exhausted");
        }
        self.remaining_calls -= 1;
        if handle != logic::HANDLE {
            bail!("forged or stale handle");
        }
        if !(logic::SHIELD..=logic::BURN_PROBE).contains(&op) {
            bail!("unauthorized action");
        }
        if op == logic::GRANT && !(0..=1024).contains(&argument) {
            bail!("invalid receipt key");
        }
        self.record(op, handle, argument)?;
        let value = match op {
            logic::SHIELD => {
                self.shield = argument != 0;
                0
            }
            logic::SUMMON if argument & 1 != 0 => -1,
            logic::SUMMON => {
                self.summons += 1;
                self.summons
            }
            logic::READ_SUMMONS => self.summons,
            logic::CLEAR => {
                self.summons = 0;
                0
            }
            // One fixed in-memory host aggregate. NO database/COMMIT/durability proof.
            logic::GRANT if self.durable.receipts.insert(argument) => {
                self.durable.money += 100;
                1
            }
            logic::GRANT => 0,
            _ => 0,
        };
        Ok((value, op == logic::SUMMON && value >= 0))
    }

    pub fn callback_finished(&mut self, result: i64) -> Result<()> {
        self.record(100, logic::HANDLE, result)
    }

    pub fn begin_transition(&mut self) {
        self.remaining_calls = 64;
        self.remaining_outputs = 64;
        self.depth = 0;
    }

    pub fn observables(&self, state: u64) -> serde_json::Value {
        serde_json::json!({"module_state":state,"phase":State(state).phase(),
            "callbacks":State(state).callbacks(),"shield":self.shield,"summons":self.summons,
            "money":self.durable.money,"receipt_keys":self.durable.receipts,
            "hostcall_attempts_including_warmup":self.hostcall_attempts})
    }
}

#[derive(Default)]
pub struct Native {
    pub aggregate: Aggregate,
    pub state: State,
    pub percent: u32,
    pub failure: Option<String>,
}

impl Native {
    pub fn new() -> Self {
        Self {
            percent: 100,
            ..Self::default()
        }
    }
    pub fn invoke(&mut self, event: u32, argument: i64) -> Result<i64> {
        self.aggregate.begin_transition();
        self.invoke_current_budget(event, argument)
    }
    pub fn invoke_current_budget(&mut self, event: u32, argument: i64) -> Result<i64> {
        self.failure = None;
        let value = logic::run(self, event, argument);
        if let Some(error) = self.failure.take() {
            bail!(error);
        }
        let value = value.map_err(|()| wasmtime::format_err!("native execution stopped"))?;
        if event == logic::TRAP_AFTER_REWARD {
            bail!("synthetic native error after effect");
        }
        Ok(value)
    }
}

impl Host for Native {
    fn state(&self) -> State {
        self.state
    }
    fn save(&mut self, state: State) {
        self.state = state;
    }
    fn percent(&self) -> u32 {
        self.percent
    }
    fn action(&mut self, op: u32, handle: u64, argument: i64) -> Result<i64, ()> {
        match self.aggregate.action(op, handle, argument) {
            Ok((value, reenter)) => {
                if reenter {
                    let result = logic::run(self, logic::CALLBACK, 0)?;
                    if let Err(error) = self.aggregate.callback_finished(result) {
                        self.failure = Some(error.to_string());
                        return Err(());
                    }
                }
                Ok(value)
            }
            Err(error) => {
                self.failure = Some(error.to_string());
                Err(())
            }
        }
    }
}
