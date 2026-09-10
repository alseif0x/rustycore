//! Session packets.
//!
//! Separated from fixtures.rs under #709.

use super::*;

pub(crate) struct SagaFixtureLikeCpp {
    pub(crate) session: WorldSession,
    pub(crate) send_rx: flume::Receiver<Vec<u8>>,
    pub(crate) store: Arc<FakeBattlePetPurchaseStoreLikeCpp>,
    pub(crate) persistence: Arc<FakeSagaPersistenceLikeCpp>,
    pub(crate) registry: Arc<BattlePetAccountRegistryLikeCpp>,
}

pub(crate) fn make_saga_session_like_cpp(
    player_counter: i64,
    money: u64,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(64);
    let mut session = WorldSession::new(
        ACCOUNT_ID,
        "SagaTest".into(),
        0,
        2,
        9,
        54_261,
        vec![0; 40],
        "enUS".into(),
        pkt_rx,
        send_tx,
    );
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        ObjectGuid::create_player(1, player_counter),
        "Buyer".to_string(),
        Position::ZERO,
        0,
        1,
        1,
        80,
        0,
    ));
    session.set_battlenet_account_id(ACCOUNT_ID);
    session.set_player_gold_like_cpp(money);
    session.set_battle_pet_species_store(saga_species_store_like_cpp());
    session.set_battle_pet_purchase_selection_override_like_cpp(Some(saga_selection_like_cpp(
        SAGA_SPECIES,
    )));
    (session, send_rx)
}

pub(crate) async fn saga_fixture_like_cpp(
    money: u64,
    seeded_pets: Vec<DurableBattlePetRowLikeCpp>,
) -> SagaFixtureLikeCpp {
    let persistence = Arc::new(FakeSagaPersistenceLikeCpp::with_seeded_pets(seeded_pets));
    let store =
        Arc::new(FakeBattlePetPurchaseStoreLikeCpp::new().with_money(PLAYER_COUNTER as u64, money));
    let registry = saga_registry_like_cpp(Arc::clone(&persistence));
    let (mut session, send_rx) = make_saga_session_like_cpp(PLAYER_COUNTER, money);
    session.set_battle_pet_purchase_persistence_port_like_cpp(store_handle_like_cpp(&store));
    let attachment = registry
        .attach_like_cpp(ACCOUNT_ID)
        .await
        .expect("saga account attaches");
    session.set_battle_pet_account_attachment_like_cpp(attachment);
    SagaFixtureLikeCpp {
        session,
        send_rx,
        store,
        persistence,
        registry,
    }
}

/// A "process restart": the old session/registry are gone and a fresh
/// registry wraps the same durable fakes (Login/Character DB survive).
pub(crate) async fn restart_saga_session_like_cpp(
    store: Arc<FakeBattlePetPurchaseStoreLikeCpp>,
    persistence: Arc<FakeSagaPersistenceLikeCpp>,
    money: u64,
) -> SagaFixtureLikeCpp {
    let registry = saga_registry_like_cpp(Arc::clone(&persistence));
    let (mut session, send_rx) = make_saga_session_like_cpp(PLAYER_COUNTER, money);
    session.set_battle_pet_purchase_persistence_port_like_cpp(store_handle_like_cpp(&store));
    let attachment = registry
        .attach_like_cpp(ACCOUNT_ID)
        .await
        .expect("saga account attaches after restart");
    session.set_battle_pet_account_attachment_like_cpp(attachment);
    SagaFixtureLikeCpp {
        session,
        send_rx,
        store,
        persistence,
        registry,
    }
}

pub(crate) async fn execute_saga_purchase_like_cpp(
    fixture: &mut SagaFixtureLikeCpp,
    offer: PreparedBattlePetTrainerOfferLikeCpp,
) -> BattlePetPurchaseExecutionLikeCpp {
    let guard = fixture
        .session
        .begin_exclusive_player_money_persistence_like_cpp()
        .await
        .expect("money exclusivity");
    fixture
        .session
        .execute_battle_pet_trainer_purchase_like_cpp(
            guard,
            saga_trainer_guid_like_cpp(),
            TRAINER_ID,
            offer,
        )
        .await
}

pub(crate) fn expected_pet_packet_like_cpp(
    fixture: &SagaFixtureLikeCpp,
    pet_guid: ObjectGuid,
) -> wow_packet::packets::misc::BattlePetJournalPet {
    owner_of(fixture)
        .pet_snapshot_like_cpp(pet_guid)
        .expect("durable pet snapshot")
        .packet_info_like_cpp(pet_guid)
}

pub(crate) fn assert_no_packets(fixture: &SagaFixtureLikeCpp) {
    assert!(
        fixture.send_rx.try_recv().is_err(),
        "no packets must be published on this path"
    );
}

pub(crate) fn expect_money_update_packet_like_cpp(
    fixture: &SagaFixtureLikeCpp,
    money: u64,
) -> Vec<u8> {
    wow_packet::packets::update::UpdateObject::player_money_update(
        fixture.session.player_guid().expect("player guid"),
        fixture.session.player_map_id_like_cpp(),
        money,
        None,
    )
    .to_bytes()
}

/// Run one async body on a 32 MiB stack, mirroring `run_player_stack_test`
/// in `wow-entities` (`object_accessor.rs`). `#[tokio::test]` runs the body
/// on a default 2 MiB thread, which cannot hold two joined debug-profile
/// purchase state machines at once.
pub(crate) fn run_async_stack_test<F>(test: impl FnOnce() -> F + Send + 'static)
where
    F: Future<Output = ()>,
{
    std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("current-thread runtime")
                .block_on(test());
        })
        .expect("stack test thread")
        .join()
        .unwrap_or_else(|payload| std::panic::resume_unwind(payload));
}

/// A fully rigged handler session: the trainer list/buy path runs the
/// real admission composition (membership, gates, conditions, price,
/// classification) before the saga. `wrapper_learned_spell` adds a
/// `SPELL_EFFECT_LEARN_SPELL` effect so the trainer spell is castable
/// (C++ `IsCastable()`), turning the offer into the normal wrapper
/// acquisition that retains its battle-pet species classification.
pub(crate) async fn saga_handler_fixture_like_cpp(
    money: u64,
    battle_pet_price: u32,
    wrapper_learned_spell: Option<u32>,
    seeded_pets: Vec<DurableBattlePetRowLikeCpp>,
) -> SagaFixtureLikeCpp {
    let persistence = Arc::new(FakeSagaPersistenceLikeCpp::with_seeded_pets(seeded_pets));
    let store =
        Arc::new(FakeBattlePetPurchaseStoreLikeCpp::new().with_money(PLAYER_COUNTER as u64, money));
    let registry = saga_registry_like_cpp(Arc::clone(&persistence));
    let (mut session, send_rx) = make_saga_session_like_cpp(PLAYER_COUNTER, money);
    let canonical = Arc::new(std::sync::Mutex::new(wow_map::MapManager::default()));
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_trainer_store_like_cpp(Arc::new(
        wow_data::TrainerStoreLikeCpp::from_rows_like_cpp(
            vec![wow_data::TrainerRowLikeCpp {
                id: TRAINER_ID,
                trainer_type: 2,
                greeting: "Train".to_string(),
            }],
            vec![wow_data::TrainerSpellRowLikeCpp {
                trainer_id: TRAINER_ID,
                spell: wow_data::TrainerSpellLikeCpp {
                    spell_id: SAGA_SPELL_ID,
                    money_cost: battle_pet_price,
                    req_skill_line: 0,
                    req_skill_rank: 0,
                    req_ability: [0; 3],
                    req_level: 1,
                },
            }],
            Vec::new(),
            vec![wow_data::CreatureTrainerRowLikeCpp {
                creature_id: SAGA_CREATURE_ENTRY,
                trainer_id: TRAINER_ID,
                menu_id: 0,
                option_id: 0,
            }],
            |_| true,
            |_| true,
            |_| true,
            |_, _| true,
        )
        .store,
    ));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_disable_mgr(Arc::new(wow_data::DisableMgrLikeCpp::default()));
    session.set_player_aura_authority_complete_like_cpp(true);
    session.set_condition_store(Arc::new(wow_data::ConditionEntriesByTypeStore::default()));
    session.set_skill_store(Arc::new(
        wow_data::SkillStore::from_skill_line_abilities_and_race_class_like_cpp([], []),
    ));
    session.set_skill_line_store(Arc::new(wow_data::SkillLineStore::from_entries([])));
    session.set_skill_tiers_store(Arc::new(wow_data::SkillTiersStoreLikeCpp::default()));
    session.set_trait_definition_store(Arc::new(
        wow_data::trait_tree::TraitDefinitionStore::from_entries([]),
    ));
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([])));
    session.set_spell_chain_store(Arc::new(wow_data::SpellChainStoreLikeCpp::default()));
    session.set_spell_custom_attribute_store(Arc::new(
        wow_data::SpellCustomAttributeStoreLikeCpp::default(),
    ));
    let mut learn_skills = wow_data::SpellLearnSkillStoreLikeCpp::default();
    learn_skills.covered_spell_ids.extend([SAGA_SPELL_ID]);
    if let Some(learned) = wrapper_learned_spell {
        learn_skills.covered_spell_ids.extend([learned]);
    }
    session.set_spell_learn_skill_store(Arc::new(learn_skills));
    if wrapper_learned_spell.is_some() {
        session.set_spell_acquisition_static_authority_like_cpp([SAGA_SPELL_ID], []);
        session.set_loot_money_persistence_test_result_like_cpp(true);
    }
    session.set_spell_learn_spell_store(Arc::new(wow_data::SpellLearnSpellStoreLikeCpp::default()));
    session.set_spell_required_store(Arc::new(wow_data::SpellRequiredStoreLikeCpp::default()));
    session.set_spell_linked_store(Arc::new(wow_data::SpellLinkedStoreLikeCpp::default()));
    session.set_spell_pet_aura_store(Arc::new(wow_data::SpellPetAuraStoreLikeCpp::default()));
    session.set_spell_target_restrictions_store(Arc::new(
        wow_data::SpellTargetRestrictionsStore::from_entries([]),
    ));
    session.set_spell_aura_restrictions_store(Arc::new(
        wow_data::SpellAuraRestrictionsStore::from_entries([]),
    ));
    let mut coverage = vec![wow_data::SpellAcquisitionCoverageSeedLikeCpp::covered(
        SAGA_SPELL_ID,
        0,
    )];
    let mut spell_effects = vec![saga_summon_effect_like_cpp(SAGA_SPELL_ID)];
    if let Some(learned) = wrapper_learned_spell {
        coverage.push(wow_data::SpellAcquisitionCoverageSeedLikeCpp::covered(
            learned, 0,
        ));
        spell_effects.push(saga_learn_effect_like_cpp(2, SAGA_SPELL_ID, learned));
    }
    session.set_spell_acquisition_catalog(Arc::new(
        wow_data::SpellAcquisitionCatalogLikeCpp::from_effective_rows_like_cpp(
            coverage,
            wow_data::EffectiveSpellAcquisitionRowsLikeCpp {
                spell_effects,
                summon_properties: vec![wow_data::SpellAcquisitionSummonPropertiesLikeCpp {
                    record_id: SAGA_SUMMON_PROPERTIES_ID,
                    slot_raw: SAGA_SUMMON_SLOT_MINIPET_RAW,
                    flags_1_raw: SAGA_SUMMON_FROM_JOURNAL_RAW,
                }],
                battle_pet_species: vec![wow_data::SpellAcquisitionBattlePetSpeciesLikeCpp {
                    species_id: SAGA_SPECIES,
                    creature_id_raw: 99,
                }],
                ..Default::default()
            },
            wow_data::SpellAcquisitionTableHashesLikeCpp::default(),
            Vec::new(),
        ),
    ));
    session.set_known_spells_like_cpp(Vec::new());
    assert!(session.set_complete_represented_player_spell_rows_like_cpp([]));
    assert!(session.set_complete_represented_spell_trait_definition_ids_like_cpp([]));
    assert!(session.set_complete_represented_override_spells_like_cpp([]));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("canonical player map");
    // Production login hydrates Player::SetFactionForRace before the
    // trainer can be used. This synthetic fixture has no ChrRaces store,
    // so install that canonical interaction prerequisite explicitly.
    session.set_player_faction_template_like_cpp(1);
    assert!(
        session.set_complete_player_skill_records_like_cpp(std::collections::HashMap::new(), 0)
    );
    insert_saga_trainer_creature_like_cpp(&canonical, saga_trainer_guid_like_cpp());
    session.set_player_trainer_interaction_like_cpp(saga_trainer_guid_like_cpp(), TRAINER_ID);
    session.set_battle_pet_purchase_persistence_port_like_cpp(store_handle_like_cpp(&store));
    let attachment = registry
        .attach_like_cpp(ACCOUNT_ID)
        .await
        .expect("saga account attaches");
    session.set_battle_pet_account_attachment_like_cpp(attachment);
    SagaFixtureLikeCpp {
        session,
        send_rx,
        store,
        persistence,
        registry,
    }
}

pub(crate) fn saga_buy_packet_like_cpp(spell_id: i32) -> WorldPacket {
    let mut packet = WorldPacket::new_empty();
    packet.write_packed_guid(&saga_trainer_guid_like_cpp());
    packet.write_int32(TRAINER_ID as i32);
    packet.write_int32(spell_id);
    packet.reset_read();
    packet
}
