//! Character lifecycle, account enumeration and world-entry application contracts.

#[path = "character_lifecycle/fixtures/mod.rs"]
mod fixtures;
use fixtures::*;

#[path = "character_lifecycle/account_enumeration.rs"]
mod account_enumeration;
#[path = "character_lifecycle/corpse_loading.rs"]
mod corpse_loading;
#[path = "character_lifecycle/creation_support.rs"]
mod creation_support;
#[path = "character_lifecycle/enumeration_support.rs"]
mod enumeration_support;
#[path = "character_lifecycle/initial_packets.rs"]
mod initial_packets;
#[path = "character_lifecycle/initial_packets_post_add_rest.rs"]
mod initial_packets_post_add_rest;
#[path = "character_lifecycle/initial_packets_post_add_scaling.rs"]
mod initial_packets_post_add_scaling;
#[path = "character_lifecycle/initial_packets_spell.rs"]
mod initial_packets_spell;
#[path = "character_lifecycle/lifecycle_corpse.rs"]
mod lifecycle_corpse;
#[path = "character_lifecycle/lifecycle_customize.rs"]
mod lifecycle_customize;
#[path = "character_lifecycle/lifecycle_hearth.rs"]
mod lifecycle_hearth;
#[path = "character_lifecycle/lifecycle_map_corpse.rs"]
mod lifecycle_map_corpse;
#[path = "character_lifecycle/lifecycle_profile.rs"]
mod lifecycle_profile;
#[path = "character_lifecycle/lifecycle_rename.rs"]
mod lifecycle_rename;
#[path = "character_lifecycle/login_context.rs"]
mod login_context;
#[path = "character_lifecycle/login_prelude.rs"]
mod login_prelude;
#[path = "character_lifecycle/login_recovery.rs"]
mod login_recovery;
#[path = "character_lifecycle/login_recovery_homebind_persistence.rs"]
mod login_recovery_homebind_persistence;
#[path = "character_lifecycle/login_support.rs"]
mod login_support;
#[path = "character_lifecycle/login_transport_support.rs"]
mod login_transport_support;
#[path = "character_lifecycle/session_state_transport.rs"]
mod session_state_transport;
#[path = "character_lifecycle/world_entry_cinematic.rs"]
mod world_entry_cinematic;

#[path = "character_lifecycle/character_save.rs"]
mod character_save;

#[path = "character_lifecycle/offline_persistence.rs"]
mod offline_persistence;

#[path = "character_lifecycle/login_claim.rs"]
mod login_claim;

#[path = "character_lifecycle/login_progression.rs"]
mod login_progression;

#[path = "character_lifecycle/logout_owner.rs"]
mod logout_owner;

#[path = "character_lifecycle/fixture_rail.rs"]
mod fixture_rail;
