//! Content-script entry points.
//!
//! C++ wires `sScriptMgr->SetScriptLoader(AddScripts)` before world startup.
//! Rust content scripts are linked through this crate and expose dispatcher
//! functions that `world-server` can call at the matching lifecycle points.

pub mod lifecycle {
    pub use wow_script::lifecycle::{
        LifecycleDispatchSummaryLikeCpp, LifecycleHookKindLikeCpp, ShutdownHookLikeCpp,
        StartupHookLikeCpp,
    };

    /// Dispatch `sScriptMgr->OnStartup()` content callbacks.
    pub async fn on_startup() -> LifecycleDispatchSummaryLikeCpp {
        wow_script::lifecycle::on_startup_like_cpp()
    }

    /// Dispatch `sScriptMgr->OnShutdown()` content callbacks.
    pub async fn on_shutdown() -> LifecycleDispatchSummaryLikeCpp {
        wow_script::lifecycle::on_shutdown_like_cpp()
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn lifecycle_facade_is_callable_with_no_content_scripts_like_cpp() {
        let startup = crate::lifecycle::on_startup().await;
        assert_eq!(
            startup.hook,
            crate::lifecycle::LifecycleHookKindLikeCpp::Startup
        );

        let shutdown = crate::lifecycle::on_shutdown().await;
        assert_eq!(
            shutdown.hook,
            crate::lifecycle::LifecycleHookKindLikeCpp::Shutdown
        );
    }
}
