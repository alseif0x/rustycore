//! Resolve the cast and per-effect target data before spell-go publication.

use super::*;

pub(super) struct ResolvedSpellEffectTargetsLikeCpp {
    pub(super) target_data: SpellTargetData,
    pub(super) spell_go_target_data: SpellTargetData,
    pub(super) effect_target_data_like_cpp: Vec<(u32, SpellTargetData)>,
}

impl WorldSession {
    pub(super) fn resolve_spell_effect_target_data_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
        spell_info: &wow_data::SpellInfo,
        mut target_data: SpellTargetData,
        represented_focus_object: Option<RepresentedSpellFocusObjectLikeCpp>,
    ) -> ResolvedSpellEffectTargetsLikeCpp {
        let mut represented_implicit_destination = false;
        let mut effect_target_data_like_cpp = Vec::new();
        for effect in spell_info.effects() {
            if effect.effect == 0 {
                continue;
            }
            for implicit_target in [effect.implicit_target_1, effect.implicit_target_2] {
                let mut isolated_effect = effect.clone();
                isolated_effect.implicit_target_1 = implicit_target;
                isolated_effect.implicit_target_2 = 0;
                let mut selection_input = target_data.clone();
                selection_input.flags &= !0x0000_0040;
                selection_input.dst_location = None;
                selection_input.map_id = None;
                let selected = match implicit_target {
                    wow_data::spell::implicit_targets::TARGET_DEST_HOME => self
                        .represented_home_destination_target_data_like_cpp(
                            caster_guid,
                            &isolated_effect,
                            &selection_input,
                        ),
                    wow_data::spell::implicit_targets::TARGET_DEST_DB => self
                        .represented_db_caster_destination_target_data_like_cpp(
                            caster_guid,
                            spell_info,
                            &isolated_effect,
                            &selection_input,
                        ),
                    wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY
                    | wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_2
                    | wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB => self
                        .represented_focus_destination_target_data_like_cpp(
                            spell_id,
                            &isolated_effect,
                            &selection_input,
                            represented_focus_object,
                        )
                        .or_else(|| {
                            self.represented_nearby_entry_destination_target_data_like_cpp(
                                spell_id,
                                &isolated_effect,
                                &selection_input,
                            )
                        }),
                    _ => None,
                };
                if let Some(selected) = selected {
                    target_data = selected;
                    represented_implicit_destination = true;
                }
            }
            // C++ snapshots m_targets.GetDst() into
            // m_destTargets[EffectIndex] after selecting each effect.
            let mut effect_target_data = target_data.clone();
            if let Some(destination) = effect_target_data.dst_location.as_mut()
                && !destination.transport.is_empty()
                && let Some(world_position) = self
                    .represented_transport_destination_world_position_like_cpp(
                        destination.transport,
                        destination.position,
                    )
            {
                // C++ serializes _transportOffset but keeps _position as
                // world coordinates for effect execution.
                destination.position = world_position;
            }
            effect_target_data_like_cpp.push((effect.effect_index, effect_target_data));
        }

        let mut spell_go_target_data = target_data.clone();
        if represented_implicit_destination {
            // C++ retains map identity on SpellDestination for effect
            // execution, while SpellCastTargets::Write emits DstLocation XYZ.
            spell_go_target_data.map_id = None;
        }

        ResolvedSpellEffectTargetsLikeCpp {
            target_data,
            spell_go_target_data,
            effect_target_data_like_cpp,
        }
    }
}
