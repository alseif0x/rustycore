// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `player.spell_cast` (#1444 H3) through the real spell execution:
//! TrinityCore `Spell::_cast` -> `OnPlayerSpellCast` (`Spell.cpp:3671`).
//! Raised once per player cast, after `SMSG_SPELL_GO`, before the effects.

use super::*;

#[derive(Default)]
struct CastProbe {
    seen: std::sync::Mutex<Vec<(u32, ObjectGuid, bool)>>,
}

impl wow_module_api::PlayerModule for CastProbe {
    fn on_spell_cast(
        &self,
        s: &wow_module_api::SpellCastSnapshot,
        e: &mut wow_module_api::ScopedEffects<'_>,
    ) {
        self.seen
            .lock()
            .unwrap()
            .push((s.spell_id, s.target, s.triggered));
        e.send_system_message_self("cast");
    }
}

fn install_cast_probe(session: &mut WorldSession) -> Arc<CastProbe> {
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
    probe
}

fn plain_spell(spell_id: i32) -> wow_data::SpellInfo {
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    }
}

#[tokio::test]
async fn a_player_cast_raises_spell_cast_once_after_spell_go() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 44_301_i32;
    let player_guid = ObjectGuid::create_player(1, 4901);
    session.set_player_guid(Some(player_guid));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, plain_spell(spell_id));
    session.set_spell_store(Arc::new(spell_store));
    let probe = install_cast_probe(&mut session);

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("the spell executes");

    assert_eq!(
        *probe.seen.lock().unwrap(),
        vec![(spell_id as u32, player_guid, false)]
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::SpellGo,
            ServerOpcodes::Chat,
            ServerOpcodes::CooldownEvent
        ],
        "the module's message follows SMSG_SPELL_GO and precedes the effects"
    );
}

#[tokio::test]
async fn zero_modules_leave_the_cast_packets_unchanged() {
    let (mut session, _, send_rx) = make_session();
    let spell_id = 44_302_i32;
    let player_guid = ObjectGuid::create_player(1, 4902);
    session.set_player_guid(Some(player_guid));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(spell_id, plain_spell(spell_id));
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell(spell_id, player_guid)
        .await
        .expect("the spell executes");

    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::SpellGo, ServerOpcodes::CooldownEvent]
    );
}
