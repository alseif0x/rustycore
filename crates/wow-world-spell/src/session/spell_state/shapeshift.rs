use crate::SessionSpellState;
use wow_world_core::session::{HubMut, HubRef};
fn shapeshift_boost_spell_ids_like_cpp(
        form_id: u32,
        mut has_visible_spell: impl FnMut(i32) -> Option<bool>,
    ) -> Vec<i32> {
        const FORM_CAT_LIKE_CPP: u32 = 1;
        const FORM_TREE_OF_LIFE_LIKE_CPP: u32 = 2;
        const FORM_TRAVEL_LIKE_CPP: u32 = 3;
        const FORM_AQUATIC_LIKE_CPP: u32 = 4;
        const FORM_BEAR_LIKE_CPP: u32 = 5;
        const FORM_GHOST_WOLF_LIKE_CPP: u32 = 16;
        const FORM_FLIGHT_EPIC_LIKE_CPP: u32 = 27;
        const FORM_SHADOWFORM_LIKE_CPP: u32 = 28;
        const FORM_FLIGHT_LIKE_CPP: u32 = 29;
        const FORM_SPIRIT_OF_REDEMPTION_LIKE_CPP: u32 = 32;
        const GLYPH_OF_SHADOW_LIKE_CPP: i32 = 107_906;
        const GLYPH_OF_SHADOWY_FRIENDS_LIKE_CPP: i32 = 126_745;
        const GLYPH_OF_SPECTRAL_WOLF_LIKE_CPP: i32 = 58_135;

        match form_id {
            FORM_CAT_LIKE_CPP => vec![3_025, 48_629, 106_840, 113_636],
            FORM_TREE_OF_LIFE_LIKE_CPP => vec![5_420, 81_097],
            FORM_TRAVEL_LIKE_CPP => vec![5_419],
            FORM_AQUATIC_LIKE_CPP => vec![5_421],
            FORM_BEAR_LIKE_CPP => vec![1_178, 21_178, 106_829, 106_899],
            FORM_GHOST_WOLF_LIKE_CPP => {
                if has_visible_spell(GLYPH_OF_SPECTRAL_WOLF_LIKE_CPP)
                    == Some(true)
                {
                    vec![160_942]
                } else {
                    Vec::new()
                }
            }
            FORM_FLIGHT_EPIC_LIKE_CPP => vec![40_122, 40_121],
            FORM_SHADOWFORM_LIKE_CPP => {
                if has_visible_spell(GLYPH_OF_SHADOW_LIKE_CPP)
                    == Some(true)
                {
                    vec![107_904]
                } else if has_visible_spell(GLYPH_OF_SHADOWY_FRIENDS_LIKE_CPP)
                    == Some(true)
                {
                    vec![142_024]
                } else {
                    vec![107_903]
                }
            }
            FORM_FLIGHT_LIKE_CPP => vec![33_948, 34_764],
            FORM_SPIRIT_OF_REDEMPTION_LIKE_CPP => vec![27_792, 27_795, 62_371],
            _ => Vec::new(),
        }
    }
fn display_power_type_like_cpp(
    classes: Option<&wow_data::character_progression::ChrClassesStore>,
    class: impl FnOnce() -> u8,
    aura_effects: impl FnOnce() -> Option<Vec<(i32, i32)>>,
    calculate: impl FnOnce(wow_constants::PowerType, Option<u8>) -> Option<wow_constants::PowerType>,
) -> Option<wow_constants::PowerType> {
    let class_display_power = classes
        .and_then(|store| store.get(u32::from(class())))
        .map(|entry| entry.display_power)
        .and_then(wow_entities::represented_power_type_from_u8_like_cpp)
        .unwrap_or(wow_constants::PowerType::Mana);
    let aura_display_power = aura_effects().unwrap_or_default().first()
        .and_then(|(misc_value, _)| u8::try_from(*misc_value).ok());
    calculate(class_display_power, aura_display_power)
}


impl SessionSpellState {
    /// C++ `Unit::CalculateDisplayPowerType` (`Unit.cpp:5550-5600`) for the
    /// session player: the class default from `ChrClasses` plus the first
    /// active `SPELL_AURA_MOD_POWER_DISPLAY` effect, which the form switch
    /// overrides. `None` when the canonical Player owner is unavailable.
    pub(crate) fn represented_display_power_type_like_cpp(
        &self,
        hub: HubRef<'_>,
    ) -> Option<wow_constants::PowerType> {
        display_power_type_like_cpp(
            hub.catalogs.chr.classes_store.as_deref(),
            || hub.player_class_like_cpp(),
            || hub.resolved_aura_effects_by_spell_aura_type_like_cpp(wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_DISPLAY),
            |class, aura| hub.core.canonical_player_snapshot_like_cpp(|player| player.unit().calculate_display_power_type_like_cpp(class, aura)),
        )
    }

    /// C++ `Unit::UpdateDisplayPower` (`Unit.cpp:5600-5603`, reached from
    /// `Player::InitDataForForm` and `AuraEffect::HandleAuraModPowerDisplay`):
    /// write `UNIT_FIELD_DISPLAYPOWER` and publish the changed value.
    pub fn sync_represented_display_power_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
    ) -> bool {
        let power = self.represented_display_power_type_like_cpp(hub.shared());
        hub.core.sync_calculated_display_power_like_cpp(power)
    }

    pub fn sync_display_power_with_access_like_cpp(
        &mut self, player: &wow_world_core::session::PlayerAuraRemovalAccessLikeCpp<'_>,
        classes: Option<&wow_data::character_progression::ChrClassesStore>,
        spell_store: Option<&wow_data::SpellStore>,
        #[cfg(any(test, feature = "test-fixtures"))] fixture_class: &u8,
    ) -> bool {
        let power = display_power_type_like_cpp(
            classes,
            || player.player_class_like_cpp(
                #[cfg(any(test, feature = "test-fixtures"))] fixture_class,
            ),
            || player.aura_effects_by_spell_aura_type_like_cpp(spell_store, wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_DISPLAY),
            |class, aura| player.calculate_display_power_type_like_cpp(class, aura),
        );
        player.sync_calculated_display_power_like_cpp(power)
    }


    /// Whether any effect of the spell applies `SPELL_AURA_MOD_POWER_DISPLAY`,
    /// the gate for the displayed-power recalculation on an aura mutation.
    pub fn represented_spell_has_power_display_effect_like_cpp(
        &self,
        hub: HubRef<'_>,
        spell_id: i32,
    ) -> bool {
        self.represented_spell_has_power_display_effect_with_store_like_cpp(hub.catalogs.spell_store().map(|store| store.as_ref()), spell_id)
    }

    pub fn represented_spell_has_power_display_effect_with_store_like_cpp(
        &self,
        spell_store: Option<&wow_data::SpellStore>,
        spell_id: i32,
    ) -> bool {
        spell_store
            .and_then(|store| store.get(spell_id))
            .is_some_and(|spell| {
                spell.effects().iter().any(|effect| {
                    effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_DISPLAY
                })
            })
    }

    /// C++ `AuraEffect::HandleShapeshiftBoosts` hardcoded form boost ids
    /// (`SpellAuraEffects.cpp:1332-1392`), including the two glyph-gated
    /// choices. `FORM_DIRE_BEAR_FORM` deliberately has none, matching C++.
    pub fn represented_shapeshift_boost_spell_ids_like_cpp(
        &self,
        hub: HubRef<'_>,
        form_id: u32,
    ) -> Vec<i32> {
        shapeshift_boost_spell_ids_like_cpp(form_id, |spell_id| hub.player_has_visible_aura_spell_like_cpp(spell_id))
    }

    pub fn represented_shapeshift_boost_spell_ids_with_access_like_cpp(
        &self, player: &wow_world_core::session::PlayerAuraRemovalAccessLikeCpp<'_>, form_id: u32,
    ) -> Vec<i32> {
        shapeshift_boost_spell_ids_like_cpp(form_id, |spell_id| player.player_has_visible_aura_spell_like_cpp(spell_id))
    }
}
