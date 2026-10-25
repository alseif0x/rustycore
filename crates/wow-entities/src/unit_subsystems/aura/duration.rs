/// Convert the former unsigned AuraApplication duration convention.
///
/// Older Rust producers used `(0, 0)` for an aura whose C++ maximum and
/// remaining durations are both `-1`. A zero maximum with a positive
/// remaining duration used the same permanent maximum but retained that
/// remaining value. Raw selected database rows must bypass this adapter.
pub fn legacy_aura_duration_pair(max_duration: u32, remaining: u32) -> Option<(i32, i32)> {
    let max_duration = i32::try_from(max_duration).ok()?;
    let remaining = i32::try_from(remaining).ok()?;

    Some(if max_duration == 0 {
        if remaining == 0 {
            (-1, -1)
        } else {
            (-1, remaining)
        }
    } else {
        (max_duration, remaining)
    })
}

#[cfg(test)]
mod tests {
    use super::legacy_aura_duration_pair;
    use crate::{AuraApplicationLikeCpp, AuraCastProvenanceLikeCpp, Player};
    use std::time::Instant;
    use wow_core::ObjectGuid;

    #[test]
    fn canonical_duration_adapter_preserves_legacy_permanence_and_checks_overflow() {
        assert_eq!(legacy_aura_duration_pair(0, 0), Some((-1, -1)));
        assert_eq!(legacy_aura_duration_pair(0, 55_000), Some((-1, 55_000)));
        assert_eq!(legacy_aura_duration_pair(12_000, 0), Some((12_000, 0)));
        assert_eq!(
            legacy_aura_duration_pair(12_000, 4_000),
            Some((12_000, 4_000))
        );
        assert_eq!(legacy_aura_duration_pair(u32::MAX, 0), None);
        assert_eq!(legacy_aura_duration_pair(0, u32::MAX), None);
    }

    #[test]
    fn canonical_duration_setter_keeps_zero_and_negative_values_literal() {
        let mut player = Player::new(Some(1), false);
        player.insert_player_visible_aura_like_cpp(AuraApplicationLikeCpp {
            spell_id: 8_326,
            difficulty_id: 0,
            caster_guid: ObjectGuid::EMPTY,
            slot: 0,
            duration_total: -1,
            duration_remaining: -1,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: None,
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: Instant::now(),
        });

        let zero_update = player.set_aura_duration(8_326, ObjectGuid::EMPTY, 0);
        assert_eq!(zero_update.len(), 1);
        assert_eq!(zero_update[0].0.duration_total, -1);
        assert_eq!(zero_update[0].0.duration_remaining, 0);

        let negative_update = player.set_aura_duration(8_326, ObjectGuid::EMPTY, -1);
        assert_eq!(negative_update.len(), 1);
        assert_eq!(negative_update[0].0.duration_total, -1);
        assert_eq!(negative_update[0].0.duration_remaining, -1);
        assert_eq!(negative_update[0].1, AuraCastProvenanceLikeCpp::default());
    }
}
