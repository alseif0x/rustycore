//! Projection of an active movement spline into CreateObject packet data.

use wow_core::Position;
use wow_movement::{MoveSpline, MoveSplineFlag};
use wow_packet::packets::movement::{
    MonsterMoveFace, MonsterSplineAnimTierTransition, MonsterSplineJumpExtraData,
    MonsterSplineSpellEffectExtraData,
};
use wow_packet::packets::update::CreateObjectSplineDataLikeCpp;

pub(crate) fn create_object_spline_data_like_cpp(
    spline: &MoveSpline,
) -> Option<CreateObjectSplineDataLikeCpp> {
    if !spline.initialized() || spline.finalized() {
        return None;
    }

    let flags = spline.flags();
    let raw_effect_start_time = spline.effect_start_time_ms();
    let effect_start_time = raw_effect_start_time.max(0) as u32;
    let duration_ms = spline.duration_ms().max(0) as u32;
    let spell_effect_extra = spline.spell_effect_extra().map(|extra| {
        MonsterSplineSpellEffectExtraData {
            target_guid: extra.target,
            spell_visual_id: extra.spell_visual_id,
            progress_curve_id: extra.progress_curve_id,
            parabolic_curve_id: extra.parabolic_curve_id,
            jump_gravity: spline.vertical_acceleration(),
        }
    });
    let jump_extra = (flags.contains(MoveSplineFlag::PARABOLIC)
        && (spell_effect_extra.is_none() || effect_start_time != 0))
        .then(|| MonsterSplineJumpExtraData {
            jump_gravity: spline.vertical_acceleration(),
            start_time: effect_start_time,
            duration: 0,
        });
    let anim_tier_transition = spline.anim_tier().map(|anim_tier| {
        MonsterSplineAnimTierTransition {
            tier_transition_id: anim_tier.tier_transition_id as i32,
            start_time: effect_start_time,
            end_time: 0,
            anim_tier: anim_tier.anim_tier,
        }
    });
    let facing = spline.facing();

    Some(CreateObjectSplineDataLikeCpp {
        id: spline.id(),
        destination: if spline.is_cyclic() {
            Position::ZERO
        } else {
            spline.final_destination().unwrap_or(Position::ZERO)
        },
        has_spline_move: !spline.finalized() && !spline.spline_is_facing_only_like_cpp(),
        flags: flags.bits(),
        elapsed_ms: spline.time_passed_ms(),
        duration_ms,
        face: match facing.kind {
            wow_movement::MonsterMoveType::Normal => MonsterMoveFace::Normal,
            wow_movement::MonsterMoveType::FacingSpot => MonsterMoveFace::FacingSpot(facing.spot),
            wow_movement::MonsterMoveType::FacingTarget => MonsterMoveFace::FacingTarget {
                direction: facing.angle,
                target_guid: facing.target,
            },
            wow_movement::MonsterMoveType::FacingAngle => {
                MonsterMoveFace::FacingAngle(facing.angle)
            }
        },
        fade_object_time: (flags.contains(MoveSplineFlag::FADE_OBJECT)
            && effect_start_time < duration_ms)
            .then_some(effect_start_time),
        points: spline.create_object_path_points_like_cpp().to_vec(),
        spell_effect_extra,
        jump_extra,
        anim_tier_transition,
    })
}

#[cfg(test)]
mod tests {
    use super::create_object_spline_data_like_cpp;
    use wow_core::{ObjectGuid, Position};
    use wow_movement::{
        AnimTierTransition, FacingInfo, MonsterMoveType, MoveSpline, MoveSplineFlag,
        MoveSplineInitArgs, SpellEffectExtraData,
    };
    use wow_packet::packets::movement::{
        MonsterMoveFace, MonsterSplineAnimTierTransition, MonsterSplineJumpExtraData,
        MonsterSplineSpellEffectExtraData,
    };

    fn launch_spline(args: MoveSplineInitArgs) -> MoveSpline {
        let mut spline = MoveSpline::new();
        spline.initialize(&args).unwrap();
        spline
    }

    fn line_args() -> MoveSplineInitArgs {
        MoveSplineInitArgs {
            path: vec![Position::xyz(0.0, 0.0, 0.0), Position::xyz(10.0, 0.0, 0.0)],
            velocity: 10.0,
            ..MoveSplineInitArgs::default()
        }
    }

    #[test]
    fn projection_omits_uninitialized_and_finalized_splines() {
        let inactive = MoveSpline::new();
        assert!(create_object_spline_data_like_cpp(&inactive).is_none());

        let mut finalized = launch_spline(line_args());
        finalized.finalize();
        assert!(create_object_spline_data_like_cpp(&finalized).is_none());
    }

    #[test]
    fn facing_only_projection_keeps_path_points_without_spline_payload() {
        let mut args = line_args();
        args.path = vec![Position::xyz(0.0, 0.0, 0.0), Position::xyz(0.05, 0.0, 0.0)];
        args.facing = FacingInfo {
            kind: MonsterMoveType::FacingAngle,
            angle: 1.25,
            ..FacingInfo::default()
        };
        let spline = launch_spline(args);

        let projected = create_object_spline_data_like_cpp(&spline).unwrap();

        assert!(!projected.has_spline_move);
        assert_eq!(projected.face, MonsterMoveFace::FacingAngle(1.25));
        assert_eq!(
            projected.points,
            spline.create_object_path_points_like_cpp().to_vec()
        );
    }

    #[test]
    fn projection_preserves_identity_destination_flags_and_timing() {
        let args = MoveSplineInitArgs {
            spline_id: 123,
            flags: MoveSplineFlag::UNCOMPRESSED_PATH,
            ..line_args()
        };
        let spline = launch_spline(args);

        let projected = create_object_spline_data_like_cpp(&spline).unwrap();

        assert_eq!(projected.id, 123);
        assert_eq!(projected.destination, spline.final_destination().unwrap());
        assert!(projected.has_spline_move);
        assert_eq!(projected.flags, spline.flags().bits());
        assert_eq!(projected.elapsed_ms, spline.time_passed_ms());
        assert_eq!(projected.duration_ms, spline.duration_ms().max(0) as u32);
        assert_eq!(
            projected.points,
            spline.create_object_path_points_like_cpp().to_vec()
        );
    }

    #[test]
    fn projection_preserves_spell_jump_and_fade_extra_values() {
        let target = ObjectGuid::create_player(1, 55);
        let args = MoveSplineInitArgs {
            flags: MoveSplineFlag::PARABOLIC | MoveSplineFlag::FADE_OBJECT,
            vertical_acceleration: 12.5,
            effect_start_time_ms: 125,
            spell_effect_extra: Some(SpellEffectExtraData {
                target,
                spell_visual_id: 777,
                progress_curve_id: 888,
                parabolic_curve_id: 999,
            }),
            ..line_args()
        };
        let spline = launch_spline(args);

        let projected = create_object_spline_data_like_cpp(&spline).unwrap();

        assert_eq!(projected.fade_object_time, Some(125));
        assert_eq!(
            projected.spell_effect_extra,
            Some(MonsterSplineSpellEffectExtraData {
                target_guid: target,
                spell_visual_id: 777,
                progress_curve_id: 888,
                parabolic_curve_id: 999,
                jump_gravity: 12.5,
            })
        );
        assert_eq!(
            projected.jump_extra,
            Some(MonsterSplineJumpExtraData {
                jump_gravity: 12.5,
                start_time: 125,
                duration: 0,
            })
        );
    }

    #[test]
    fn negative_start_time_clamps_before_parabolic_extra_predicate() {
        let target = ObjectGuid::create_player(1, 55);
        let args = MoveSplineInitArgs {
            flags: MoveSplineFlag::PARABOLIC | MoveSplineFlag::FADE_OBJECT,
            vertical_acceleration: 12.5,
            effect_start_time_ms: -1,
            spell_effect_extra: Some(SpellEffectExtraData {
                target,
                spell_visual_id: 777,
                progress_curve_id: 888,
                parabolic_curve_id: 999,
            }),
            ..line_args()
        };
        let spline = launch_spline(args);

        let projected = create_object_spline_data_like_cpp(&spline).unwrap();

        assert_eq!(projected.fade_object_time, Some(0));
        assert!(projected.spell_effect_extra.is_some());
        assert!(projected.jump_extra.is_none());
    }

    #[test]
    fn projection_preserves_animation_tier_and_start_time_clamp() {
        let mut flags = MoveSplineFlag::empty();
        flags.enable_animation();
        let args = MoveSplineInitArgs {
            flags,
            effect_start_time_ms: -1,
            anim_tier: Some(AnimTierTransition {
                tier_transition_id: 44,
                anim_tier: 2,
            }),
            ..line_args()
        };
        let spline = launch_spline(args);

        let projected = create_object_spline_data_like_cpp(&spline).unwrap();

        assert_eq!(
            projected.anim_tier_transition,
            Some(MonsterSplineAnimTierTransition {
                tier_transition_id: 44,
                start_time: 0,
                end_time: 0,
                anim_tier: 2,
            })
        );
    }
}
