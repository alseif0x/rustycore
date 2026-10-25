use super::*;
use std::time::Instant;
use wow_core::{ObjectGuid, Position};
use wow_entities::{AuraApplicationLikeCpp as AuraApplication, Player};

const MAP_ID: u32 = 571;

fn manager() -> MapManager {
    let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 10);
    manager.create_world_map(MAP_ID, 0);
    manager
}

fn add_player(manager: &mut MapManager, counter: i64, pending_spell: Option<i32>) -> ObjectGuid {
    let guid = ObjectGuid::create_player(1, counter);
    let mut player = Box::new(Player::new(Some(1), false));
    player.unit_mut().world_mut().object_mut().create(guid);
    player.unit_mut().set_level(80);
    if let Some(spell_id) = pending_spell {
        let slot = 0;
        player
            .unit_mut()
            .subsystems_mut()
            .auras
            .insert_runtime_application_like_cpp(AuraApplication {
                spell_id,
                difficulty_id: 0,
                caster_guid: guid,
                slot,
                duration_total: 0,
                duration_remaining: 0,
                stack_count: 1,
                aura_flags: 1,
                effect_mask: 1,
                aura_interrupt_flags: 0,
                aura_interrupt_flags2: 0,
                represented_effect: None,
                represented_amount: 0,
                represented_effect_amounts: Vec::new(),
                represented_misc_value: None,
                represented_multiplier: 1.0,
                applied_at: Instant::now(),
            });
        player
            .unit_mut()
            .subsystems_mut()
            .auras
            .visible_auras_to_update
            .insert(slot);
    }
    let handle = manager.install_detached_player_like_cpp(player).unwrap();
    manager
        .attach_player_like_cpp(handle, MapKey::new(MAP_ID, 0), Position::ZERO)
        .unwrap();
    guid
}

fn has_pending(manager: &MapManager, guid: ObjectGuid) -> bool {
    let (handle, _) = manager.current_player_admission_like_cpp(guid).unwrap();
    manager
        .with_player_like_cpp(handle, |player| {
            !player
                .unit()
                .subsystems()
                .auras
                .visible_auras_to_update
                .is_empty()
        })
        .unwrap()
}

#[test]
fn admitted_map_plan_collects_all_players_and_consumes_each_delta_once() {
    let mut manager = manager();
    let attacker = add_player(&mut manager, 8_410_001, Some(8_326));
    let non_attacker = add_player(&mut manager, 8_410_002, Some(20_584));
    let plan = manager
        .begin_tick_like_cpp(100)
        .into_started()
        .expect("admitted map tick");

    let updates = collect_pending_selected_aura_updates(&mut manager, &plan);
    assert_eq!(updates.len(), 2, "all map Players are considered");
    assert!(!has_pending(&manager, attacker));
    assert!(!has_pending(&manager, non_attacker));
    assert!(collect_pending_selected_aura_updates(&mut manager, &plan).is_empty());
}

#[test]
fn refused_epoch_preserves_pending_aura_slots() {
    let mut plan_manager = manager();
    let plan = plan_manager
        .begin_tick_like_cpp(100)
        .into_started()
        .expect("admitted source plan");
    let mut current_manager = manager();
    let player = add_player(&mut current_manager, 8_410_003, Some(8_326));

    assert!(collect_pending_selected_aura_updates(&mut current_manager, &plan).is_empty());
    let first = current_manager
        .begin_tick_like_cpp(100)
        .into_started()
        .expect("first current-manager tick");
    current_manager.abandon_tick_like_cpp(first);
    let _second = current_manager
        .begin_tick_like_cpp(100)
        .into_started()
        .expect("different current-manager epoch");
    assert!(collect_pending_selected_aura_updates(&mut current_manager, &plan).is_empty());
    assert!(has_pending(&current_manager, player));
}

#[test]
fn replaced_map_incarnation_cannot_consume_new_players_pending_aura() {
    let mut manager = manager();
    let key = MapKey::new(MAP_ID, 0);
    let plan = manager
        .begin_tick_like_cpp(100)
        .into_started()
        .expect("admitted source map plan");
    let old_incarnation = manager.map_incarnation_like_cpp(key).unwrap();
    assert!(manager.destroy_map(MAP_ID, 0));
    manager.create_world_map(MAP_ID, 0);
    assert_ne!(manager.map_incarnation_like_cpp(key), Some(old_incarnation));
    let replacement = add_player(&mut manager, 8_410_004, Some(20_584));

    assert!(collect_pending_selected_aura_updates(&mut manager, &plan).is_empty());
    assert!(has_pending(&manager, replacement));
}
