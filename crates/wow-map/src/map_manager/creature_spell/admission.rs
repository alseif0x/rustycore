//! Current canonical targetability and lazy faction/reputation admission.
use super::*;
const PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP: u32 = 0x0000_0100;

pub fn target_is_valid(
    caster: &wow_entities::Creature,
    victim: &wow_entities::Player,
    spell_attributes: &[u32; 15],
    policies: &mut SpellPolicies<'_>,
) -> bool {
    if !caster.unit().world().object().is_in_world()
        || !victim.unit().world().object().is_in_world()
        || !caster.unit().is_alive()
        || !victim.unit().is_alive()
        || victim.is_game_master_like_cpp()
        || victim.unit().unit_state() & (UnitState::DIED | UnitState::IN_FLIGHT).bits() != 0
        || !caster
            .unit()
            .can_see_or_detect_unit_like_cpp(victim.unit(), false, false, false)
    {
        return false;
    }

    if !(policies.faction_authority)() { return false; }
    let Ok(caster_faction_template_id) = u32::try_from(caster.unit().data().faction_template)
    else {
        return false;
    };
    let Ok(victim_faction_template_id) = u32::try_from(victim.unit().data().faction_template)
    else {
        return false;
    };
    let Some(factions) = (policies.factions)(caster_faction_template_id, victim_faction_template_id)
        else { return false; };

    let mut victim_flags = victim.unit().unit_flags_like_cpp();
    if spell_attributes[6] & 0x0100_0000 != 0 {
        victim_flags.remove(UnitFlags::NON_ATTACKABLE_2);
    }
    let mut context = wow_entities::UnitAttackContextLikeCpp {
        victim_is_game_master_player: victim.is_game_master_like_cpp(),
        visibility_represented: true,
        attacker_can_see_or_detect_target: true,
        victim_unit_state: victim.unit().unit_state(),
        attacker_unit_flags: caster.unit().unit_flags_like_cpp().bits(),
        victim_unit_flags: victim_flags.bits(),
        relation_represented: true,
        attacker_is_hostile_to_victim: factions.caster_hostile,
        victim_is_hostile_to_attacker: factions.victim_hostile,
        attacker_is_friendly_to_victim: factions.caster_friendly,
        victim_is_friendly_to_attacker: factions.victim_friendly,
        victim_has_affecting_player: true,
        ..Default::default()
    };

    let creature_faction_id = factions.creature_faction_id;
    if creature_faction_id != 0 {
        if victim.has_forced_reputation_rank_like_cpp(creature_faction_id) {
            // The canonical player currently retains only the presence of a
            // forced reaction, not its rank. Its exact reaction is therefore
            // unrepresented at this cast-time boundary.
            return false;
        }
        let Some(can_have_reputation) = (policies.can_have_reputation)(creature_faction_id)
            else { return false; };
        if can_have_reputation
            && victim.has_reputation_state_like_cpp(creature_faction_id)
        {
            context.player_creature_reputation_represented = true;
            context.creature_is_contested_guard =
                factions.contested_guard;
            context.player_has_contested_pvp_flag =
                victim.has_player_flag(PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP);
            context.player_at_war_with_creature_faction =
                victim.is_at_war_with_faction_like_cpp(creature_faction_id);
        }
    }

    wow_entities::Unit::is_valid_attack_target_represented_like_cpp(&context)
}
