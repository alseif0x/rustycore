use super::*;
use conformance_contract::Snapshot;

#[derive(Default)]
struct Fixture {
    state: PolicyState,
    revision: u64,
    contribution: i64,
    writes: usize,
    actions: usize,
}

impl Host for Fixture {
    fn read(&mut self) -> Result<Snapshot> {
        Ok(Snapshot {
            revision: self.revision,
            bytes: self.state.encode(),
        })
    }
    fn write(&mut self, revision: u64, bytes: &[u8]) -> Result<()> {
        if self.revision != revision {
            return Err(Fault::Revision);
        }
        let state = PolicyState::decode(bytes)?;
        self.revision = self.revision.checked_add(1).ok_or(Fault::Overflow)?;
        self.state = state;
        self.writes += 1;
        Ok(())
    }
    fn query(&mut self, query: Query) -> Result<i64> {
        if query == Query::Shield {
            Ok(1)
        } else {
            Err(Fault::Invalid)
        }
    }
    fn action(&mut self, action: Action, argument: i64) -> Result<i64> {
        assert_eq!(
            action,
            Action::Contribution,
            "policy may only change its contribution"
        );
        self.actions += 1;
        self.contribution = argument;
        Ok(0)
    }
}

#[test]
fn custom_policy_uses_owned_codec_and_bounded_multiplication() {
    let mut host = Fixture {
        state: PolicyState {
            calls: 0,
            percent: 150,
        },
        ..Fixture::default()
    };
    assert_eq!(Policy::invoke(&mut host, event::POLICY, 101), Ok(151));
    assert_eq!(host.state.calls, 1);
    assert_eq!(host.contribution, 150);
    assert_eq!(host.revision, 1);
}

#[test]
fn invalid_policy_inputs_and_overflow_have_no_effect() {
    for argument in [-1, 1_000_001, i64::MAX] {
        let mut host = Fixture::default();
        assert_eq!(
            Policy::invoke(&mut host, event::POLICY, argument),
            Err(Fault::Invalid)
        );
        assert_eq!((host.writes, host.actions), (0, 0));
    }
    let mut host = Fixture {
        state: PolicyState {
            calls: u64::MAX,
            percent: 100,
        },
        ..Fixture::default()
    };
    assert_eq!(
        Policy::invoke(&mut host, event::POLICY, 100),
        Err(Fault::Overflow)
    );
    assert_eq!((host.writes, host.actions), (0, 0));
}

#[test]
fn callback_observer_does_not_rewrite_state_or_contributions() {
    let mut host = Fixture {
        state: PolicyState {
            calls: 9,
            percent: 100,
        },
        ..Fixture::default()
    };
    assert_eq!(Policy::invoke(&mut host, event::CALLBACK, 0), Ok(9));
    assert_eq!((host.writes, host.actions), (0, 0));
    assert_eq!(host.revision, 0);
}

#[test]
fn reset_and_removal_clear_only_own_contribution() {
    for event in [event::RESET, event::REMOVING] {
        let mut host = Fixture::default();
        Policy::invoke(&mut host, event::POLICY, 200).unwrap();
        assert_eq!(host.contribution, 100);
        assert_eq!(Policy::invoke(&mut host, event, 0), Ok(0));
        assert_eq!(host.state, PolicyState::default());
        assert_eq!(host.contribution, 0);
        assert_eq!(host.revision, 2);
    }
}

#[test]
fn schema_one_is_explicit_little_endian_and_rejects_bad_ranges() {
    let state = PolicyState {
        calls: 0x0102_0304_0506_0708,
        percent: 1000,
    };
    assert_eq!(state.encode(), [8, 7, 6, 5, 4, 3, 2, 1, 232, 3, 0, 0]);
    assert_eq!(PolicyState::decode(&state.encode()), Ok(state));
    assert_eq!(PolicyState::decode(&[0; 11]), Err(Fault::Invalid));
    let mut bytes = [0; 12];
    bytes[8..].copy_from_slice(&1001_u32.to_le_bytes());
    assert_eq!(PolicyState::decode(&bytes), Err(Fault::Invalid));
}
