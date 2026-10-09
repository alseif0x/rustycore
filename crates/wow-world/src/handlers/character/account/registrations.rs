//! Character packet registrations grouped by request responsibility.
//!
//! #1263 F5: the four registration files that lived here
//! (`world_services.rs` 14, `inventory_actions.rs` 6, `world_queries.rs` 2 and
//! `logout.rs` 1 = 23 entries) moved to the application crate's explicit area
//! registrar `wow_world_application::character_account_handlers`, published by
//! that crate's root facade and invoked from
//! `crate::handler_composition::compose_packet_handlers_like_cpp`. They no
//! longer reach the shared registry through the legacy `inventory::submit!`
//! drain. The character-select stub below remains and registers nothing.

mod character_setup;
