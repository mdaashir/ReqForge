use crate::request::KeyValue;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Variable scope determines resolution order
///
/// Variables are resolved from most specific to least specific scope.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum VariableScope {
    /// Local/session variables (highest priority)
    Local,
    /// Collection-scoped variables
    Collection,
    /// Environment-scoped variables
    Environment,
    /// Global variables (lowest priority)
    Global,
}

/// Variable type determines how the value is stored and displayed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum VariableType {
    #[default]
    String,
    Number,
    Boolean,
    /// Sensitive variable, masked in UI and excluded from exports
    Secret,
}

/// A single variable in an environment or scope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub var_type: VariableType,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

impl Variable {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            var_type: VariableType::String,
            enabled: true,
        }
    }

    pub fn secret(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            var_type: VariableType::Secret,
            enabled: true,
        }
    }
}

/// A named set of variables (e.g., dev, staging, production)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub variables: Vec<Variable>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

impl Environment {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            variables: Vec::new(),
            color: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Variable> {
        self.variables.iter().find(|v| v.enabled && v.key == key)
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        if let Some(v) = self.variables.iter_mut().find(|v| v.key == key) {
            v.value = value.into();
        } else {
            self.variables.push(Variable::new(key, value.into()));
        }
        self.updated_at = Utc::now();
    }
}

/// Workspace-level global variables (not tied to an environment)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalVariables {
    pub variables: Vec<Variable>,
}

impl GlobalVariables {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, key: &str) -> Option<&Variable> {
        self.variables.iter().find(|v| v.enabled && v.key == key)
    }
}

/// Convert legacy KeyValue to typed Variable
impl From<KeyValue> for Variable {
    fn from(kv: KeyValue) -> Self {
        let var_type = if kv.key.to_lowercase().contains("secret")
            || kv.key.to_lowercase().contains("password")
            || kv.key.to_lowercase().contains("token")
            || kv.key.to_lowercase().contains("api_key")
        {
            VariableType::Secret
        } else {
            VariableType::String
        };

        Self {
            key: kv.key,
            value: kv.value,
            var_type,
            enabled: kv.enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_environment() {
        let env = Environment::new("Development");
        assert_eq!(env.name, "Development");
        assert_eq!(env.variables.len(), 0);
    }

    #[test]
    fn test_set_and_get_variable() {
        let mut env = Environment::new("Dev");
        env.set("base_url", "https://api.dev.example.com");

        assert_eq!(
            env.get("base_url").unwrap().value,
            "https://api.dev.example.com"
        );
    }

    #[test]
    fn test_update_existing_variable() {
        let mut env = Environment::new("Dev");
        env.set("base_url", "https://old.example.com");
        env.set("base_url", "https://new.example.com");

        assert_eq!(env.variables.len(), 1);
        assert_eq!(
            env.get("base_url").unwrap().value,
            "https://new.example.com"
        );
    }

    #[test]
    fn test_secret_variable() {
        let v = Variable::secret("api_key", "sk-1234567890");
        assert_eq!(v.var_type, VariableType::Secret);
    }

    #[test]
    fn test_keyvalue_to_variable() {
        let kv = KeyValue {
            key: "API_KEY".to_string(),
            value: "secret".to_string(),
            enabled: true,
            description: None,
        };
        let v: Variable = kv.into();
        assert_eq!(v.var_type, VariableType::Secret);
    }
}
