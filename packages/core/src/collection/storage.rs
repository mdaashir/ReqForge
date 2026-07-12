use crate::collection::Collection;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Serialised on-disk representation of a collection
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CollectionFile {
    version: u32,
    collection: Collection,
}

/// File-based collection storage using YAML
///
/// Collections are stored as `collection.yaml` files inside a workspace
/// directory. Layout:
/// ```text
/// workspace/
/// ├── collections/
/// │   ├── <id>/
/// │   │   └── collection.yaml
/// ```
pub struct CollectionStorage {
    workspace_root: PathBuf,
}

impl CollectionStorage {
    /// Create storage rooted at the given workspace directory
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            workspace_root: workspace_root.into(),
        }
    }

    /// Resolve the directory for a collection id
    fn collection_dir(&self, id: &str) -> PathBuf {
        self.workspace_root.join("collections").join(id)
    }

    /// Resolve the YAML file for a collection id
    fn collection_file(&self, id: &str) -> PathBuf {
        self.collection_dir(id).join("collection.yaml")
    }

    /// Save a collection to disk as YAML
    pub async fn save(&self, collection: &Collection) -> Result<()> {
        let dir = self.collection_dir(&collection.id);
        fs::create_dir_all(&dir).await?;

        let file = CollectionFile {
            version: 1,
            collection: collection.clone(),
        };

        let yaml = serde_yaml::to_string(&file)?;
        let path = self.collection_file(&collection.id);
        // Atomic write: write to .tmp then rename for crash safety
        let tmp_path = path.with_extension("yaml.tmp");
        fs::write(&tmp_path, yaml).await?;
        fs::rename(&tmp_path, &path).await?;

        Ok(())
    }

    /// Load a single collection by id from disk
    pub async fn load(&self, id: &str) -> Result<Collection> {
        let path = self.collection_file(id);
        if !path.exists() {
            return Err(Error::storage(format!("Collection not found: {}", id)));
        }

        let content = fs::read_to_string(&path).await?;
        let file: CollectionFile = serde_yaml::from_str(&content)?;
        Ok(file.collection)
    }

    /// Delete a collection directory from disk
    pub async fn delete(&self, id: &str) -> Result<()> {
        let dir = self.collection_dir(id);
        if dir.exists() {
            fs::remove_dir_all(&dir).await?;
        }
        Ok(())
    }

    /// List all collection ids in the workspace
    pub async fn list_ids(&self) -> Result<Vec<String>> {
        let collections_dir = self.workspace_root.join("collections");
        if !collections_dir.exists() {
            return Ok(Vec::new());
        }

        let mut ids = Vec::new();
        let mut entries = fs::read_dir(&collections_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    ids.push(name.to_string());
                }
            }
        }
        Ok(ids)
    }

    /// Load all collections in the workspace
    pub async fn list_all(&self) -> Result<Vec<Collection>> {
        let ids = self.list_ids().await?;
        let mut collections = Vec::with_capacity(ids.len());
        for id in ids {
            match self.load(&id).await {
                Ok(c) => collections.push(c),
                Err(e) => {
                    // Log but don't fail; skip corrupt collections
                    eprintln!("Failed to load collection {}: {}", id, e);
                }
            }
        }
        Ok(collections)
    }

    /// Get the workspace root path
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_save_and_load_collection() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = CollectionStorage::new(tmp.path());

        let collection = Collection::new("Test API");
        storage.save(&collection).await.unwrap();

        let loaded = storage.load(&collection.id).await.unwrap();
        assert_eq!(loaded.name, "Test API");
        assert_eq!(loaded.id, collection.id);
    }

    #[tokio::test]
    async fn test_delete_collection() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = CollectionStorage::new(tmp.path());

        let collection = Collection::new("Test");
        storage.save(&collection).await.unwrap();
        storage.delete(&collection.id).await.unwrap();

        let result = storage.load(&collection.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_list_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = CollectionStorage::new(tmp.path());

        storage.save(&Collection::new("A")).await.unwrap();
        storage.save(&Collection::new("B")).await.unwrap();

        let mut ids = storage.list_ids().await.unwrap();
        ids.sort();
        assert_eq!(ids.len(), 2);
    }
}
