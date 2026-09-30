//! Represented taxi, transport and vehicle operations.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

mod vehicle_state;
mod admission_routes;
mod protocol;
mod flight_state;
mod transport_state;
