//! Lifecycle dispatch entry points keep their shape after the wow-scripts facade
//! was folded into this crate (wave A1). Assertions preserved from the facade test.

#[test]
fn lifecycle_dispatch_is_callable_with_no_content_scripts_like_cpp() {
    let startup = wow_script::lifecycle::on_startup_like_cpp();
    assert_eq!(
        startup.hook,
        wow_script::lifecycle::LifecycleHookKindLikeCpp::Startup
    );
    let shutdown = wow_script::lifecycle::on_shutdown_like_cpp();
    assert_eq!(
        shutdown.hook,
        wow_script::lifecycle::LifecycleHookKindLikeCpp::Shutdown
    );
}
