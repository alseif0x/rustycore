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
        // packet so a pending visibility refresh can run first. A committed
        // AuraUpdate is a prefix barrier: it must remain before a visibility
        // refresh even when an older gated spell precedes it. The rails do not
        // share an enqueue ordinal, so this is not a global cross-rail ordering
        // guarantee.
        let first_visible = durable_commands
            .iter()
            .position(SessionCommand::is_visibility_gated_like_cpp)
            .unwrap_or(durable_commands.len());
        let mut commands = durable_commands;
        let after_committed_aura = commands
            .iter()
            .rposition(SessionCommand::is_committed_aura_update)
            .map_or(0, |index| index + 1);
        let durable_prefix_end = first_visible.max(after_committed_aura);
        let deferred_durable_suffix = commands.split_off(durable_prefix_end);
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
