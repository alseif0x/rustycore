//! Canonical Session integration of target row projection and ordered packets.
use super::*;
use std::future::Future;
use wow_persistence::forever::selection::{CharacterRow, CustomizationRow, SelectionRows};

fn row(guid: u64) -> CharacterRow {
    CharacterRow {
        guid,
        name: "Synthetic".into(),
        race: 1,
        class: 1,
        level: 12,
        personal_tabard: [-1; 5],
        ..Default::default()
    }
}
fn request() -> Request {
    Request {
        opcode: ENUM_CHARACTERS,
        payload: vec![],
    }
}

#[tokio::test]
async fn source_empty_codec_remains_byte_identical_with_real_unlock_sections() {
    let mut session = session(Arc::new(Repository::good()));
    let catalog = CharacterCatalog::fixture();
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    let output = session
        .dispatch(&catalog, request())
        .await
        .unwrap()
        .unwrap();
    let expected = wow_packet::forever::EmptyEnumCharactersResult {
        success: true,
        realmless: false,
        is_deleted_characters: false,
        ignore_new_player_restrictions: false,
        is_restricted_new_player: false,
        is_newcomer_chat_completed: false,
        is_restricted_trial: false,
        is_account_lapsed_player: false,
        force_character_list_sort: false,
        max_character_level: 1,
        class_disable_mask: Some(0),
        race_unlock_data: vec![wow_packet::forever::RaceUnlock {
            race_id: 1,
            has_unlocked_license: true,
            has_unlocked_achievement: false,
            has_heritage_armor_unlock_achievement: false,
            has_entitlement: true,
            hide_race_on_client: false,
            faction_balance_disabled: false,
            does_not_have_available_classes: false,
            class_unlocks: vec![wow_packet::forever::ClassUnlock {
                class_id: 1,
                achievement_id: 0,
                has_expansion: true,
                has_unlocked_achievement: true,
                has_entitlement: true,
            }],
        }],
    }
    .encode_payload()
    .unwrap();
    assert_eq!(output[0].payload(), expected);
    assert!(session.legitimate_characters.is_empty());
}

#[test]
fn enum_vectors_are_not_capped_by_creates_250_element_array() {
    use wow_packet::forever::{
        character_create::CustomizationChoice,
        character_list::{
            CharacterInfo, CharacterInfoBasic, CharacterRestrictionAndMailData,
            EnumCharactersResult,
        },
    };
    let mut basic = CharacterInfoBasic::new("Synthetic", "");
    basic.customizations = (1..=251)
        .map(|option_id| CustomizationChoice {
            option_id,
            choice_id: option_id + 1,
        })
        .collect();
    let mut packet = EnumCharactersResult::new();
    packet.characters.push(CharacterInfo::new(
        basic,
        CharacterRestrictionAndMailData::empty(),
    ));
    assert!(packet.encode_payload().is_ok());
}

#[tokio::test]
async fn populated_holder_is_not_replaced_by_empty_success_and_banned_row_is_not_authorized() {
    let mut banned = row(18);
    banned.active_ban_guid = 18;
    let repository = Arc::new(Repository {
        selection: SelectionRows {
            characters: vec![row(17), banned],
            customizations: vec![],
        },
        ..Repository::good()
    });
    let mut session = session(repository.clone());
    let catalog = CharacterCatalog::fixture();
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    let output = session
        .dispatch(&catalog, request())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        output.iter().map(Outgoing::opcode).collect::<Vec<_>>(),
        [0x460018, 0x460362, 0x460360]
    );
    assert_eq!(&output[0].payload()[2..6], &2_u32.to_le_bytes());
    assert_eq!(&output[0].payload()[10..14], &12_i32.to_le_bytes());
    assert_eq!(output[1].payload(), &[0; 9]);
    assert_eq!(output[2].payload(), &[0; 4]);
    assert_eq!(session.legitimate_characters.len(), 1);
    assert!(
        session
            .legitimate_characters
            .contains(&wow_core::ObjectGuid::create_player(1, 17))
    );
    assert!(repository.recustomized.lock().unwrap().is_empty());
    session.close();
    assert!(session.legitimate_characters.is_empty());
    assert!(!session.has_enumerated());
}

#[tokio::test]
async fn recustomize_ack_precedes_authority_and_error_publishes_no_list() {
    for failure in [None, Some(LoadError::Database)] {
        let repository = Arc::new(Repository {
            selection: SelectionRows {
                characters: vec![row(17)],
                customizations: vec![CustomizationRow {
                    guid: 17,
                    option: 10,
                    choice: 99,
                }],
            },
            recustomize_error: failure,
            ..Repository::good()
        });
        let mut session = session(repository.clone());
        let catalog = CharacterCatalog::fixture();
        session.initialize(&catalog, &policy(), 17).await.unwrap();
        let result = session.dispatch(&catalog, request()).await;
        assert_eq!(*repository.recustomized.lock().unwrap(), vec![17]);
        if failure.is_some() {
            assert!(matches!(
                result,
                Err(SessionError::Persistence(LoadError::Database))
            ));
            assert!(session.is_closed());
            assert!(!session.has_enumerated());
            assert!(session.legitimate_characters.is_empty());
        } else {
            assert!(result.unwrap().is_some());
            assert!(session.has_enumerated());
            assert_eq!(session.legitimate_characters.len(), 1);
        }
    }
}

#[tokio::test]
async fn codec_failure_does_not_perform_recustomization_or_publish_authority() {
    let mut character = row(17);
    character.name = "x".repeat(64);
    let repository = Arc::new(Repository {
        selection: SelectionRows {
            characters: vec![character],
            customizations: vec![CustomizationRow {
                guid: 17,
                option: 10,
                choice: 99,
            }],
        },
        ..Repository::good()
    });
    let mut session = session(repository.clone());
    let catalog = CharacterCatalog::fixture();
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    assert!(matches!(
        session.dispatch(&catalog, request()).await,
        Err(SessionError::Codec)
    ));
    assert!(repository.recustomized.lock().unwrap().is_empty());
    assert!(session.is_closed());
    assert!(session.legitimate_characters.is_empty());
}

#[tokio::test]
async fn cancelled_refresh_revokes_previous_list_and_cannot_resume_authenticated_actions() {
    struct PendingSelection(Repository);
    impl SessionRepository for PendingSelection {
        fn load_account(
            &self,
            account: u32,
            bnet: u32,
            realm: u32,
        ) -> PersistenceFutureLikeCpp<'_, Result<AccountSnapshot, LoadError>> {
            self.0.load_account(account, bnet, realm)
        }
        fn load_character_selection(
            &self,
            _: u32,
            _: bool,
        ) -> PersistenceFutureLikeCpp<'_, Result<SelectionRows, LoadError>> {
            Box::pin(std::future::pending())
        }
        fn require_recustomization(
            &self,
            account: u32,
            guid: u64,
        ) -> PersistenceFutureLikeCpp<'_, Result<(), LoadError>> {
            self.0.require_recustomization(account, guid)
        }
        fn name_in_use<'a>(
            &'a self,
            name: &'a str,
        ) -> PersistenceFutureLikeCpp<'a, Result<bool, LoadError>> {
            self.0.name_in_use(name)
        }
    }
    let mut session = session(Arc::new(PendingSelection(Repository::good())));
    let catalog = CharacterCatalog::fixture();
    session.initialize(&catalog, &policy(), 17).await.unwrap();
    session
        .legitimate_characters
        .insert(wow_core::ObjectGuid::create_player(1, 17));
    session.enumerated = true;
    let mut operation = Box::pin(session.dispatch(&catalog, request()));
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(operation.as_mut().poll(cx).is_pending()))
            .await
    );
    drop(operation);
    assert_eq!(session.phase, Phase::Selecting);
    assert!(session.legitimate_characters.is_empty());
    assert!(!session.has_enumerated());
    assert!(matches!(
        session.dispatch(&catalog, request()).await,
        Err(SessionError::Phase)
    ));
    session.close();
    assert!(session.is_closed());
}
