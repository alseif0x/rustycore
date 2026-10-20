// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! `player.spell_cast` (#1444 H3a review) through real represented casts:
//! raised once SMSG_SPELL_GO is out and before the effects apply, with no map
//! guard held; a cast that fails before SPELL_GO and a creature-caster
//! override raise nothing.

use super::*;

struct CastProbe {
    map: SharedCanonicalMapManager,
    /// `(spell_id, map lock free, GameObjects on the map)` at each callback.
    seen: std::sync::Mutex<Vec<(u32, bool, usize)>>,
}

impl wow_module_api::PlayerModule for CastProbe {
    fn enabled_hooks(&self) -> &'static [wow_module_api::PlayerHook] {
        &[wow_module_api::PlayerHook::SpellCast]
    }
    fn on_spell_cast(
        &self,
        s: &wow_module_api::SpellCastSnapshot,
        _e: &mut wow_module_api::ScopedEffects<'_>,
    ) {
        let (free, objects) = match self.map.try_lock() {
            Ok(manager) => (
                true,
                manager
                    .find_map(571, 0)
                    .map_or(0, |managed| managed.map().map_object_count()),
            ),
            Err(_) => (false, 0),
        };
        self.seen.lock().unwrap().push((s.spell_id, free, objects));
    }
}

fn install_cast_probe(
    session: &mut WorldSession,
    map: &SharedCanonicalMapManager,
) -> Arc<CastProbe> {
    let probe = Arc::new(CastProbe {
        map: Arc::clone(map),
        seen: std::sync::Mutex::new(Vec::new()),
    });
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

/// A GameObject summon at the caster (TARGET_DEST_DB with no row): its
/// effect adds one map object, so the probe sees whether it already ran.
fn configure_summon_cast(
    session: &mut WorldSession,
    canonical: &SharedCanonicalMapManager,
    spell_id: i32,
    template_entry: u32,
    player_guid: ObjectGuid,
    player_position: Position,
) {
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 = wow_data::spell::implicit_targets::TARGET_DEST_DB;
    configure_gameobject_summon_live_session_like_cpp(
        session,
        canonical,
        player_guid,
        player_position,
        summon_go_template_store_like_cpp(template_entry),
        gameobject_summon_spell_info_like_cpp(spell_id, 0, vec![summon_effect]),
    );
}

fn map_object_count(canonical: &SharedCanonicalMapManager) -> usize {
    canonical
        .lock()
        .unwrap()
        .find_map(571, 0)
        .map_or(0, |managed| managed.map().map_object_count())
}

#[tokio::test]
async fn spell_cast_runs_after_spell_go_before_the_effects_with_no_map_lock() {
    let (mut session, _, _) = make_session();
    let spell_id = 86_951_i32;
    let player_guid = ObjectGuid::create_player(1, 7951);
    let canonical = shared_canonical_map_manager();
    configure_summon_cast(
        &mut session,
        &canonical,
        spell_id,
        9051,
        player_guid,
        Position::new(240.0, 340.0, 48.0, 0.25),
    );
    let probe = install_cast_probe(&mut session, &canonical);
    let before = map_object_count(&canonical);

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData::default(),
        )
        .await
        .expect("the summon cast executes");

    assert_eq!(
        map_object_count(&canonical),
        before + 1,
        "the summon effect ran"
    );
    assert_eq!(
        *probe.seen.lock().unwrap(),
        vec![(spell_id as u32, true, before)],
        "one dispatch, map guard free, before the effect added its GameObject"
    );
}

#[tokio::test]
async fn a_cast_that_fails_before_spell_go_raises_no_spell_cast() {
    let (mut session, _, _) = make_session();
    let spell_id = 86_952_i32;
    let template_entry = 9052_u32;
    let player_guid = ObjectGuid::create_player(1, 7952);
    let canonical = shared_canonical_map_manager();
    let mut summon_effect =
        summon_object_wild_effect_like_cpp(i32::try_from(template_entry).unwrap());
    summon_effect.implicit_target_1 = wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY;
    configure_gameobject_summon_live_session_like_cpp(
        &mut session,
        &canonical,
        player_guid,
        Position::new(340.0, 440.0, 63.0, 0.0),
        summon_go_template_store_like_cpp(template_entry),
        gameobject_summon_spell_info_like_cpp(spell_id, 0, vec![summon_effect]),
    );
    let mut misc = summon_go_spell_misc_entry_like_cpp(spell_id as u32, 0);
    misc.range_index = 89;
    session.set_spell_misc_store(Arc::new(wow_data::SpellMiscStore::from_entries([misc])));
    session.set_spell_range_store(Arc::new(wow_data::SpellRangeStore::from_entries([
        wow_data::SpellRangeEntry {
            id: 89,
            display_name: String::new(),
            display_name_short: String::new(),
            flags: 0,
            range_min: [0.0, 0.0],
            range_max: [30.0, 30.0],
        },
    ])));
    let probe = install_cast_probe(&mut session, &canonical);

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData::default(),
        )
        .await
        .expect("BAD_IMPLICIT_TARGETS is reported as a cast failure");

    assert!(
        probe.seen.lock().unwrap().is_empty(),
        "a cast that never reached SMSG_SPELL_GO is not observed"
    );
}

#[tokio::test]
async fn a_creature_caster_override_raises_no_spell_cast() {
    let (mut session, _, _) = make_session();
    let spell_id = 86_953_i32;
    let player_guid = ObjectGuid::create_player(1, 7953);
    let creature_guid = test_creature_guid(7954);
    let canonical = shared_canonical_map_manager();
    configure_summon_cast(
        &mut session,
        &canonical,
        spell_id,
        9053,
        player_guid,
        Position::new(240.0, 340.0, 48.0, 0.25),
    );
    add_canonical_test_creature_on_map(
        &canonical,
        creature_guid,
        9054,
        Position::new(246.0, 344.0, 49.0, 1.75),
        0,
        571,
        0,
    );
    let probe = install_cast_probe(&mut session, &canonical);

    session
        .execute_spell_with_visual_and_target_data_with_metadata(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData::default(),
            SpellCastMetadata {
                caster_guid_override: Some(creature_guid),
                ..SpellCastMetadata::default()
            },
        )
        .await
        .expect("the creature cast executes");

    assert!(
        probe.seen.lock().unwrap().is_empty(),
        "C++ OnPlayerSpellCast is raised only for a Player caster"
    );
}
