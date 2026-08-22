//! __MODULE_DISPLAY__ — a RustyCore trusted linked module.
//!
//! This is in-process code compiled into the server with full privileges.
//! Changing it requires a rebuild and restart; there is no sandbox and no
//! hot reload.

use wow_module_api::{
    ModuleDescriptor, ModuleRegistrationError, ModuleRegistry, ModuleVersion, PlayerLoginModule,
    PlayerLoginSnapshot, ScopedEffects,
};

pub(crate) struct Module;

impl PlayerLoginModule for Module {
    fn on_player_login(&self, snapshot: &PlayerLoginSnapshot, effects: &mut ScopedEffects<'_>) {
        if snapshot.first_login {
            effects.send_system_message_self("__MODULE_DISPLAY__ is installed.");
        }
    }
}

/// Registrar called by the generated compositor. Keep the name in sync with
/// `registrar` in `module.toml`.
pub fn register(registry: &mut ModuleRegistry) -> Result<(), ModuleRegistrationError> {
    let descriptor = ModuleDescriptor::new(
        "__MODULE_ID__",
        ModuleVersion { major: 0, minor: 1, patch: 0 },
        "__MODULE_DISPLAY__",
    )?;
    registry.register_player_login_module(descriptor, Box::new(Module))
}
