use super::Environment;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum EnvironmentError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Environment not found: {0}")]
    NotFound(String),
}

pub struct EnvironmentStorage {
    storage_dir: PathBuf,
}

impl EnvironmentStorage {
    pub fn new<P: AsRef<Path>>(storage_dir: P) -> Result<Self, EnvironmentError> {
        let storage_dir = storage_dir.as_ref().to_path_buf();
        fs::create_dir_all(&storage_dir)?;
        Ok(Self { storage_dir })
    }

    fn env_path(&self, name: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.env.yaml", name))
    }

    pub fn save(&self, env: &Environment) -> Result<(), EnvironmentError> {
        let path = self.env_path(&env.name);
        let yaml = serde_yaml::to_string(env)?;
        // Atomic write: write to .tmp then rename for crash safety
        let tmp_path = path.with_extension("env.yaml.tmp");
        fs::write(&tmp_path, yaml)?;
        fs::rename(&tmp_path, path)?;
        Ok(())
    }

    pub fn load(&self, name: &str) -> Result<Environment, EnvironmentError> {
        let path = self.env_path(name);
        if !path.exists() {
            return Err(EnvironmentError::NotFound(name.to_string()));
        }
        let yaml = fs::read_to_string(path)?;
        let env: Environment = serde_yaml::from_str(&yaml)?;
        Ok(env)
    }

    pub fn list(&self) -> Result<Vec<String>, EnvironmentError> {
        let mut names = Vec::new();
        for entry in fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name() {
                let name_str = name.to_string_lossy();
                if name_str.ends_with(".env.yaml") {
                    let env_name = name_str
                        .trim_end_matches(".env.yaml")
                        .to_string();
                    names.push(env_name);
                }
            }
        }
        names.sort();
        Ok(names)
    }

    pub fn delete(&self, name: &str) -> Result<(), EnvironmentError> {
        let path = self.env_path(name);
        if !path.exists() {
            return Err(EnvironmentError::NotFound(name.to_string()));
        }
        fs::remove_file(path)?;
        Ok(())
    }

    pub fn exists(&self, name: &str) -> bool {
        self.env_path(name).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_save_and_load() {
        let temp = TempDir::new().unwrap();
        let storage = EnvironmentStorage::new(temp.path()).unwrap();

        let mut env = Environment::new("production");
        env.set("API_URL", "https://api.example.com");
        env.set("TOKEN", "secret123");

        storage.save(&env).unwrap();
        let loaded = storage.load("production").unwrap();

        assert_eq!(loaded.name, "production");
        assert_eq!(loaded.get("API_URL").unwrap().value, "https://api.example.com");
        assert_eq!(loaded.get("TOKEN").unwrap().value, "secret123");
    }

    #[test]
    fn test_list_environments() {
        let temp = TempDir::new().unwrap();
        let storage = EnvironmentStorage::new(temp.path()).unwrap();

        let env1 = Environment::new("dev");
        let env2 = Environment::new("staging");

        storage.save(&env1).unwrap();
        storage.save(&env2).unwrap();

        let names = storage.list().unwrap();
        assert_eq!(names, vec!["dev", "staging"]);
    }

    #[test]
    fn test_delete() {
        let temp = TempDir::new().unwrap();
        let storage = EnvironmentStorage::new(temp.path()).unwrap();

        let env = Environment::new("test");

        storage.save(&env).unwrap();
        assert!(storage.exists("test"));

        storage.delete("test").unwrap();
        assert!(!storage.exists("test"));
    }

    #[test]
    fn test_not_found() {
        let temp = TempDir::new().unwrap();
        let storage = EnvironmentStorage::new(temp.path()).unwrap();

        let result = storage.load("nonexistent");
        assert!(matches!(result, Err(EnvironmentError::NotFound(_))));
    }
}
