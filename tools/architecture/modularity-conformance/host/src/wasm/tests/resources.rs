use super::*;
use conformance_contract::{FUEL, Fault, event};

#[test]
fn real_rust_and_c_guests_cannot_grow_past_their_memory_cap_or_spin_forever() {
    for frontend in Frontend::ALL {
        let (mut runtime, handle) = frontend.runtime();
        assert_eq!(runtime.probe_grow(handle, 1, 1000), Ok(-1), "{frontend:?}");
        assert_eq!(
            runtime.probe_spin(handle, 1),
            Err(Fault::Limit),
            "{frontend:?}"
        );
        assert_eq!(runtime.core().depth(), 0);
        // A later root gets a new budget; the failed invocation did not poison frames.
        assert_eq!(runtime.dispatch_one(handle, 1, event::UPDATE, 0), Ok(0));
    }
}

#[test]
fn cumulative_guest_fuel_fails_after_effect_and_the_same_probe_finishes_with_injected_refill() {
    for frontend in Frontend::ALL {
        // Calibrate deterministic instruction fuel, not wall time/performance. Rust
        // and C compiler output have different instruction costs. Every variant
        // below uses a fresh runtime and the SAME chosen input for that frontend.
        let (mut calibration, handle) = frontend.runtime();
        calibration.probe_burn(handle, 1, 1000).unwrap();
        let spent = FUEL - calibration.fuel_remaining().unwrap();
        assert!(spent > 0 && spent < FUEL / 2);
        let iterations = u32::try_from((FUEL * 3 / 5) * 1000 / spent).unwrap();

        let (mut correct, handle) = frontend.runtime();
        assert_eq!(
            correct.probe_nested(handle, 1, iterations),
            Err(Fault::Limit),
            "{frontend:?}, n={iterations}"
        );
        assert_eq!(correct.core().depth(), 0);
        let after_failure = correct.snapshot(handle).unwrap();
        let callbacks = u64::from_le_bytes(
            correct.state(handle, 1).unwrap().bytes[4..12]
                .try_into()
                .unwrap(),
        );
        assert_eq!(
            callbacks, 1,
            "the callback effect survives the later fuel trap"
        );
        let trace = correct.core().trace().to_vec();
        assert_eq!(
            correct.probe_stage(1),
            Ok(3),
            "second finite computation exhausted fuel"
        );

        let (mut injected, handle) = frontend.runtime();
        injected.store.data_mut().refill_nested_fuel = true;
        assert_eq!(
            injected.probe_nested(handle, 1, iterations),
            Ok(0),
            "{frontend:?}, same n={iterations}"
        );
        // Nested callback results/effects match. The final diagnostic Leave
        // deliberately differs: correct exhaustion versus injected completion.
        let injected_trace = injected.core().trace();
        assert_eq!(
            &injected_trace[..injected_trace.len() - 1],
            &trace[..trace.len() - 1]
        );
        assert_eq!(injected.snapshot(handle).unwrap(), after_failure);
        assert_eq!(injected.probe_stage(1), Ok(4));
        assert_eq!(injected.core().depth(), 0);
    }
}

#[test]
fn real_guest_recursive_dispatch_hits_host_depth_and_keeps_prior_nested_writes() {
    for frontend in Frontend::ALL {
        let (mut runtime, handle) = frontend.runtime();
        assert_eq!(
            runtime.dispatch_one(handle, 1, event::CALLBACK, 100),
            Err(Fault::Limit)
        );
        assert_eq!(runtime.core().depth(), 0);
        let callbacks = u64::from_le_bytes(
            runtime.state(handle, 1).unwrap().bytes[4..12]
                .try_into()
                .unwrap(),
        );
        assert!(callbacks > 0 && callbacks <= conformance_contract::MAX_DEPTH as u64);
    }
}
