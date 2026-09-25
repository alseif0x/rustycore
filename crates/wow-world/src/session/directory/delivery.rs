// Copyright (c) 2026 alseif0x
//! Current-incarnation command, packet, and runtime publication delivery.

use super::*;

impl PlayerRegistry {
    /// Queue a command only if the selected incarnation is still current.
    pub fn try_send_current_command(
        &self,
        registration: PlayerRegistration,
        command: SessionCommand,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let tx = entry.command_tx.clone();
        drop(entry);
        tx.try_send(command).map_err(|error| match error {
            flume::TrySendError::Full(_) => PlayerDirectorySendError::Full,
            flume::TrySendError::Disconnected(_) => PlayerDirectorySendError::Disconnected,
        })
    }

    /// Queue packet bytes on the normal socket only for the current incarnation.
    pub fn try_send_current_packet(
        &self,
        registration: PlayerRegistration,
        packet: Vec<u8>,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let tx = entry.send_tx.clone();
        drop(entry);
        tx.try_send(packet).map_err(|error| match error {
            flume::TrySendError::Full(_) => PlayerDirectorySendError::Full,
            flume::TrySendError::Disconnected(_) => PlayerDirectorySendError::Disconnected,
        })
    }

    /// Send packet bytes on the normal socket for the selected incarnation.
    pub fn send_current_packet(
        &self,
        registration: PlayerRegistration,
        packet: Vec<u8>,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let tx = entry.send_tx.clone();
        drop(entry);
        tx.send(packet)
            .map_err(|_| PlayerDirectorySendError::Disconnected)
    }

    /// Send packet bytes on the realm socket for the selected incarnation.
    pub fn send_current_realm_packet(
        &self,
        registration: PlayerRegistration,
        packet: Vec<u8>,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let tx = entry.realm_send_tx.clone();
        drop(entry);
        tx.send(packet)
            .map_err(|_| PlayerDirectorySendError::Disconnected)
    }

    /// Wait for command-queue capacity for the selected incarnation.
    pub async fn send_current_command(
        &self,
        registration: PlayerRegistration,
        command: SessionCommand,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let tx = entry.command_tx.clone();
        drop(entry);
        tx.send_async(command)
            .await
            .map_err(|_| PlayerDirectorySendError::Disconnected)
    }

    /// Wait for command-queue capacity up to the caller's delivery deadline.
    pub async fn send_current_command_timeout(
        &self,
        registration: PlayerRegistration,
        command: SessionCommand,
        timeout: std::time::Duration,
    ) -> Result<(), PlayerDirectorySendError> {
        tokio::time::timeout(timeout, self.send_current_command(registration, command))
            .await
            .map_err(|_| PlayerDirectorySendError::Full)?
    }

    /// Blocking timeout variant used by synchronous publication adapters.
    pub fn send_current_command_blocking_timeout(
        &self,
        registration: PlayerRegistration,
        command: SessionCommand,
        timeout: std::time::Duration,
    ) -> Result<(), PlayerDirectorySendError> {
        let entry = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
            .ok_or(PlayerDirectorySendError::StaleRegistration)?;
        let tx = entry.command_tx.clone();
        drop(entry);
        tx.send_timeout(command, timeout)
            .map_err(|error| match error {
                flume::SendTimeoutError::Timeout(_) => PlayerDirectorySendError::Full,
                flume::SendTimeoutError::Disconnected(_) => PlayerDirectorySendError::Disconnected,
            })
    }

    /// Retain an authoritative command across bounded queue backpressure.
    pub fn queue_current_command_reliably(
        &self,
        registration: PlayerRegistration,
        command: SessionCommand,
    ) -> PlayerDirectoryReliableSendOutcome {
        let Some(entry) = self
            .entries
            .get(&registration.guid)
            .filter(|entry| entry.generation == registration.generation)
        else {
            return PlayerDirectoryReliableSendOutcome::StaleOrDisconnected;
        };
        let tx = entry.command_tx.clone();
        drop(entry);
        match tx.try_send(command) {
            Ok(()) => PlayerDirectoryReliableSendOutcome::Queued,
            Err(flume::TrySendError::Disconnected(_)) => {
                PlayerDirectoryReliableSendOutcome::StaleOrDisconnected
            }
            Err(flume::TrySendError::Full(command)) => {
                tokio::spawn(async move {
                    let _ = tx.send_async(command).await;
                });
                PlayerDirectoryReliableSendOutcome::Retrying
            }
        }
    }

    fn with_current_durable_runtime(
        &self,
        registration: PlayerRegistration,
    ) -> Option<Arc<Mutex<DurableCreatureRuntimeCommandsLikeCpp>>> {
        let entry = self.entries.get(&registration.guid)?;
        (entry.generation == registration.generation)
            .then(|| Arc::clone(&entry.durable_creature_runtime_commands_like_cpp))
    }

    pub fn publish_current_attack_start(
        &self,
        registration: PlayerRegistration,
        command: CreatureAttackStartLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable
                    .lock()
                    .ok()
                    .map(|mut durable| durable.publish_attack_start_like_cpp(command))
            })
            .unwrap_or(false)
    }

    pub fn publish_current_attack_stop(
        &self,
        registration: PlayerRegistration,
        command: CreatureAttackStopLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable
                    .lock()
                    .ok()
                    .map(|mut durable| durable.publish_attack_stop_like_cpp(command))
            })
            .unwrap_or(false)
    }

    pub fn publish_current_melee_damage(
        &self,
        registration: PlayerRegistration,
        command: ApplyCreatureMeleeDamageLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable
                    .lock()
                    .ok()
                    .map(|mut durable| durable.publish_melee_damage_like_cpp(command))
            })
            .unwrap_or(false)
    }

    /// Publish one map-owned player auto-attack resolution to its attacker.
    ///
    /// Generation-checked like every other current-incarnation publish: a
    /// result resolved for a session that has since reconnected is dropped
    /// here rather than delivered to the new one.
    pub fn publish_current_player_melee_result(
        &self,
        registration: PlayerRegistration,
        command: ApplyPlayerMeleeResultLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable
                    .lock()
                    .ok()
                    .map(|mut durable| durable.publish_player_melee_result_like_cpp(command))
            })
            .unwrap_or(false)
    }

    pub fn publish_current_send_if_visible(
        &self,
        registration: PlayerRegistration,
        command: SendIfVisibleLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable
                    .lock()
                    .ok()
                    .map(|mut durable| durable.publish_send_if_visible_like_cpp(command))
            })
            .unwrap_or(false)
    }

    pub fn publish_current_destroy_visible_object(
        &self,
        registration: PlayerRegistration,
        command: DestroyVisibleObjectLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable
                    .lock()
                    .ok()
                    .map(|mut durable| durable.publish_destroy_visible_object_like_cpp(command))
            })
            .unwrap_or(false)
    }

    pub fn publish_current_player_spell_if_visible(
        &self,
        registration: PlayerRegistration,
        command: SendPlayerSpellIfVisibleLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable
                    .lock()
                    .ok()
                    .map(|mut queue| queue.publish_player_spell_if_visible_like_cpp(command))
            })
            .unwrap_or(false)
    }

    pub fn publish_current_creature_spell_cast_if_visible(
        &self,
        registration: PlayerRegistration,
        command: SendCreatureSpellCastIfVisibleLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable.lock().ok().map(|mut durable| {
                    durable.publish_creature_spell_cast_if_visible_like_cpp(command)
                })
            })
            .unwrap_or(false)
    }

    pub fn publish_current_pvp_combat_expiry(
        &self,
        registration: PlayerRegistration,
        command: ReconcilePvpCombatExpiryLikeCppCommand,
    ) -> bool {
        self.with_current_durable_runtime(registration)
            .and_then(|durable| {
                durable.lock().ok().map(|mut durable| {
                    durable.publish_pvp_combat_expiry_like_cpp(command);
                    true
                })
            })
            .unwrap_or(false)
    }
}
