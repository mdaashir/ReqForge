use crate::environment::model::{Environment, GlobalVariables};
use crate::error::{Error, Result};
use regex::Regex;
use std::collections::HashMap;
use uuid::Uuid;

/// Resolves `{{variable}}` placeholders in strings using a layered variable
/// scope (local > collection > environment > global).
///
/// Also supports dynamic variables like `{{$uuid}}`, `{{$randomEmail}}`, etc.
pub struct VariableResolver {
    local: HashMap<String, String>,
    collection: HashMap<String, String>,
    environment: Option<Environment>,
    global: GlobalVariables,
    dynamic: DynamicVariables,
}

impl VariableResolver {
    /// Create an empty resolver
    pub fn new() -> Self {
        Self {
            local: HashMap::new(),
            collection: HashMap::new(),
            environment: None,
            global: GlobalVariables::new(),
            dynamic: DynamicVariables::new(),
        }
    }

    pub fn set_environment(&mut self, env: Environment) {
        self.environment = Some(env);
    }

    pub fn set_global(&mut self, global: GlobalVariables) {
        self.global = global;
    }

    pub fn set_collection(&mut self, vars: HashMap<String, String>) {
        self.collection = vars;
    }

    pub fn set_local(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.local.insert(key.into(), value.into());
    }

    /// Look up a single variable in priority order
    pub fn get(&self, key: &str) -> Option<String> {
        if let Some(v) = self.local.get(key) {
            return Some(v.clone());
        }
        if let Some(v) = self.collection.get(key) {
            return Some(v.clone());
        }
        if let Some(env) = &self.environment {
            if let Some(v) = env.get(key) {
                return Some(v.value.clone());
            }
        }
        if let Some(v) = self.global.get(key) {
            return Some(v.value.clone());
        }
        if let Some(v) = self.dynamic.get(key) {
            return Some(v);
        }
        None
    }

    /// Resolve all `{{var}}` placeholders in the given template string
    pub fn resolve(&self, template: &str) -> Result<String> {
        let re = Regex::new(r"\{\{\s*([^}]+?)\s*\}\}").unwrap();
        let mut result = template.to_string();

        // Find all matches and replace
        let mut missing: Vec<String> = Vec::new();

        for caps in re.captures_iter(template) {
            let key = caps.get(1).unwrap().as_str().trim();
            if let Some(value) = self.get(key) {
                let pattern = format!("{{{{{}}}}}", key);
                result = result.replace(&pattern, &value);
                // Also replace trimmed version
                let pattern_trimmed = format!("{{{{ {} }}}}", key);
                result = result.replace(&pattern_trimmed, &value);
            } else {
                missing.push(key.to_string());
            }
        }

        if !missing.is_empty() {
            return Err(Error::VariableNotFound(missing.join(", ")));
        }

        Ok(result)
    }

    /// Resolve a value but tolerate missing variables (leave placeholders intact)
    pub fn resolve_lenient(&self, template: &str) -> String {
        let re = Regex::new(r"\{\{\s*([^}]+?)\s*\}\}").unwrap();
        let mut result = template.to_string();

        for caps in re.captures_iter(template) {
            let key = caps.get(1).unwrap().as_str().trim();
            if let Some(value) = self.get(key) {
                result = result.replace(&format!("{{{{{}}}}}", key), &value);
                result = result.replace(&format!("{{{{ {} }}}}", key), &value);
            }
        }

        result
    }

    /// Update a variable in the environment (creates a new one if missing)
    pub fn set_in_environment(&mut self, key: impl Into<String>, value: impl Into<String>) {
        let key = key.into();
        let value = value.into();
        if let Some(env) = self.environment.as_mut() {
            env.set(&key, &value);
        }
    }
}

impl Default for VariableResolver {
    fn default() -> Self {
        Self::new()
    }
}

/// Dynamic variable generator (e.g., $uuid, $randomEmail, $timestamp)
pub struct DynamicVariables;

impl Default for DynamicVariables {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicVariables {
    pub fn new() -> Self {
        Self
    }

    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "$uuid" => Some(Uuid::new_v4().to_string()),
            "$timestamp" => Some(chrono::Utc::now().timestamp().to_string()),
            "$isoTimestamp" => Some(chrono::Utc::now().to_rfc3339()),
            "$randomInt" => Some(rand_int(0, 1000).to_string()),
            "$randomEmail" => Some(format!("user{}@example.com", rand_int(1000, 99999))),
            "$randomName" => Some(random_name()),
            "$randomUUID" => Some(Uuid::new_v4().to_string()),
            "$randomBoolean" => Some(rand_int(0, 2).to_string()),
            _ => None,
        }
    }
}

fn rand_int(min: i32, max: i32) -> i32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    let mut hasher = DefaultHasher::new();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
        .hash(&mut hasher);
    let h = hasher.finish();
    min + (h % (max - min) as u64) as i32
}

fn random_name() -> String {
    let first = ["Alex", "Jordan", "Taylor", "Morgan", "Casey", "Riley", "Sam", "Drew"];
    let last = ["Smith", "Jones", "Brown", "Wilson", "Davis", "Miller", "Lee", "Garcia"];
    let f = first[rand_int(0, first.len() as i32) as usize];
    let l = last[rand_int(0, last.len() as i32) as usize];
    format!("{} {}", f, l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_resolution() {
        let mut r = VariableResolver::new();
        r.set_local("base_url", "https://api.example.com");
        assert_eq!(r.resolve("{{base_url}}/users").unwrap(), "https://api.example.com/users");
    }

    #[test]
    fn test_missing_variable() {
        let r = VariableResolver::new();
        assert!(r.resolve("{{missing}}").is_err());
    }

    #[test]
    fn test_priority_order() {
        let mut r = VariableResolver::new();
        r.set_local("x", "local");
        r.set_collection({
            let mut m = HashMap::new();
            m.insert("x".to_string(), "collection".to_string());
            m
        });

        // Local should win
        assert_eq!(r.get("x"), Some("local".to_string()));

        // Without local, collection wins
        r.local.clear();
        assert_eq!(r.get("x"), Some("collection".to_string()));
    }

    #[test]
    fn test_dynamic_variables() {
        let r = VariableResolver::new();
        assert!(r.get("$uuid").is_some());
        assert!(r.get("$timestamp").is_some());
    }

    #[test]
    fn test_lenient_resolution() {
        let r = VariableResolver::new();
        // Should not error on missing
        assert_eq!(r.resolve_lenient("{{missing}}"), "{{missing}}");
    }
}
