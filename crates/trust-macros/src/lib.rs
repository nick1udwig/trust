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

    let metadata = metadata_json(&total.fn_info, &source, &total.contracts);
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

#[derive(Debug)]
struct FnInfo {
    name: String,
    visibility: &'static str,
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

    loop {
        let Some(after_given) = strip_keyword(rest, "given") else {
            break;
        };
        let Some(after_executable) = strip_keyword(after_given, "executable") else {
            return Err("error[trust]: only `given executable` contracts are supported in this Trust MVP slice");
        };
        let Some((block, after_block)) = extract_braced(after_executable.trim_start()) else {
            return Err("error[trust]: `given executable` requires a braced contract block");
        };

        for expression in split_contract_expressions(block) {
            contracts.push(Contract {
                class: "given executable",
                expression_display: normalize_contract_display(&expression),
                expression_code: expression,
            });
        }
        rest = after_block.trim_start();
    }

    let fn_source = rest.to_string();
    let fn_info = inspect_total_fn(&fn_source)?;

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

    let mut assertions = String::new();
    for contract in &total.contracts {
        if contract.class != "given executable" {
            continue;
        }
        assertions.push_str(&format!(
            "::trust::__rt::assert_precondition(({}), {:?}, {:?});",
            contract.expression_code, total.fn_info.name, contract.expression_display
        ));
    }

    let mut function = total.fn_source.clone();
    function.insert_str(body_start + 1, &assertions);
    function
}

fn metadata_json(fn_info: &FnInfo, source: &str, contracts: &[Contract]) -> String {
    let hash = short_hash(source);
    format!(
        "{{\"schema_version\":{schema},\"trust_macro_version\":\"{version}\",\"module_id\":\"unknown\",\"item_id\":\"total:{name}:{hash}\",\"item_kind\":\"total\",\"source_span\":\"unknown\",\"rust_function_path\":\"{name}\",\"visibility\":\"{visibility}\",\"contracts_original\":{contracts_original},\"contracts_normalized\":{contracts_normalized},\"contract_classes\":{contract_classes},\"assertion_policy\":\"always\",\"body_hash_placeholder\":\"{hash}\",\"trust_model_dependencies\":[]}}",
        schema = SCHEMA_VERSION,
        version = env!("CARGO_PKG_VERSION"),
        name = json_escape(&fn_info.name),
        hash = hash,
        visibility = fn_info.visibility,
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
}
