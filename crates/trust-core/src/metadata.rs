use std::fmt;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustMetadata {
    pub schema_version: u32,
    pub item_kind: String,
    pub item_id: String,
    pub rust_function_path: String,
    pub contracts_original: Vec<String>,
    pub contract_classes: Vec<String>,
    pub function_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetadataError {
    MissingField(&'static str),
    InvalidNumber(&'static str),
    UnsupportedSchema(u32),
}

impl fmt::Display for MetadataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MetadataError::MissingField(field) => {
                write!(f, "missing Trust metadata field `{field}`")
            }
            MetadataError::InvalidNumber(field) => {
                write!(f, "invalid numeric Trust metadata field `{field}`")
            }
            MetadataError::UnsupportedSchema(schema) => {
                write!(f, "unsupported Trust metadata schema version {schema}")
            }
        }
    }
}

impl std::error::Error for MetadataError {}

pub fn parse_metadata_line(line: &str) -> Result<TrustMetadata, MetadataError> {
    let schema_version = extract_u32(line, "schema_version")?;
    if schema_version != SCHEMA_VERSION {
        return Err(MetadataError::UnsupportedSchema(schema_version));
    }

    Ok(TrustMetadata {
        schema_version,
        item_kind: extract_string(line, "item_kind")?,
        item_id: extract_string(line, "item_id")?,
        rust_function_path: extract_string(line, "rust_function_path")?,
        contracts_original: extract_string_array(line, "contracts_original")?,
        contract_classes: extract_string_array(line, "contract_classes")?,
        function_source: extract_string(line, "function_source")?,
    })
}

fn extract_u32(input: &str, field: &'static str) -> Result<u32, MetadataError> {
    let after_colon = after_field_colon(input, field)?;
    let digits: String = after_colon
        .chars()
        .skip_while(|ch| ch.is_whitespace())
        .take_while(|ch| ch.is_ascii_digit())
        .collect();

    if digits.is_empty() {
        return Err(MetadataError::InvalidNumber(field));
    }

    digits
        .parse()
        .map_err(|_| MetadataError::InvalidNumber(field))
}

fn extract_string(input: &str, field: &'static str) -> Result<String, MetadataError> {
    let mut chars = after_field_colon(input, field)?
        .chars()
        .skip_while(|ch| ch.is_whitespace());

    if chars.next() != Some('"') {
        return Err(MetadataError::MissingField(field));
    }

    let mut out = String::new();
    let mut escaping = false;
    for ch in chars {
        if escaping {
            out.push(match ch {
                '"' => '"',
                '\\' => '\\',
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaping = false;
            continue;
        }

        match ch {
            '\\' => escaping = true,
            '"' => return Ok(out),
            other => out.push(other),
        }
    }

    Err(MetadataError::MissingField(field))
}

fn extract_string_array(input: &str, field: &'static str) -> Result<Vec<String>, MetadataError> {
    let mut chars = after_field_colon(input, field)?
        .chars()
        .skip_while(|ch| ch.is_whitespace())
        .peekable();

    if chars.next() != Some('[') {
        return Err(MetadataError::MissingField(field));
    }

    let mut values = Vec::new();
    loop {
        while matches!(chars.peek(), Some(ch) if ch.is_whitespace() || *ch == ',') {
            chars.next();
        }

        match chars.peek() {
            Some(']') => {
                chars.next();
                return Ok(values);
            }
            Some('"') => {
                chars.next();
            }
            _ => return Err(MetadataError::MissingField(field)),
        }

        let mut out = String::new();
        let mut escaping = false;
        for ch in chars.by_ref() {
            if escaping {
                out.push(match ch {
                    '"' => '"',
                    '\\' => '\\',
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    other => other,
                });
                escaping = false;
                continue;
            }

            match ch {
                '\\' => escaping = true,
                '"' => {
                    values.push(out);
                    break;
                }
                other => out.push(other),
            }
        }
    }
}

fn after_field_colon<'a>(input: &'a str, field: &'static str) -> Result<&'a str, MetadataError> {
    let needle = format!("\"{field}\"");
    let field_start = input
        .find(&needle)
        .ok_or(MetadataError::MissingField(field))?;
    let after_field = &input[field_start + needle.len()..];
    let colon = after_field
        .find(':')
        .ok_or(MetadataError::MissingField(field))?;
    Ok(&after_field[colon + 1..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_total_metadata() {
        let metadata = parse_metadata_line(
            r#"{"schema_version":1,"item_id":"total:id:abc","item_kind":"total","rust_function_path":"id","contracts_original":["x < i32::MAX"],"contract_classes":["given executable"],"function_source":"pub fn id(x: i32) -> i32 { x }"}"#,
        )
        .unwrap();

        assert_eq!(metadata.schema_version, 1);
        assert_eq!(metadata.item_kind, "total");
        assert_eq!(metadata.rust_function_path, "id");
        assert_eq!(metadata.contracts_original, ["x < i32::MAX"]);
        assert_eq!(metadata.contract_classes, ["given executable"]);
        assert_eq!(metadata.function_source, "pub fn id(x: i32) -> i32 { x }");
    }

    #[test]
    fn rejects_unknown_schema() {
        let err = parse_metadata_line(
            r#"{"schema_version":99,"item_id":"total:id:abc","item_kind":"total","rust_function_path":"id","contracts_original":[],"contract_classes":[],"function_source":"pub fn id(x: i32) -> i32 { x }"}"#,
        )
        .unwrap_err();

        assert_eq!(err, MetadataError::UnsupportedSchema(99));
    }

    #[test]
    fn parses_empty_string_array() {
        let metadata = parse_metadata_line(
            r#"{"schema_version":1,"item_id":"total:id:abc","item_kind":"total","rust_function_path":"id","contracts_original":[],"contract_classes":[],"function_source":"pub fn id(x: i32) -> i32 { x }"}"#,
        )
        .unwrap();

        assert!(metadata.contracts_original.is_empty());
    }
}
