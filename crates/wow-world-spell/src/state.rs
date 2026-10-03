use std::collections::BTreeSet;
#[cfg(any(test, feature = "test-fixtures"))]
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

#[cfg(any(test, feature = "test-fixtures"))]
use wow_packet::packets::misc::{SpellChargeEntry, SpellHistoryEntry};
use wow_spell_acquisition::{
    SpellAcquisitionCastAuthorityLikeCpp, SpellAcquisitionCraftValidityAuthorityLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_spell_acquisition::SpellAcquisitionPostCommitActionLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
use crate::records::{
    RepresentedCharacterSpellChargeLikeCpp, RepresentedCharacterSpellCooldownLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use crate::test_support::PlayerSpellAndTraitTestFixtureLikeCpp;

/// The session's spell-side represented state: the cached spell-script id sets
/// the startup audit installs, the spell-acquisition authorities, the execute-log
/// effects and the offhand re-check switch, until the owning Player runtime and the
/// spell-acquisition module take them over.
pub struct SessionSpellState {
    pub(crate) legacy_spell_script_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub(crate) spell_linked_rejected_trigger_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    pub(crate) spell_script_all_rank_root_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    /// Effective C++ spell-script hooks. These remain optional so a session
    /// constructed without the startup audit fails closed.
    pub(crate) spell_script_exact_spell_ids_like_cpp: Option<Arc<BTreeSet<u32>>>,
    /// C++ `CONFIG_OFFHAND_CHECK_AT_SPELL_UNLEARN` represented switch.
    pub(crate) represented_offhand_check_at_spell_unlearn_like_cpp: bool,
    pub(crate) represented_spell_execute_log_effects_like_cpp:
        Vec<wow_packet::packets::combat::SpellLogEffect>,
    pub(crate) spell_acquisition_cast_authority_like_cpp:
        Option<Arc<SpellAcquisitionCastAuthorityLikeCpp>>,
    pub(crate) spell_acquisition_craft_authority_like_cpp:
        Option<Arc<SpellAcquisitionCraftValidityAuthorityLikeCpp>>,
    /// Handle-less test fixture for Player spell and trait data.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) player_spell_test_fixture_like_cpp: PlayerSpellAndTraitTestFixtureLikeCpp,
    /// Test-only causal trace. Production applies every represented
    /// post-commit action immediately; retaining a second action history on
    /// the Session would be audit state, not C++ runtime authority.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_spell_acquisition_post_commit_actions_like_cpp:
        Vec<SpellAcquisitionPostCommitActionLikeCpp>,
    /// Login snapshot of the player's spell history + charge packets. C++ reads these
    /// live from `Player::GetSpellHistory()` in `SendInitialPacketsBeforeAddToMap`; Rust
    /// persists the login snapshot so the before-add helper can re-send it on far teleport
    /// without a DB round trip. #NEXT.R8.ENTITIES.1229.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_spell_history_packets_like_cpp:
        (Vec<SpellHistoryEntry>, Vec<SpellChargeEntry>),
    /// C++ `ActivePlayerData::SelfResSpells`, represented until update-field
    /// ownership is canonical.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_self_res_spells_like_cpp: BTreeSet<i32>,
    /// C++ `Player::m_overrideSpells`, represented until active player spell
    /// cast resolution owns override lookup.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_override_spells_like_cpp: HashMap<i32, BTreeSet<i32>>,
    /// True only when all C++ `Player::m_overrideSpells` edges were replaced
    /// from a complete source rather than accumulated opportunistically.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_override_spells_complete_like_cpp: bool,
    /// Currently active spell cast (if any). Set when a cast starts, cleared when it completes.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) active_spell_cast: Option<wow_entities::SpellCastState>,
    /// C++ `Player::_pendingSpellCastRequest`, represented separately from
    /// `active_spell_cast` so cancel queued spell does not interrupt a cast
    /// already in progress.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_pending_spell_cast_request_like_cpp:
        Option<wow_entities::PendingSpellCastRequestLikeCpp>,
    /// Last time a spell was executed (used to enforce global cooldown timers).
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) last_spell_cast_time: Option<std::time::Instant>,
    /// Per-spell cooldown tracking: spell_id → last cast time.
    /// Used to enforce spell-specific cooldown timers.
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) last_spell_cast_time_per_spell: HashMap<i32, std::time::Instant>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_character_spell_cooldowns_like_cpp:
        HashMap<u32, RepresentedCharacterSpellCooldownLikeCpp>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_character_spell_cooldowns_loaded_like_cpp: bool,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_character_spell_charges_like_cpp:
        BTreeMap<u32, Vec<RepresentedCharacterSpellChargeLikeCpp>>,
    #[cfg(any(test, feature = "test-fixtures"))]
    pub(crate) represented_character_spell_charges_loaded_like_cpp: bool,
}

impl SessionSpellState {
    /// Construct the represented fields in their former WorldSession order.
    pub fn new_like_cpp() -> Self {
        Self {
            legacy_spell_script_spell_ids_like_cpp: None,
            spell_linked_rejected_trigger_spell_ids_like_cpp: None,
            spell_script_all_rank_root_spell_ids_like_cpp: None,
            spell_script_exact_spell_ids_like_cpp: None,
            represented_offhand_check_at_spell_unlearn_like_cpp: true,
            represented_spell_execute_log_effects_like_cpp: Vec::new(),
            spell_acquisition_cast_authority_like_cpp: None,
            spell_acquisition_craft_authority_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            player_spell_test_fixture_like_cpp: PlayerSpellAndTraitTestFixtureLikeCpp::default(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_spell_acquisition_post_commit_actions_like_cpp: Vec::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_spell_history_packets_like_cpp: (Vec::new(), Vec::new()),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_self_res_spells_like_cpp: BTreeSet::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_override_spells_like_cpp: HashMap::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_override_spells_complete_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            active_spell_cast: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_pending_spell_cast_request_like_cpp: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            last_spell_cast_time: None,
            #[cfg(any(test, feature = "test-fixtures"))]
            last_spell_cast_time_per_spell: HashMap::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_character_spell_cooldowns_like_cpp: HashMap::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_character_spell_cooldowns_loaded_like_cpp: false,
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_character_spell_charges_like_cpp: BTreeMap::new(),
            #[cfg(any(test, feature = "test-fixtures"))]
            represented_character_spell_charges_loaded_like_cpp: false,
        }
    }

    pub fn install_spell_runtime_script_authority_like_cpp(
        &mut self,
        exact_spell_ids: Arc<BTreeSet<u32>>,
        all_rank_root_spell_ids: Arc<BTreeSet<u32>>,
        legacy_spell_ids: Arc<BTreeSet<u32>>,
        rejected_linked_trigger_spell_ids: Arc<BTreeSet<u32>>,
    ) {
        self.spell_script_exact_spell_ids_like_cpp = Some(exact_spell_ids);
        self.spell_script_all_rank_root_spell_ids_like_cpp = Some(all_rank_root_spell_ids);
        self.legacy_spell_script_spell_ids_like_cpp = Some(legacy_spell_ids);
        self.spell_linked_rejected_trigger_spell_ids_like_cpp =
            Some(rejected_linked_trigger_spell_ids);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn clear_rejected_spell_linked_trigger_authority_for_test_like_cpp(&mut self) {
        self.spell_linked_rejected_trigger_spell_ids_like_cpp = None;
    }

    pub fn set_spell_acquisition_cast_authority_like_cpp(
        &mut self,
        cast: Arc<SpellAcquisitionCastAuthorityLikeCpp>,
    ) {
        self.spell_acquisition_cast_authority_like_cpp = Some(cast);
    }

    pub fn set_spell_acquisition_craft_authority_like_cpp(
        &mut self,
        craft: Arc<SpellAcquisitionCraftValidityAuthorityLikeCpp>,
    ) {
        self.spell_acquisition_craft_authority_like_cpp = Some(craft);
    }

    pub fn spell_acquisition_cast_authority_like_cpp(
        &self,
    ) -> Option<&Arc<SpellAcquisitionCastAuthorityLikeCpp>> {
        self.spell_acquisition_cast_authority_like_cpp.as_ref()
    }

    pub fn spell_acquisition_craft_authority_like_cpp(
        &self,
    ) -> Option<&Arc<SpellAcquisitionCraftValidityAuthorityLikeCpp>> {
        self.spell_acquisition_craft_authority_like_cpp.as_ref()
    }

    pub fn set_offhand_check_at_spell_unlearn_like_cpp(&mut self, enabled: bool) {
        self.represented_offhand_check_at_spell_unlearn_like_cpp = enabled;
    }

    pub fn offhand_check_at_spell_unlearn_like_cpp(&self) -> bool {
        self.represented_offhand_check_at_spell_unlearn_like_cpp
    }

    pub fn represented_spell_execute_log_effect_like_cpp(
        &mut self,
        effect: i32,
    ) -> &mut wow_packet::packets::combat::SpellLogEffect {
        if let Some(index) = self
            .represented_spell_execute_log_effects_like_cpp
            .iter()
            .position(|entry| entry.effect == effect)
        {
            return &mut self.represented_spell_execute_log_effects_like_cpp[index];
        }
        self.represented_spell_execute_log_effects_like_cpp
            .push(wow_packet::packets::combat::SpellLogEffect {
                effect,
                ..Default::default()
            });
        let index = self.represented_spell_execute_log_effects_like_cpp.len() - 1;
        &mut self.represented_spell_execute_log_effects_like_cpp[index]
    }

    pub fn clear_represented_spell_execute_log_effects_like_cpp(&mut self) {
        self.represented_spell_execute_log_effects_like_cpp.clear();
    }

    pub fn take_represented_spell_execute_log_effects_like_cpp(
        &mut self,
    ) -> Vec<wow_packet::packets::combat::SpellLogEffect> {
        std::mem::take(&mut self.represented_spell_execute_log_effects_like_cpp)
    }

    pub fn has_represented_spell_execute_log_effects_like_cpp(&self) -> bool {
        !self.represented_spell_execute_log_effects_like_cpp.is_empty()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn add_represented_self_res_spell_for_test_like_cpp(&mut self, spell_id: i32) {
        self.represented_self_res_spells_like_cpp.insert(spell_id);
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_self_res_spells_for_test_like_cpp(&self) -> &BTreeSet<i32> {
        &self.represented_self_res_spells_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn has_represented_self_res_spell_for_test_like_cpp(&self, spell_id: i32) -> bool {
        self.represented_self_res_spells_like_cpp.contains(&spell_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn replace_represented_override_spell_fixture_like_cpp(
        &mut self,
        overrides: HashMap<i32, BTreeSet<i32>>,
        complete: bool,
    ) {
        self.represented_override_spells_like_cpp = overrides;
        self.represented_override_spells_complete_like_cpp = complete;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_override_spell_fixture_like_cpp(&self) -> &HashMap<i32, BTreeSet<i32>> {
        &self.represented_override_spells_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_override_spell_fixture_mut_like_cpp(
        &mut self,
    ) -> &mut HashMap<i32, BTreeSet<i32>> {
        &mut self.represented_override_spells_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_override_spell_fixture_complete_like_cpp(&self) -> bool {
        self.represented_override_spells_complete_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn clear_spell_acquisition_post_commit_actions_for_test_like_cpp(&mut self) {
        self.represented_spell_acquisition_post_commit_actions_like_cpp.clear();
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_character_spell_cooldowns_for_test_like_cpp(
        &self,
    ) -> &HashMap<u32, RepresentedCharacterSpellCooldownLikeCpp> {
        &self.represented_character_spell_cooldowns_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_character_spell_cooldowns_loaded_for_test_like_cpp(&self) -> bool {
        self.represented_character_spell_cooldowns_loaded_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_character_spell_charges_for_test_like_cpp(
        &self,
    ) -> &BTreeMap<u32, Vec<RepresentedCharacterSpellChargeLikeCpp>> {
        &self.represented_character_spell_charges_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_character_spell_charges_loaded_for_test_like_cpp(&self) -> bool {
        self.represented_character_spell_charges_loaded_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn cast_execution_fixture_for_test_like_cpp(
        &self,
    ) -> (
        Option<wow_entities::SpellCastState>,
        Option<std::time::Instant>,
        HashMap<i32, std::time::Instant>,
    ) {
        (
            self.active_spell_cast.clone(),
            self.last_spell_cast_time,
            self.last_spell_cast_time_per_spell.clone(),
        )
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn pending_spell_cast_fixture_for_test_like_cpp(
        &self,
    ) -> Option<wow_entities::PendingSpellCastRequestLikeCpp> {
        self.represented_pending_spell_cast_request_like_cpp.clone()
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_spell_test_fixture_like_cpp(&self) -> &PlayerSpellAndTraitTestFixtureLikeCpp {
        &self.player_spell_test_fixture_like_cpp
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn player_spell_test_fixture_mut_like_cpp(
        &mut self,
    ) -> &mut PlayerSpellAndTraitTestFixtureLikeCpp {
        &mut self.player_spell_test_fixture_like_cpp
    }
}
