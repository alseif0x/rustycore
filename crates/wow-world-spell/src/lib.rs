//! World-session spell state and its application-owned operations.

pub mod aura_effects;
pub mod login_spell_rules;
pub mod melee_damage;
pub mod melee_rules;
pub mod player_cast;
mod records;
mod session;
mod spell_acquisition;
pub mod spell_cast_adapter;
mod state;
#[cfg(any(test, feature = "test-fixtures"))]
mod test_support;

pub use player_cast::PlayerCastPublicationPhaseLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
pub use records::canonical_player_spell_runtime_like_cpp;
pub use records::{
    RepresentedCharacterSpellChargeLikeCpp, RepresentedCharacterSpellCooldownLikeCpp,
    RepresentedPlayerSpellLikeCpp, RepresentedPlayerSpellRuntimeLikeCpp,
    RepresentedPlayerSpellStateLikeCpp, RepresentedSpellFocusObjectLikeCpp,
    canonical_player_spell_record_like_cpp, represented_player_spell_record_like_cpp,
    represented_player_spell_runtime_like_cpp,
};
pub use session::{
    AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP, RepresentedShapeshiftMutationLikeCpp,
    player_aura_info_like_cpp, shapeshift_form_of_spell_like_cpp,
    unit_owned_apply_aura_effect_mask_like_cpp,
};
pub use spell_cast_adapter::present_visual;
pub use state::SessionSpellState;
#[cfg(any(test, feature = "test-fixtures"))]
pub use test_support::PlayerSpellAndTraitTestFixtureLikeCpp;
