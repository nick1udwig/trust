use std::fmt;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustMetadata {
    pub schema_version: u32,
    pub item_kind: String,
    pub item_id: String,
    pub rust_function_path: String,
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
            r#"{"schema_version":1,"item_id":"total:id:abc","item_kind":"total","rust_function_path":"id"}"#,
        )
        .unwrap();

        assert_eq!(metadata.schema_version, 1);
        assert_eq!(metadata.item_kind, "total");
        assert_eq!(metadata.rust_function_path, "id");
    }

    #[test]
    fn rejects_unknown_schema() {
        let err = parse_metadata_line(
            r#"{"schema_version":99,"item_id":"total:id:abc","item_kind":"total","rust_function_path":"id"}"#,
        )
        .unwrap_err();

        assert_eq!(err, MetadataError::UnsupportedSchema(99));
    }
}
