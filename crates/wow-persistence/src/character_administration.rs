//! SQLx-free persistence contract for character-list administration.
//!
//! This is the persistence half of C++ `CharacterHandler.cpp`: protocol and
//! gameplay validation remain in `wow-world`, while the MariaDB adapter owns
//! prepared statements, row decoding and the rename/customize transactions.

use crate::PersistenceFutureLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterCustomizationPersistenceLikeCpp {
    pub option_id: i32,
    pub choice_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CharacterCreatePersistenceRequestLikeCpp {
    pub guid: u64,
    pub account_id: u32,
    pub name: String,
    pub race: u8,
    pub class: u8,
    pub sex: u8,
    pub rest_state: u8,
    pub map_id: i32,
    pub position: [f32; 4],
    pub create_time: i64,
    pub health: u32,
    pub power1: u32,
    pub last_login_build: u32,
    pub customizations: Vec<CharacterCustomizationPersistenceLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterRenameCandidateLikeCpp {
    pub old_name: String,
    pub at_login_flags: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterCustomizeCandidateLikeCpp {
    pub old_name: String,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub at_login_flags: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterAdministrationLoadOutcomeLikeCpp<T> {
    Loaded(T),
    NotFound,
    Failed { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CharacterAdministrationMutationOutcomeLikeCpp {
    Applied,
    Failed { reason: String },
}

/// One cohesive capability for character-list create/delete/rename/customize.
/// It deliberately exposes semantic operations rather than statements or a
/// generic transaction recorder.
pub trait CharacterAdministrationPersistencePortLikeCpp: Send + Sync {
    fn find_character_name_like_cpp(
        &self,
        name: &str,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationLoadOutcomeLikeCpp<()>>;

    fn load_account_character_count_like_cpp(
        &self,
        account_id: u32,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationLoadOutcomeLikeCpp<u64>>;

    fn create_character_like_cpp(
        &self,
        request: CharacterCreatePersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationMutationOutcomeLikeCpp>;

    fn delete_owned_character_like_cpp(
        &self,
        guid: u64,
        account_id: u32,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationMutationOutcomeLikeCpp>;

    fn load_rename_candidate_like_cpp(
        &self,
        guid: u64,
        new_name: &str,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CharacterAdministrationLoadOutcomeLikeCpp<CharacterRenameCandidateLikeCpp>,
    >;

    fn commit_rename_like_cpp(
        &self,
        guid: u64,
        new_name: &str,
        at_login_flags: u16,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationMutationOutcomeLikeCpp>;

    fn load_customize_candidate_like_cpp(
        &self,
        guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CharacterAdministrationLoadOutcomeLikeCpp<CharacterCustomizeCandidateLikeCpp>,
    >;

    fn commit_customize_like_cpp(
        &self,
        guid: u64,
        name: &str,
        at_login_flags: u16,
        customizations: Vec<CharacterCustomizationPersistenceLikeCpp>,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationMutationOutcomeLikeCpp>;
}
