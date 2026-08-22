// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Fixture module proving the #229 composition path end to end.

use wow_module_api::{
    ModuleDescriptor, ModuleRegistrationError, ModuleRegistry, ModuleVersion, PlayerLoginModule,
    PlayerLoginSnapshot, ScopedEffects,
};

struct Greeter;

impl PlayerLoginModule for Greeter {
    fn on_player_login(&self, snapshot: &PlayerLoginSnapshot, effects: &mut ScopedEffects<'_>) {
        if snapshot.first_login {
            effects.send_system_message_self("Welcome to RustyCore!");
        }
    }
}

/// The registrar the generated compositor calls.
pub fn register(registry: &mut ModuleRegistry) -> Result<(), ModuleRegistrationError> {
    let descriptor = ModuleDescriptor::new(
        "example.greeter",
        ModuleVersion { major: 1, minor: 0, patch: 0 },
        "Example Greeter",
    )?;
    registry.register_player_login_module(descriptor, Box::new(Greeter))
}
