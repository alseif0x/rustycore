//! The example module through a real `ModuleRegistry`: configuration, the
//! greeting, milestones, the quest counter, the chat veto, per-hook interest
//! and the cross-family announcement.

use std::collections::BTreeMap;

use wow_module_api::{
    ChatDecision, ChatKind, ChatMessageSnapshot, CreatureDiedSnapshot, CreatureKillSnapshot,
    CreatureLocation, HookFamily, LogoutReason, ModuleConfig, ModuleConfigValue, ModuleId,
    ModuleRegistry, NotificationEffect, ObjectGuid, PlayerHook, PlayerLevelChangedSnapshot,
    PlayerLoginSnapshot, PlayerLogoutSnapshot, QuestRewardedSnapshot, WorldEffect,
    WorldUpdateSnapshot,
};

fn config(pairs: &[(&str, ModuleConfigValue)]) -> ModuleConfig {
    ModuleConfig::new(
        &ModuleId::new("example.milestones").expect("valid id"),
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), v.clone()))
            .collect::<BTreeMap<_, _>>(),
    )
}

fn module(
    pairs: &[(&str, ModuleConfigValue)],
) -> (
    ModuleRegistry,
    std::sync::Arc<example_milestones::Milestones>,
) {
    let module = example_milestones::build_for_test(config(pairs))
        .expect("valid configuration")
        .expect("enabled");
    let mut registry = ModuleRegistry::new();
    example_milestones::register_with(&mut registry, module.clone()).expect("registration");
    (registry, module)
}

fn kill(player: ObjectGuid) -> CreatureKillSnapshot {
    CreatureKillSnapshot::new(player, ObjectGuid::EMPTY, 299, 5, 0)
}

#[test]
fn it_declares_only_the_player_hooks_it_implements() {
    let (registry, _) = module(&[]);
    assert!(registry.handles(HookFamily::Player));
    assert!(registry.handles(HookFamily::World));
    assert!(registry.handles(HookFamily::Creature));
    assert!(registry.handles(HookFamily::PlayerLogin));
    assert!(registry.handles_player_hook(PlayerHook::LevelChanged));
    assert!(registry.handles_player_hook(PlayerHook::QuestRewarded));
    assert!(registry.handles_player_hook(PlayerHook::CreatureKill));
    assert!(registry.handles_player_hook(PlayerHook::Logout));
    assert!(!registry.handles_player_hook(PlayerHook::SpellCast));
    assert!(!registry.handles_player_hook(PlayerHook::LootMoney));
    assert!(!registry.handles_player_hook(PlayerHook::QuestAccepted));
    assert!(
        !registry.handles(HookFamily::Chat),
        "no blocked word: the chat family is not registered"
    );
}

#[test]
fn every_level_step_is_congratulated() {
    let (registry, _) = module(&[("level_step", ModuleConfigValue::Integer(5))]);
    let player = ObjectGuid::create_player(1, 1);
    let at = |level: u8| {
        registry
            .dispatch_player_level_changed(&PlayerLevelChangedSnapshot::new(
                player,
                level - 1,
                level,
            ))
            .effects
            .expect("valid")
            .len()
    };
    assert_eq!(at(4), 0);
    assert_eq!(at(5), 1);
    assert_eq!(at(6), 0);
    assert_eq!(at(10), 1);
}

#[test]
fn kills_are_counted_per_player_and_forgotten_on_logout() {
    let (registry, module) = module(&[("kill_milestone", ModuleConfigValue::Integer(3))]);
    let alice = ObjectGuid::create_player(1, 1);
    let bob = ObjectGuid::create_player(1, 2);
    let mut messages = Vec::new();
    for player in [alice, alice, bob, alice] {
        messages.push(
            registry
                .dispatch_creature_kill(&kill(player))
                .effects
                .expect("valid")
                .len(),
        );
    }
    assert_eq!(
        messages,
        vec![0, 0, 0, 1],
        "alice's third kill is the milestone"
    );
    assert_eq!(module.kills_of(alice), 3);
    assert_eq!(module.kills_of(bob), 1);

    let _ = registry.dispatch_player_logout(&PlayerLogoutSnapshot::new(
        alice,
        1,
        LogoutReason::CharacterSelection,
    ));
    assert_eq!(module.kills_of(alice), 0);
    assert_eq!(module.kills_of(bob), 1);
}

#[test]
fn a_level_milestone_is_announced_on_the_next_world_update_when_enabled() {
    let (registry, _) = module(&[("announce_levels", ModuleConfigValue::Bool(true))]);
    let player = ObjectGuid::create_player(1, 1);
    let _ =
        registry.dispatch_player_level_changed(&PlayerLevelChangedSnapshot::new(player, 19, 20));

    let first = registry
        .dispatch_world_update(&WorldUpdateSnapshot::new(50, 1))
        .effects
        .expect("valid");
    let announced: Vec<_> = first
        .iter()
        .filter(|(_, effect)| matches!(effect, WorldEffect::AnnounceWorld { .. }))
        .collect();
    assert_eq!(announced.len(), 1);
    let second = registry
        .dispatch_world_update(&WorldUpdateSnapshot::new(50, 2))
        .effects
        .expect("valid");
    assert!(second.is_empty(), "each milestone is announced once");
}

#[test]
fn creature_deaths_are_counted() {
    let (registry, module) = module(&[]);
    let location = CreatureLocation::new(0, 0, 0.0, 0.0, 0.0, 0.0);
    for _ in 0..3 {
        let _ = registry.dispatch_creature_died(&CreatureDiedSnapshot::new(
            ObjectGuid::EMPTY,
            299,
            location,
        ));
    }
    assert_eq!(module.creature_deaths(), 3);
}

#[test]
fn invalid_or_disabled_configuration() {
    for bad in [
        vec![("level_step", ModuleConfigValue::Integer(0))],
        vec![("level_step", ModuleConfigValue::Integer(256))],
        vec![("kill_milestone", ModuleConfigValue::Integer(0))],
        vec![("kill_milestone", ModuleConfigValue::Integer(-1))],
        vec![("kill_milestone", ModuleConfigValue::Text("ten".to_owned()))],
        vec![("kill_step", ModuleConfigValue::Integer(10))],
        vec![("level_message", ModuleConfigValue::Text("  ".to_owned()))],
        vec![("level_message", ModuleConfigValue::Text("a\tb".to_owned()))],
        vec![("level_message", ModuleConfigValue::Text("x".repeat(201)))],
        vec![("level_message", ModuleConfigValue::Integer(1))],
        vec![(
            "blocked_words",
            ModuleConfigValue::Text("two words".to_owned()),
        )],
        vec![("blocked_words", ModuleConfigValue::Bool(true))],
    ] {
        let mut registry = ModuleRegistry::new();
        assert!(
            example_milestones::register(&mut registry, config(&bad)).is_err(),
            "{bad:?} must prevent activation"
        );
        assert!(registry.is_empty());
    }
    let mut registry = ModuleRegistry::new();
    example_milestones::register(
        &mut registry,
        config(&[("enabled", ModuleConfigValue::Bool(false))]),
    )
    .expect("a disabled module registers nothing");
    assert!(registry.is_empty());
}

#[test]
fn a_kill_milestone_is_logged_on_the_next_world_update() {
    let (registry, _) = module(&[("kill_milestone", ModuleConfigValue::Integer(1))]);
    let player = ObjectGuid::create_player(1, 1);
    let _ = registry.dispatch_creature_kill(&kill(player));
    let effects = registry
        .dispatch_world_update(&WorldUpdateSnapshot::new(50, 1))
        .effects
        .expect("valid");
    let logs: Vec<_> = effects
        .iter()
        .filter(|(_, effect)| matches!(effect, WorldEffect::Log { .. }))
        .collect();
    assert_eq!(logs.len(), 1);
    assert!(
        registry
            .dispatch_world_update(&WorldUpdateSnapshot::new(50, 2))
            .effects
            .expect("valid")
            .is_empty()
    );
}

fn texts(effects: &wow_module_api::EffectBatch<NotificationEffect>) -> Vec<String> {
    effects
        .iter()
        .map(|(_, NotificationEffect::SendSystemMessageSelf { text })| text.clone())
        .collect()
}

fn login(player: ObjectGuid, first_login: bool) -> PlayerLoginSnapshot {
    PlayerLoginSnapshot {
        guid: player,
        name: "Testbot".to_owned(),
        race: 1,
        class: 1,
        level: 1,
        map_id: 0,
        first_login,
    }
}

#[test]
fn a_login_is_greeted_once() {
    let (registry, _) = module(&[("kill_milestone", ModuleConfigValue::Integer(7))]);
    let player = ObjectGuid::create_player(1, 1);
    let greeting = registry
        .dispatch_player_login(&login(player, false))
        .expect("valid");
    assert_eq!(
        texts(&greeting),
        vec!["Welcome back, Testbot! A milestone every 7 kills.".to_owned()]
    );
    let first = registry
        .dispatch_player_login(&login(player, true))
        .expect("valid");
    assert_eq!(
        texts(&first),
        vec!["Welcome, Testbot! A milestone every 7 kills.".to_owned()]
    );
}

#[test]
fn the_level_message_replaces_the_level_placeholder() {
    let (registry, _) = module(&[(
        "level_message",
        ModuleConfigValue::Text("Level {level}, well done".to_owned()),
    )]);
    let player = ObjectGuid::create_player(1, 1);
    let effects = registry
        .dispatch_player_level_changed(&PlayerLevelChangedSnapshot::new(player, 1, 2))
        .effects
        .expect("valid");
    assert_eq!(texts(&effects), vec!["Level 2, well done".to_owned()]);
    let default = module(&[]).0;
    let effects = default
        .dispatch_player_level_changed(&PlayerLevelChangedSnapshot::new(player, 2, 3))
        .effects
        .expect("valid");
    assert_eq!(
        texts(&effects),
        vec!["Milestone reached: level 3!".to_owned()],
        "the default step is every level"
    );
}

#[test]
fn quests_rewarded_are_counted_per_player_and_forgotten_on_logout() {
    let (registry, module) = module(&[]);
    let alice = ObjectGuid::create_player(1, 1);
    let bob = ObjectGuid::create_player(1, 2);
    let reward = |player, quest_id| {
        registry
            .dispatch_quest_rewarded(&QuestRewardedSnapshot::new(
                player,
                quest_id,
                ObjectGuid::EMPTY,
                0,
                100,
                0,
                0,
            ))
            .effects
            .expect("valid")
    };
    assert_eq!(
        texts(&reward(alice, 10)),
        vec!["Quest 10 done: 1 this session.".to_owned()]
    );
    assert_eq!(
        texts(&reward(alice, 11)),
        vec!["Quest 11 done: 2 this session.".to_owned()]
    );
    assert_eq!(
        texts(&reward(bob, 10)),
        vec!["Quest 10 done: 1 this session.".to_owned()]
    );
    assert_eq!(module.quests_rewarded_of(alice), 2);

    let _ = registry.dispatch_player_logout(&PlayerLogoutSnapshot::new(
        alice,
        1,
        LogoutReason::CharacterSelection,
    ));
    assert_eq!(module.quests_rewarded_of(alice), 0);
    assert_eq!(module.quests_rewarded_of(bob), 1);
}

fn say(text: &str) -> ChatMessageSnapshot {
    ChatMessageSnapshot::new(
        ObjectGuid::create_player(1, 1),
        ChatKind::Say,
        0,
        text,
        None,
    )
}

#[test]
fn a_message_with_a_blocked_word_is_denied() {
    let (registry, _) = module(&[(
        "blocked_words",
        ModuleConfigValue::Text(" Spoiler, gold4sale ,".to_owned()),
    )]);
    assert!(registry.handles(HookFamily::Chat));
    for denied in [
        "no spoiler please",
        "SPOILER!",
        "buy gold4sale now",
        "(spoiler)",
    ] {
        let outcome = registry.dispatch_chat_message(&say(denied));
        assert_eq!(outcome.decision, ChatDecision::Deny, "{denied:?}");
        assert_eq!(
            outcome.denied_by.as_ref().map(ModuleId::as_str),
            Some("example.milestones")
        );
    }
    for allowed in ["hello there", "spoilers are fine", "gold4", ""] {
        let outcome = registry.dispatch_chat_message(&say(allowed));
        assert_eq!(outcome.decision, ChatDecision::Allow, "{allowed:?}");
        assert!(outcome.denied_by.is_none());
    }
}
