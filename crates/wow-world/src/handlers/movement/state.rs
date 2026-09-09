//! Movement handlers state definitions, part 1 of 1.
//!
//! Separated from the movement.rs root under #654. Behaviour is preserved.

use super::*;

// C++ `HandleMoveSetVehicleRecAck` has no session-visible side effect, so
// the #142 wire-dispatch test proves reachability with a test-only call
// counter instead of inventing production state.
#[cfg(test)]
pub(super) static MOVE_SET_VEHICLE_REC_ID_ACK_HANDLER_CALLS_FOR_TEST: std::sync::Mutex<
    Vec<(std::thread::ThreadId, usize)>,
> = std::sync::Mutex::new(Vec::new());

#[cfg(test)]
pub(super) fn record_move_set_vehicle_rec_id_ack_handler_call_for_test() {
    let thread_id = std::thread::current().id();
    let mut calls_by_thread = MOVE_SET_VEHICLE_REC_ID_ACK_HANDLER_CALLS_FOR_TEST
        .lock()
        .expect("vehicle ACK test-call counter poisoned");
    if let Some((_, calls)) = calls_by_thread
        .iter_mut()
        .find(|(candidate, _)| *candidate == thread_id)
    {
        *calls += 1;
    } else {
        calls_by_thread.push((thread_id, 1));
    }
}

#[cfg(test)]
pub(crate) fn take_move_set_vehicle_rec_id_ack_handler_calls_for_test() -> usize {
    let thread_id = std::thread::current().id();
    let mut calls_by_thread = MOVE_SET_VEHICLE_REC_ID_ACK_HANDLER_CALLS_FOR_TEST
        .lock()
        .expect("vehicle ACK test-call counter poisoned");
    calls_by_thread
        .iter()
        .position(|(candidate, _)| *candidate == thread_id)
        .map(|index| calls_by_thread.swap_remove(index).1)
        .unwrap_or(0)
}
