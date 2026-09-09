//! Represented summon-object effects and their slots.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Dormant live resolver for C++ `Spell::EffectSummonObjectWild`.
    ///
    /// C++ anchors: `SpellEffects.cpp:2937-2986` and
    /// `GameObject.cpp:1187-1200`. This resolves the DB-backed template,
    /// spell destination or caster close-point fallback, and spell duration,
    /// then delegates the map-owned body to `wow_map`.
    #[allow(dead_code)]
    pub(crate) fn apply_effect_summon_object_wild_like_cpp(
        &mut self,
        spell_id: i32,
        effect: &wow_data::SpellEffectInfo,
        target_data: &SpellTargetData,
    ) -> Option<ApplyEffectSummonObjectWildSessionOutcomeLikeCpp> {
        self.apply_effect_summon_object_wild_with_focus_like_cpp(
            spell_id,
            effect,
            target_data,
            None,
        )
    }
    pub(crate) fn apply_effect_summon_object_wild_with_focus_like_cpp(
        &mut self,
        spell_id: i32,
        effect: &wow_data::SpellEffectInfo,
        target_data: &SpellTargetData,
        focus_object: Option<RepresentedSpellFocusObjectLikeCpp>,
    ) -> Option<ApplyEffectSummonObjectWildSessionOutcomeLikeCpp> {
        if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_WILD {
            return None;
        }
        let Ok(template_entry) = u32::try_from(effect.effect_misc_value_1) else {
            return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::InvalidTemplateEntry,
                template_entry: None,
                duration_ms: None,
                explicit_destination_used: false,
                close_point_fallback_represented: false,
                map_outcome: None,
            });
        };
        let Some(template_store) = self.gameobject_template_lifecycle_store_like_cpp.as_deref()
        else {
            return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingTemplateStore,
                template_entry: Some(template_entry),
                duration_ms: None,
                explicit_destination_used: false,
                close_point_fallback_represented: false,
                map_outcome: None,
            });
        };
        let Some(template) = template_store.get(template_entry) else {
            return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingTemplate,
                template_entry: Some(template_entry),
                duration_ms: None,
                explicit_destination_used: false,
                close_point_fallback_represented: false,
                map_outcome: None,
            });
        };
        let Some(caster_guid) = self.player_guid() else {
            return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingCaster,
                template_entry: Some(template_entry),
                duration_ms: None,
                explicit_destination_used: true,
                close_point_fallback_represented: false,
                map_outcome: None,
            });
        };
        let position_outcome = if let Some(dest) = target_data.dst_location {
            wow_map::map::spell_effect_summon_object_wild_position_like_cpp(
                Position::ZERO,
                0.0,
                0.0,
                Some(dest.position),
            )
        } else {
            let Some(caster_position) = self.player_position_like_cpp() else {
                return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
                    status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingCasterPosition,
                    template_entry: Some(template_entry),
                    duration_ms: None,
                    explicit_destination_used: false,
                    close_point_fallback_represented: false,
                    map_outcome: None,
                });
            };
            let close_point_source_position = focus_object
                .map(|focus| focus.position)
                .unwrap_or(caster_position);
            let represented_source_reach = if focus_object.is_some() {
                // C++ calls `target->GetClosePoint`; represented GameObject size
                // is not carried here yet, so focus-backed fallback uses the
                // focus position with no extra object reach instead of the
                // caster combat reach.
                0.0
            } else {
                self.canonical_player_combat_reach_snapshot_like_cpp()
            };
            wow_map::map::spell_effect_summon_object_wild_position_like_cpp(
                close_point_source_position,
                represented_source_reach,
                close_point_source_position.orientation,
                None,
            )
        };
        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingCanonicalMapManager,
                template_entry: Some(template_entry),
                duration_ms: None,
                explicit_destination_used: position_outcome.explicit_destination_used,
                close_point_fallback_represented: position_outcome.close_point_fallback_used,
                map_outcome: None,
            });
        };
        let Some(player_map_key) = focus_object
            .map(|focus| focus.map_key)
            .or_else(|| self.current_canonical_player_map_key_like_cpp())
        else {
            return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingCanonicalPlayerMap,
                template_entry: Some(template_entry),
                duration_ms: None,
                explicit_destination_used: position_outcome.explicit_destination_used,
                close_point_fallback_represented: position_outcome.close_point_fallback_used,
                map_outcome: None,
            });
        };
        let spell_id_u32 = u32::try_from(spell_id).unwrap_or(0);
        let duration_index = self
            .spell_misc_store
            .as_deref()
            .and_then(|store| store.get_by_spell_id(spell_id_u32))
            .map(|entry| u32::from(entry.duration_index))
            .unwrap_or(0);
        let duration_ms =
            spell_duration_ms_like_cpp(duration_index, self.spell_duration_store.as_deref());
        let lifecycle_record = wow_data::gameobject_template_lifecycle_record_like_cpp(template);
        let Ok(mut manager) = manager.lock() else {
            return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingCanonicalMapManager,
                template_entry: Some(template_entry),
                duration_ms: Some(duration_ms),
                explicit_destination_used: position_outcome.explicit_destination_used,
                close_point_fallback_represented: position_outcome.close_point_fallback_used,
                map_outcome: None,
            });
        };
        let Some(managed) = manager.find_map_mut(player_map_key.map_id, player_map_key.instance_id)
        else {
            return Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MissingManagedMap,
                template_entry: Some(template_entry),
                duration_ms: Some(duration_ms),
                explicit_destination_used: position_outcome.explicit_destination_used,
                close_point_fallback_represented: position_outcome.close_point_fallback_used,
                map_outcome: None,
            });
        };
        let map_outcome = managed.map_mut().spell_effect_summon_object_wild_like_cpp(
            caster_guid,
            spell_id_u32,
            lifecycle_record,
            position_outcome.position,
            duration_ms,
        );

        Some(ApplyEffectSummonObjectWildSessionOutcomeLikeCpp {
            status: ApplyEffectSummonObjectWildSessionStatusLikeCpp::MapResolved,
            template_entry: Some(template_entry),
            duration_ms: Some(duration_ms),
            explicit_destination_used: position_outcome.explicit_destination_used,
            close_point_fallback_represented: position_outcome.close_point_fallback_used,
            map_outcome: Some(map_outcome),
        })
    }
    /// Dormant live resolver for C++ `Spell::EffectSummonObject`.
    ///
    /// C++ anchors: `SpellEffects.cpp:3541-3597`, `Unit.cpp:5213-5251`,
    /// and `GameObject.cpp:1187-1200`. This resolver keeps the effect dormant
    /// until live spell-loop wiring is explicitly chosen: it resolves the
    /// DB-backed template, spell destination or caster close-point fallback,
    /// duration, map, and owner slot, then delegates cleanup/create/slot writes
    /// to map-owned helpers.
    #[allow(dead_code)]
    pub(crate) fn apply_effect_summon_object_slot_like_cpp(
        &mut self,
        spell_id: i32,
        effect: &wow_data::SpellEffectInfo,
        target_data: &SpellTargetData,
    ) -> Option<ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp> {
        let slot_base = wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_SLOT1;
        if effect.effect < slot_base {
            return None;
        }
        let slot = usize::try_from(effect.effect - slot_base).unwrap_or(usize::MAX);
        if slot >= MAX_GAMEOBJECT_SLOT_LIKE_CPP {
            return Some(ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp::InvalidSlot,
                slot: Some(slot),
                template_entry: None,
                duration_ms: None,
                explicit_destination_used: false,
                close_point_fallback_represented: false,
                cleanup_outcome: None,
                map_outcome: None,
            });
        }
        let Ok(template_entry) = u32::try_from(effect.effect_misc_value_1) else {
            return Some(ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp::InvalidTemplateEntry,
                slot: Some(slot),
                template_entry: None,
                duration_ms: None,
                explicit_destination_used: false,
                close_point_fallback_represented: false,
                cleanup_outcome: None,
                map_outcome: None,
            });
        };
        let Some(template_store) = self.gameobject_template_lifecycle_store_like_cpp.as_deref()
        else {
            return Some(ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp::MissingTemplateStore,
                slot: Some(slot),
                template_entry: Some(template_entry),
                duration_ms: None,
                explicit_destination_used: false,
                close_point_fallback_represented: false,
                cleanup_outcome: None,
                map_outcome: None,
            });
        };
        let Some(template) = template_store.get(template_entry) else {
            return Some(ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp::MissingTemplate,
                slot: Some(slot),
                template_entry: Some(template_entry),
                duration_ms: None,
                explicit_destination_used: false,
                close_point_fallback_represented: false,
                cleanup_outcome: None,
                map_outcome: None,
            });
        };
        let Some(caster_guid) = self.player_guid() else {
            return Some(ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp::MissingCaster,
                slot: Some(slot),
                template_entry: Some(template_entry),
                duration_ms: None,
                explicit_destination_used: false,
                close_point_fallback_represented: false,
                cleanup_outcome: None,
                map_outcome: None,
            });
        };
        let position_outcome = if let Some(dest) = target_data.dst_location {
            wow_map::map::spell_effect_summon_object_wild_position_like_cpp(
                Position::ZERO,
                0.0,
                0.0,
                Some(dest.position),
            )
        } else {
            let Some(caster_position) = self.player_position_like_cpp() else {
                return Some(ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
                    status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp::MissingCasterPosition,
                    slot: Some(slot),
                    template_entry: Some(template_entry),
                    duration_ms: None,
                    explicit_destination_used: false,
                    close_point_fallback_represented: false,
                    cleanup_outcome: None,
                    map_outcome: None,
                });
            };
            wow_map::map::spell_effect_summon_object_wild_position_like_cpp(
                caster_position,
                self.canonical_player_combat_reach_snapshot_like_cpp(),
                caster_position.orientation,
                None,
            )
        };
        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return Some(ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp::MissingCanonicalMapManager,
                slot: Some(slot),
                template_entry: Some(template_entry),
                duration_ms: None,
                explicit_destination_used: position_outcome.explicit_destination_used,
                close_point_fallback_represented: position_outcome.close_point_fallback_used,
                cleanup_outcome: None,
                map_outcome: None,
            });
        };
        let Some(player_map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return Some(ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp::MissingCanonicalPlayerMap,
                slot: Some(slot),
                template_entry: Some(template_entry),
                duration_ms: None,
                explicit_destination_used: position_outcome.explicit_destination_used,
                close_point_fallback_represented: position_outcome.close_point_fallback_used,
                cleanup_outcome: None,
                map_outcome: None,
            });
        };
        let spell_id_u32 = u32::try_from(spell_id).unwrap_or(0);
        let duration_index = self
            .spell_misc_store
            .as_deref()
            .and_then(|store| store.get_by_spell_id(spell_id_u32))
            .map(|entry| u32::from(entry.duration_index))
            .unwrap_or(0);
        let duration_ms =
            spell_duration_ms_like_cpp(duration_index, self.spell_duration_store.as_deref());
        let lifecycle_record = wow_data::gameobject_template_lifecycle_record_like_cpp(template);
        let Ok(mut manager) = manager.lock() else {
            return Some(ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp::MissingCanonicalMapManager,
                slot: Some(slot),
                template_entry: Some(template_entry),
                duration_ms: Some(duration_ms),
                explicit_destination_used: position_outcome.explicit_destination_used,
                close_point_fallback_represented: position_outcome.close_point_fallback_used,
                cleanup_outcome: None,
                map_outcome: None,
            });
        };
        let Some(managed) = manager.find_map_mut(player_map_key.map_id, player_map_key.instance_id)
        else {
            return Some(ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
                status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp::MissingManagedMap,
                slot: Some(slot),
                template_entry: Some(template_entry),
                duration_ms: Some(duration_ms),
                explicit_destination_used: position_outcome.explicit_destination_used,
                close_point_fallback_represented: position_outcome.close_point_fallback_used,
                cleanup_outcome: None,
                map_outcome: None,
            });
        };
        let map = managed.map_mut();
        let cleanup_outcome =
            map.gameobject_prepare_owner_slot_for_summon_like_cpp(caster_guid, slot, spell_id_u32);
        let map_outcome = map.gameobject_summon_object_for_owner_slot_like_cpp(
            caster_guid,
            slot,
            spell_id_u32,
            lifecycle_record,
            position_outcome.position,
            duration_ms,
        );

        Some(ApplyEffectSummonObjectSlotSessionOutcomeLikeCpp {
            status: ApplyEffectSummonObjectSlotSessionStatusLikeCpp::MapResolved,
            slot: Some(slot),
            template_entry: Some(template_entry),
            duration_ms: Some(duration_ms),
            explicit_destination_used: position_outcome.explicit_destination_used,
            close_point_fallback_represented: position_outcome.close_point_fallback_used,
            cleanup_outcome: Some(cleanup_outcome),
            map_outcome: Some(map_outcome),
        })
    }
}
