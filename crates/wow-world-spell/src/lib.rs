//! World-session spell state and its application-owned operations.

pub mod aura_effects;
pub mod melee_damage;
mod records;
mod state;
pub mod player_cast;
mod session;
pub mod spell_cast_adapter;
mod spell_acquisition;
#[cfg(any(test, feature = "test-fixtures"))]
mod test_support;

pub use records::{
    RepresentedCharacterSpellChargeLikeCpp, RepresentedCharacterSpellCooldownLikeCpp,
    RepresentedPlayerSpellLikeCpp, RepresentedPlayerSpellRuntimeLikeCpp,
    RepresentedSpellFocusObjectLikeCpp,
    RepresentedPlayerSpellStateLikeCpp, canonical_player_spell_record_like_cpp,
    represented_player_spell_record_like_cpp, represented_player_spell_runtime_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use records::canonical_player_spell_runtime_like_cpp;
pub use state::SessionSpellState;
pub use player_cast::PlayerCastPublicationPhaseLikeCpp;
pub use session::{
    AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP, RepresentedShapeshiftMutationLikeCpp,
    player_aura_info_like_cpp, shapeshift_form_of_spell_like_cpp,
    unit_owned_apply_aura_effect_mask_like_cpp,
};
pub use spell_cast_adapter::present_visual;
#[cfg(any(test, feature = "test-fixtures"))]
pub use test_support::PlayerSpellAndTraitTestFixtureLikeCpp;
