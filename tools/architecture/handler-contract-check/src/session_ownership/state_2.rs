//! Session ownership inventory state definitions, part 2 of 3.
//!
//! Separated from the session_ownership.rs root under #660. Behaviour is preserved.

use super::*;

pub(super) fn collect_world_session_impl(
    item: &ItemImpl,
    module: &str,
    cfg: &[String],
    availability: Availability,
    builder: &mut BaselineBuilder,
) {
    let (impl_cfg, impl_availability) = item_context(
        cfg,
        availability,
        &item.attrs,
        "WorldSession impl",
        &mut builder.errors,
    );
    let Some(impl_source_class) = impl_availability.source_class() else {
        return;
    };
    if !type_path_ends_with(&item.self_ty, WORLD_SESSION_NAME) {
        return;
    }
    let module = logical_world_session_owner(module);
    let trait_path = normalized_trait_path(item);
    let impl_target = trait_path.as_ref().map_or_else(
        || format!("impl {WORLD_SESSION_NAME}"),
        |trait_path| format!("impl {trait_path} for {WORLD_SESSION_NAME}"),
    );
    builder
        .generated_surface_inputs
        .extend(generated_attribute_inputs(
            module,
            &impl_target,
            &item.attrs,
            &impl_cfg,
            impl_availability,
        ));
    builder.world_session_impls.insert((
        module.to_owned(),
        trait_path.clone(),
        impl_cfg.clone(),
        impl_source_class.to_owned(),
    ));

    for impl_item in &item.items {
        match impl_item {
            ImplItem::Fn(function) => {
                let (cfg, availability) = item_context(
                    &impl_cfg,
                    impl_availability,
                    &function.attrs,
                    "WorldSession method",
                    &mut builder.errors,
                );
                let Some(source_class) = availability.source_class() else {
                    continue;
                };
                let name = function.sig.ident.to_string();
                builder
                    .generated_surface_inputs
                    .extend(generated_attribute_inputs(
                        module,
                        &format!("{WORLD_SESSION_NAME}::{name}"),
                        &function.attrs,
                        &cfg,
                        availability,
                    ));
                let surface = impl_item_surface(
                    module,
                    &trait_path,
                    "method",
                    name.clone(),
                    &function.vis,
                    normalized_tokens(&function.sig),
                    cfg,
                    source_class,
                    name.starts_with("set_"),
                );
                builder.world_session_impl_items.insert(surface);
            }
            ImplItem::Const(constant) => {
                let (cfg, availability) = item_context(
                    &impl_cfg,
                    impl_availability,
                    &constant.attrs,
                    "WorldSession associated const",
                    &mut builder.errors,
                );
                let Some(source_class) = availability.source_class() else {
                    continue;
                };
                builder
                    .generated_surface_inputs
                    .extend(generated_attribute_inputs(
                        module,
                        &format!("{WORLD_SESSION_NAME}::{}", constant.ident),
                        &constant.attrs,
                        &cfg,
                        availability,
                    ));
                let signature = format!(
                    "const {} : {}",
                    constant.ident,
                    normalized_tokens(&constant.ty)
                );
                let surface = impl_item_surface(
                    module,
                    &trait_path,
                    "const",
                    constant.ident.to_string(),
                    &constant.vis,
                    signature,
                    cfg,
                    source_class,
                    false,
                );
                builder.world_session_impl_items.insert(surface);
            }
            ImplItem::Type(item_type) => {
                let (cfg, availability) = item_context(
                    &impl_cfg,
                    impl_availability,
                    &item_type.attrs,
                    "WorldSession associated type",
                    &mut builder.errors,
                );
                let Some(source_class) = availability.source_class() else {
                    continue;
                };
                builder
                    .generated_surface_inputs
                    .extend(generated_attribute_inputs(
                        module,
                        &format!("{WORLD_SESSION_NAME}::{}", item_type.ident),
                        &item_type.attrs,
                        &cfg,
                        availability,
                    ));
                let signature = format!(
                    "type {} = {}",
                    item_type.ident,
                    normalized_tokens(&item_type.ty)
                );
                let surface = impl_item_surface(
                    module,
                    &trait_path,
                    "type",
                    item_type.ident.to_string(),
                    &item_type.vis,
                    signature,
                    cfg,
                    source_class,
                    false,
                );
                builder.world_session_impl_items.insert(surface);
            }
            ImplItem::Macro(item_macro) => {
                let (_, availability) = item_context(
                    &impl_cfg,
                    impl_availability,
                    &item_macro.attrs,
                    "WorldSession impl macro",
                    &mut builder.errors,
                );
                if availability.source_class().is_some() {
                    builder.errors.push(format!(
                        "{module} contains macro {}! inside impl {WORLD_SESSION_NAME}; generated \
                         associated items are outside the exact ownership grammar",
                        normalized_tokens(&item_macro.mac.path)
                    ));
                }
            }
            ImplItem::Verbatim(_) => builder.errors.push(format!(
                "{module} contains unparsed verbatim syntax inside impl {WORLD_SESSION_NAME}"
            )),
            _ => {}
        }
    }
}

pub(super) fn call_key(
    module: &str,
    callee: &str,
    argument_count: usize,
    cfg: Vec<String>,
    source_class: &str,
) -> (String, String, usize, Vec<String>, String) {
    (
        module.to_owned(),
        callee.to_owned(),
        argument_count,
        cfg,
        source_class.to_owned(),
    )
}

pub(super) struct ExpressionSurfaceCollector<'a> {
    pub(super) module: &'a str,
    pub(super) cfg: &'a [String],
    pub(super) availability: Availability,
    pub(super) inside_session_factory: bool,
    pub(super) builder: &'a mut BaselineBuilder,
}

impl ExpressionSurfaceCollector<'_> {
    pub(super) fn expression_context(
        &mut self,
        attributes: &[syn::Attribute],
    ) -> (Vec<String>, Availability) {
        item_context(
            self.cfg,
            self.availability,
            attributes,
            "session factory expression",
            &mut self.builder.errors,
        )
    }

    pub(super) fn increment(
        map: &mut BTreeMap<(String, String, usize, Vec<String>, String), usize>,
        module: &str,
        callee: &str,
        argument_count: usize,
        cfg: Vec<String>,
        source_class: &str,
    ) {
        *map.entry(call_key(module, callee, argument_count, cfg, source_class))
            .or_default() += 1;
    }
}

impl<'ast> Visit<'ast> for ExpressionSurfaceCollector<'_> {
    fn visit_expr_struct(&mut self, expression: &'ast syn::ExprStruct) {
        let (cfg, availability) = self.expression_context(&expression.attrs);
        if let Some(source_class) = availability.source_class()
            && expression
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == SESSION_RESOURCES_NAME)
        {
            Self::increment(
                &mut self.builder.session_resources_constructions,
                self.module,
                SESSION_RESOURCES_NAME,
                expression.fields.len(),
                cfg,
                source_class,
            );
        }
        syn::visit::visit_expr_struct(self, expression);
    }

    fn visit_expr_call(&mut self, expression: &'ast syn::ExprCall) {
        let (cfg, availability) = self.expression_context(&expression.attrs);
        if let Some(source_class) = availability.source_class() {
            if let Expr::Path(path) = expression.func.as_ref() {
                let segments: Vec<_> = path
                    .path
                    .segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect();
                if self.inside_session_factory
                    && expression.args.iter().any(|argument| {
                        token_stream_mentions_ident(&argument.to_token_stream(), "session")
                    })
                {
                    self.builder
                        .session_factory_helper_calls
                        .insert((self.module.to_owned(), segments.clone()));
                }
                if segments
                    .last()
                    .is_some_and(|name| name == SESSION_FACTORY_NAME)
                {
                    Self::increment(
                        &mut self.builder.session_factory_calls,
                        self.module,
                        SESSION_FACTORY_NAME,
                        expression.args.len(),
                        cfg.clone(),
                        source_class,
                    );
                }
                if segments
                    .as_slice()
                    .ends_with(&[WORLD_SESSION_NAME.to_owned(), "new".to_owned()])
                {
                    Self::increment(
                        &mut self.builder.world_session_new_calls,
                        self.module,
                        "WorldSession::new",
                        expression.args.len(),
                        cfg,
                        source_class,
                    );
                }
            }
        }
        syn::visit::visit_expr_call(self, expression);
    }

    fn visit_expr_method_call(&mut self, expression: &'ast syn::ExprMethodCall) {
        let (cfg, availability) = self.expression_context(&expression.attrs);
        let method = expression.method.to_string();
        if let Some(source_class) = availability.source_class()
            && self.inside_session_factory
            && (method.starts_with("set_") || method.starts_with("install_"))
        {
            let receiver = normalized_tokens(expression.receiver.as_ref());
            Self::increment(
                &mut self.builder.session_factory_setter_calls,
                self.module,
                &format!("{receiver}.{method}"),
                expression.args.len(),
                cfg,
                source_class,
            );
        }
        syn::visit::visit_expr_method_call(self, expression);
    }
}

pub(super) fn collect_expression_surfaces(
    item: &Item,
    module: &str,
    cfg: &[String],
    availability: Availability,
    inside_session_factory: bool,
    builder: &mut BaselineBuilder,
) {
    let mut collector = ExpressionSurfaceCollector {
        module,
        cfg,
        availability,
        inside_session_factory,
        builder,
    };
    collector.visit_item(item);
}

pub(super) fn collect_items(
    role: PackageRole,
    items: &[Item],
    module: &str,
    cfg: &[String],
    availability: Availability,
    builder: &mut BaselineBuilder,
) {
    for item in items {
        let attributes: &[syn::Attribute] = match item {
            Item::Const(item) => &item.attrs,
            Item::Enum(item) => &item.attrs,
            Item::ExternCrate(item) => &item.attrs,
            Item::Fn(item) => &item.attrs,
            Item::ForeignMod(item) => &item.attrs,
            Item::Impl(item) => &item.attrs,
            Item::Macro(item) => &item.attrs,
            Item::Mod(item) => &item.attrs,
            Item::Static(item) => &item.attrs,
            Item::Struct(item) => &item.attrs,
            Item::Trait(item) => &item.attrs,
            Item::TraitAlias(item) => &item.attrs,
            Item::Type(item) => &item.attrs,
            Item::Union(item) => &item.attrs,
            Item::Use(item) => &item.attrs,
            Item::Verbatim(_) => &[],
            _ => &[],
        };
        let (item_cfg, item_availability) = item_context(
            cfg,
            availability,
            attributes,
            &format!("item in {module}"),
            &mut builder.errors,
        );
        let Some(item_source_class) = item_availability.source_class() else {
            continue;
        };

        if let Item::Mod(item_mod) = item {
            if let Some((_, inline_items)) = &item_mod.content {
                collect_items(
                    role,
                    inline_items,
                    &format!("{module}::{}", item_mod.ident),
                    &item_cfg,
                    item_availability,
                    builder,
                );
            }
            continue;
        }

        if role == PackageRole::Server
            && let Item::Fn(function) = item
        {
            let name = function.sig.ident.to_string();
            builder
                .server_function_bodies
                .entry((module.to_owned(), name.clone()))
                .or_default()
                .insert(SessionFactoryHelperSurface {
                    module: module.to_owned(),
                    name,
                    signature: normalized_tokens(&function.sig),
                    body_fingerprint: compact_token_fingerprint(&function.block),
                    cfg: item_cfg.clone(),
                    source_class: item_source_class.to_owned(),
                });
        }

        if let Item::Use(item_use) = item
            && use_tree_renames_ident(&item_use.tree, WORLD_SESSION_NAME)
        {
            builder.errors.push(format!(
                "{module} renames {WORLD_SESSION_NAME}; aliases can bypass exact impl ownership"
            ));
        }
        if let Item::Type(item_type) = item
            && type_path_ends_with(&item_type.ty, WORLD_SESSION_NAME)
            && item_type.ident != WORLD_SESSION_NAME
        {
            builder.errors.push(format!(
                "{module} aliases {WORLD_SESSION_NAME} as {}; type aliases are outside the exact \
                 impl ownership grammar",
                item_type.ident
            ));
        }
        if let Item::Macro(item_macro) = item
            && let Some(target) = token_stream_mentions_ownership_target(&item_macro.mac.tokens)
        {
            builder.errors.push(format!(
                "{module} macro {}! mentions {target}; macro-generated ownership surfaces are \
                 not allowed",
                normalized_tokens(&item_macro.mac.path),
            ));
        }

        if role == PackageRole::Network
            || (role == PackageRole::World
                && (module == WORLD_SESSION_DIRECTORY_MODULE
                    || module == WORLD_SESSION_MAILBOX_MODULE
                    || module.starts_with(&format!("{WORLD_SESSION_MAILBOX_MODULE}::"))
                    || module == WORLD_LOOT_PERSISTENCE_MODULE))
            || (role == PackageRole::Social
                && (module == SOCIAL_GROUP_MODULE
                    || module.starts_with(&format!("{SOCIAL_GROUP_MODULE}::"))))
        {
            collect_contract_type(item, module, &item_cfg, item_availability, builder);
        }

        match (role, module, item) {
            (PackageRole::World, WORLD_SESSION_MODULE, Item::Struct(item_struct)) => {
                collect_struct(
                    item_struct,
                    module,
                    cfg,
                    availability,
                    WORLD_SESSION_NAME,
                    &mut builder.world_session_definition,
                    &mut builder.world_session_fields,
                    &mut builder.generated_surface_inputs,
                    &mut builder.errors,
                );
            }
            (PackageRole::Server, SESSION_RESOURCES_MODULE, Item::Struct(item_struct)) => {
                collect_struct(
                    item_struct,
                    module,
                    cfg,
                    availability,
                    SESSION_RESOURCES_NAME,
                    &mut builder.session_resources_definition,
                    &mut builder.session_resources_fields,
                    &mut builder.generated_surface_inputs,
                    &mut builder.errors,
                );
            }
            (PackageRole::World, _, Item::Impl(item_impl)) => {
                collect_world_session_impl(item_impl, module, cfg, availability, builder)
            }
            (PackageRole::Server, SESSION_FACTORY_MODULE, Item::Fn(function))
                if function.sig.ident == SESSION_FACTORY_NAME =>
            {
                let surface = definition_surface(
                    module,
                    SESSION_FACTORY_NAME,
                    &function.vis,
                    item_cfg.clone(),
                    item_source_class,
                );
                set_once(
                    &mut builder.session_factory_definition,
                    surface,
                    SESSION_FACTORY_NAME,
                    &mut builder.errors,
                );
                set_once(
                    &mut builder.session_factory_signature,
                    normalized_tokens(&function.sig),
                    "create_session signature",
                    &mut builder.errors,
                );
                set_once(
                    &mut builder.session_factory_body_fingerprint,
                    compact_token_fingerprint(&function.block),
                    "create_session body fingerprint",
                    &mut builder.errors,
                );
                builder
                    .generated_surface_inputs
                    .extend(generated_attribute_inputs(
                        module,
                        SESSION_FACTORY_NAME,
                        &function.attrs,
                        &item_cfg,
                        item_availability,
                    ));
            }
            _ => {}
        }

        let inside_session_factory = matches!(
            item,
            Item::Fn(function)
                if role == PackageRole::Server
                    && module == SESSION_FACTORY_MODULE
                    && function.sig.ident == SESSION_FACTORY_NAME
        );
        collect_expression_surfaces(
            item,
            module,
            &item_cfg,
            item_availability,
            inside_session_factory,
            builder,
        );
    }
}

pub(super) fn call_surfaces(
    values: BTreeMap<(String, String, usize, Vec<String>, String), usize>,
) -> Vec<CallSurface> {
    values
        .into_iter()
        .map(
            |((module, callee, argument_count, cfg, source_class), count)| CallSurface {
                module,
                callee,
                argument_count,
                cfg,
                source_class,
                count,
            },
        )
        .collect()
}

pub(super) fn unique_contract_type<'a>(
    types: &'a BTreeMap<String, Vec<SessionContractTypeDefinition>>,
    name: &str,
) -> Result<&'a SessionContractTypeDefinition, String> {
    let definitions = types
        .get(name)
        .ok_or_else(|| format!("missing session contract type {name}"))?;
    let [definition] = definitions.as_slice() else {
        let modules: Vec<_> = definitions
            .iter()
            .map(|definition| definition.surface.definition.module.as_str())
            .collect();
        return Err(format!(
            "session contract type {name} is ambiguous across logical modules {modules:?}"
        ));
    };
    Ok(definition)
}

pub(super) fn network_contract(
    types: &BTreeMap<String, Vec<SessionContractTypeDefinition>>,
) -> Result<
    (
        TypeSurface,
        Vec<TypeSurface>,
        BTreeSet<GeneratedSurfaceInput>,
    ),
    String,
> {
    let session_command = unique_contract_type(types, SESSION_COMMAND_NAME)?;
    if session_command.surface.kind != "enum" {
        return Err(format!("{SESSION_COMMAND_NAME} must remain an enum"));
    }
    let mut pending: Vec<_> = session_command.referenced_types.iter().cloned().collect();
    let mut visited = BTreeSet::from([SESSION_COMMAND_NAME.to_owned()]);
    let mut payloads = Vec::new();
    let mut generated_surface_inputs = session_command.generated_surface_inputs.clone();
    while let Some(name) = pending.pop() {
        if !visited.insert(name.clone()) {
            continue;
        }
        let Some(definitions) = types.get(&name) else {
            // Primitives and types imported from other packages are pinned by
            // the exact payload type expression but are outside this package's
            // local ownership closure.
            continue;
        };
        let [definition] = definitions.as_slice() else {
            let modules: Vec<_> = definitions
                .iter()
                .map(|definition| definition.surface.definition.module.as_str())
                .collect();
            return Err(format!(
                "transitive {SESSION_COMMAND_NAME} payload {name} is ambiguous across {modules:?}"
            ));
        };
        payloads.push(definition.surface.clone());
        generated_surface_inputs.extend(definition.generated_surface_inputs.iter().cloned());
        pending.extend(definition.referenced_types.iter().cloned());
    }
    payloads.sort();
    Ok((
        session_command.surface.clone(),
        payloads,
        generated_surface_inputs,
    ))
}

pub(super) fn session_helper_candidate_keys(
    caller_module: &str,
    path: &[String],
) -> BTreeSet<(String, String)> {
    let Some(name) = path.last().cloned() else {
        return BTreeSet::new();
    };
    let mut candidates = BTreeSet::new();
    if path.len() == 1 {
        candidates.insert((caller_module.to_owned(), name));
        return candidates;
    }

    let qualifiers = &path[..path.len() - 1];
    match qualifiers.first().map(String::as_str) {
        Some("crate") => {
            candidates.insert((qualifiers.join("::"), name));
        }
        Some("self") => {
            let suffix = &qualifiers[1..];
            let module = if suffix.is_empty() {
                caller_module.to_owned()
            } else {
                format!("{caller_module}::{}", suffix.join("::"))
            };
            candidates.insert((module, name));
        }
        Some("super") => {
            let mut module: Vec<_> = caller_module.split("::").collect();
            let mut index = 0;
            while qualifiers
                .get(index)
                .is_some_and(|segment| segment == "super")
            {
                if module.len() > 1 {
                    module.pop();
                }
                index += 1;
            }
            module.extend(qualifiers[index..].iter().map(String::as_str));
            candidates.insert((module.join("::"), name));
        }
        Some(_) | None => {
            candidates.insert((
                format!("{caller_module}::{}", qualifiers.join("::")),
                name.clone(),
            ));
            candidates.insert((format!("crate::{}", qualifiers.join("::")), name));
        }
    }
    candidates
}

impl BaselineBuilder {
    pub(super) fn session_helper_bodies(&self) -> Vec<SessionFactoryHelperSurface> {
        let mut helpers = BTreeSet::new();
        for (caller_module, path) in &self.session_factory_helper_calls {
            for key in session_helper_candidate_keys(caller_module, path) {
                if key
                    == (
                        SESSION_FACTORY_MODULE.to_owned(),
                        SESSION_FACTORY_NAME.to_owned(),
                    )
                {
                    continue;
                }
                if let Some(surfaces) = self.server_function_bodies.get(&key) {
                    helpers.extend(surfaces.iter().cloned());
                }
            }
        }
        helpers.into_iter().collect()
    }

    pub(super) fn finish(
        self,
        registry_accesses: RegistryAccessBaseline,
        persistence_accesses: PersistenceAccessBaseline,
        bridge_accesses: BridgeAccessBaseline,
    ) -> Result<SessionSyntaxBaseline, String> {
        let session_helper_bodies = self.session_helper_bodies();
        let network_contract = network_contract(&self.contract_types);
        let mut errors = self.errors;
        let world_session_definition = self.world_session_definition.ok_or_else(|| {
            format!("missing {WORLD_SESSION_MODULE}::{WORLD_SESSION_NAME} definition")
        });
        let session_resources_definition = self.session_resources_definition.ok_or_else(|| {
            format!("missing {SESSION_RESOURCES_MODULE}::{SESSION_RESOURCES_NAME} definition")
        });
        let session_factory_definition = self.session_factory_definition.ok_or_else(|| {
            format!("missing {SESSION_FACTORY_MODULE}::{SESSION_FACTORY_NAME} definition")
        });
        let session_factory_signature = self
            .session_factory_signature
            .ok_or_else(|| format!("missing {SESSION_FACTORY_NAME} signature"));
        let session_factory_body_fingerprint = self
            .session_factory_body_fingerprint
            .clone()
            .ok_or_else(|| format!("missing {SESSION_FACTORY_NAME} body fingerprint"));
        for result in [
            world_session_definition.as_ref().map(|_| ()),
            session_resources_definition.as_ref().map(|_| ()),
            session_factory_definition.as_ref().map(|_| ()),
            session_factory_signature.as_ref().map(|_| ()),
            session_factory_body_fingerprint.as_ref().map(|_| ()),
        ] {
            if let Err(error) = result {
                errors.push(error.clone());
            }
        }
        if let Err(error) = &network_contract {
            errors.push(error.clone());
        }
        if !errors.is_empty() {
            return Err(errors.join("\n"));
        }
        let (session_command, session_command_payload_types, network_generated_surface_inputs) =
            network_contract.expect("validated network contract");
        let mut generated_surface_inputs = self.generated_surface_inputs;
        generated_surface_inputs.extend(network_generated_surface_inputs);

        Ok(SessionSyntaxBaseline {
            world_session: WorldSessionSurface {
                definition: world_session_definition.expect("validated definition"),
                fields: self.world_session_fields.into_iter().collect(),
                impls: self
                    .world_session_impls
                    .into_iter()
                    .map(|(module, trait_path, cfg, source_class)| ImplSurface {
                        module,
                        trait_path,
                        cfg,
                        source_class,
                    })
                    .collect(),
                impl_items: self.world_session_impl_items.into_iter().collect(),
            },
            session_resources: SessionResourcesSurface {
                definition: session_resources_definition.expect("validated definition"),
                fields: self.session_resources_fields.into_iter().collect(),
                construction_sites: call_surfaces(self.session_resources_constructions),
            },
            session_factory: SessionFactorySurface {
                definition: session_factory_definition.expect("validated definition"),
                signature: session_factory_signature.expect("validated signature"),
                body_fingerprint: session_factory_body_fingerprint
                    .expect("validated body fingerprint"),
                session_helper_bodies,
                call_sites: call_surfaces(self.session_factory_calls),
                world_session_new_sites: call_surfaces(self.world_session_new_calls),
                setter_call_sites: call_surfaces(self.session_factory_setter_calls),
            },
            session_command,
            session_command_payload_types,
            generated_surface_inputs: generated_surface_inputs.into_iter().collect(),
            registry_accesses,
            persistence_accesses,
            bridge_accesses,
        })
    }
}
