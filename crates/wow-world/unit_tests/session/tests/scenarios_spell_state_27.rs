//! Production spell execution coverage for resurrection offer effects.

use super::*;

const RESURRECT: u32 = 18;
const RESURRECT_NEW: u32 = 113;
const RESURRECT_WITH_AURA: u32 = 172;
const SPELL_ID: i32 = 96_101;
const TRIGGER_ID: i32 = 96_102;

struct OfferFixture {
    session: crate::session::WorldSession,
    canonical: SharedCanonicalMapManager,
    session_player: ObjectGuid,
    caster: ObjectGuid,
    target: ObjectGuid,
    target_handle: wow_map::PlayerHandle,
    send_rx: flume::Receiver<Vec<u8>>,
    ordinary_rx: flume::Receiver<Vec<u8>>,
    realm_rx: flume::Receiver<Vec<u8>>,
    target_position: Position,
}

struct DecodedOffer {
    offerer: ObjectGuid,
    realm: u32,
    pet_number: u32,
    spell_id: i32,
    use_timer: bool,
    sickness: bool,
    name: String,
}

fn offer_fixture(target_map_id: u32, locale: &str) -> OfferFixture {
    let (mut session, _, send_rx) = make_session();
    let session_player = ObjectGuid::create_player(1, 96_100);
    session.set_player_guid(Some(session_player));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    crate::canonical_player_access::install_canonical_player_owner_for_test(&mut session, 571, 0);
    let canonical = session
        .core
        .canonical_map_manager
        .as_ref()
        .cloned()
        .expect("installed canonical manager");

    let registry = Arc::new(PlayerRegistry::new());
    session.set_player_registry(Arc::clone(&registry));
    let target = ObjectGuid::create_player(1, 96_103);
    let target_position = Position::new(41.0, 42.0, 43.0, 0.25);
    let target_handle = {
        let mut manager = canonical.lock().expect("canonical manager");
        manager.create_world_map(target_map_id, 0);
        let mut player = Box::new(wow_entities::Player::new(Some(1), false));
        player.unit_mut().world_mut().object_mut().create(target);
        player.unit_mut().set_max_health(120);
        player.unit_mut().set_max_power(PowerType::Mana, 200);
        player.unit_mut().set_health(0);
        player
            .unit_mut()
            .set_death_state(wow_constants::DeathState::Dead);
        let handle = manager
            .install_detached_player_like_cpp(player)
            .expect("install canonical target");
        manager
            .attach_player_like_cpp(
                handle,
                wow_map::MapKey::new(target_map_id, 0),
                target_position,
            )
            .expect("attach canonical target");
        handle
    };

    let caster = test_creature_guid(96_104);
    let caster_position = Position::new(51.0, 52.0, 53.0, 0.75);
    add_canonical_test_creature_on_map(
        &canonical,
        caster,
        96_105,
        caster_position,
        wow_constants::unit::NPCFlags1::SPIRIT_HEALER.bits(),
        571,
        0,
    );
    canonical
        .lock()
        .expect("canonical manager")
        .find_map_mut(571, 0)
        .expect("caster map")
        .map_mut()
        .with_creature_mut_like_cpp(caster, |creature| {
            creature
                .unit_mut()
                .world_mut()
                .set_name("Actual Spirit Guide");
        })
        .expect("canonical creature");

    let (ordinary_tx, ordinary_rx) = flume::unbounded();
    let (realm_tx, realm_rx) = flume::unbounded();
    let mut registration = broadcast_info(target, ordinary_tx);
    registration.identity = PlayerDirectoryIdentityLikeCpp::new_with_bnet(
        "OfferTarget",
        1,
        0,
        0,
        1,
        1,
        0,
        2,
        locale,
        Some(target_handle),
    );
    registration.realm_send_tx = realm_tx;
    // This projection intentionally disagrees with the canonical active Player.
    // Resurrection admission must use the MapManager owner and only the
    // generation/handle identity from this directory registration.
    registration.placement.map_id = 42;
    registration.placement.instance_id = 19;
    registration.placement.is_in_world = false;
    registration.placement.is_alive = true;
    registry.register_or_replace(target, registration, Default::default());

    OfferFixture {
        session,
        canonical,
        session_player,
        caster,
        target,
        target_handle,
        send_rx,
        ordinary_rx,
        realm_rx,
        target_position,
    }
}

fn creature_template(entry: u32) -> wow_data::CreatureQueryTemplateLikeCpp {
    wow_data::CreatureQueryTemplateLikeCpp {
        entry,
        name: "Template Spirit Guide".into(),
        subname: String::new(),
        title_alt: String::new(),
        icon_name: String::new(),
        creature_type: 0,
        creature_family: 0,
        classification: 0,
        kill_credits: [0; 2],
        civilian: false,
        racial_leader: false,
        movement_id: 0,
        required_expansion: 0,
        vignette_id: 0,
        unit_class: 0,
        widget_set_id: 0,
        widget_set_unit_condition_id: 0,
        hp_multi: 1.0,
        energy_multi: 1.0,
        creature_difficulty_id: 0,
        type_flags: [0; 2],
        displays: Vec::new(),
    }
}

fn install_creature_catalog(fixture: &mut OfferFixture, localized_name: Option<&str>) {
    let locales = localized_name.map(|name| wow_data::CreatureQueryLocaleLikeCpp {
        entry: 96_105,
        locale: "esES".into(),
        name: name.into(),
        subname: String::new(),
        title_alt: String::new(),
    });
    let catalog = wow_data::CreatureQueryCatalogLikeCpp::from_rows_like_cpp(
        [creature_template(96_105)],
        locales,
    );
    let mut object_mgr = wow_world_core::session::ObjectMgrCatalogsLikeCpp::default();
    object_mgr.creature = Arc::new(catalog);
    fixture
        .session
        .set_object_mgr_catalogs_like_cpp(Arc::new(object_mgr));
}

fn spell_info(
    spell_id: i32,
    effect: u32,
    amount: i32,
    mana_amount: i32,
    trigger_spell: i32,
) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: effect,
        // The effect row below is authoritative for the interim calculation.
        effect_base_points: -900,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: vec![wow_data::SpellEffectInfo {
            effect_index: 0,
            effect,
            effect_base_points: amount,
            effect_misc_value_1: mana_amount,
            effect_trigger_spell: trigger_spell,
            ..Default::default()
        }],
    }
}

fn install_spell(
    fixture: &mut OfferFixture,
    spell_id: i32,
    effect: u32,
    amount: i32,
    mana_amount: i32,
    trigger_spell: i32,
    valid_trigger: bool,
    no_res_timer: bool,
) {
    let mut store = wow_data::SpellStore::new();
    store.insert(
        spell_id,
        spell_info(spell_id, effect, amount, mana_amount, trigger_spell),
    );
    if valid_trigger {
        store.insert(trigger_spell, spell_info(trigger_spell, 0, 0, 0, 0));
    }
    let mut attributes = [0; 15];
    if no_res_timer {
        attributes[3] |= 0x0000_0010;
    }
    store.insert_spell_misc_attributes_like_cpp(spell_id, attributes);
    fixture.session.set_spell_store(Arc::new(store));
}

fn install_spell_with_die_sides(
    fixture: &mut OfferFixture,
    spell_id: i32,
    effect: u32,
    base_points: i32,
    mana_amount: i32,
    die_sides: i32,
) {
    let mut info = spell_info(spell_id, effect, base_points, mana_amount, 0);
    info.effects[0].effect_die_sides = die_sides;
    let mut store = wow_data::SpellStore::new();
    store.insert(spell_id, info);
    store.insert_spell_misc_attributes_like_cpp(spell_id, [0_u32; 15]);
    fixture.session.set_spell_store(Arc::new(store));
}

async fn cast_offer(
    fixture: &mut OfferFixture,
    spell_id: i32,
    hit_guid: ObjectGuid,
    caster_guid: ObjectGuid,
) {
    fixture
        .session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            hit_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            wow_packet::packets::spell::SpellTargetData {
                flags: 0x2,
                unit: hit_guid,
                ..Default::default()
            },
            SpellCastMetadata {
                caster_guid_override: Some(caster_guid),
                ..Default::default()
            },
        )
        .await
        .expect("represented resurrection spell execution");
}

fn decode_offer(bytes: &[u8]) -> DecodedOffer {
    let mut packet = wow_packet::WorldPacket::from_bytes(bytes);
    assert_eq!(
        packet.read_uint16().expect("opcode"),
        ServerOpcodes::ResurrectRequest as u16
    );
    let offerer = packet.read_packed_guid().expect("offerer GUID");
    let realm = packet.read_uint32().expect("virtual realm address");
    let pet_number = packet.read_uint32().expect("pet number");
    let spell_id = packet.read_int32().expect("spell ID");
    let name_len = packet.read_bits(11).expect("name byte length") as usize;
    let use_timer = packet.read_bit().expect("timer bit");
    let sickness = packet.read_bit().expect("sickness bit");
    let name = packet.read_string(name_len).expect("offerer name");
    assert!(packet.is_empty());
    DecodedOffer {
        offerer,
        realm,
        pet_number,
        spell_id,
        use_timer,
        sickness,
        name,
    }
}

fn current_request(
    fixture: &OfferFixture,
) -> Option<wow_entities::PlayerResurrectionRequestLikeCpp> {
    fixture
        .canonical
        .lock()
        .expect("canonical manager")
        .with_player_like_cpp(fixture.target_handle, |player| {
            player.resurrection_state_like_cpp().request.clone()
        })
        .flatten()
}

fn add_corpse_for_target(fixture: &OfferFixture) -> ObjectGuid {
    let corpse_guid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 571, 0, 0, 96_106);
    let mut corpse = wow_entities::Corpse::new_at(wow_entities::CorpseType::ResurrectablePve, 1);
    corpse.world_mut().object_mut().create(corpse_guid);
    corpse.world_mut().set_map(571, 0).expect("corpse map");
    corpse
        .world_mut()
        .relocate(Position::new(51.0, 52.0, 53.0, 0.75));
    corpse.world_mut().object_mut().add_to_world();
    corpse.set_owner_guid(fixture.target);
    fixture
        .canonical
        .lock()
        .expect("canonical manager")
        .find_map_mut(571, 0)
        .expect("caster map")
        .map_mut()
        .add_map_object_record_to_map_like_cpp(
            wow_entities::MapObjectRecord::new_corpse(corpse).expect("corpse record"),
        )
        .expect("corpse insertion");
    corpse_guid
}

#[tokio::test]
async fn resurrect_resolves_remote_corpse_owner_and_sends_localized_realm_offer() {
    let mut fixture = offer_fixture(0, "esES");
    install_creature_catalog(&mut fixture, Some("Espíritu guía"));
    install_spell(&mut fixture, SPELL_ID, RESURRECT, 40, 99, 0, false, true);
    let corpse_guid = add_corpse_for_target(&fixture);

    let caster = fixture.caster;
    cast_offer(&mut fixture, SPELL_ID, corpse_guid, caster).await;

    let bytes = fixture
        .realm_rx
        .try_recv()
        .expect("realm-routed resurrection request");
    let decoded = decode_offer(&bytes);
    assert_eq!(decoded.offerer, fixture.caster);
    assert_eq!(decoded.realm, fixture.session.core.virtual_realm_address());
    assert_eq!(decoded.pet_number, 0);
    assert_eq!(decoded.spell_id, SPELL_ID);
    assert!(!decoded.use_timer, "NO_RES_TIMER clears the timer bit");
    assert!(decoded.sickness, "the offerer is a spirit healer");
    assert_eq!(decoded.name, "Espíritu guía");
    assert!(fixture.ordinary_rx.try_recv().is_err());

    let request = current_request(&fixture).expect("canonical request data");
    assert_eq!(request.resurrecter, fixture.caster);
    assert_eq!(request.map_id, 571);
    assert_eq!(request.position, Position::new(51.0, 52.0, 53.0, 0.75));
    assert_eq!((request.health, request.mana, request.aura), (48, 80, 0));
    assert!(drain_server_opcodes(&fixture.send_rx).contains(&ServerOpcodes::SpellExecuteLog));
}

#[tokio::test]
async fn resurrect_new_admits_just_died_and_reads_the_players_current_pet() {
    let mut fixture = offer_fixture(571, "frFR");
    let pet_guid = test_pet_guid(96_107);
    add_canonical_test_pet_with_number(
        &fixture.canonical,
        pet_guid,
        fixture.target,
        96_108,
        fixture.target_position,
        77,
        0,
        0,
    );
    fixture
        .canonical
        .lock()
        .expect("canonical manager")
        .with_player_mut_like_cpp(fixture.target_handle, |player| {
            player.unit_mut().set_health(0);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::JustDied);
            player.set_pet_guid_like_cpp(Some(pet_guid));
        })
        .expect("canonical target");
    install_creature_catalog(&mut fixture, None);
    install_spell(
        &mut fixture,
        SPELL_ID,
        RESURRECT_NEW,
        31,
        17,
        0,
        false,
        false,
    );

    let target = fixture.target;
    let caster = fixture.caster;
    cast_offer(&mut fixture, SPELL_ID, target, caster).await;

    let decoded = decode_offer(
        &fixture
            .realm_rx
            .try_recv()
            .expect("JustDied target receives its offer"),
    );
    assert_eq!(decoded.offerer, fixture.caster);
    assert_eq!(decoded.pet_number, 77);
    assert_eq!(decoded.spell_id, SPELL_ID);
    assert!(decoded.use_timer);
    assert!(decoded.sickness);
    assert_eq!(decoded.name, "Actual Spirit Guide");
    let request = current_request(&fixture).expect("canonical request data");
    assert_eq!((request.health, request.mana), (31, 17));
    assert_eq!(request.resurrecter, fixture.caster);
}

#[tokio::test]
async fn resurrect_new_uses_one_sided_die_for_data_style_effect_value() {
    let mut fixture = offer_fixture(571, "enUS");
    install_spell_with_die_sides(&mut fixture, SPELL_ID, RESURRECT_NEW, 31, 17, 1);

    let target = fixture.target;
    let caster = fixture.caster;
    cast_offer(&mut fixture, SPELL_ID, target, caster).await;

    let decoded = decode_offer(
        &fixture
            .realm_rx
            .try_recv()
            .expect("effect 113 sends its data-style resurrection offer"),
    );
    assert_eq!(decoded.spell_id, SPELL_ID);
    let request = current_request(&fixture).expect("canonical request data");
    assert_eq!((request.health, request.mana), (32, 17));
}

#[tokio::test]
async fn resurrect_with_aura_accepts_valid_trigger_and_player_caster_name_is_empty() {
    let mut fixture = offer_fixture(571, "esES");
    install_spell(
        &mut fixture,
        SPELL_ID,
        RESURRECT_WITH_AURA,
        25,
        0,
        TRIGGER_ID,
        true,
        false,
    );

    let target = fixture.target;
    let caster = fixture.session_player;
    cast_offer(&mut fixture, SPELL_ID, target, caster).await;

    let decoded = decode_offer(
        &fixture
            .realm_rx
            .try_recv()
            .expect("valid trigger spell offers resurrection"),
    );
    assert_eq!(decoded.offerer, fixture.session_player);
    assert_eq!(decoded.name, "");
    assert!(!decoded.sickness);
    assert_eq!(decoded.pet_number, 0);
    assert_eq!(
        current_request(&fixture).expect("request").aura,
        TRIGGER_ID as u32
    );
}

#[tokio::test]
async fn resurrect_with_aura_uses_zero_for_missing_trigger_and_rejects_existing_aura() {
    let mut missing = offer_fixture(571, "enUS");
    install_spell(
        &mut missing,
        SPELL_ID,
        RESURRECT_WITH_AURA,
        25,
        0,
        TRIGGER_ID,
        false,
        false,
    );
    let missing_target = missing.target;
    let missing_caster = missing.caster;
    cast_offer(&mut missing, SPELL_ID, missing_target, missing_caster).await;
    let decoded = decode_offer(
        &missing
            .realm_rx
            .try_recv()
            .expect("missing trigger still sends a plain resurrection offer"),
    );
    assert_eq!(decoded.spell_id, SPELL_ID);
    assert_eq!(current_request(&missing).expect("request").aura, 0);

    let mut existing = offer_fixture(571, "enUS");
    existing
        .canonical
        .lock()
        .expect("canonical manager")
        .with_player_mut_like_cpp(existing.target_handle, |player| {
            player.unit_mut().subsystems_mut().auras.add_applied(
                wow_entities::AppliedAuraRef::new(TRIGGER_ID as u32, existing.caster, 0, 1),
            );
        })
        .expect("canonical target");
    install_spell(
        &mut existing,
        SPELL_ID,
        RESURRECT_WITH_AURA,
        25,
        0,
        TRIGGER_ID,
        true,
        false,
    );
    let existing_target = existing.target;
    let existing_caster = existing.caster;
    cast_offer(&mut existing, SPELL_ID, existing_target, existing_caster).await;
    assert!(existing.realm_rx.try_recv().is_err());
    assert!(current_request(&existing).is_none());
}

#[tokio::test]
async fn duplicate_offer_does_not_replace_or_resend_the_pending_request() {
    let mut fixture = offer_fixture(571, "enUS");
    install_spell(&mut fixture, SPELL_ID, RESURRECT, 40, 0, 0, false, false);
    let target = fixture.target;
    let caster = fixture.caster;

    cast_offer(&mut fixture, SPELL_ID, target, caster).await;
    let first_packet = fixture
        .realm_rx
        .try_recv()
        .expect("first offer reaches the target");
    let first_request = current_request(&fixture).expect("first request data");

    cast_offer(&mut fixture, SPELL_ID, target, caster).await;

    assert!(fixture.realm_rx.try_recv().is_err());
    assert_eq!(current_request(&fixture), Some(first_request));
    assert_eq!(decode_offer(&first_packet).offerer, fixture.caster);
}

#[tokio::test]
async fn resurrection_offers_reject_alive_pending_continuation_and_stale_incarnations() {
    let mut alive = offer_fixture(571, "enUS");
    alive
        .canonical
        .lock()
        .expect("canonical manager")
        .with_player_mut_like_cpp(alive.target_handle, |player| {
            player.unit_mut().set_health(120);
            player
                .unit_mut()
                .set_death_state(wow_constants::DeathState::Alive);
        })
        .expect("canonical target");
    install_spell(&mut alive, SPELL_ID, RESURRECT, 40, 0, 0, false, false);
    let alive_target = alive.target;
    let alive_caster = alive.caster;
    cast_offer(&mut alive, SPELL_ID, alive_target, alive_caster).await;
    assert!(alive.realm_rx.try_recv().is_err());
    assert!(current_request(&alive).is_none());

    let mut pending = offer_fixture(571, "enUS");
    let prior_request = wow_entities::PlayerResurrectionRequestLikeCpp {
        resurrecter: pending.caster,
        map_id: 571,
        position: Position::ZERO,
        health: 7,
        mana: 8,
        aura: 9,
    };
    pending
        .canonical
        .lock()
        .expect("canonical manager")
        .with_player_mut_like_cpp(pending.target_handle, |player| {
            player.set_resurrection_request_like_cpp(prior_request.clone());
        })
        .expect("canonical target");
    install_spell(&mut pending, SPELL_ID, RESURRECT, 40, 0, 0, false, false);
    let pending_target = pending.target;
    let pending_caster = pending.caster;
    cast_offer(&mut pending, SPELL_ID, pending_target, pending_caster).await;
    assert!(pending.realm_rx.try_recv().is_err());
    assert_eq!(current_request(&pending), Some(prior_request));

    let mut continuation = offer_fixture(571, "enUS");
    continuation
        .canonical
        .lock()
        .expect("canonical manager")
        .with_player_mut_like_cpp(continuation.target_handle, |player| {
            player
                .resurrection_state_mut_like_cpp()
                .delayed_after_teleport = Some(wow_entities::PlayerResurrectionContinuation {
                operation: wow_entities::PlayerResurrectionOperation::RequestData(
                    wow_entities::PlayerResurrectionRequestLikeCpp {
                        resurrecter: continuation.caster,
                        map_id: 571,
                        position: Position::ZERO,
                        health: 7,
                        mana: 8,
                        aura: 9,
                    },
                ),
                phase: wow_entities::PlayerResurrectionPhase::TeleportInFlight,
            });
        })
        .expect("canonical target");
    install_spell(
        &mut continuation,
        SPELL_ID,
        RESURRECT,
        40,
        0,
        0,
        false,
        false,
    );
    let continuation_target = continuation.target;
    let continuation_caster = continuation.caster;
    cast_offer(
        &mut continuation,
        SPELL_ID,
        continuation_target,
        continuation_caster,
    )
    .await;
    assert!(continuation.realm_rx.try_recv().is_err());
    assert!(current_request(&continuation).is_none());

    let mut stale = offer_fixture(571, "enUS");
    let replacement_handle = {
        let mut manager = stale.canonical.lock().expect("canonical manager");
        let mut replacement = wow_entities::Player::new(Some(1), false);
        replacement
            .unit_mut()
            .world_mut()
            .object_mut()
            .create(stale.target);
        replacement.unit_mut().set_health(0);
        replacement
            .unit_mut()
            .set_death_state(wow_constants::DeathState::JustDied);
        let handle = manager
            .install_detached_player_like_cpp(Box::new(replacement))
            .expect("replace canonical incarnation");
        manager
            .attach_player_like_cpp(handle, wow_map::MapKey::new(571, 0), stale.target_position)
            .expect("attach replacement incarnation");
        handle
    };
    stale.target_handle = replacement_handle;
    install_spell(&mut stale, SPELL_ID, RESURRECT, 40, 0, 0, false, false);
    let stale_target = stale.target;
    let stale_caster = stale.caster;
    cast_offer(&mut stale, SPELL_ID, stale_target, stale_caster).await;
    assert!(stale.realm_rx.try_recv().is_err());
    assert!(current_request(&stale).is_none());
}
