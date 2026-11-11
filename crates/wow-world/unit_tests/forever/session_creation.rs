//! Session fencing/publication only; synthetic rows do not prove Player::Create.
use super::*;
use crate::forever::creation::{
    CharacterCreator, ConstructedCharacter, CreationFailure, PlayerCreationLifecycle,
};
use std::future::Future;
use wow_packet::{WorldPacket, forever::character_create::CharacterCreatePayload};
use wow_persistence::forever::creation::save::{
    CreationLimits, NewAchievementSave, NewBattlegroundData, NewCharacterSessionSave,
    NewCharacterState, NewQuestCriteriaSave, NewQuestSave, NewSpecializations, NewSpellHistory,
    NewTutorialSave, NewVisualItem,
};

#[derive(Default)]
struct Creator {
    owner: Option<(u32, u32)>,
    replace_name: bool,
    failure: Option<CreationFailure>,
    replace_guid: bool,
    observed_identity: Option<Arc<std::sync::Mutex<Option<CreationIdentity>>>>,
    allocated_items: Option<Arc<std::sync::Mutex<Vec<u64>>>>,
    lifetime: Arc<std::sync::Mutex<Vec<&'static str>>>,
}

struct TracedOwner(Arc<std::sync::Mutex<Vec<&'static str>>>);

impl PlayerCreationLifecycle for TracedOwner {
    fn committed(&mut self, created: &CreatedCharacter) {
        assert_eq!(created.guid, 17);
        self.0.lock().unwrap().push("committed");
    }

    fn cleanup(&mut self) {
        self.0.lock().unwrap().push("cleanup");
    }
}

impl Drop for TracedOwner {
    fn drop(&mut self) {
        self.0.lock().unwrap().push("destroyed");
    }
}

impl CharacterCreator for Creator {
    fn limits(&self) -> CreationLimits {
        CreationLimits {
            account: 50,
            realm: 10,
        }
    }

    fn construct(
        &self,
        identities: &mut dyn CreationIdentities,
        request: &CharacterCreatePayload,
        name: &str,
        _: &CharacterCatalog,
        _: u8,
        latency: u32,
    ) -> Result<ConstructedCharacter, CreationFailure> {
        if let Some(failure) = self.failure {
            return Err(failure);
        }
        let identity = identities.identity();
        if let Some(observed) = &self.observed_identity {
            *observed.lock().unwrap() = Some(identity);
        }
        if let Some(allocated) = &self.allocated_items {
            for _ in 0..2 {
                allocated.lock().unwrap().push(
                    identities
                        .next_item_guid()
                        .map_err(|_| CreationFailure::ERROR)?,
                );
            }
        }
        let (account, realm) = self.owner.unwrap_or((identity.account, identity.realm));
        let projection = NewCharacter {
            guid: if self.replace_guid {
                identity.guid + 1
            } else {
                identity.guid
            },
            account,
            realm,
            name: if self.replace_name { "Other" } else { name }.into(),
            surname: request.surname().into(),
            race: request.race,
            class: request.class,
            gender: request.sex,
            level: 1,
            experience: 0,
            money: 0,
            map: 0,
            position: [0.0; 4],
            create_time: 17,
            create_mode: 0,
            cinematic: 0,
            at_login: 0,
            health: 1,
            powers: [0; 10],
            latency,
            primary_specialization: 0,
            active_talent_group: 0,
            client_version: Some([1, 60, 1]),
            state: NewCharacterState {
                inventory_slots: 16,
                inventory_bag_flags: 0,
                bag_slot_flags: [0; 5],
                bank_slots: 0,
                bank_tabs: 0,
                bank_bag_flags: 0,
                rest_state: 0,
                player_flags: 0,
                player_flags_ex: 0,
                instance_id: 0,
                dungeon_difficulty: 0,
                raid_difficulty: 0,
                legacy_raid_difficulty: 0,
                transport_position: [0.0; 4],
                transport_guid: 0,
                taxi_mask: vec![],
                total_played_time: 0,
                level_played_time: 0,
                rest_bonus: 0.0,
                save_time: 17,
                logout_resting: false,
                talent_reset_cost: 0,
                talent_reset_time: 0,
                extra_flags: 0,
                death_expire_time: 0,
                taxi_path: None,
                lifetime_honorable_kills: 0,
                today_honorable_kills: 0,
                yesterday_honorable_kills: 0,
                chosen_title: 0,
                watched_faction: u32::MAX,
                drunk: 0,
                loot_specialization: 0,
                explored_zones: vec![],
                known_titles: vec![],
                action_bars: 0,
                personal_tabard: [-1; 5],
                transmog_outfit: 0,
                transmog_locked: false,
            },
            fishing_steps: 0,
            battleground: NewBattlegroundData {
                instance: 0,
                team: 0,
                position: [0.0; 4],
                map: u16::MAX,
                taxi: [0; 2],
                mount_spell: 0,
                queue_id: 0,
            },
            quests: NewQuestSave {
                keep_abandoned: true,
                statuses: std::collections::BTreeMap::new(),
                rewarded: std::collections::BTreeMap::new(),
                daily: None,
                weekly: None,
                seasonal: None,
                monthly: None,
            },
            specializations: NewSpecializations {
                glyphs: std::array::from_fn(|_| Vec::new()),
                talents: std::array::from_fn(|_| Vec::new()),
                pvp: [[0; 4]; 5],
            },
            traits: Vec::new(),
            spell_history: NewSpellHistory {
                cooldowns: Vec::new(),
                charges: Vec::new(),
            },
            auras: Vec::new(),
            stored_aura_locations: Vec::new(),
            achievements: NewAchievementSave {
                completed: Vec::new(),
                progress: Vec::new(),
            },
            reputation: std::collections::BTreeMap::new(),
            quest_criteria: NewQuestCriteriaSave {
                completed_objectives: Vec::new(),
                progress: Vec::new(),
            },
            equipment_sets: std::collections::BTreeMap::new(),
            transmog_outfits: std::collections::BTreeMap::new(),
            session_save: NewCharacterSessionSave {
                tutorials: NewTutorialSave::Unchanged,
                instance_restrictions: Vec::new(),
            },
            currencies: Vec::new(),
            cuf_profiles: std::array::from_fn(|_| None),
            player_data: wow_persistence::forever::creation::save::NewPlayerDataSave {
                elements: std::collections::BTreeMap::new(),
                flags: std::collections::BTreeMap::new(),
            },
            bank_tabs: Vec::new(),
            garrison: None,
            statistics: None,
            mail: wow_persistence::forever::creation::save::NewMailSave {
                updated: false,
                messages: Vec::new(),
            },
            customizations: vec![],
            actions: vec![],
            skills: vec![],
            spells: vec![],
            items: vec![],
            visual_items: [NewVisualItem::default(); 19],
        };
        self.lifetime.lock().unwrap().push("constructed");
        Ok(ConstructedCharacter::new(
            projection,
            Box::new(TracedOwner(self.lifetime.clone())),
        ))
    }
}

fn request() -> Request {
    let mut packet = WorldPacket::new_empty();
    packet.write_bits(5, 6);
    for _ in 0..6 {
        packet.write_bit(false);
    }
    packet.write_bits(0, 6);
    packet.flush_bits();
    packet.write_uint8(1);
    packet.write_uint8(1);
    packet.write_uint8(0);
    packet.write_uint32(0);
    packet.write_int32(-1);
    packet.write_int32(0);
    packet.write_string("Elune");
    Request {
        opcode: 0x440070,
        payload: packet.into_data(),
    }
}

async fn ready(repository: Arc<Repository>, creator: Option<Creator>) -> Session {
    let mut session = session(repository);
    session
        .initialize(&CharacterCatalog::fixture(), &policy(), 17)
        .await
        .unwrap();
    if let Some(creator) = creator {
        session.attach_creator(Arc::new(creator));
    }
    session.enumerated = true;
    session
        .legitimate_characters
        .insert(wow_core::ObjectGuid::create_player(1, 9));
    session
}

fn assert_response(output: &[Outgoing], code: u32, guid: wow_core::ObjectGuid) {
    assert_eq!(output.len(), 1);
    assert_eq!(
        output[0].opcode(),
        wow_packet::forever::character_result::CREATE_CHAR_CLASSIC_OPCODE
    );
    let mut packet = WorldPacket::from_bytes(output[0].payload());
    assert_eq!(packet.read_uint32().unwrap(), code);
    assert_eq!(packet.read_packed_guid().unwrap(), guid);
    assert!(packet.is_empty());
}

#[tokio::test]
async fn creation_remains_disabled_without_a_composition_owned_constructor() {
    let repository = Arc::new(Repository::good());
    let mut session = ready(repository.clone(), None).await;
    let output = session
        .dispatch(&CharacterCatalog::fixture(), request())
        .await
        .unwrap()
        .unwrap();
    assert_response(&output, 25, wow_core::ObjectGuid::EMPTY);
    assert_eq!(repository.creations.load(Ordering::SeqCst), 0);
    assert_eq!(repository.begins.load(Ordering::SeqCst), 0);
    assert!(!session.is_closed());
}

#[tokio::test]
async fn confirmed_creation_publishes_guid_and_revokes_previous_selection() {
    let repository = Arc::new(Repository::good());
    let creator = Creator::default();
    let lifetime = creator.lifetime.clone();
    let mut session = ready(repository.clone(), Some(creator)).await;
    let output = session
        .dispatch(&CharacterCatalog::fixture(), request())
        .await
        .unwrap()
        .unwrap();
    assert_response(&output, 24, wow_core::ObjectGuid::create_player(1, 17));
    assert_eq!(repository.creations.load(Ordering::SeqCst), 1);
    assert!(!session.has_enumerated());
    assert!(session.legitimate_characters.is_empty());
    assert_eq!(session.phase, Phase::Authenticated);
    assert_eq!(
        *lifetime.lock().unwrap(),
        ["constructed", "committed", "cleanup", "destroyed"]
    );
}

#[tokio::test]
async fn known_creation_failures_publish_only_the_corresponding_error() {
    for (error, code) in [
        (CreateError::Database, 25),
        (CreateError::InvalidRow, 25),
        (CreateError::NameInUse, 27),
        (CreateError::AccountLimit, 31),
        (CreateError::RealmLimit, 30),
    ] {
        let repository = Arc::new(Repository {
            create_error: Some(error),
            ..Repository::good()
        });
        let creator = Creator::default();
        let lifetime = creator.lifetime.clone();
        let mut session = ready(repository.clone(), Some(creator)).await;
        let output = session
            .dispatch(&CharacterCatalog::fixture(), request())
            .await
            .unwrap()
            .unwrap();
        assert_response(&output, code, wow_core::ObjectGuid::EMPTY);
        assert_eq!(repository.creations.load(Ordering::SeqCst), 1);
        assert_eq!(session.phase, Phase::Authenticated);
        assert!(!session.has_enumerated());
        assert_eq!(
            *lifetime.lock().unwrap(),
            ["constructed", "cleanup", "destroyed"]
        );
    }
}

#[tokio::test]
async fn uncertain_commit_closes_without_publishing_an_operation_result() {
    for error in [
        CreateError::CommitUnknown,
        CreateError::LoginCommitUnknown,
        CreateError::RecoveryRequired,
    ] {
        let repository = Arc::new(Repository {
            create_error: Some(error),
            ..Repository::good()
        });
        let creator = Creator::default();
        let lifetime = creator.lifetime.clone();
        let mut session = ready(repository.clone(), Some(creator)).await;
        assert!(matches!(
            session.dispatch(&CharacterCatalog::fixture(), request()).await,
            Err(SessionError::Creation(actual)) if actual == error
        ));
        assert!(session.is_closed());
        assert_eq!(
            *lifetime.lock().unwrap(),
            ["constructed", "cleanup", "destroyed"]
        );
        assert!(session.legitimate_characters.is_empty());
        assert!(!session.has_enumerated());
        assert_eq!(repository.creations.load(Ordering::SeqCst), 1);
        assert!(matches!(
            session
                .dispatch(&CharacterCatalog::fixture(), request())
                .await,
            Err(SessionError::Phase)
        ));
        assert_eq!(repository.creations.load(Ordering::SeqCst), 1);
    }
}

#[tokio::test]
async fn cancelled_save_fences_the_incarnation_and_prevents_another_write() {
    let repository = Arc::new(Repository {
        create_pending: true,
        ..Repository::good()
    });
    let creator = Creator::default();
    let lifetime = creator.lifetime.clone();
    let mut session = ready(repository.clone(), Some(creator)).await;
    let catalog = CharacterCatalog::fixture();
    let mut pending = Box::pin(session.dispatch(&catalog, request()));
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(pending.as_mut().poll(cx).is_pending()))
            .await
    );
    assert_eq!(*lifetime.lock().unwrap(), ["constructed"]);
    drop(pending);
    assert_eq!(
        *lifetime.lock().unwrap(),
        ["constructed", "cleanup", "destroyed"]
    );
    assert_eq!(session.phase, Phase::Creating);
    assert!(!session.has_enumerated());
    assert!(session.legitimate_characters.is_empty());
    for opcode in [0x440070, ENUM_CHARACTERS, 0x440071] {
        assert!(matches!(
            session
                .dispatch(
                    &catalog,
                    Request {
                        opcode,
                        payload: vec![]
                    }
                )
                .await,
            Err(SessionError::Phase)
        ));
    }
    assert_eq!(repository.creations.load(Ordering::SeqCst), 1);
    session.close();
    assert!(session.is_closed());
}

#[tokio::test]
async fn constructor_cannot_replace_account_realm_or_validated_name() {
    for creator in [
        Creator {
            owner: Some((2, 0x02010001)),
            ..Creator::default()
        },
        Creator {
            owner: Some((1, 0x02010002)),
            ..Creator::default()
        },
        Creator {
            replace_name: true,
            ..Creator::default()
        },
        Creator {
            replace_guid: true,
            ..Creator::default()
        },
    ] {
        let repository = Arc::new(Repository::good());
        let lifetime = creator.lifetime.clone();
        let mut session = ready(repository.clone(), Some(creator)).await;
        assert!(matches!(
            session
                .dispatch(&CharacterCatalog::fixture(), request())
                .await,
            Err(SessionError::Protocol)
        ));
        assert_eq!(repository.creations.load(Ordering::SeqCst), 0);
        assert!(session.is_closed());
        assert_eq!(
            *lifetime.lock().unwrap(),
            ["constructed", "cleanup", "destroyed"]
        );
    }
}

#[tokio::test]
async fn construction_failure_does_not_reach_persistence() {
    let repository = Arc::new(Repository::good());
    let creator = Creator {
        failure: Some(CreationFailure::FAILED),
        ..Creator::default()
    };
    let lifetime = creator.lifetime.clone();
    let mut session = ready(repository.clone(), Some(creator)).await;
    let output = session
        .dispatch(&CharacterCatalog::fixture(), request())
        .await
        .unwrap()
        .unwrap();
    assert_response(&output, 26, wow_core::ObjectGuid::EMPTY);
    assert_eq!(repository.creations.load(Ordering::SeqCst), 0);
    assert_eq!(session.phase, Phase::Authenticated);
    assert!(lifetime.lock().unwrap().is_empty());
}

#[tokio::test]
async fn wrong_confirmed_guid_cleans_owner_without_committed_or_response() {
    let repository = Arc::new(Repository {
        create_bad_guid: true,
        ..Repository::good()
    });
    let creator = Creator::default();
    let lifetime = creator.lifetime.clone();
    let mut session = ready(repository.clone(), Some(creator)).await;
    assert!(matches!(
        session
            .dispatch(&CharacterCatalog::fixture(), request())
            .await,
        Err(SessionError::Creation(CreateError::RecoveryRequired))
    ));
    assert!(session.is_closed());
    assert_eq!(
        *lifetime.lock().unwrap(),
        ["constructed", "cleanup", "destroyed"]
    );
    assert_eq!(repository.creations.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn reserved_counter_boundaries_reject_before_construction_or_save() {
    let maximum = wow_core::ObjectGuid::max_counter(wow_core::guid::HighGuid::Player) as u64;
    for guid in [0, maximum - 1, maximum, u64::MAX] {
        let repository = Arc::new(Repository {
            reserved_guid: guid,
            ..Repository::good()
        });
        let creator = Creator::default();
        let lifetime = creator.lifetime.clone();
        let mut session = ready(repository.clone(), Some(creator)).await;
        assert!(matches!(
            session
                .dispatch(&CharacterCatalog::fixture(), request())
                .await,
            Err(SessionError::Protocol)
        ));
        assert!(session.is_closed());
        assert!(lifetime.lock().unwrap().is_empty());
        assert_eq!(repository.creations.load(Ordering::SeqCst), 0);
    }
    // The last valid counter is allowed through construction; a mapped synthetic
    // save failure avoids claiming a durable allocator or using the success probe.
    let repository = Arc::new(Repository {
        reserved_guid: maximum - 2,
        create_error: Some(CreateError::Database),
        ..Repository::good()
    });
    let observed = Arc::new(std::sync::Mutex::new(None));
    let creator = Creator {
        observed_identity: Some(observed.clone()),
        ..Creator::default()
    };
    let mut session = ready(repository.clone(), Some(creator)).await;
    let output = session
        .dispatch(&CharacterCatalog::fixture(), request())
        .await
        .unwrap()
        .unwrap();
    assert_response(&output, 25, wow_core::ObjectGuid::EMPTY);
    assert_eq!(observed.lock().unwrap().unwrap().guid, maximum - 2);
    assert_eq!(repository.creations.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn confirmed_creation_preserves_sixteen_realm_bits_in_response_guid() {
    let repository = Arc::new(Repository::good());
    let creator = Creator::default();
    let lifetime = creator.lifetime.clone();
    let mut session = ready(repository, Some(creator)).await;
    session.identity.realm_address = 0x0201_8001;
    let output = session
        .dispatch(&CharacterCatalog::fixture(), request())
        .await
        .unwrap()
        .unwrap();
    assert_response(
        &output,
        24,
        wow_core::ObjectGuid::new(0x0A00_0400_0000_0000, 17),
    );
    assert_eq!(
        *lifetime.lock().unwrap(),
        ["constructed", "committed", "cleanup", "destroyed"]
    );
    assert_eq!(session.phase, Phase::Authenticated);
}

#[tokio::test]
async fn construction_uses_the_reserved_identity_and_allocates_items_before_save() {
    let repository = Arc::new(Repository::good());
    let identity = Arc::new(std::sync::Mutex::new(None));
    let items = Arc::new(std::sync::Mutex::new(vec![]));
    let mut session = ready(
        repository.clone(),
        Some(Creator {
            observed_identity: Some(identity.clone()),
            allocated_items: Some(items.clone()),
            ..Creator::default()
        }),
    )
    .await;
    let output = session
        .dispatch(&CharacterCatalog::fixture(), request())
        .await
        .unwrap()
        .unwrap();
    assert_response(&output, 24, wow_core::ObjectGuid::create_player(1, 17));
    assert_eq!(
        *identity.lock().unwrap(),
        Some(CreationIdentity {
            guid: 17,
            account: 1,
            realm: 0x02010001
        })
    );
    assert_eq!(*items.lock().unwrap(), [41, 42]);
    assert_eq!(repository.begins.load(Ordering::SeqCst), 1);
    assert_eq!(repository.creations.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn admission_failure_never_invokes_the_constructor_or_save() {
    for (error, code) in [
        (CreateError::Database, 25),
        (CreateError::InvalidRow, 25),
        (CreateError::NameInUse, 27),
        (CreateError::AccountLimit, 31),
        (CreateError::RealmLimit, 30),
    ] {
        let repository = Arc::new(Repository {
            begin_error: Some(error),
            ..Repository::good()
        });
        let identity = Arc::new(std::sync::Mutex::new(None));
        let mut session = ready(
            repository.clone(),
            Some(Creator {
                observed_identity: Some(identity.clone()),
                ..Creator::default()
            }),
        )
        .await;
        let output = session
            .dispatch(&CharacterCatalog::fixture(), request())
            .await
            .unwrap()
            .unwrap();
        assert_response(&output, code, wow_core::ObjectGuid::EMPTY);
        assert!(identity.lock().unwrap().is_none());
        assert_eq!(repository.begins.load(Ordering::SeqCst), 1);
        assert_eq!(repository.creations.load(Ordering::SeqCst), 0);
        assert_eq!(session.phase, Phase::Authenticated);
    }
}

#[tokio::test]
async fn cancelled_admission_leaves_this_session_fenced_without_constructing() {
    let repository = Arc::new(Repository {
        begin_pending: true,
        ..Repository::good()
    });
    let identity = Arc::new(std::sync::Mutex::new(None));
    let mut session = ready(
        repository.clone(),
        Some(Creator {
            observed_identity: Some(identity.clone()),
            ..Creator::default()
        }),
    )
    .await;
    let catalog = CharacterCatalog::fixture();
    let mut operation = Box::pin(session.dispatch(&catalog, request()));
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(operation.as_mut().poll(cx).is_pending()))
            .await
    );
    drop(operation);
    assert_eq!(session.phase, Phase::Creating);
    assert!(!session.has_enumerated());
    assert!(session.legitimate_characters.is_empty());
    assert!(identity.lock().unwrap().is_none());
    assert_eq!(repository.begins.load(Ordering::SeqCst), 1);
    assert_eq!(repository.creations.load(Ordering::SeqCst), 0);
    assert!(matches!(
        session.dispatch(&catalog, request()).await,
        Err(SessionError::Phase)
    ));
    assert_eq!(repository.begins.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn pending_recovery_during_admission_closes_without_constructing_or_responding() {
    let repository = Arc::new(Repository {
        begin_error: Some(CreateError::RecoveryRequired),
        ..Repository::good()
    });
    let identity = Arc::new(std::sync::Mutex::new(None));
    let mut session = ready(
        repository.clone(),
        Some(Creator {
            observed_identity: Some(identity.clone()),
            ..Creator::default()
        }),
    )
    .await;
    assert!(matches!(
        session
            .dispatch(&CharacterCatalog::fixture(), request())
            .await,
        Err(SessionError::Creation(CreateError::RecoveryRequired))
    ));
    assert!(session.is_closed());
    assert!(identity.lock().unwrap().is_none());
    assert_eq!(repository.creations.load(Ordering::SeqCst), 0);
}
