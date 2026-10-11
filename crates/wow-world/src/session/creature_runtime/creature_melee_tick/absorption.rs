//! Canonical shield resolution and mutation for creature melee.
//!
//! This private phase updates school and mana absorb pools on the canonical
//! victim alongside the map-owned melee transition.

use super::{ObjectGuid, RecipientRule, RuntimeEvent};

/// C++ `Unit::CalcAbsorbResist`'s represented stages for a player victim
/// (`Unit.cpp:1789-1930`), committed inside the same map-owned phase as the
/// victim's health write: the school-absorb loop, then the mana-shield loop.
///
/// C++ spends each shield effect's amount and the mana-shield drain while it
/// calculates the swing, so both are pool data and this map-owned stage is
/// their writer; the session's aura transition at delivery owns the *removal*
/// of an exhausted shield and its publication. A shield C++ would remove is
/// left at zero here, which the session removes through the same `remove_aura`
/// path it owns for every other aura.
///
/// Returns `None` when the map or the canonical player cannot be resolved,
/// which keeps the caller's pre-absorb damage unchanged. Otherwise the tuple is
/// `(absorbed, remaining damage, mana spent, consumptions)` in the delivery
/// command's shape, with the outcomes in `AbsorbAuraOrderPred` order followed
/// by the mana shields.
#[allow(clippy::type_complexity)]
pub(super) fn apply_melee_absorb_to_canonical_player_like_cpp(
    canonical_manager: &mut wow_map::MapManager,
    map_id: u32,
    instance_id: u32,
    victim_guid: ObjectGuid,
    school_mask: u32,
    damage: u32,
    spell_store: &wow_data::SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    // C++ `CalcAbsorbResist`'s `auraAbsorbMod` from the attacker's
    // `SPELL_AURA_MOD_TARGET_ABSORB_SCHOOL`.
    ignore_absorb_pct: f32,
) -> Option<(
    u32,
    u32,
    u32,
    Vec<crate::session::mailbox::CreatureMeleeAbsorbConsumptionLikeCpp>,
)> {
    let managed = canonical_manager.find_map_mut(map_id, instance_id)?;
    let player = managed.map_mut().get_typed_player_mut(victim_guid)?;
    let auras = player
        .unit()
        .subsystems()
        .auras
        .runtime_applications_like_cpp()
        .clone();
    let shields = crate::session_rules::player_absorb_shields_like_cpp(
        &auras,
        spell_store,
        difficulty_id,
        difficulty_store,
        school_mask,
    );
    let absorb = crate::session_rules::represented_melee_absorb_like_cpp(
        &shields,
        damage,
        ignore_absorb_pct,
    );
    let mut consumptions = Vec::with_capacity(absorb.consumed.len());
    for consumption in &absorb.consumed {
        write_absorbed_shield_amount_like_cpp(player, consumption);
        consumptions.push(
            crate::session::mailbox::CreatureMeleeAbsorbConsumptionLikeCpp {
                slot: consumption.slot,
                consumed: consumption.consumed,
                removed: consumption.removed,
            },
        );
    }

    // C++ runs the mana-shield loop after the school-absorb loop
    // (`Unit.cpp:1886-1930`) over the damage the school shields left.
    let mana_shields = crate::session_rules::player_mana_shields_like_cpp(
        &auras,
        spell_store,
        difficulty_id,
        difficulty_store,
        school_mask,
    );
    let mana_before = player
        .unit()
        .get_power(wow_constants::PowerType::Mana)
        .max(0);
    let mana_absorb = crate::session_rules::represented_melee_mana_absorb_like_cpp(
        &mana_shields,
        absorb.damage,
        mana_before as u32,
        ignore_absorb_pct,
    );
    if mana_absorb.mana_spent > 0 {
        // `Unit::ModifyPower(POWER_MANA, -manaReduction)`: the same locked map
        // phase that commits the health write owns the drain, and the canonical
        // setter clamps it like C++.
        player.unit_mut().set_power(
            wow_constants::PowerType::Mana,
            mana_before - i32::try_from(mana_absorb.mana_spent).unwrap_or(i32::MAX),
        );
    }
    for consumption in &mana_absorb.consumed {
        write_absorbed_shield_amount_like_cpp(
            player,
            &crate::session_rules::RepresentedAbsorbConsumptionLikeCpp {
                slot: consumption.slot,
                effect_index: consumption.effect_index,
                consumed: consumption.consumed,
                remaining: consumption.remaining,
                removed: consumption.removed,
            },
        );
        consumptions.push(
            crate::session::mailbox::CreatureMeleeAbsorbConsumptionLikeCpp {
                slot: consumption.slot,
                consumed: consumption.consumed,
                removed: consumption.removed,
            },
        );
    }
    Some((
        absorb.absorbed + mana_absorb.absorbed,
        mana_absorb.damage,
        mana_absorb.mana_spent,
        consumptions,
    ))
}

/// Commit one spent shield's `AuraEffect` remainder on the canonical player.
fn write_absorbed_shield_amount_like_cpp(
    player: &mut wow_entities::Player,
    consumption: &crate::session_rules::RepresentedAbsorbConsumptionLikeCpp,
) {
    crate::session::combat::write_absorbed_shield_amount_like_cpp(
        player,
        consumption.slot,
        consumption.effect_index,
        consumption.remaining,
    );
}

/// C++ `Unit::CalcAbsorbResist`'s school-absorb loop for a creature victim.
///
/// The creature is map-owned, so this stage spends the canonical aura amount
/// beside the health write. Its combat-log and exhausted-aura publications are
/// returned as map events and therefore retain C++'s order before the
/// `AttackerStateUpdate` event.
#[allow(clippy::type_complexity)]
pub(super) fn apply_melee_absorb_to_canonical_creature_like_cpp(
    canonical_manager: &mut wow_map::MapManager,
    map_id: u16,
    instance_id: u32,
    attacker_guid: ObjectGuid,
    victim_guid: ObjectGuid,
    school_mask: u32,
    damage: u32,
    original_damage: i32,
    spell_store: &wow_data::SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    ignore_absorb_pct: f32,
) -> Option<(u32, u32, Vec<RuntimeEvent>)> {
    use wow_packet::ServerPacket;

    let managed = canonical_manager.find_map_mut(u32::from(map_id), instance_id)?;
    let victim = managed.map_mut().get_typed_creature_mut(victim_guid)?;
    let shields = crate::session_rules::creature_absorb_shields_like_cpp(
        &victim.unit().subsystems().auras,
        spell_store,
        difficulty_id,
        difficulty_store,
        school_mask,
    );
    let absorb = crate::session_rules::represented_melee_absorb_like_cpp(
        &shields,
        damage,
        ignore_absorb_pct,
    );
    if absorb.consumed.is_empty() {
        return Some((0, damage, Vec::new()));
    }

    let mut events = Vec::new();
    for consumption in &absorb.consumed {
        let Some(applied) = victim
            .unit()
            .subsystems()
            .auras
            .applied_auras
            .iter()
            .find(|aura| {
                aura.slot == consumption.slot
                    && 1_u32
                        .checked_shl(u32::from(consumption.effect_index))
                        .is_some_and(|bit| aura.effect_mask & bit != 0)
            })
            .copied()
        else {
            continue;
        };

        if consumption.consumed > 0 {
            events.push(RuntimeEvent {
                source_guid: victim_guid,
                recipients: RecipientRule::MapBroadcastVisible {
                    map_id,
                    instance_id,
                },
                packet_bytes: wow_packet::packets::combat::SpellAbsorbLog {
                    attacker: attacker_guid,
                    victim: victim_guid,
                    absorbed_spell_id: 0,
                    absorb_spell_id: i32::try_from(applied.spell_id).unwrap_or(i32::MAX),
                    caster: applied.caster_guid,
                    absorbed: consumption.consumed,
                    original_damage,
                }
                .to_bytes(),
            });
        }

        if consumption.removed {
            let aura_ref = applied.aura_ref();
            let covered: Vec<_> = victim
                .unit()
                .subsystems()
                .auras
                .applied_auras
                .iter()
                .filter(|candidate| candidate.aura_ref() == aura_ref)
                .copied()
                .collect();
            let auras = &mut victim.unit_mut().subsystems_mut().auras;
            for covered in covered {
                auras.unapply_aura(covered, 0);
            }
            let _ = auras.clear_visible(applied.slot);
            events.push(RuntimeEvent {
                source_guid: victim_guid,
                recipients: RecipientRule::MapBroadcastVisible {
                    map_id,
                    instance_id,
                },
                packet_bytes: wow_packet::packets::misc::AuraUpdate {
                    unit_guid: victim_guid,
                    update_all: false,
                    auras: vec![wow_packet::packets::misc::AuraInfoLikeCpp {
                        slot: applied.slot,
                        aura_data: None,
                    }],
                }
                .to_bytes(),
            });
        } else if let Some(amount) = victim
            .unit_mut()
            .subsystems_mut()
            .auras
            .applied_aura_amounts
            .get_mut(&applied)
        {
            *amount = consumption.remaining.max(0);
        }
    }

    Some((absorb.absorbed, absorb.damage, events))
}
