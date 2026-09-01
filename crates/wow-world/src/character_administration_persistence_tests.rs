//! Focused contract tests for the character-administration capability.

use std::sync::{Arc, Mutex};

use crate::handlers::character::tests::make_session_with_send_capacity;
use wow_constants::ServerOpcodes;
use wow_core::ObjectGuid;
use wow_packet::WorldPacket;
use wow_packet::packets::character::CharacterRenameRequest;
use wow_persistence::{
    CharacterAdministrationLoadOutcomeLikeCpp, CharacterAdministrationMutationOutcomeLikeCpp,
    CharacterAdministrationPersistencePortLikeCpp, CharacterCreatePersistenceRequestLikeCpp,
    CharacterCustomizationPersistenceLikeCpp, CharacterCustomizeCandidateLikeCpp,
    CharacterRenameCandidateLikeCpp, PersistenceFutureLikeCpp,
};

const AT_LOGIN_RENAME_LIKE_CPP: u16 = 0x001;
const AT_LOGIN_CUSTOMIZE_LIKE_CPP: u16 = 0x008;

#[derive(Debug, Clone, PartialEq, Eq)]
enum RenameTraceLikeCpp {
    Load { guid: u64, name: String },
    Commit { guid: u64, name: String, flags: u16 },
}

#[derive(Default)]
struct RenamePortFixtureLikeCpp {
    trace: Mutex<Vec<RenameTraceLikeCpp>>,
}

impl CharacterAdministrationPersistencePortLikeCpp for RenamePortFixtureLikeCpp {
    fn find_character_name_like_cpp(
        &self,
        _name: &str,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationLoadOutcomeLikeCpp<()>> {
        panic!("rename test must not perform create admission")
    }

    fn load_account_character_count_like_cpp(
        &self,
        _account_id: u32,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationLoadOutcomeLikeCpp<u64>> {
        panic!("rename test must not perform create admission")
    }

    fn create_character_like_cpp(
        &self,
        _request: CharacterCreatePersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationMutationOutcomeLikeCpp> {
        panic!("rename test must not create a character")
    }

    fn delete_owned_character_like_cpp(
        &self,
        _guid: u64,
        _account_id: u32,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationMutationOutcomeLikeCpp> {
        panic!("rename test must not delete a character")
    }

    fn load_rename_candidate_like_cpp(
        &self,
        guid: u64,
        new_name: &str,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CharacterAdministrationLoadOutcomeLikeCpp<CharacterRenameCandidateLikeCpp>,
    > {
        self.trace.lock().unwrap().push(RenameTraceLikeCpp::Load {
            guid,
            name: new_name.into(),
        });
        Box::pin(async {
            CharacterAdministrationLoadOutcomeLikeCpp::Loaded(CharacterRenameCandidateLikeCpp {
                old_name: "Oldname".into(),
                at_login_flags: AT_LOGIN_RENAME_LIKE_CPP | AT_LOGIN_CUSTOMIZE_LIKE_CPP,
            })
        })
    }

    fn commit_rename_like_cpp(
        &self,
        guid: u64,
        new_name: &str,
        at_login_flags: u16,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationMutationOutcomeLikeCpp> {
        self.trace.lock().unwrap().push(RenameTraceLikeCpp::Commit {
            guid,
            name: new_name.into(),
            flags: at_login_flags,
        });
        Box::pin(async { CharacterAdministrationMutationOutcomeLikeCpp::Applied })
    }

    fn load_customize_candidate_like_cpp(
        &self,
        _guid: u64,
    ) -> PersistenceFutureLikeCpp<
        '_,
        CharacterAdministrationLoadOutcomeLikeCpp<CharacterCustomizeCandidateLikeCpp>,
    > {
        panic!("rename test must not customize a character")
    }

    fn commit_customize_like_cpp(
        &self,
        _guid: u64,
        _name: &str,
        _at_login_flags: u16,
        _customizations: Vec<CharacterCustomizationPersistenceLikeCpp>,
    ) -> PersistenceFutureLikeCpp<'_, CharacterAdministrationMutationOutcomeLikeCpp> {
        panic!("rename test must not customize a character")
    }
}

#[tokio::test]
async fn character_rename_uses_typed_administration_port_in_cpp_order() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let guid = ObjectGuid::create_player(1, 42);
    session.set_legit_characters(vec![guid]);
    let port = Arc::new(RenamePortFixtureLikeCpp::default());
    session.set_character_administration_persistence_port_like_cpp(port.clone());

    session
        .handle_character_rename_request(CharacterRenameRequest {
            guid,
            new_name: "Newname".into(),
        })
        .await;

    assert_eq!(
        *port.trace.lock().unwrap(),
        vec![
            RenameTraceLikeCpp::Load {
                guid: 42,
                name: "Newname".into(),
            },
            RenameTraceLikeCpp::Commit {
                guid: 42,
                name: "Newname".into(),
                flags: AT_LOGIN_CUSTOMIZE_LIKE_CPP,
            },
        ]
    );
    let sent = send_rx.try_recv().expect("rename success");
    let mut packet = WorldPacket::from_bytes(&sent);
    assert_eq!(
        packet.server_opcode(),
        Some(ServerOpcodes::CharacterRenameResult)
    );
    packet.skip_opcode();
    assert_eq!(packet.read_uint8().unwrap(), 0);
    assert!(packet.read_bit().unwrap());
}
