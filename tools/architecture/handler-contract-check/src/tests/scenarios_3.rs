//! Handler-contract regressions, part 3 of 3.
//!
//! Moved out of the tests.rs root under #660; every test is unchanged.

use super::*;

#[test]
fn source_guard_rejects_path_grammar_it_cannot_resolve_exactly() {
    for (source, expected_error) in [
        (
            r#"#[cfg_attr(windows, path = "hidden.rs")] mod hidden;"#,
            "module #[cfg_attr(..., path = ...)] is not allowed",
        ),
        (
            r#"#[path = "hidden.rs"] mod inline { pub fn visible() {} }"#,
            "inline module inline",
        ),
        (
            r#"mod inline { #[path = "hidden.rs"] mod hidden; }"#,
            "declared inside an inline module",
        ),
    ] {
        let error = analyze_inline_source(source)
            .expect_err("ambiguous #[path] grammar must fail in the handler analyzer");
        assert!(
            error.contains(expected_error),
            "expected {expected_error:?}, got {error:?}"
        );
    }
}

#[test]
fn source_guard_rejects_submit_imports_and_alias_generators() {
    for (source, expected_error) in [
        (
            r#"
                use inv::submit as s;
                fn hidden() { s! { E { opcode: ClientOpcodes::Hidden } } }
            "#,
            "can alias an inventory registration macro",
        ),
        (
            r#"
                macro_rules! hidden {
                    () => { inv::submit! { E { opcode: ClientOpcodes::Hidden } } };
                }
                const INSTALL: () = { hidden!(); };
            "#,
            "handler-capable macro hidden",
        ),
        (
            r#"
                macro_rules! hidden {
                    () => {
                        use inventory::submit as s;
                        s! { E { opcode: ClientOpcodes::Hidden } }
                    };
                }
                const INSTALL: () = { hidden!(); };
            "#,
            "aliases/imports an inventory registration macro",
        ),
    ] {
        let error = analyze_inline_source(source)
            .expect_err("submit aliases and forwarding macros must fail closed");
        assert!(
            error.contains(expected_error),
            "expected {expected_error:?}, got {error:?}"
        );
    }
}

#[test]
fn source_guard_rejects_cfg_on_direct_macro_definition_invocation_and_ancestor() {
    let cases = [
        (
            r#"
                #[cfg(debug_assertions)]
                inventory::submit! {
                    PacketHandlerEntry {
                        opcode: ClientOpcodes::Alpha,
                        status: SessionStatus::LoggedIn,
                        processing: PacketProcessing::Inplace,
                        handler_name: "alpha",
                    }
                }
            "#,
            "handler registration submit! is conditionally compiled",
        ),
        (
            r#"
                #[cfg(feature = "conditional-handler")]
                macro_rules! register_handler {
                    ($opcode:ident) => {
                        inventory::submit! {
                            PacketHandlerEntry {
                                opcode: ClientOpcodes::$opcode,
                                status: SessionStatus::LoggedIn,
                                processing: PacketProcessing::Inplace,
                                handler_name: "macro",
                            }
                        }
                    };
                }
                register_handler!(Alpha);
            "#,
            "registration macro register_handler is conditionally compiled",
        ),
        (
            r#"
                macro_rules! register_handler {
                    ($opcode:ident) => {
                        inventory::submit! {
                            PacketHandlerEntry {
                                opcode: ClientOpcodes::$opcode,
                                status: SessionStatus::LoggedIn,
                                processing: PacketProcessing::Inplace,
                                handler_name: "macro",
                            }
                        }
                    };
                }
                #[cfg(target_os = "linux")]
                register_handler!(Alpha);
            "#,
            "handler registration register_handler! is conditionally compiled",
        ),
        (
            r#"
                #[cfg(not(debug_assertions))]
                mod release_only {
                    inventory::submit! {
                        PacketHandlerEntry {
                            opcode: ClientOpcodes::Alpha,
                            status: SessionStatus::LoggedIn,
                            processing: PacketProcessing::Inplace,
                            handler_name: "alpha",
                        }
                    }
                }
            "#,
            "handler registration submit! is conditionally compiled",
        ),
    ];

    for (source, expected_error) in cases {
        let error =
            analyze_inline_source(source).expect_err("conditional registration must be rejected");
        assert!(
            error.contains(expected_error),
            "expected {expected_error:?}, got {error:?}"
        );
    }
}

#[test]
fn source_guard_rejects_cfg_hidden_inside_a_registration_macro() {
    let error = analyze_inline_source(
        r#"
            macro_rules! register_handler {
                ($opcode:ident) => {
                    #[cfg_attr(debug_assertions, allow(dead_code))]
                    inventory::submit! {
                        PacketHandlerEntry {
                            opcode: ClientOpcodes::$opcode,
                            status: SessionStatus::LoggedIn,
                            processing: PacketProcessing::Inplace,
                            handler_name: "macro",
                        }
                    }
                };
            }
            register_handler!(Alpha);
        "#,
    )
    .expect_err("conditional tokens inside a registration macro must be rejected");

    assert!(
        error.contains("registration macro register_handler contains cfg/cfg_attr tokens"),
        "{error}"
    );
}

#[test]
fn source_guard_rejects_nested_conditional_registration_and_unresolved_include() {
    let nested_error = analyze_inline_source(
        r#"
            #[cfg(target_os = "linux")]
            const REGISTER: () = {
                inventory::submit! {
                    PacketHandlerEntry {
                        opcode: ClientOpcodes::Alpha,
                        status: SessionStatus::LoggedIn,
                        processing: PacketProcessing::Inplace,
                        handler_name: "alpha",
                    }
                }
            };
        "#,
    )
    .expect_err("nested conditional registration must be rejected");
    assert!(
        nested_error.contains("registration grammar is allowed only at module item level"),
        "{nested_error}"
    );

    let nested_forwarder_error = analyze_inline_source(
        r#"
            type Hidden = PacketHandlerEntry;
            #[cfg(windows)]
            const REGISTER: () = {
                external_forward!(
                    inventory::submit,
                    Hidden {
                        opcode: ClientOpcodes::Alpha,
                    }
                );
            };
        "#,
    )
    .expect_err("a nested macro must not forward an inventory registration path");
    assert!(
        nested_forwarder_error.contains("external_forward")
            && nested_forwarder_error
                .contains("registration grammar is allowed only at module item level"),
        "{nested_forwarder_error}"
    );

    let include_error = analyze_inline_source(r#"include!("generated_handlers.rs");"#)
        .expect_err("unresolved source inclusion must be rejected");
    assert!(
        include_error.contains("unsupported item-level macro include!"),
        "{include_error}"
    );
}

#[test]
fn source_guard_rejects_handler_grammar_inside_blocks() {
    let cases = [
        (
            r#"
                fn hidden() {
                    inventory::submit! {
                        PacketHandlerEntry {
                            opcode: ClientOpcodes::Alpha,
                            status: SessionStatus::LoggedIn,
                            processing: PacketProcessing::Inplace,
                            handler_name: "hidden",
                        }
                    }
                }
            "#,
            "inventory::submit!",
        ),
        (
            r#"
                macro_rules! register_handler {
                    ($opcode:ident) => {
                        inventory::submit! {
                            PacketHandlerEntry {
                                opcode: ClientOpcodes::$opcode,
                                status: SessionStatus::LoggedIn,
                                processing: PacketProcessing::Inplace,
                                handler_name: "hidden",
                            }
                        }
                    };
                }
                fn hidden() {
                    register_handler!(Alpha);
                }
            "#,
            "register_handler!",
        ),
        (
            r#"
                fn hidden() {
                    include!("generated_handlers.rs");
                }
            "#,
            "include!",
        ),
        (
            r#"
                fn hidden() {
                    submit_alias! {
                        PacketHandlerEntry {
                            opcode: ClientOpcodes::Alpha,
                        }
                    }
                }
            "#,
            "submit_alias!",
        ),
        (
            r#"
                fn hidden() {
                    submit! {
                        E {
                            opcode: ClientOpcodes::Alpha,
                        }
                    }
                }
            "#,
            "submit!",
        ),
        (
            r#"
                fn hidden() {
                    inventory::collect! {
                        E
                    }
                }
            "#,
            "inventory::collect!",
        ),
        (
            r#"
                fn hidden() {
                    inv::__do_submit! {
                        E {
                            opcode: ClientOpcodes::Alpha,
                        }
                    }
                }
            "#,
            "inv::__do_submit!",
        ),
    ];

    for (source, macro_name) in cases {
        let error =
            analyze_inline_source(source).expect_err("nested handler grammar must fail closed");
        assert!(
            error.contains("registration grammar is allowed only at module item level"),
            "{error}"
        );
        assert!(error.contains(macro_name), "{error}");
    }
}

#[test]
fn source_guard_rejects_collector_inside_handler_owner() {
    let error = analyze_inline_source("inventory::collect!(PacketHandlerEntry);")
        .expect_err("the generic PacketHandlerEntry collector is not the World legacy bridge");
    assert!(
        error.contains("unsupported item-level macro inventory::collect!"),
        "{error}"
    );
}

#[test]
fn qualified_legacy_wrapper_counts_as_one_direct_submission_without_a_template_name() {
    let report = analyze_inline_source(
        r#"
            crate::session::registry::register_packet_handler_like_cpp!(
                PacketHandlerEntry { opcode: ClientOpcodes::Hidden }
            );
        "#,
    )
    .expect("the one qualified wrapper invocation with one literal entry is auditable");
    assert_eq!(report.direct_submissions, 1);
    assert_eq!(report.registration_macro_invocations, 0);
    assert!(report.registration_macro_names.is_empty());

    let template = analyze_inline_source(
        r#"
            macro_rules! register_move {
                ($opcode:ident) => {
                    inventory::submit! {
                        PacketHandlerEntry { opcode: ClientOpcodes::$opcode }
                    }
                };
            }
            register_move!(MoveStartForward);
        "#,
    )
    .expect("the existing register_move template remains recognized");
    assert_eq!(template.direct_submissions, 0);
    assert_eq!(template.registration_macro_invocations, 1);
    assert_eq!(
        template.registration_macro_names,
        BTreeSet::from(["register_move".to_owned()])
    );

    let wrapper_template = analyze_inline_source(
        r#"
            macro_rules! register_move {
                ($opcode:ident) => {
                    crate::session::registry::register_packet_handler_like_cpp! {
                        PacketHandlerEntry {
                            opcode: ClientOpcodes::$opcode,
                            status: SessionStatus::LoggedIn,
                            processing: PacketProcessing::ThreadSafe,
                            handler_name: concat!("handle_movement_", stringify!($opcode)),
                            handler: |session, catalogs, pkt| {
                                Box::pin(async move {
                                    session
                                        .handle_movement_with_catalogs_like_cpp(
                                            catalogs.area_triggers.as_ref(),
                                            catalogs.creature_spawns.as_ref(),
                                            catalogs.progression.as_ref(),
                                            &catalogs.player_grid_loader,
                                            pkt,
                                        )
                                        .await
                                })
                            },
                        }
                    }
                };
            }
            register_move!(MoveStartForward);
        "#,
    )
    .expect("the real register_move wrapper template accepts its opcode metavariable");
    assert_eq!(wrapper_template.direct_submissions, 0);
    assert_eq!(wrapper_template.registration_macro_invocations, 1);
    assert_eq!(
        wrapper_template.registration_macro_names,
        BTreeSet::from(["register_move".to_owned()])
    );
}

#[test]
fn qualified_legacy_wrapper_rejects_forwarded_or_multiple_entries() {
    for source in [
        r#"crate::session::registry::register_packet_handler_like_cpp!(forwarded!());"#,
        r#"crate::session::registry::register_packet_handler_like_cpp!((
            PacketHandlerEntry { opcode: ClientOpcodes::First },
            PacketHandlerEntry { opcode: ClientOpcodes::Second }
        ));"#,
        r#"other::registry::register_packet_handler_like_cpp!(
            PacketHandlerEntry { opcode: ClientOpcodes::Hidden }
        );"#,
    ] {
        let error = analyze_inline_source(source)
            .expect_err("only one qualified wrapper call with one literal entry is accepted");
        assert!(error.contains("unsupported item-level macro"), "{error}");
    }

    let forwarder = analyze_inline_source(
        r#"
            macro_rules! forward_entry {
                ($entry:expr) => {
                    crate::session::registry::register_packet_handler_like_cpp!($entry)
                };
            }
            forward_entry!(PacketHandlerEntry { opcode: ClientOpcodes::Hidden });
        "#,
    )
    .expect_err("a template that forwards an arbitrary entry is not an audited registration");
    assert!(
        forwarder.contains("handler-capable macro forward_entry"),
        "{forwarder}"
    );

    let repetition = analyze_inline_source(
        r#"
            macro_rules! repeated_entries {
                ($($opcode:ident),*) => {
                    $(crate::session::registry::register_packet_handler_like_cpp! {
                        PacketHandlerEntry { opcode: ClientOpcodes::$opcode }
                    };)*
                };
            }
            repeated_entries!(First, Second);
        "#,
    )
    .expect_err("a repeated wrapper template cannot prove one entry per invocation");
    assert!(repetition.contains("macro repetition"), "{repetition}");
}

#[test]
fn source_guard_rejects_unknown_item_macro_that_can_expand_a_cfg_module() {
    let error = analyze_inline_source(
        r#"
            macro_rules! m {
                () => {
                    #[cfg(windows)]
                    mod windows_handlers;
                };
            }
            m!();
        "#,
    )
    .expect_err("unknown item macros must fail closed before expansion");
    assert!(error.contains("unsupported item-level macro m!"), "{error}");
}

#[test]
fn source_guard_rejects_nested_handler_macro_definition() {
    let error = analyze_inline_source(
        r#"
            fn install_conditionally() {
                #[cfg(windows)]
                macro_rules! hidden_handler {
                    ($opcode:ident) => {
                        inventory::submit! {
                            PacketHandlerEntry {
                                opcode: ClientOpcodes::$opcode,
                                status: SessionStatus::LoggedIn,
                                processing: PacketProcessing::Inplace,
                                handler_name: "hidden",
                            }
                        }
                    };
                }
                hidden_handler!(Alpha);
            }
        "#,
    )
    .expect_err("nested handler-capable macro definition must fail closed");
    assert!(
        error.contains("nested macro_rules! hidden_handler may generate a handler registration"),
        "{error}"
    );

    for (source, macro_name) in [
        (
            r#"
                fn define_local_generator() {
                    macro_rules! hidden_module {
                        () => {
                            #[cfg(windows)]
                            mod windows_handlers;
                        };
                    }
                }
            "#,
            "hidden_module",
        ),
        (
            r#"
                fn define_local_generator() {
                    macro_rules! hidden_include {
                        () => {
                            include!("generated_handlers.rs");
                        };
                    }
                }
            "#,
            "hidden_include",
        ),
    ] {
        let error = analyze_inline_source(source)
            .expect_err("nested module/include source generator must fail closed");
        assert!(
            error.contains(&format!(
                "nested macro_rules! {macro_name} may generate a handler registration"
            )),
            "{error}"
        );
    }
}

#[test]
fn source_guard_rejects_repeating_or_multi_arm_registration_macros() {
    let repeating = analyze_inline_source(
        r#"
            macro_rules! register_handler {
                ($($opcode:ident),+) => {
                    $(inventory::submit! {
                        PacketHandlerEntry {
                            opcode: ClientOpcodes::$opcode,
                            status: SessionStatus::LoggedIn,
                            processing: PacketProcessing::Inplace,
                            handler_name: "macro",
                        }
                    })+
                };
            }
            register_handler!(Alpha, Beta);
        "#,
    )
    .expect_err("repeating registration expansion must be rejected");
    assert!(
        repeating.contains("contains a macro repetition"),
        "{repeating}"
    );

    let multi_arm = analyze_inline_source(
        r#"
            macro_rules! register_handler {
                ($opcode:ident) => {
                    inventory::submit! {
                        PacketHandlerEntry {
                            opcode: ClientOpcodes::$opcode,
                            status: SessionStatus::LoggedIn,
                            processing: PacketProcessing::Inplace,
                            handler_name: "macro",
                        }
                    }
                };
                () => {};
            }
            register_handler!(Alpha);
        "#,
    )
    .expect_err("multi-arm registration expansion must be rejected");
    assert!(multi_arm.contains("has 2 rule arms"), "{multi_arm}");
}
