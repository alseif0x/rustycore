use crate::session::mailbox::SessionCommand;
use crate::session::state::SessionCore;

impl SessionCore {
    pub fn drain_session_commands(&self) -> Vec<SessionCommand> {
        let durable_commands = self
            .durable_creature_runtime_commands_like_cpp
            .lock()
            .map(|mut pending| pending.drain_like_cpp())
            .unwrap_or_default();
        // Drain the bounded general rail before the first durable presentation
        // packet so a pending visibility refresh can run first. The rails do
        // not yet share an enqueue ordinal, so this is not a global cross-rail
        // ordering guarantee. The spell pair itself still occupies one durable
        // command and therefore cannot be split by this merge.
        let first_visible = durable_commands
            .iter()
            .position(SessionCommand::is_visibility_gated_like_cpp)
            .unwrap_or(durable_commands.len());
        let mut commands = durable_commands;
        let deferred_durable_suffix = commands.split_off(first_visible);
        while let Ok(command) = self.session_command_rx.try_recv() {
            commands.push(command);
        }
        commands.extend(deferred_durable_suffix);
        commands
    }

    pub fn take_durable_creature_runtime_overflow_like_cpp(&self) -> bool {
        self.durable_creature_runtime_commands_like_cpp
            .lock()
            .map(|mut pending| pending.take_overflowed_and_discard_like_cpp())
            .unwrap_or(true)
    }
}
