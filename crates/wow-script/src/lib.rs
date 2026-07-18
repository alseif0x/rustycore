//! Script hook registry surfaces ported from TrinityCore `ScriptMgr`.
//!
//! This crate intentionally starts small. It provides the common registration
//! and dispatch mechanics that content crates can use while the concrete script
//! families are ported incrementally from C++.

pub mod player {
    use wow_core::ObjectGuid;

    /// Stable represented identity passed to `PlayerScript::OnGiveXP` hooks.
    ///
    /// C++ passes mutable `Player*`/`Unit*` objects. The current Rust script
    /// boundary exposes their GUID identities and the mutable XP amount, which
    /// is sufficient for the bundled `xp_boost_PlayerScript`. Hooks that need
    /// richer Player/Unit state remain part of the wider PlayerScript port.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GivePlayerXpContextLikeCpp {
        pub player_guid: ObjectGuid,
        /// `ObjectGuid::EMPTY` represents C++ `victim == nullptr`.
        pub victim_guid: ObjectGuid,
    }

    /// Registered `PlayerScript::OnGiveXP` callback.
    pub struct GivePlayerXpHookLikeCpp {
        pub name: &'static str,
        pub callback: fn(GivePlayerXpContextLikeCpp, &mut u32),
    }

    inventory::collect!(GivePlayerXpHookLikeCpp);

    /// Summary for one `ScriptMgr::OnGivePlayerXP` dispatch pass.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct GivePlayerXpDispatchSummaryLikeCpp {
        pub callbacks: usize,
    }

    /// Dispatch every registered callback over the same mutable amount, like
    /// C++ `FOREACH_SCRIPT(PlayerScript)->OnGiveXP(player, amount, victim)`.
    ///
    /// There is intentionally no registration-order contract here. Both C++
    /// references store code-only `PlayerScript` objects in an
    /// `unordered_multimap` and iterate that container directly
    /// (`ScriptMgr.cpp:1089-1100,1166-1186,1987-1990`); `inventory` likewise
    /// documents arbitrary iteration order. The current C++ trees contain one
    /// mutating `OnGiveXP` override (`xp_boost_PlayerScript`), so ordering is
    /// not observable. If another mutating override is ported, its C++ runtime
    /// order needs capture evidence rather than an invented name/loader sort.
    pub fn on_give_player_xp_like_cpp(
        context: GivePlayerXpContextLikeCpp,
        amount: &mut u32,
    ) -> GivePlayerXpDispatchSummaryLikeCpp {
        let mut callbacks = 0;
        for hook in inventory::iter::<GivePlayerXpHookLikeCpp> {
            let _name = hook.name;
            (hook.callback)(context, amount);
            callbacks += 1;
        }
        GivePlayerXpDispatchSummaryLikeCpp { callbacks }
    }
}

pub mod lifecycle {
    /// Lifecycle hook kind matching the worldserver-level `ScriptMgr` callbacks.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum LifecycleHookKindLikeCpp {
        Startup,
        Shutdown,
    }

    /// Summary for one lifecycle dispatch pass.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct LifecycleDispatchSummaryLikeCpp {
        pub hook: LifecycleHookKindLikeCpp,
        pub callbacks: usize,
    }

    /// Registered `sScriptMgr->OnStartup()` callback.
    ///
    /// C++ calls this after the realm is marked online and the freeze detector
    /// is armed. Rust keeps the same worldserver dispatch point and lets content
    /// crates register callbacks through `inventory::submit!`.
    pub struct StartupHookLikeCpp {
        pub name: &'static str,
        pub callback: fn(),
    }

    /// Registered `sScriptMgr->OnShutdown()` callback.
    ///
    /// C++ calls this during shutdown after network/threadpool teardown and
    /// before the realm is marked offline.
    pub struct ShutdownHookLikeCpp {
        pub name: &'static str,
        pub callback: fn(),
    }

    inventory::collect!(StartupHookLikeCpp);
    inventory::collect!(ShutdownHookLikeCpp);

    /// Dispatch all registered startup callbacks like `ScriptMgr::OnStartup`.
    pub fn on_startup_like_cpp() -> LifecycleDispatchSummaryLikeCpp {
        let mut callbacks = 0;
        for hook in inventory::iter::<StartupHookLikeCpp> {
            let _name = hook.name;
            (hook.callback)();
            callbacks += 1;
        }
        LifecycleDispatchSummaryLikeCpp {
            hook: LifecycleHookKindLikeCpp::Startup,
            callbacks,
        }
    }

    /// Dispatch all registered shutdown callbacks like `ScriptMgr::OnShutdown`.
    pub fn on_shutdown_like_cpp() -> LifecycleDispatchSummaryLikeCpp {
        let mut callbacks = 0;
        for hook in inventory::iter::<ShutdownHookLikeCpp> {
            let _name = hook.name;
            (hook.callback)();
            callbacks += 1;
        }
        LifecycleDispatchSummaryLikeCpp {
            hook: LifecycleHookKindLikeCpp::Shutdown,
            callbacks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::lifecycle::{
        LifecycleDispatchSummaryLikeCpp, LifecycleHookKindLikeCpp, ShutdownHookLikeCpp,
        StartupHookLikeCpp, on_shutdown_like_cpp, on_startup_like_cpp,
    };
    use super::player::{
        GivePlayerXpContextLikeCpp, GivePlayerXpHookLikeCpp, on_give_player_xp_like_cpp,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use wow_core::ObjectGuid;

    static STARTUP_CALLS: AtomicUsize = AtomicUsize::new(0);
    static SHUTDOWN_CALLS: AtomicUsize = AtomicUsize::new(0);

    fn add_three_xp_like_cpp(context: GivePlayerXpContextLikeCpp, amount: &mut u32) {
        if context.player_guid == ObjectGuid::create_player(1, 101)
            && context.victim_guid == ObjectGuid::create_player(1, 202)
        {
            *amount += 3;
        }
    }

    fn add_seven_xp_like_cpp(context: GivePlayerXpContextLikeCpp, amount: &mut u32) {
        if context.player_guid == ObjectGuid::create_player(1, 101)
            && context.victim_guid == ObjectGuid::create_player(1, 202)
        {
            *amount += 7;
        }
    }

    fn record_startup_like_cpp() {
        STARTUP_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    fn record_shutdown_like_cpp() {
        SHUTDOWN_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    inventory::submit! {
        StartupHookLikeCpp {
            name: "test_startup_like_cpp",
            callback: record_startup_like_cpp,
        }
    }

    inventory::submit! {
        ShutdownHookLikeCpp {
            name: "test_shutdown_like_cpp",
            callback: record_shutdown_like_cpp,
        }
    }

    inventory::submit! {
        GivePlayerXpHookLikeCpp {
            name: "test_add_three_xp_like_cpp",
            callback: add_three_xp_like_cpp,
        }
    }

    inventory::submit! {
        GivePlayerXpHookLikeCpp {
            name: "test_add_seven_xp_like_cpp",
            callback: add_seven_xp_like_cpp,
        }
    }

    #[test]
    fn lifecycle_dispatch_runs_registered_callbacks_like_cpp() {
        let startup_before = STARTUP_CALLS.load(Ordering::SeqCst);
        let startup_summary = on_startup_like_cpp();
        assert_eq!(
            startup_summary,
            LifecycleDispatchSummaryLikeCpp {
                hook: LifecycleHookKindLikeCpp::Startup,
                callbacks: 1,
            }
        );
        assert_eq!(STARTUP_CALLS.load(Ordering::SeqCst), startup_before + 1);

        let shutdown_before = SHUTDOWN_CALLS.load(Ordering::SeqCst);
        let shutdown_summary = on_shutdown_like_cpp();
        assert_eq!(
            shutdown_summary,
            LifecycleDispatchSummaryLikeCpp {
                hook: LifecycleHookKindLikeCpp::Shutdown,
                callbacks: 1,
            }
        );
        assert_eq!(SHUTDOWN_CALLS.load(Ordering::SeqCst), shutdown_before + 1);
    }

    #[test]
    fn give_player_xp_dispatch_shares_mutable_amount_without_inventing_order() {
        let mut amount = 10;
        let summary = on_give_player_xp_like_cpp(
            GivePlayerXpContextLikeCpp {
                player_guid: ObjectGuid::create_player(1, 101),
                victim_guid: ObjectGuid::create_player(1, 202),
            },
            &mut amount,
        );

        assert_eq!(summary.callbacks, 2);
        assert_eq!(amount, 20);
    }
}
