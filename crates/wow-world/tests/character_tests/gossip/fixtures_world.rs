use super::*;

pub(crate) fn insert_bank_test_player_in_world(
    session: &WorldSession,
    canonical: &Arc<Mutex<wow_map::MapManager>>,
) {
    let player_guid = session.player_guid().expect("player guid");
    let mut player = Player::new(Some(1), false);
    player.unit_mut().world_mut().object_mut().create(player_guid);
    player.unit_mut().world_mut().set_map(571, 0).unwrap();
    player
        .unit_mut()
        .world_mut()
        .relocate(Position::new(0.0, 0.0, 0.0, 0.0));
    player.unit_mut().world_mut().object_mut().add_to_world();
    canonical
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
}

pub(crate) fn gameobject_guid(entry: u32, counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, entry, counter)
}

pub(crate) fn insert_gossip_gameobject(
    manager: &Arc<Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
    entry: u32,
    position: Position,
    go_type: u8,
    is_in_world: bool,
) {
    let mut gameobject = GameObject::new();
    gameobject.world_mut().object_mut().create(guid);
    gameobject.world_mut().object_mut().set_entry(entry);
    gameobject.world_mut().set_map(571, 0).unwrap();
    gameobject.world_mut().relocate(position);
    gameobject.set_go_type(go_type);
    if is_in_world {
        gameobject.world_mut().object_mut().add_to_world();
    }
    manager
        .lock()
        .unwrap()
        .create_world_map(571, 0)
        .map_mut()
        .insert_map_object_record(MapObjectRecord::new_game_object(gameobject).unwrap())
        .unwrap();
}

pub(crate) fn seed_represented_feign_death_like_cpp(session: &mut WorldSession, slot: u8) {
    let player_guid = session.player_guid().expect("player guid");
    mutate_canonical_player_for_test(&*session, |player| {
            player
                .unit_mut()
                .add_unit_state(wow_constants::unit::UnitState::DIED.bits());
        })
        .expect("canonical player");
    insert_gossip_visible_aura_for_test(
        session,
        slot,
        AuraApplication {
            spell_id: 5384,
            difficulty_id: 0,
            caster_guid: player_guid,
            slot,
            duration_total: 0,
            duration_remaining: 0,
            stack_count: 1,
            aura_flags: 0,
            effect_mask: 1,
            aura_interrupt_flags: 0,
            aura_interrupt_flags2: 0,
            represented_effect: Some(RepresentedAuraEffectLikeCpp::FeignDeath),
            represented_amount: 0,
            represented_effect_amounts: Vec::new(),
            represented_misc_value: None,
            represented_multiplier: 1.0,
            applied_at: std::time::Instant::now(),
        },
    );
}

pub(crate) fn canonical_player_has_died_state_like_cpp(session: &mut WorldSession) -> bool {
    mutate_canonical_player_for_test(&*session, |player| {
            player
                .unit()
                .has_unit_state(wow_constants::unit::UnitState::DIED.bits())
        })
        .expect("canonical player")
}

pub(crate) fn drain_server_opcodes(send_rx: &flume::Receiver<Vec<u8>>) -> Vec<ServerOpcodes> {
    let mut opcodes = Vec::new();
    while let Ok(bytes) = send_rx.try_recv() {
        let packet = WorldPacket::from_bytes(&bytes);
        if let Some(opcode) = packet.server_opcode() {
            opcodes.push(opcode);
        }
    }
    opcodes
}

pub(crate) fn insert_binder_innkeeper(
    manager: &Arc<Mutex<wow_map::MapManager>>,
    guid: ObjectGuid,
) {
    insert_binder_creature(manager, guid, NPCFlags1::INNKEEPER.bits());
}

pub(crate) fn install_binder_spell_fixture(session: &mut WorldSession) {
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        3286,
        wow_data::SpellInfo {
            spell_id: 3286,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
}
