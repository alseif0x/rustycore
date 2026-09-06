use super::*;
use conformance_contract::Snapshot;

#[derive(Default)]
struct Fixture {
    state: EncounterState,
    revision: u64,
    shield: bool,
    summons: i64,
    trace: Vec<&'static str>,
}

impl Host for Fixture {
    fn read(&mut self) -> Result<Snapshot> {
        self.trace.push("read");
        Ok(Snapshot {
            revision: self.revision,
            bytes: self.state.encode(),
        })
    }

    fn write(&mut self, revision: u64, bytes: &[u8]) -> Result<()> {
        if revision != self.revision {
            self.trace.push("reject_stale");
            return Err(Fault::Revision);
        }
        let state = EncounterState::decode(bytes)?;
        self.revision = self.revision.checked_add(1).ok_or(Fault::Overflow)?;
        self.state = state;
        self.trace.push("write");
        Ok(())
    }

    fn query(&mut self, query: Query) -> Result<i64> {
        match query {
            Query::Shield => {
                self.trace.push("query_shield");
                Ok(i64::from(self.shield))
            }
            Query::Summons => {
                self.trace.push("query_summons");
                Ok(self.summons)
            }
            _ => Err(Fault::Invalid),
        }
    }

    fn action(&mut self, action: Action, argument: i64) -> Result<i64> {
        match action {
            Action::Shield => {
                self.trace.push("shield");
                self.shield = argument == 1;
                Ok(0)
            }
            Action::Summon => {
                self.trace.push("summon");
                if argument == 0 {
                    return Ok(0);
                }
                self.summons += 1;
                self.trace.push("callback");
                Encounter::invoke(self, event::CALLBACK, 0)?;
                Ok(1)
            }
            Action::Reenter => Encounter::invoke(self, event::CALLBACK, argument),
            Action::Fail => Err(Fault::ActionFailed),
            _ => Err(Fault::Invalid),
        }
    }
}

#[test]
fn ordered_action_callback_read_uses_nested_state() {
    let mut host = Fixture::default();
    assert_eq!(Encounter::invoke(&mut host, event::UPDATE, 1), Ok(1));
    assert_eq!(
        host.state,
        EncounterState {
            phase: 1,
            callbacks: 1
        }
    );
    assert_eq!(
        host.trace,
        [
            "read",
            "write",
            "shield",
            "summon",
            "callback",
            "read",
            "query_shield",
            "write",
            "read",
            "query_summons",
            "read",
        ]
    );
    assert_eq!(Encounter::invoke(&mut host, event::UPDATE, 1), Ok(1));
    assert_eq!(host.summons, 1, "phase gate must not summon twice");
}

#[test]
fn nullable_summon_keeps_published_phase_and_shield() {
    let mut host = Fixture::default();
    assert_eq!(Encounter::invoke(&mut host, event::UPDATE, 0), Ok(0));
    assert_eq!(
        host.state,
        EncounterState {
            phase: 1,
            callbacks: 0
        }
    );
    assert!(host.shield);
    assert_eq!(host.summons, 0);
}

#[test]
fn obsolete_outer_write_cannot_erase_nested_callback() {
    let mut host = Fixture::default();
    assert_eq!(
        Encounter::invoke(&mut host, STALE_OUTER_WRITE, 0),
        Err(Fault::Revision)
    );
    assert_eq!(
        host.state,
        EncounterState {
            phase: 0,
            callbacks: 1
        }
    );
    assert_eq!(host.revision, 1);
    assert_eq!(host.summons, 1);
}

#[test]
fn failure_after_effect_is_not_rollback() {
    let mut host = Fixture::default();
    assert_eq!(
        Encounter::invoke(&mut host, FAIL_AFTER_SHIELD, 0),
        Err(Fault::ActionFailed)
    );
    assert!(host.shield);
    assert_eq!(host.state, EncounterState::default());
}

#[test]
fn reset_and_removing_clear_own_state_without_revision_aba() {
    for event in [event::RESET, event::REMOVING] {
        let mut host = Fixture::default();
        Encounter::invoke(&mut host, event::UPDATE, 1).unwrap();
        let revision = host.revision;
        assert_eq!(Encounter::invoke(&mut host, event, 0), Ok(0));
        assert_eq!(host.state, EncounterState::default());
        assert!(!host.shield);
        assert_eq!(host.revision, revision + 1);
    }
}

#[test]
fn malformed_state_and_invalid_input_are_rejected() {
    assert_eq!(EncounterState::decode(&[0; 11]), Err(Fault::Invalid));
    let mut bytes = [0; 12];
    bytes[0] = 2;
    assert_eq!(EncounterState::decode(&bytes), Err(Fault::Invalid));
    let state = EncounterState {
        phase: 1,
        callbacks: 0x0102_0304_0506_0708,
    };
    assert_eq!(state.encode(), [1, 0, 0, 0, 8, 7, 6, 5, 4, 3, 2, 1]);
    assert_eq!(EncounterState::decode(&state.encode()), Ok(state));
    let mut host = Fixture::default();
    assert_eq!(
        Encounter::invoke(&mut host, event::UPDATE, -1),
        Err(Fault::Invalid)
    );
    assert!(host.trace.is_empty());
}

#[test]
fn callback_overflow_does_not_write() {
    let mut host = Fixture {
        state: EncounterState {
            phase: 1,
            callbacks: u64::MAX,
        },
        ..Fixture::default()
    };
    assert_eq!(
        Encounter::invoke(&mut host, event::CALLBACK, 0),
        Err(Fault::Overflow)
    );
    assert_eq!(host.revision, 0);
    assert_eq!(host.state.callbacks, u64::MAX);
}
