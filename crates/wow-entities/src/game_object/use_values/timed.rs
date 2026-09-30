use super::*;

impl<'a> GameObjectUseValues<'a> {
    // GameObject.cpp UseDoorOrButton/SwitchDoorOrButton. Retain represented
    // Option defaults and supplied restore time; no template fallback is added.
    pub fn use_door_or_button(
        self,
        user_guid: ObjectGuid,
        restore_time_ms: u32,
        now: Instant,
    ) -> Option<GoState> {
        if self
            .loot_state
            .is_some_and(|loot_state| loot_state != LootState::Ready)
        {
            return None;
        }

        let current_go_state = self.go_state.unwrap_or(GoState::Ready);
        let next_go_state = if current_go_state == GoState::Ready {
            GoState::Active
        } else {
            GoState::Ready
        };
        *self.prev_go_state = Some(current_go_state);
        *self.go_state = Some(next_go_state);
        *self.loot_state = Some(LootState::Activated);
        *self.loot_state_unit_guid = user_guid;
        *self.gameobject_flags |= GO_FLAG_IN_USE;
        *self.cooldown_until = (restore_time_ms != 0)
            .then_some(now + Duration::from_millis(u64::from(restore_time_ms)));
        Some(next_go_state)
    }

    // GameObject.cpp ResetDoorOrButton; None is intentionally not Ready.
    pub fn reset_door_or_button(self) -> Option<GoState> {
        if matches!(
            *self.loot_state,
            Some(LootState::Ready | LootState::JustDeactivated)
        ) {
            return None;
        }

        *self.gameobject_flags &= !GO_FLAG_IN_USE;
        let restored_go_state = self.prev_go_state.unwrap_or(GoState::Ready);
        *self.go_state = Some(restored_go_state);
        *self.loot_state = Some(LootState::JustDeactivated);
        *self.cooldown_until = None;
        Some(restored_go_state)
    }

    pub fn tick_door_or_button(self, now: Instant) -> Option<GoState> {
        let expired = self
            .cooldown_until
            .is_some_and(|cooldown_until| cooldown_until <= now);
        if !expired {
            return None;
        }
        self.reset_door_or_button()
    }

    // GameObject.cpp Use, trap branch. Emit at each original mutation boundary;
    // an interrupted caller must not receive a fully committed buffered result.
    pub fn use_trap(
        self,
        source: TrapUseSource,
        now: Instant,
        mut emit: impl FnMut(TrapUseEffect),
    ) -> bool {
        if self
            .cooldown_until
            .is_some_and(|cooldown_until| cooldown_until > now)
        {
            emit(TrapUseEffect::CooldownRejected);
            return false;
        }
        *self.go_type = Some(GAMEOBJECT_TYPE_TRAP as u8);
        *self.trap_use_source = Some(source);

        if source.spell_id != 0 {
            emit(TrapUseEffect::CastSpell {
                spell_id: source.spell_id,
            });
        }

        let cooldown_secs = if source.cooldown_secs != 0 {
            source.cooldown_secs
        } else {
            4
        };
        *self.cooldown_until =
            Some(now + Duration::from_millis(u64::from(cooldown_secs).saturating_mul(1000)));
        emit(TrapUseEffect::CooldownStarted { cooldown_secs });

        if source.charges == 1 {
            *self.loot_state = Some(LootState::JustDeactivated);
        }

        true
    }

    // Zero must bypass both the caller's clock and entry preparation.
    pub fn apply_cooldown(
        cooldown_secs: u32,
        prepare: impl FnOnce() -> (Instant, &'a mut Option<Instant>),
    ) -> CooldownOutcome {
        if cooldown_secs == 0 {
            return CooldownOutcome::NoCooldown;
        }
        let (now, cooldown_until) = prepare();
        if cooldown_until.is_some_and(|cooldown_until| cooldown_until > now) {
            return CooldownOutcome::Rejected;
        }
        *cooldown_until =
            Some(now + Duration::from_millis(u64::from(cooldown_secs).saturating_mul(1000)));
        CooldownOutcome::Started { cooldown_secs }
    }

    // The batch's type/Activated gates are deliberately stronger than the
    // standalone expired-door operation.
    pub fn door_reset_due(
        go_type: Option<u8>,
        loot_state: Option<LootState>,
        cooldown_until: Option<Instant>,
        now: Instant,
    ) -> bool {
        let is_door_or_button = matches!(
            go_type.map(u32::from),
            Some(GAMEOBJECT_TYPE_DOOR | GAMEOBJECT_TYPE_BUTTON)
        );
        let is_activated = loot_state == Some(LootState::Activated);
        let cooldown_expired = cooldown_until.is_some_and(|cooldown_until| cooldown_until <= now);
        is_door_or_button && is_activated && cooldown_expired
    }
}
