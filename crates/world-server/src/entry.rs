//! The public entry points; the startup body is `app::run_inner`.

use super::*;

/// Run the world server with explicit process arguments.
///
/// Boxing keeps the enormous startup future private to this crate and gives
/// embedders a stable, compact library boundary.
pub fn run(args: Vec<String>) -> Pin<Box<dyn Future<Output = Result<ExitCode>> + Send + 'static>> {
    run_with_modules(args, wow_module_api::ModuleRegistry::new())
}

/// Run the world server with a pre-composed trusted module registry.
///
/// The generated compositor crate (issue #229) calls this after invoking every
/// installed module's registrar in the operator's declared order. `run` is the
/// zero-module case and passes an empty registry, so the ordinary build is
/// unchanged and never observes a module.
pub fn run_with_modules(
    args: Vec<String>,
    modules: wow_module_api::ModuleRegistry,
) -> Pin<Box<dyn Future<Output = Result<ExitCode>> + Send + 'static>> {
    Box::pin(super::run_inner(args, Arc::new(modules), ""))
}

/// Run the world server with a composed registry and the composition's
/// `modules.lock.toml` (#1444 H6).
///
/// The generated compositor passes the lock it was built from when a module
/// ships SQL or a removed module left a tombstone. The startup schema check
/// then requires every composed module's migrations and accepts the history
/// of tombstoned ones; `run_with_modules` accepts no module history at all.
pub fn run_with_composition(
    args: Vec<String>,
    modules: wow_module_api::ModuleRegistry,
    module_lock: &'static str,
) -> Pin<Box<dyn Future<Output = Result<ExitCode>> + Send + 'static>> {
    Box::pin(super::run_inner(args, Arc::new(modules), module_lock))
}
