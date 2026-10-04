mod aura;
mod aura_publication;
mod aura_application;
pub use aura_application::unit_owned_apply_aura_effect_mask_like_cpp;
mod cast;
mod cooldown;
mod shapeshift;
mod acquisition;
mod catalog;
mod spell;
mod spellbook;

pub use aura::{
    RepresentedShapeshiftMutationLikeCpp, shapeshift_form_of_spell_like_cpp,
};
pub use aura_publication::{
    AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP, player_aura_info_like_cpp,
};
