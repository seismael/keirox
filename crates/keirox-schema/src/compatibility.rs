//! Schema compatibility levels and evolution rules per `KEI-ARC-024` §4.

use keirox_core::error::{KeiroxError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Compatibility level governing allowed schema mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityLevel {
    /// New schema can read data written by previous schema versions (e.g. only adding optional fields).
    Backward,
    /// Previous schema versions can read data written by new schema (e.g. only removing optional fields).
    Forward,
    /// Both Backward and Forward compatible.
    Full,
    /// No compatibility constraints enforced.
    None,
}

/// Primitive field type representation for schema validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    /// Boolean flag.
    Boolean,
    /// 32-bit signed integer.
    Int32,
    /// 64-bit signed integer.
    Int64,
    /// 32-bit floating point.
    Float32,
    /// 64-bit floating point.
    Float64,
    /// UTF-8 string.
    Utf8,
    /// Binary blob.
    Binary,
    /// Nested JSON object.
    Struct(HashMap<String, FieldType>),
}

/// Normalized schema definition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaDefinition {
    /// Mapping of field name to data type.
    pub fields: HashMap<String, FieldType>,
    /// Set of required (non-nullable) field names.
    pub required_fields: HashSet<String>,
}

impl SchemaDefinition {
    /// Create a new schema definition.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            required_fields: HashSet::new(),
        }
    }

    /// Add a field to the schema definition.
    pub fn add_field(&mut self, name: impl Into<String>, field_type: FieldType, required: bool) {
        let name_str = name.into();
        if required {
            self.required_fields.insert(name_str.clone());
        }
        self.fields.insert(name_str, field_type);
    }
}

impl Default for SchemaDefinition {
    fn default() -> Self {
        Self::new()
    }
}

/// Schema compatibility validator.
pub struct SchemaValidator;

impl SchemaValidator {
    /// Validate evolution from `old_schema` to `new_schema` against `level`.
    pub fn validate_evolution(
        old_schema: &SchemaDefinition,
        new_schema: &SchemaDefinition,
        level: CompatibilityLevel,
    ) -> Result<()> {
        match level {
            CompatibilityLevel::None => Ok(()),
            CompatibilityLevel::Backward => Self::check_backward(old_schema, new_schema),
            CompatibilityLevel::Forward => Self::check_forward(old_schema, new_schema),
            CompatibilityLevel::Full => {
                Self::check_backward(old_schema, new_schema)?;
                Self::check_forward(old_schema, new_schema)?;
                Ok(())
            }
        }
    }

    fn check_backward(old: &SchemaDefinition, new: &SchemaDefinition) -> Result<()> {
        // In backward compatibility, any new required field in `new` breaks old data reads
        for req in &new.required_fields {
            if !old.required_fields.contains(req) {
                return Err(KeiroxError::Internal(format!(
                    "Backward compatibility violation: newly introduced field '{req}' cannot be required"
                )));
            }
        }

        // Existing fields cannot have incompatible type changes
        for (name, old_type) in &old.fields {
            if let Some(new_type) = new.fields.get(name) {
                if old_type != new_type {
                    return Err(KeiroxError::Internal(format!(
                        "Backward compatibility violation: field '{name}' type changed from {old_type:?} to {new_type:?}"
                    )));
                }
            }
        }

        Ok(())
    }

    fn check_forward(old: &SchemaDefinition, new: &SchemaDefinition) -> Result<()> {
        // In forward compatibility, deleting a required field breaks old readers
        for req in &old.required_fields {
            if !new.fields.contains_key(req) {
                return Err(KeiroxError::Internal(format!(
                    "Forward compatibility violation: required field '{req}' was removed in new schema"
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backward_compatibility_rules() {
        let mut old = SchemaDefinition::new();
        old.add_field("id", FieldType::Int64, true);
        old.add_field("user", FieldType::Utf8, false);

        let mut valid_new = old.clone();
        valid_new.add_field("email", FieldType::Utf8, false); // Optional field added

        assert!(SchemaValidator::validate_evolution(
            &old,
            &valid_new,
            CompatibilityLevel::Backward
        )
        .is_ok());

        let mut invalid_new = old.clone();
        invalid_new.add_field("mandatory_key", FieldType::Utf8, true); // Required field added
        assert!(SchemaValidator::validate_evolution(
            &old,
            &invalid_new,
            CompatibilityLevel::Backward
        )
        .is_err());
    }
}
