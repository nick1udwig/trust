use proc_macro::TokenStream;
use std::collections::hash_map::DefaultHasher;
use std::env;
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::Path;

const SCHEMA_VERSION: u32 = 1;

#[proc_macro_attribute]
pub fn module(_attr: TokenStream, item: TokenStream) -> TokenStream {
    if !wrapper_active() {
        return compile_error("error[trust]: Trust verification requires trust-rustc");
    }

    let source = item.to_string();
    if let Err(message) = inspect_module(&source) {
        return compile_error(message);
    }

    item
}

#[proc_macro]
pub fn total(input: TokenStream) -> TokenStream {
    if !wrapper_active() {
        return compile_error("error[trust]: Trust verification requires trust-rustc");
    }

    let source = input.to_string();
    let total = match parse_total_source(&source) {
        Ok(total) => total,
        Err(message) => return compile_error(message),
    };

    let metadata = metadata_json(&total.fn_info, &source, &total.fn_source, &total.contracts);
    if let Err(err) = write_metadata_sidecar(&metadata) {
        return compile_error(&format!(
            "error[trust]: failed to write Trust metadata: {err}"
        ));
    }

    let const_name = format!(
        "__TRUST_META_{}_{}",
        sanitize_ident(&total.fn_info.name),
        short_hash(&source)
    );
    let function = render_function(&total);
    let expanded = format!(
        r###"
        {input}

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        const {const_name}: &str = r##"{metadata}"##;
        "###,
        input = function,
        const_name = const_name,
        metadata = metadata
    );

    expanded.parse().unwrap_or_else(|_| {
        compile_error("error[trust]: failed to generate Rust for trust::total!")
    })
}

#[proc_macro]
pub fn trusted_model(input: TokenStream) -> TokenStream {
    if !wrapper_active() {
        return compile_error("error[trust]: Trust verification requires trust-rustc");
    }

    let source = input.to_string();
    if source.trim().is_empty() {
        return compile_error("error[trust]: trust::trusted_model! requires a declaration");
    }

    let metadata = trusted_model_metadata_json(&source);
    if let Err(err) = write_metadata_sidecar(&metadata) {
        return compile_error(&format!(
            "error[trust]: failed to write Trust metadata: {err}"
        ));
    }

    let const_name = format!("__TRUST_TRUSTED_MODEL_STUB_{}", short_hash(&source));
    let expanded = format!(
        r###"
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        const {const_name}: &str = r##"{metadata}"##;
        "###,
        const_name = const_name,
        metadata = metadata
    );

    expanded.parse().unwrap_or_else(|_| {
        compile_error("error[trust]: failed to generate Rust for trust::trusted_model!")
    })
}

#[proc_macro_derive(TrustModel)]
pub fn derive_trust_model(input: TokenStream) -> TokenStream {
    if !wrapper_active() {
        return compile_error("error[trust]: Trust verification requires trust-rustc");
    }

    let source = input.to_string();
    let model = match parse_trust_model_source(&source) {
        Ok(model) => model,
        Err(message) => return compile_error(message),
    };
    let metadata = model_metadata_json(&model, &source);
    if let Err(err) = write_metadata_sidecar(&metadata) {
        return compile_error(&format!(
            "error[trust]: failed to write Trust metadata: {err}"
        ));
    }

    let const_name = format!(
        "__TRUST_MODEL_META_{}_{}",
        sanitize_ident(&model.name),
        short_hash(&source)
    );
    let name = &model.name;
    let expanded = format!(
        r###"
        impl ::trust::TrustModel for {name} {{}}

        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        const {const_name}: &str = r##"{metadata}"##;
        "###,
        name = name,
        const_name = const_name,
        metadata = metadata
    );

    expanded.parse().unwrap_or_else(|_| {
        compile_error("error[trust]: failed to generate Rust for #[derive(TrustModel)]")
    })
}

#[derive(Debug)]
struct FnInfo {
    name: String,
    visibility: &'static str,
}

#[derive(Debug)]
struct ModelInfo {
    name: String,
}

#[derive(Debug)]
struct TotalExpansion {
    fn_source: String,
    fn_info: FnInfo,
    contracts: Vec<Contract>,
}

#[derive(Debug)]
struct Contract {
    class: &'static str,
    expression_code: String,
    expression_display: String,
}

fn parse_total_source(source: &str) -> Result<TotalExpansion, &'static str> {
    let mut rest = source.trim();
    let mut contracts = Vec::new();

    while !rest.is_empty() {
        if let Some(after_given) = strip_keyword(rest, "given") {
            let (class, after_kind) = if let Some(after_executable) =
                strip_keyword(after_given, "executable")
            {
                ("given executable", after_executable)
            } else if let Some(after_ghost) = strip_keyword(after_given, "ghost") {
                ("given ghost", after_ghost)
            } else {
                return Err("error[trust]: `given` must be `given executable` or `given ghost`");
            };
            let Some((block, after_block)) = extract_braced(after_kind.trim_start()) else {
                return Err("error[trust]: `given` requires a braced contract block");
            };

            for expression in split_contract_expressions(block) {
                if class == "given executable" {
                    validate_executable_contract(&expression)?;
                }
                contracts.push(Contract {
                    class,
                    expression_display: normalize_contract_display(&expression),
                    expression_code: expression,
                });
            }
            rest = after_block.trim_start();
            continue;
        }

        let Some(after_gives) = strip_keyword(rest, "gives") else {
            break;
        };
        let (class, after_kind) =
            if let Some(after_executable) = strip_keyword(after_gives, "executable") {
                ("gives executable", after_executable)
            } else if let Some(after_ghost) = strip_keyword(after_gives, "ghost") {
                ("gives ghost", after_ghost)
            } else {
                return Err("error[trust]: `gives` must be `gives executable` or `gives ghost`");
            };
        let Some((_binder, after_binder)) = extract_pipe_binder(after_kind.trim_start()) else {
            return Err("error[trust]: `gives` requires a result binder like `|out|`");
        };
        let Some((block, after_block)) = extract_braced(after_binder.trim_start()) else {
            return Err("error[trust]: `gives` requires a braced contract block");
        };

        for expression in split_contract_expressions(block) {
            contracts.push(Contract {
                class,
                expression_display: normalize_contract_display(&expression),
                expression_code: expression,
            });
        }
        rest = after_block.trim_start();
    }

    let fn_source = rest.to_string();
    let fn_info = inspect_total_fn(&fn_source)?;
    if fn_info.visibility == "public"
        && contracts
            .iter()
            .any(|contract| contract.class == "given ghost")
    {
        return Err("error[trust]: public function has ghost-only precondition");
    }

    Ok(TotalExpansion {
        fn_source,
        fn_info,
        contracts,
    })
}

fn inspect_total_fn(input: &str) -> Result<FnInfo, &'static str> {
    let tokens = lex(input);
    let mut fn_positions = Vec::new();

    for (idx, token) in tokens.iter().enumerate() {
        if matches!(token, LexToken::Ident(ident) if ident == "fn") {
            fn_positions.push(idx);
        }
    }

    match fn_positions.len() {
        0 => return Err("error[trust]: trust::total! requires exactly one Rust fn item"),
        1 => {}
        _ => return Err("error[trust]: trust::total! accepts exactly one Rust fn item"),
    }

    let fn_idx = fn_positions[0];
    let name = match tokens.get(fn_idx + 1) {
        Some(LexToken::Ident(ident)) => ident.to_string(),
        _ => return Err("error[trust]: trust::total! could not read function name"),
    };

    if has_ident_before(&tokens, fn_idx, "async") {
        return Err("error[trust]: async functions are not supported in Trust MVP");
    }
    if has_ident_before(&tokens, fn_idx, "unsafe") {
        return Err("error[trust]: unsafe functions are not supported in Trust MVP");
    }
    if has_ident_before(&tokens, fn_idx, "extern") {
        return Err("error[trust]: extern functions are not supported in Trust MVP");
    }
    if matches!(tokens.get(fn_idx + 2), Some(LexToken::Punct('<'))) {
        return Err("error[trust]: generic total functions are not supported in MVP");
    }
    if !has_body_group(&tokens) {
        return Err("error[trust]: trust::total! requires a function body");
    }

    let visibility = if matches!(tokens.first(), Some(LexToken::Ident(ident)) if ident == "pub") {
        "public"
    } else {
        "private"
    };

    Ok(FnInfo { name, visibility })
}

fn parse_trust_model_source(source: &str) -> Result<ModelInfo, &'static str> {
    let tokens = lex(source);
    let Some(struct_idx) = tokens
        .iter()
        .position(|token| matches!(token, LexToken::Ident(ident) if ident == "struct"))
    else {
        return Err("error[trust]: TrustModel derive supports simple structs in this MVP");
    };

    let name = match tokens.get(struct_idx + 1) {
        Some(LexToken::Ident(ident)) => ident.to_string(),
        _ => return Err("error[trust]: TrustModel derive could not read type name"),
    };
    if matches!(tokens.get(struct_idx + 2), Some(LexToken::Punct('<'))) {
        return Err("error[trust]: generic TrustModel types are not supported in MVP");
    }
    if !has_body_group(&tokens) {
        return Err("error[trust]: TrustModel derive requires a braced struct body");
    }

    Ok(ModelInfo { name })
}

fn inspect_module(input: &str) -> Result<(), &'static str> {
    let tokens = lex(input);
    let Some(module_body_start) = tokens
        .iter()
        .position(|token| matches!(token, LexToken::Punct('{')))
    else {
        return Err("error[trust]: #[trust::module] must be applied to a module");
    };

    let mut idx = module_body_start + 1;
    let mut depth = 1usize;

    while idx < tokens.len() && depth > 0 {
        match &tokens[idx] {
            LexToken::Ident(ident) if depth == 1 && ident == "total" => {
                idx = skip_macro_invocation_group(&tokens, idx);
            }
            LexToken::Ident(ident) if depth == 1 && ident == "unsafe" => {
                return Err(
                    "error[trust]: unsafe items are not allowed inside #[trust::module] in the MVP",
                );
            }
            LexToken::Ident(ident) if depth == 1 && ident == "fn" => {
                return Err("error[trust]: unverified Rust functions are not allowed inside #[trust::module] in the MVP");
            }
            LexToken::Punct('{') => {
                depth += 1;
                idx += 1;
            }
            LexToken::Punct('}') => {
                depth -= 1;
                idx += 1;
            }
            _ => idx += 1,
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LexToken {
    Ident(String),
    Punct(char),
}

fn lex(input: &str) -> Vec<LexToken> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.peek().copied() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }

        if ch.is_ascii_alphabetic() || ch == '_' {
            let mut ident = String::new();
            while let Some(ch) = chars.peek().copied() {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    ident.push(ch);
                    chars.next();
                } else {
                    break;
                }
            }
            tokens.push(LexToken::Ident(ident));
            continue;
        }

        tokens.push(LexToken::Punct(ch));
        chars.next();
    }

    tokens
}

fn has_ident_before(tokens: &[LexToken], end: usize, ident_name: &str) -> bool {
    tokens[..end]
        .iter()
        .any(|token| matches!(token, LexToken::Ident(ident) if ident == ident_name))
}

fn has_body_group(tokens: &[LexToken]) -> bool {
    tokens
        .iter()
        .any(|token| matches!(token, LexToken::Punct('{')))
}

fn skip_macro_invocation_group(tokens: &[LexToken], ident_idx: usize) -> usize {
    if !matches!(tokens.get(ident_idx + 1), Some(LexToken::Punct('!'))) {
        return ident_idx + 1;
    }

    let Some(open_idx) = tokens[ident_idx + 2..]
        .iter()
        .position(|token| matches!(token, LexToken::Punct('{')))
        .map(|offset| ident_idx + 2 + offset)
    else {
        return ident_idx + 1;
    };

    let mut depth = 0usize;
    for (idx, token) in tokens.iter().enumerate().skip(open_idx) {
        match token {
            LexToken::Punct('{') => depth += 1,
            LexToken::Punct('}') => {
                depth -= 1;
                if depth == 0 {
                    return idx + 1;
                }
            }
            _ => {}
        }
    }

    tokens.len()
}

fn render_function(total: &TotalExpansion) -> String {
    if total.fn_info.visibility != "public" || total.contracts.is_empty() {
        return total.fn_source.clone();
    }

    let Some(body_start) = total.fn_source.find('{') else {
        return total.fn_source.clone();
    };
    let Some(body_end) = total.fn_source.rfind('}') else {
        return total.fn_source.clone();
    };

    let mut precondition_assertions = String::new();
    let mut postcondition_assertions = String::new();
    for contract in &total.contracts {
        if contract.class == "given executable" {
            precondition_assertions.push_str(&format!(
                "::trust::__rt::assert_precondition(({}), {:?}, {:?});",
                contract.expression_code, total.fn_info.name, contract.expression_display
            ));
        } else if contract.class == "gives executable" {
            let expression = replace_result_binder(&contract.expression_code);
            postcondition_assertions.push_str(&format!(
                "::trust::__rt::assert_postcondition(({}), {:?}, {:?});",
                expression, total.fn_info.name, contract.expression_display
            ));
        }
    }

    if postcondition_assertions.is_empty() {
        let mut function = total.fn_source.clone();
        function.insert_str(body_start + 1, &precondition_assertions);
        return function;
    }

    let signature = &total.fn_source[..body_start + 1];
    let body = &total.fn_source[body_start + 1..body_end];
    let suffix = &total.fn_source[body_end..];
    format!(
        "{signature}{precondition_assertions}let __trust_out = {{ {body} }};{postcondition_assertions}__trust_out{suffix}"
    )
}

fn metadata_json(
    fn_info: &FnInfo,
    source: &str,
    function_source: &str,
    contracts: &[Contract],
) -> String {
    let hash = short_hash(source);
    format!(
        "{{\"schema_version\":{schema},\"trust_macro_version\":\"{version}\",\"module_id\":\"unknown\",\"item_id\":\"total:{name}:{hash}\",\"item_kind\":\"total\",\"source_span\":\"unknown\",\"rust_function_path\":\"{name}\",\"visibility\":\"{visibility}\",\"contracts_original\":{contracts_original},\"contracts_normalized\":{contracts_normalized},\"contract_classes\":{contract_classes},\"assertion_policy\":\"always\",\"function_source\":\"{function_source}\",\"body_hash_placeholder\":\"{hash}\",\"trust_model_dependencies\":[]}}",
        schema = SCHEMA_VERSION,
        version = env!("CARGO_PKG_VERSION"),
        name = json_escape(&fn_info.name),
        hash = hash,
        visibility = fn_info.visibility,
        function_source = json_escape(function_source),
        contracts_original = json_string_array(
            contracts
                .iter()
                .map(|contract| contract.expression_display.as_str())
        ),
        contracts_normalized = json_string_array(
            contracts
                .iter()
                .map(|contract| contract.expression_display.as_str())
        ),
        contract_classes = json_string_array(contracts.iter().map(|contract| contract.class)),
    )
}

fn model_metadata_json(model: &ModelInfo, source: &str) -> String {
    let hash = short_hash(source);
    format!(
        "{{\"schema_version\":{schema},\"trust_macro_version\":\"{version}\",\"module_id\":\"unknown\",\"item_id\":\"model:{name}:{hash}\",\"item_kind\":\"trust_model\",\"source_span\":\"unknown\",\"rust_function_path\":\"{name}\",\"visibility\":\"unknown\",\"contracts_original\":[],\"contracts_normalized\":[],\"contract_classes\":[],\"assertion_policy\":\"always\",\"function_source\":\"{source}\",\"body_hash_placeholder\":\"{hash}\",\"trust_model_dependencies\":[]}}",
        schema = SCHEMA_VERSION,
        version = env!("CARGO_PKG_VERSION"),
        name = json_escape(&model.name),
        hash = hash,
        source = json_escape(source),
    )
}

fn trusted_model_metadata_json(source: &str) -> String {
    let hash = short_hash(source);
    format!(
        "{{\"schema_version\":{schema},\"trust_macro_version\":\"{version}\",\"module_id\":\"unknown\",\"item_id\":\"trusted_model_stub:{hash}\",\"item_kind\":\"trusted_model_stub\",\"source_span\":\"unknown\",\"rust_function_path\":\"trusted_model_stub\",\"visibility\":\"unknown\",\"contracts_original\":[],\"contracts_normalized\":[],\"contract_classes\":[],\"assertion_policy\":\"always\",\"function_source\":\"{source}\",\"body_hash_placeholder\":\"{hash}\",\"trust_model_dependencies\":[]}}",
        schema = SCHEMA_VERSION,
        version = env!("CARGO_PKG_VERSION"),
        hash = hash,
        source = json_escape(source),
    )
}

fn strip_keyword<'a>(input: &'a str, keyword: &str) -> Option<&'a str> {
    let input = input.trim_start();
    let rest = input.strip_prefix(keyword)?;
    let boundary = rest
        .chars()
        .next()
        .is_none_or(|ch| !(ch.is_ascii_alphanumeric() || ch == '_'));
    boundary.then_some(rest)
}

fn extract_braced(input: &str) -> Option<(&str, &str)> {
    let input = input.trim_start();
    if !input.starts_with('{') {
        return None;
    }

    let mut depth = 0usize;
    let mut body_start = None;
    for (idx, ch) in input.char_indices() {
        match ch {
            '{' => {
                if depth == 0 {
                    body_start = Some(idx + ch.len_utf8());
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let body_start = body_start?;
                    return Some((&input[body_start..idx], &input[idx + ch.len_utf8()..]));
                }
            }
            _ => {}
        }
    }

    None
}

fn extract_pipe_binder(input: &str) -> Option<(&str, &str)> {
    let input = input.trim_start();
    let after_open = input.strip_prefix('|')?;
    let close_idx = after_open.find('|')?;
    let binder = after_open[..close_idx].trim();
    if binder.is_empty() {
        return None;
    }
    Some((binder, &after_open[close_idx + 1..]))
}

fn split_contract_expressions(block: &str) -> Vec<String> {
    block
        .split(';')
        .map(str::trim)
        .filter(|expression| !expression.is_empty())
        .map(str::to_string)
        .collect()
}

fn normalize_contract_display(expression: &str) -> String {
    expression
        .replace(" . ", ".")
        .replace(" (", "(")
        .replace("( ", "(")
        .replace(" )", ")")
        .replace("[ ", "[")
        .replace(" ]", "]")
}

fn replace_result_binder(expression: &str) -> String {
    let tokens = lex(expression);
    let mut out = String::new();
    for token in tokens {
        match token {
            LexToken::Ident(ident) if ident == "out" => out.push_str("__trust_out"),
            LexToken::Ident(ident) => out.push_str(&ident),
            LexToken::Punct(punct) => out.push(punct),
        }
    }
    out
}

fn validate_executable_contract(expression: &str) -> Result<(), &'static str> {
    let tokens = lex(expression);

    for (idx, token) in tokens.iter().enumerate() {
        match token {
            LexToken::Ident(ident) if ident == "forall" || ident == "exists" => {
                return Err(
                    "error[trust]: quantifiers are not supported in executable preconditions",
                );
            }
            LexToken::Punct('|') => {
                return Err("error[trust]: closures are not supported in executable preconditions");
            }
            LexToken::Punct('(') if idx > 0 => {
                let Some(LexToken::Ident(callee)) = tokens.get(idx - 1) else {
                    continue;
                };
                let is_method =
                    idx >= 2 && matches!(tokens.get(idx - 2), Some(LexToken::Punct('.')));
                if !is_method || callee != "len" {
                    return Err(
                        "error[trust]: unsupported function call in executable precondition",
                    );
                }
            }
            _ => {}
        }
    }

    Ok(())
}

fn json_string_array<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let values = values
        .map(|value| format!("\"{}\"", json_escape(value)))
        .collect::<Vec<_>>()
        .join(",");
    format!("[{values}]")
}

fn write_metadata_sidecar(metadata: &str) -> Result<(), String> {
    let Ok(path) = env::var("TRUST_METADATA_OUT") else {
        return Ok(());
    };

    let path = Path::new(&path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| err.to_string())?;
    writeln!(file, "{metadata}").map_err(|err| err.to_string())
}

fn wrapper_active() -> bool {
    env::var("TRUST_RUSTC_ACTIVE").as_deref() == Ok("1")
        || env::var("TRUST_MACRO_UNIT_TEST").is_ok()
}

fn compile_error(message: &str) -> TokenStream {
    format!("compile_error!({:?});", message)
        .parse()
        .expect("compile_error! should parse")
}

fn short_hash(input: &str) -> String {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn sanitize_ident(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn json_escape(input: &str) -> String {
    input
        .chars()
        .flat_map(|ch| match ch {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspects_single_public_function() {
        let info = inspect_total_fn("pub fn id_i32(x: i32) -> i32 { x }").unwrap();

        assert_eq!(info.name, "id_i32");
        assert_eq!(info.visibility, "public");
    }

    #[test]
    fn rejects_multiple_functions() {
        let err = parse_total_source("fn a() {} fn b() {}").unwrap_err();

        assert_eq!(
            err,
            "error[trust]: trust::total! accepts exactly one Rust fn item"
        );
    }

    #[test]
    fn module_accepts_total_function() {
        inspect_module(
            r#"
            mod verified {
                trust::total! {
                    pub fn id_i32(x: i32) -> i32 { x }
                }
            }
            "#,
        )
        .unwrap();
    }

    #[test]
    fn module_rejects_unverified_function() {
        let err = inspect_module(
            r#"
            mod verified {
                pub fn helper(x: i32) -> i32 { x }
            }
            "#,
        )
        .unwrap_err();

        assert_eq!(
            err,
            "error[trust]: unverified Rust functions are not allowed inside #[trust::module] in the MVP"
        );
    }

    #[test]
    fn trust_model_derive_accepts_simple_struct() {
        let model =
            parse_trust_model_source("pub struct Account { pub id: u64, pub balance: i64 }")
                .unwrap();

        assert_eq!(model.name, "Account");
        assert!(model_metadata_json(
            &model,
            "pub struct Account { pub id: u64, pub balance: i64 }"
        )
        .contains("\"item_kind\":\"trust_model\""));
    }

    #[test]
    fn trust_model_derive_rejects_generic_struct() {
        let err = parse_trust_model_source("pub struct Boxed<T> { pub value: T }").unwrap_err();

        assert_eq!(
            err,
            "error[trust]: generic TrustModel types are not supported in MVP"
        );
    }

    #[test]
    fn trusted_model_metadata_is_inert_stub() {
        let metadata = trusted_model_metadata_json("axiom false_is_true: false;");

        assert!(metadata.contains("\"item_kind\":\"trusted_model_stub\""));
        assert!(metadata.contains("\"contracts_original\":[]"));
    }

    #[test]
    fn total_parses_executable_given_and_renders_assertion() {
        let total = parse_total_source(
            "given executable { i < xs . len(); } pub fn get(xs: &[i32], i: usize) -> i32 { xs[i] }",
        )
        .unwrap();

        assert_eq!(total.fn_info.name, "get");
        assert_eq!(total.contracts[0].expression_display, "i < xs.len()");

        let function = render_function(&total);
        assert!(function.contains(
            "::trust::__rt::assert_precondition((i < xs . len()), \"get\", \"i < xs.len()\");"
        ));
    }

    #[test]
    fn total_parses_private_ghost_given_without_runtime_assertion() {
        let total = parse_total_source(
            "given ghost { sorted(xs); } fn private_first_sorted(xs: &[i32]) -> i32 { 0 }",
        )
        .unwrap();

        assert_eq!(total.fn_info.visibility, "private");
        assert_eq!(total.contracts[0].class, "given ghost");
        assert_eq!(total.contracts[0].expression_display, "sorted(xs)");

        let function = render_function(&total);
        assert!(!function.contains("assert_precondition"));
        assert!(!function.contains("sorted(xs)"));
    }

    #[test]
    fn total_rejects_public_ghost_given() {
        let err = parse_total_source(
            "given ghost { sorted(xs); } pub fn first_sorted(xs: &[i32]) -> i32 { xs[0] }",
        )
        .unwrap_err();

        assert_eq!(
            err,
            "error[trust]: public function has ghost-only precondition"
        );
    }

    #[test]
    fn total_parses_gives_contracts_without_runtime_precondition_assertion() {
        let total = parse_total_source(
            "gives executable | out | { out == x; } pub fn id_i32(x: i32) -> i32 { x }",
        )
        .unwrap();

        assert_eq!(total.contracts[0].class, "gives executable");
        assert_eq!(total.contracts[0].expression_display, "out == x");

        let function = render_function(&total);
        assert!(!function.contains("assert_precondition"));
        assert!(function.contains("assert_postcondition"));
        assert!(function.contains("__trust_out==x"));
    }

    #[test]
    fn executable_given_rejects_quantifier() {
        let err =
            parse_total_source("given executable { forall i; } pub fn id_i32(x: i32) -> i32 { x }")
                .unwrap_err();

        assert_eq!(
            err,
            "error[trust]: quantifiers are not supported in executable preconditions"
        );
    }

    #[test]
    fn executable_given_rejects_arbitrary_call() {
        let err = parse_total_source(
            "given executable { valid_index(i); } pub fn id_i32(x: i32) -> i32 { x }",
        )
        .unwrap_err();

        assert_eq!(
            err,
            "error[trust]: unsupported function call in executable precondition"
        );
    }
}
