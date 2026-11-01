// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `player.spell_cast` (#1444 H3a review): the innkeeper's bind spell 3286
//! is a creature `CastSpell` on the player (`HandleBinderActivateOpcode`), so
//! C++ `OnPlayerSpellCast` is not raised for it.

use super::*;

#[derive(Default)]
struct CastProbe {
    seen: std::sync::Mutex<Vec<u32>>,
}

impl wow_module_api::PlayerModule for CastProbe {
    fn on_spell_cast(
        &self,
        s: &wow_module_api::SpellCastSnapshot,
        _e: &mut wow_module_api::ScopedEffects<'_>,
    ) {
        self.seen.lock().unwrap().push(s.spell_id);
    }
}

#[tokio::test]
async fn the_innkeeper_bind_cast_raises_no_spell_cast() {
    let (mut session, instance_rx, canonical) = make_bank_slot_session(16);
    insert_bank_test_player_in_world(&session, &canonical);
    assert!(session.adopt_registered_canonical_player_fixture_like_cpp());
    assert!(
        crate::canonical_player_access::configure_canonical_player_vitals_for_test(
            &canonical,
            session.player_guid().expect("loaded player"),
            (100, 100, wow_constants::PowerType::Mana, 100, 100, 100),
        )
    );
    session.set_player_faction_template_like_cpp(1);
    let homebind_port =
        HomebindPortFixtureLikeCpp::new([PersistenceOutcomeLikeCpp::Applied { rows: 1 }]);
    session.set_player_lifecycle_port_like_cpp(homebind_port);
    let (realm_tx, _realm_rx) = flume::bounded::<Vec<u8>>(16);
    session.install_realm_send_channel_for_test(realm_tx);
    let innkeeper = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 2456, 32);
    insert_banker_creature(&canonical, innkeeper, NPCFlags1::INNKEEPER.bits());
    crate::session::hub_mut(&mut session).set_player_zone_area_like_cpp(12, 34);
    install_bind_spell_fixture(&mut session);
    session.set_player_trainer_interaction_like_cpp(innkeeper, 77);
    let probe = Arc::new(CastProbe::default());
    let mut registry = wow_module_api::ModuleRegistry::new();
    registry
        .register_module(
            wow_module_api::ModuleDescriptor::new(
                "alpha.casts",
                wow_module_api::ModuleVersion {
                    major: 1,
                    minor: 0,
                    patch: 0,
                },
                "Casts",
            )
            .unwrap(),
            wow_module_api::ModuleHooks::new().player(Arc::clone(&probe) as _),
        )
        .unwrap();
    session.install_module_hooks_like_cpp(Arc::new(registry));

    session
        .handle_binder_activate(Hello { unit: innkeeper })
        .await;

    let opcodes: Vec<_> = instance_rx
        .try_iter()
        .filter_map(|bytes| WorldPacket::from_bytes(&bytes).server_opcode())
        .collect();
    assert!(
        opcodes.contains(&ServerOpcodes::SpellGo),
        "the bind spell was cast: {opcodes:?}"
    );
    assert!(
        probe.seen.lock().unwrap().is_empty(),
        "a creature-cast bind spell raises no player.spell_cast"
    );
}
