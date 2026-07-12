//! JSON Schema validation for test assertions.
//!
//! Validates a JSON response body against a JSON Schema draft-07+ spec.
//! Uses the `jsonschema` crate for validation.

use crate::error::{Error, Result};
use serde_json::Value;

/// Validate a JSON response body against a JSON Schema.
///
/// Returns `Ok(())` if the data conforms to the schema, or an `Err` with
/// a detailed message listing every validation failure.
pub fn validate_json_schema(schema: &str, data: &str) -> Result<()> {
    let schema_value: Value = serde_json::from_str(schema)
        .map_err(|e| Error::assertion(format!("invalid JSON Schema: {e}")))?;
    let data_value: Value = serde_json::from_str(data)
        .map_err(|e| Error::assertion(format!("invalid JSON data: {e}")))?;

    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema_value)
        .map_err(|e| Error::assertion(format!("schema compilation error: {e}")))?;

    let result = compiled.validate(&data_value);
    match result {
        Ok(_) => Ok(()),
        Err(errors) => {
            let details: Vec<String> = errors
                .map(|e| format!("  - {}: {}", e.instance_path, e))
                .collect();
            Err(Error::assertion(format!(
                "JSON Schema validation failed ({} errors):\n{}",
                details.len(),
                details.join("\n")
            )))
        }
    }
}

/// Try to compile a schema string — useful for pre-checking user input.
pub fn validate_schema_is_valid(schema: &str) -> Result<()> {
    let schema_value: Value = serde_json::from_str(schema)
        .map_err(|e| Error::assertion(format!("invalid JSON: {e}")))?;
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .compile(&schema_value)
        .map_err(|e| Error::assertion(format!("schema compilation error: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validates_correct_data() {
        let schema = r#"{"type": "object", "properties": {"name": {"type": "string"}}, "required": ["name"]}"#;
        let data = r#"{"name": "Alice"}"#;
        assert!(validate_json_schema(schema, data).is_ok());
    }

    #[test]
    fn test_rejects_invalid_data() {
        let schema = r#"{"type": "object", "properties": {"age": {"type": "integer"}}, "required": ["age"]}"#;
        let data = r#"{"age": "not-a-number"}"#;
        assert!(validate_json_schema(schema, data).is_err());
    }

    #[test]
    fn test_rejects_missing_field() {
        let schema = r#"{"type": "object", "required": ["email"]}"#;
        let data = r#"{"name": "Bob"}"#;
        assert!(validate_json_schema(schema, data).is_err());
    }

    #[test]
    fn test_invalid_schema_returns_error() {
        let result = validate_schema_is_valid("{invalid json}");
        assert!(result.is_err());
    }

    #[test]
    fn test_nested_objects() {
        let schema = r#"{
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "id": {"type": "integer"},
                        "roles": {"type": "array", "items": {"type": "string"}}
                    },
                    "required": ["id"]
                }
            }
        }"#;
        let data = r#"{"user": {"id": 1, "roles": ["admin", "user"]}}"#;
        assert!(validate_json_schema(schema, data).is_ok());
    }
}
