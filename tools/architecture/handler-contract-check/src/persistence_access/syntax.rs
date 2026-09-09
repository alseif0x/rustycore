//! Normalised syntax spelling shared by every analysis phase.
//!
//! Separated from the persistence-access root under #634. Behaviour is
//! preserved; this module owns no new state.

use super::*;

pub(super) fn normalized_ident(ident: &proc_macro2::Ident) -> String {
    let value = ident.to_string();
    value.strip_prefix("r#").unwrap_or(&value).to_owned()
}

pub(super) fn normalized_tokens(value: &impl ToTokens) -> String {
    value.to_token_stream().to_string()
}

pub(super) fn normalized_visibility(visibility: &Visibility) -> String {
    normalized_tokens(visibility)
}

pub(super) fn canonical_path(path: &syn::Path) -> String {
    path.segments
        .iter()
        .map(|segment| normalized_ident(&segment.ident))
        .collect::<Vec<_>>()
        .join("::")
}

pub(super) fn path_names(path: &syn::Path) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| normalized_ident(&segment.ident))
        .collect()
}

pub(super) fn last_path_name(path: &syn::Path) -> Option<String> {
    path.segments
        .last()
        .map(|segment| normalized_ident(&segment.ident))
}

/// Names of the type parameters declared by a signature (`fn make<T, U>`),
/// in declaration order, so an explicit turbofish at the call site can be
/// mapped positionally onto the recorded return.
pub(super) fn generic_type_param_names(generics: &syn::Generics) -> Vec<String> {
    generics
        .params
        .iter()
        .filter_map(|parameter| match parameter {
            syn::GenericParam::Type(parameter) => Some(normalized_ident(&parameter.ident)),
            _ => None,
        })
        .collect()
}

pub(super) const RECEIVER_INPUT_MARKER: &str = "$receiver";

#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct GenericInputSpec {
    pub(super) params: BTreeSet<String>,
    pub(super) tuple_paths: BTreeMap<String, Vec<Vec<usize>>>,
}

pub(super) fn projected_generic_argument(
    argument: &VariableInfo,
    input: &GenericInputSpec,
    param: &str,
) -> VariableInfo {
    let Some(paths) = input.tuple_paths.get(param) else {
        return argument.clone();
    };
    let mut result = VariableInfo::default();
    let mut projected_any = false;
    for path in paths {
        let mut projected = argument;
        let mut complete = true;
        for index in path {
            let Some(item) = projected.tuple_items.get(*index) else {
                complete = false;
                break;
            };
            projected = item;
        }
        if complete {
            projected_any = true;
            result.union(projected);
        }
    }
    if projected_any {
        result
    } else {
        argument.clone()
    }
}

pub(super) fn collect_generic_tuple_paths(
    ty: &Type,
    params: &BTreeSet<String>,
    path: &mut Vec<usize>,
    output: &mut BTreeMap<String, Vec<Vec<usize>>>,
) {
    match ty {
        Type::Path(type_path) => {
            if let Some(name) = last_path_name(&type_path.path)
                && params.contains(&name)
                && !path.is_empty()
            {
                output.entry(name).or_default().push(path.clone());
            }
            for segment in &type_path.path.segments {
                if let syn::PathArguments::AngleBracketed(arguments) = &segment.arguments {
                    for argument in &arguments.args {
                        if let syn::GenericArgument::Type(inner) = argument {
                            collect_generic_tuple_paths(inner, params, path, output);
                        }
                    }
                }
            }
        }
        Type::Tuple(tuple) => {
            for (index, element) in tuple.elems.iter().enumerate() {
                path.push(index);
                collect_generic_tuple_paths(element, params, path, output);
                path.pop();
            }
        }
        Type::Reference(reference) => {
            collect_generic_tuple_paths(&reference.elem, params, path, output);
        }
        Type::Ptr(pointer) => collect_generic_tuple_paths(&pointer.elem, params, path, output),
        Type::Paren(paren) => collect_generic_tuple_paths(&paren.elem, params, path, output),
        Type::Group(group) => collect_generic_tuple_paths(&group.elem, params, path, output),
        _ => {}
    }
}

pub(super) fn generic_params_by_input(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
    params: &[String],
) -> Vec<GenericInputSpec> {
    let all_params = params.iter().cloned().collect::<BTreeSet<_>>();
    inputs
        .iter()
        .map(|input| match input {
            FnArg::Typed(typed) => {
                let params = params
                    .iter()
                    .filter(|param| {
                        tokens_contain_identifier(
                            typed.ty.to_token_stream(),
                            &BTreeSet::from([(*param).clone()]),
                        )
                    })
                    .cloned()
                    .collect();
                let mut tuple_paths = BTreeMap::new();
                collect_generic_tuple_paths(
                    &typed.ty,
                    &all_params,
                    &mut Vec::new(),
                    &mut tuple_paths,
                );
                GenericInputSpec {
                    params,
                    tuple_paths,
                }
            }
            // Keep the receiver in the formal-input sequence. A method-call
            // expression omits it and skips this marker below, while UFCS
            // supplies it explicitly and therefore keeps later generic
            // arguments aligned with their declared inputs.
            FnArg::Receiver(_) => GenericInputSpec {
                params: BTreeSet::from([RECEIVER_INPUT_MARKER.to_owned()]),
                tuple_paths: BTreeMap::new(),
            },
        })
        .collect()
}

pub(super) fn canonical_call(call: &ExprCall) -> String {
    let arguments = call
        .args
        .iter()
        .map(normalized_tokens)
        .collect::<Vec<_>>()
        .join(" | ");
    format!("{}({arguments})", normalized_tokens(&call.func))
}

pub(super) fn canonical_method(method: &ExprMethodCall) -> String {
    let arguments = method
        .args
        .iter()
        .map(normalized_tokens)
        .collect::<Vec<_>>()
        .join(" | ");
    format!(
        "{}.{}({arguments})",
        normalized_tokens(&method.receiver),
        normalized_ident(&method.method)
    )
}

pub(super) fn sqlx_calls_in_tokens(tokens: TokenStream, output: &mut Vec<(String, String)>) {
    let trees = tokens.into_iter().collect::<Vec<_>>();
    let mut index = 0;
    while index < trees.len() {
        if let TokenTree::Group(group) = &trees[index] {
            sqlx_calls_in_tokens(group.stream(), output);
        }
        let is_sqlx =
            matches!(&trees[index], TokenTree::Ident(ident) if normalized_ident(ident) == "sqlx");
        if is_sqlx && index + 3 < trees.len() {
            let separator = matches!(&trees[index + 1], TokenTree::Punct(punct) if punct.as_char() == ':')
                && matches!(&trees[index + 2], TokenTree::Punct(punct) if punct.as_char() == ':');
            if separator && let TokenTree::Ident(callable) = &trees[index + 3] {
                let callable = normalized_ident(callable);
                if is_query_name(&callable) {
                    let fingerprint = trees[index + 4..]
                        .iter()
                        .find_map(|token| match token {
                            TokenTree::Group(group)
                                if group.delimiter() == proc_macro2::Delimiter::Parenthesis =>
                            {
                                Some(format!("sqlx::{callable}({})", group.stream()))
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| format!("sqlx::{callable}(opaque-macro-arguments)"));
                    output.push((callable, fingerprint));
                }
            }
        }
        index += 1;
    }
}

pub(super) fn persistence_methods_in_tokens(
    tokens: TokenStream,
    names: &BTreeSet<String>,
    output: &mut Vec<String>,
) {
    let trees = tokens.into_iter().collect::<Vec<_>>();
    for (index, token) in trees.iter().enumerate() {
        if let TokenTree::Group(group) = token {
            persistence_methods_in_tokens(group.stream(), names, output);
        }
        if matches!(token, TokenTree::Punct(punct) if punct.as_char() == '.')
            && let Some(TokenTree::Ident(method)) = trees.get(index + 1)
        {
            let method = normalized_ident(method);
            if names.contains(&method) {
                output.push(method);
            }
        }
    }
}
