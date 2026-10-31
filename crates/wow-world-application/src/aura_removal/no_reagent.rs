use super::AuraRemovalCxLikeCpp;
use wow_entities::AuraApplicationLikeCpp;

const NO_REAGENT_USE_AURA_TYPE_LIKE_CPP: i32 = 256;

#[derive(Clone, Copy)]
struct ActiveAuraEffectLikeCpp {
    index: i32,
    aura_type: i32,
    class_mask: [u32; 4],
}

impl AuraRemovalCxLikeCpp<'_> {
    /// Rebuild the native Player mask only when the complete persisted aura
    /// source is available and has not been tombstoned for this lifetime.
    pub fn recompute_no_reagent_mask_from_active_auras_like_cpp(&self) -> Result<(), &'static str> {
        if self.player.player_aura_authority_ready_like_cpp() != Some(true) {
            return Err("Complete untombstoned Player aura authority unavailable");
        }
        let active = self
            .player
            .visible_auras_snapshot_like_cpp()
            .ok_or("Active Player aura applications unavailable")?
            .into_values()
            .collect::<Vec<_>>();
        let mask = self.no_reagent_mask_for_auras_like_cpp(&active)?;
        self.player
            .set_no_reagent_cost_mask_like_cpp(mask)
            .ok_or("Missing Player owner for NoReagentCostMask")
    }

    pub(super) fn no_reagent_mask_after_aura_apply_like_cpp(
        &self,
        aura: &AuraApplicationLikeCpp,
    ) -> Result<Option<[u32; 4]>, &'static str> {
        if !self
            .resolved_aura_effects_like_cpp(aura)?
            .iter()
            .any(|effect| effect.aura_type == NO_REAGENT_USE_AURA_TYPE_LIKE_CPP)
        {
            return Ok(None);
        }
        let mut active = self
            .player
            .visible_auras_snapshot_like_cpp()
            .ok_or("Active Player aura applications unavailable")?
            .into_values()
            .collect::<Vec<_>>();
        active.push(aura.clone());
        self.no_reagent_mask_for_auras_like_cpp(&active).map(Some)
    }

    pub(super) fn no_reagent_mask_after_aura_remove_like_cpp(
        &self,
        slot: u8,
    ) -> Result<Option<[u32; 4]>, &'static str> {
        let active = self
            .player
            .visible_auras_snapshot_like_cpp()
            .ok_or("Active Player aura applications unavailable")?;
        let removed = active
            .get(&slot)
            .ok_or("Active Player aura application unavailable for removal")?;
        if !self
            .resolved_aura_effects_like_cpp(removed)?
            .iter()
            .any(|effect| effect.aura_type == NO_REAGENT_USE_AURA_TYPE_LIKE_CPP)
        {
            return Ok(None);
        }
        // Removal during character-aura restoration cannot prove that the
        // remaining applications are complete. Keep the current native value;
        // its consumer is gated until the post-load recomputation below.
        if self.player.player_aura_authority_ready_like_cpp() != Some(true) {
            return Ok(None);
        }
        let remaining = active
            .into_iter()
            .filter_map(|(active_slot, aura)| (active_slot != slot).then_some(aura))
            .collect::<Vec<_>>();
        self.no_reagent_mask_for_auras_like_cpp(&remaining)
            .map(Some)
    }

    fn no_reagent_mask_for_auras_like_cpp(
        &self,
        auras: &[AuraApplicationLikeCpp],
    ) -> Result<[u32; 4], &'static str> {
        let mut mask = [0u32; 4];
        for aura in auras {
            for effect in self.resolved_aura_effects_like_cpp(aura)? {
                if effect.aura_type == NO_REAGENT_USE_AURA_TYPE_LIKE_CPP {
                    for (target, source) in mask.iter_mut().zip(effect.class_mask) {
                        *target |= source;
                    }
                }
            }
        }
        Ok(mask)
    }

    fn resolved_aura_effects_like_cpp(
        &self,
        aura: &AuraApplicationLikeCpp,
    ) -> Result<Vec<ActiveAuraEffectLikeCpp>, &'static str> {
        use wow_data::ServersideSpellInfoLikeCpp;

        enum Source<'a> {
            Regular(u8),
            Server(&'a ServersideSpellInfoLikeCpp),
        }

        let spell_id = aura.spell_id as u32;
        let regular = self.spell_store.map(|store| store.as_ref());
        let server = self.serverside_spell_store.map(|store| store.as_ref());
        let has_any_key = regular
            .is_some_and(|store| store.contains_spell_info_any_difficulty_like_cpp(spell_id))
            || server.is_some_and(|store| {
                (u8::MIN..=u8::MAX).any(|difficulty| {
                    store
                        .get_serverside_spell_like_cpp(spell_id, u32::from(difficulty))
                        .is_some()
                })
            });
        let source = wow_data::spell::resolve_spell_catalog_value(
            aura.difficulty_id,
            self.difficulty_store,
            has_any_key,
            |difficulty| {
                let regular_key = regular.is_some_and(|store| {
                    store.contains_spell_info_exact_like_cpp(spell_id, difficulty)
                });
                let server_key = server.and_then(|store| {
                    store.get_serverside_spell_like_cpp(spell_id, u32::from(difficulty))
                });
                match (regular_key, server_key) {
                    (true, Some(_)) => Some(Err("Aura SpellInfo source collision")),
                    (true, None) => Some(Ok(Source::Regular(difficulty))),
                    (false, Some(info)) => Some(Ok(Source::Server(info))),
                    (false, None) => None,
                }
            },
        )
        .map_err(|_| "Aura SpellInfo difficulty authority unavailable")?
        .ok_or("Aura SpellInfo key unavailable")??;

        let effects = match source {
            Source::Regular(difficulty) => {
                let store = regular.ok_or("Regular aura SpellInfo store unavailable")?;
                wow_data::spell::resolve_spell_effect_rows(
                    store,
                    aura.spell_id,
                    difficulty,
                    self.difficulty_store,
                )
                .map_err(|_| "Aura SpellEffect metadata unavailable")?
                .into_iter()
                .map(|effect| {
                    Ok(ActiveAuraEffectLikeCpp {
                        index: i32::try_from(effect.effect_index)
                            .map_err(|_| "Aura SpellEffect index unavailable")?,
                        aura_type: effect.effect_aura,
                        class_mask: effect.effect_spell_class_mask,
                    })
                })
                .collect::<Result<Vec<_>, &'static str>>()?
            }
            Source::Server(info) => info
                .effects
                .iter()
                .map(|effect| ActiveAuraEffectLikeCpp {
                    index: effect.effect_index,
                    aura_type: effect.effect_aura,
                    class_mask: effect.effect_spell_class_mask.map(|word| word as u32),
                })
                .collect(),
        };

        let mut active_effects = Vec::new();
        for index in 0..32u32 {
            if aura.effect_mask & (1 << index) == 0 {
                continue;
            }
            let effect = effects
                .iter()
                .find(|effect| effect.index == index as i32)
                .ok_or("Active aura effect metadata unavailable")?;
            active_effects.push(*effect);
        }
        Ok(active_effects)
    }
}
