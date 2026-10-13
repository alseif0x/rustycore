//! Represented spell-effect execution and the ticks that drive it.
//!
//! Separated from the Session root under #621.

use super::*;
use wow_world_spell::{
    apply_add_extra_attacks_effect_like_cpp, apply_block_effect_like_cpp,
    apply_dismiss_pet_effect_like_cpp, apply_distract_effect_like_cpp,
    apply_dual_wield_effect_like_cpp, apply_effect_add_farsight_like_cpp,
    apply_effect_bind_like_cpp, apply_effect_environmental_damage_like_cpp,
    apply_effect_summon_object_slot_like_cpp, apply_effect_summon_object_wild_with_focus_like_cpp,
    apply_effect_teleport_units_like_cpp, apply_energize_effect_like_cpp,
    apply_give_honor_effect_like_cpp, apply_heal_from_caster_like_cpp,
    apply_heal_max_health_like_cpp, apply_heal_pct_like_cpp, apply_health_leech_like_cpp,
    apply_inebriate_effect_like_cpp, apply_instakill_like_cpp,
    apply_modify_cooldown_effect_like_cpp, apply_parry_effect_like_cpp,
    apply_play_movie_effect_like_cpp, apply_power_drain_effect_like_cpp,
    apply_sanctuary_effect_like_cpp, apply_titan_grip_effect_like_cpp,
    apply_upgrade_heirloom_effect_like_cpp, has_implicit_target_conditions_like_cpp,
    implicit_target_conditions_like_cpp, power_drain_pre_scaled_damage_like_cpp,
    represented_disallowed_mount_form_like_cpp, represented_homebind_like_cpp,
    represented_object_target_position_like_cpp, represented_spell_damage_bonus_done_like_cpp,
    represented_spell_healing_bonus_done_like_cpp, send_spell_execute_log_like_cpp,
};
mod checks;
mod destinations;
mod effect_apply;
mod effect_combat;
mod effect_host;
mod effects;
mod effects_player;
mod effects_power;
mod effects_progress;
mod execution;
mod execution_overloads;
mod threat;
