//! Script hook registry surfaces ported from TrinityCore `ScriptMgr`.
//!
//! This crate intentionally starts small. It provides the common registration
//! and dispatch mechanics that content crates can use while the concrete script
//! families are ported incrementally from C++.

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
    use std::sync::atomic::{AtomicUsize, Ordering};

    static STARTUP_CALLS: AtomicUsize = AtomicUsize::new(0);
    static SHUTDOWN_CALLS: AtomicUsize = AtomicUsize::new(0);

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
}
