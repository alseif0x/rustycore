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
                macro_rules! register_move {
                    ($builder:ident, $opcode:ident) => {
                        $builder.register(PacketHandlerEntry {
                            opcode: ClientOpcodes::$opcode,
                            status: SessionStatus::LoggedIn,
                            processing: PacketProcessing::Inplace,
                            handler_name: "macro",
                            handler: hidden,
                        })?
                    };
                }
            "#,
            "registration macro register_move is conditionally compiled",
        ),
        (
            r#"
                macro_rules! register_move {
                    ($builder:ident, $opcode:ident) => {
                        $builder.register(PacketHandlerEntry {
                            opcode: ClientOpcodes::$opcode,
                            status: SessionStatus::LoggedIn,
                            processing: PacketProcessing::Inplace,
                            handler_name: "macro",
                            handler: hidden,
                        })?
                    };
                }
                #[cfg(target_os = "linux")]
                register_move!(builder, Alpha);
            "#,
            "must be invoked inside the movement tail registrar",
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
            macro_rules! register_move {
                ($builder:ident, $opcode:ident) => {
                    #[cfg_attr(debug_assertions, allow(dead_code))]
                    $builder.register(PacketHandlerEntry {
                        opcode: ClientOpcodes::$opcode,
                        status: SessionStatus::LoggedIn,
                        processing: PacketProcessing::Inplace,
                        handler_name: "macro",
                        handler: hidden,
                    })?
                };
            }
        "#,
    )
    .expect_err("conditional tokens inside a registration macro must be rejected");

    assert!(
        error.contains("registration macro register_move contains cfg/cfg_attr tokens"),
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

    // A tail registration macro inside a block is rejected because the statement
    // macros are permitted only inside the movement tail registrar.
    let registration_macro = analyze_inline_source(
        r#"
            macro_rules! register_move {
                ($builder:ident, $opcode:ident) => {
                    $builder.register(PacketHandlerEntry {
                        opcode: ClientOpcodes::$opcode,
                        status: SessionStatus::LoggedIn,
                        processing: PacketProcessing::Inplace,
                        handler_name: "hidden",
                        handler: hidden,
                    })?
                };
            }
            fn hidden() {
                register_move!(builder, Alpha);
            }
        "#,
    )
    .expect_err("a registration macro inside a block must fail closed");
    assert!(
        registration_macro.contains("may be invoked only inside the movement tail registrar"),
        "{registration_macro}"
    );
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

    // The relocated statement macro is recognized only inside the movement tail
    // registrar, and its invocation counts once.
    let template = crate::registrations::analyze_inline_source_at_module(
        r#"
            macro_rules! register_move {
                ($builder:ident, $opcode:ident) => {
                    $builder.register(PacketHandlerEntry {
                        opcode: ClientOpcodes::$opcode,
                        status: SessionStatus::LoggedIn,
                        processing: PacketProcessing::ThreadSafe,
                        handler_name: concat!("handle_movement_", stringify!($opcode)),
                        handler: handle_movement_tail_move_thunk::<S, C>,
                    })?
                };
            }
            pub fn register_movement_tail_handlers_like_cpp<S, C>(
                builder: &mut RegistryBuilder<S, C>,
            ) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
            where
                S: MovementHandlerHostLikeCpp<C> + Send,
                C: Sync,
            {
                register_move!(builder, MoveStartForward);
                Ok(())
            }
        "#,
        crate::registrations::MOVEMENT_TAIL_OWNER_MODULE,
    )
    .expect("the relocated tail macro template is recognized inside its registrar");
    assert_eq!(template.direct_submissions, 0);
    assert_eq!(template.registration_macro_invocations, 1);
    assert_eq!(
        template.registration_macro_names,
        BTreeSet::from(["register_move".to_owned()])
    );

    // The legacy inventory expansion is no longer the registration grammar.
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
                            handler: hidden,
                        }
                    }
                };
            }
            register_move!(MoveStartForward);
        "#,
    )
    .expect_err("the legacy wrapper expansion is no longer a registration macro");
    assert!(
        wrapper_template.contains("still expands through the legacy inventory submission grammar"),
        "{wrapper_template}"
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
            macro_rules! register_move {
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
            register_move!(Alpha, Beta);
        "#,
    )
    .expect_err("repeating registration expansion must be rejected");
    assert!(
        repeating.contains("contains a macro repetition"),
        "{repeating}"
    );

    let multi_arm = analyze_inline_source(
        r#"
            macro_rules! register_move {
                ($builder:ident, $opcode:ident) => {
                    $builder.register(PacketHandlerEntry {
                        opcode: ClientOpcodes::$opcode,
                        status: SessionStatus::LoggedIn,
                        processing: PacketProcessing::Inplace,
                        handler_name: "macro",
                        handler: hidden,
                    })?
                };
                () => {};
            }
            register_move!(builder, Alpha);
        "#,
    )
    .expect_err("multi-arm registration expansion must be rejected");
    assert!(multi_arm.contains("has 2 rule arms"), "{multi_arm}");
}

const REAL_MOVEMENT_TAIL_SOURCE: &str = include_str!(
    "../../../../../crates/wow-world-application/src/movement_handlers/tail_registrations.rs"
);

#[test]
fn movement_tail_macros_are_owned_by_the_tail_and_counted_once() {
    let report = crate::registrations::analyze_inline_source_at_module(
        REAL_MOVEMENT_TAIL_SOURCE,
        crate::registrations::MOVEMENT_TAIL_OWNER_MODULE,
    )
    .expect("the real movement tail source is inside the closed registration grammar");
    assert_eq!(report.direct_submissions, 0);
    assert_eq!(
        report.registration_macro_invocations, 53,
        "the tail invokes 28 + 16 + 9 statement macros"
    );
    assert_eq!(
        report.registration_macro_names,
        BTreeSet::from([
            "register_move".to_owned(),
            "register_movement_ack_message".to_owned(),
            "register_movement_speed_ack".to_owned(),
        ])
    );
}

#[test]
fn movement_tail_contract_accounts_for_one_direct_entry_and_the_trainer_tail_for_two() {
    let tail = crate::registrations::analyze_contract_source(
        crate::registrations::MOVEMENT_TAIL_REGISTRAR,
        "wow-world-application",
        crate::registrations::MOVEMENT_TAIL_OWNER_MODULE,
        Path::new("crates/wow-world-application/src/movement_handlers/tail_registrations.rs"),
        REAL_MOVEMENT_TAIL_SOURCE,
    )
    .expect("the real movement tail registrar matches its finite contract");
    assert_eq!(
        tail.entries, 1,
        "only MoveSplineDone is a direct entry; the other 53 are macro-origin"
    );

    let trainer = crate::registrations::analyze_contract_source(
        crate::registrations::TRAINER_REGISTRAR,
        "wow-world-application",
        "crate::trainer_handlers",
        Path::new("crates/wow-world-application/src/trainer_handlers.rs"),
        include_str!("../../../../../crates/wow-world-application/src/trainer_handlers.rs"),
    )
    .expect("the real trainer registrar matches its finite contract");
    assert_eq!(trainer.entries, 2);
}

#[test]
fn movement_tail_registrar_rejects_a_direct_and_macro_opcode_collision() {
    let mutated = REAL_MOVEMENT_TAIL_SOURCE.replace(
        "    builder.register(PacketHandlerEntry {\n        opcode: ClientOpcodes::MoveSplineDone,",
        "    register_move!(builder, MoveSplineDone);\n    builder.register(PacketHandlerEntry {\n        opcode: ClientOpcodes::MoveSplineDone,",
    );
    assert_ne!(
        mutated, REAL_MOVEMENT_TAIL_SOURCE,
        "fixture mutation matched"
    );
    let error = crate::registrations::analyze_contract_source(
        crate::registrations::MOVEMENT_TAIL_REGISTRAR,
        "wow-world-application",
        crate::registrations::MOVEMENT_TAIL_OWNER_MODULE,
        Path::new("crates/wow-world-application/src/movement_handlers/tail_registrations.rs"),
        &mutated,
    )
    .expect_err("a direct/macro opcode collision must be rejected");
    assert!(
        error.contains("duplicate ApplicationMovementTail handler opcode entry"),
        "{error}"
    );
}

#[test]
fn builder_registration_macro_declaration_set_is_closed() {
    crate::registrations::validate_registration_macro_declaration_set(&BTreeSet::from([
        "register_move".to_owned(),
        "register_movement_ack_message".to_owned(),
        "register_movement_speed_ack".to_owned(),
    ]))
    .expect("the three tail macros are the closed declaration set");

    let error =
        crate::registrations::validate_registration_macro_declaration_set(&BTreeSet::from([
            "register_move".to_owned(),
            "register_movement_ack_message".to_owned(),
            "register_movement_speed_ack".to_owned(),
            "register_move_extra".to_owned(),
        ]))
        .expect_err("a fourth registration macro must be rejected");
    assert!(error.contains("grammar changed"), "{error}");
}

#[test]
fn relocated_or_unowned_builder_registration_macro_is_rejected() {
    let error = crate::registrations::analyze_inline_source_at_module(
        r#"
            macro_rules! register_move {
                ($builder:ident, $opcode:ident) => {
                    $builder.register(PacketHandlerEntry {
                        opcode: ClientOpcodes::$opcode,
                        status: SessionStatus::LoggedIn,
                        processing: PacketProcessing::Inplace,
                        handler_name: "hidden",
                        handler: hidden,
                    })?
                };
            }
            pub fn register_movement_tail_handlers_like_cpp<S, C>(
                builder: &mut RegistryBuilder<S, C>,
            ) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
            where
                S: MovementHandlerHostLikeCpp<C> + Send,
                C: Sync,
            {
                register_move!(builder, Alpha);
                Ok(())
            }
        "#,
        "crate::handlers::other",
    )
    .expect_err("a relocated statement registration macro must be rejected");
    assert!(
        error.contains("instead of the movement tail owner"),
        "{error}"
    );
}

#[test]
fn tail_registration_macro_invocations_outside_the_tail_registrar_are_rejected() {
    let item_level = crate::registrations::analyze_inline_source_at_module(
        r#"
            macro_rules! register_move {
                ($builder:ident, $opcode:ident) => {
                    $builder.register(PacketHandlerEntry {
                        opcode: ClientOpcodes::$opcode,
                        status: SessionStatus::LoggedIn,
                        processing: PacketProcessing::Inplace,
                        handler_name: "hidden",
                        handler: hidden,
                    })?
                };
            }
            register_move!(builder, Alpha);
        "#,
        crate::registrations::MOVEMENT_TAIL_OWNER_MODULE,
    )
    .expect_err("an item-level tail registration macro invocation must be rejected");
    assert!(
        item_level.contains("not at module item level"),
        "{item_level}"
    );

    let other_function = crate::registrations::analyze_inline_source_at_module(
        r#"
            macro_rules! register_move {
                ($builder:ident, $opcode:ident) => {
                    $builder.register(PacketHandlerEntry {
                        opcode: ClientOpcodes::$opcode,
                        status: SessionStatus::LoggedIn,
                        processing: PacketProcessing::Inplace,
                        handler_name: "hidden",
                        handler: hidden,
                    })?
                };
            }
            fn other(builder: &mut RegistryBuilder<(), ()>) {
                register_move!(builder, Alpha);
            }
        "#,
        crate::registrations::MOVEMENT_TAIL_OWNER_MODULE,
    )
    .expect_err("a tail registration macro invoked outside its registrar must be rejected");
    assert!(
        other_function.contains("may be invoked only inside the movement tail registrar"),
        "{other_function}"
    );

    let bad_arguments = crate::registrations::analyze_inline_source_at_module(
        r#"
            macro_rules! register_move {
                ($builder:ident, $opcode:ident) => {
                    $builder.register(PacketHandlerEntry {
                        opcode: ClientOpcodes::$opcode,
                        status: SessionStatus::LoggedIn,
                        processing: PacketProcessing::Inplace,
                        handler_name: "hidden",
                        handler: hidden,
                    })?
                };
            }
            pub fn register_movement_tail_handlers_like_cpp<S, C>(
                builder: &mut RegistryBuilder<S, C>,
            ) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
            where
                S: MovementHandlerHostLikeCpp<C> + Send,
                C: Sync,
            {
                register_move!(other_builder, Alpha);
                Ok(())
            }
        "#,
        crate::registrations::MOVEMENT_TAIL_OWNER_MODULE,
    )
    .expect_err("a tail registration macro must pass `builder` explicitly");
    assert!(
        bad_arguments.contains("must be invoked as `macro!(builder, OpcodeVariant)`"),
        "{bad_arguments}"
    );
}
