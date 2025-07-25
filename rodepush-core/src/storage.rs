//! Storage abstraction layer for RodePush.
//!
//! This module provides a trait for storage operations and a file system implementation.

use crate::error::{Result, RodePushError, StorageError};
use crate::bundle::Bundle;
use crate::assets::AssetCollection;
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Storage key for identifying stored objects
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StorageKey(pub String);

impl StorageKey {
    /// Create a new storage key
    pub fn new(key: String) -> Self {
        Self(key)
    }
    
    /// Get the key as a string reference
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Trait for storage operations
#[async_trait]
pub trait Storage: Send + Sync {
    /// Store a bundle
    async fn store_bundle(&self, bundle: &Bundle) -> Result<StorageKey>;
    
    /// Retrieve a bundle
    async fn retrieve_bundle(&self, key: &StorageKey) -> Result<Bundle>;
    
    /// Store an asset collection
    async fn store_asset_collection(&self, collection: &AssetCollection) -> Result<StorageKey>;
    
    /// Retrieve an asset collection
    async fn retrieve_asset_collection(&self, key: &StorageKey) -> Result<AssetCollection>;
    
    /// Delete an object by key
    async fn delete(&self, key: &StorageKey) -> Result<()>;
    
    /// Check if an object exists
    async fn exists(&self, key: &StorageKey) -> Result<bool>;
}

/// File system storage implementation
pub struct FilesystemStorage {
    base_path: PathBuf,
}

impl FilesystemStorage {
    /// Create a new filesystem storage
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        
        // Create the base directory if it doesn't exist
        if !base_path.exists() {
            std::fs::create_dir_all(&base_path)
                .map_err(|e| RodePushError::Storage(StorageError::Io { message: e.to_string() }))?;
        }
        
        Ok(Self { base_path })
    }
    
    /// Get the full path for a storage key
    fn get_path(&self, key: &StorageKey) -> PathBuf {
        self.base_path.join(&key.0)
    }
}

#[async_trait]
impl Storage for FilesystemStorage {
    async fn store_bundle(&self, bundle: &Bundle) -> Result<StorageKey> {
        let key = StorageKey::new(format!("bundles/{}.json", bundle.id().as_str()));
        let path = self.get_path(&key);
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| RodePushError::Storage(StorageError::Io { message: e.to_string() }))?;
        }
        
        // Serialize and write the bundle
        let json = serde_json::to_string_pretty(bundle)
            .map_err(|e| RodePushError::Storage(StorageError::Serialization { message: e.to_string() }))?;
        
        fs::write(&path, json)
            .await
            .map_err(|e| RodePushError::Storage(StorageError::Io { message: e.to_string() }))?;
        
        Ok(key)
    }
    
    async fn retrieve_bundle(&self, key: &StorageKey) -> Result<Bundle> {
        let path = self.get_path(key);
        
        // Read and deserialize the bundle
        let json = fs::read_to_string(&path)
            .await
            .map_err(|e| RodePushError::Storage(StorageError::Io { message: e.to_string() }))?;
        
        let bundle: Bundle = serde_json::from_str(&json)
            .map_err(|e| RodePushError::Storage(StorageError::Serialization { message: e.to_string() }))?;
        
        Ok(bundle)
    }
    
    async fn store_asset_collection(&self, collection: &AssetCollection) -> Result<StorageKey> {
        let key = StorageKey::new(format!("assets/{}.json", collection.id.0));
        let path = self.get_path(&key);
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| RodePushError::Storage(StorageError::Io { message: e.to_string() }))?;
        }
        
        // Serialize and write the asset collection
        let json = serde_json::to_string_pretty(collection)
            .map_err(|e| RodePushError::Storage(StorageError::Serialization { message: e.to_string() }))?;
        
        fs::write(&path, json)
            .await
            .map_err(|e| RodePushError::Storage(StorageError::Io { message: e.to_string() }))?;
        
        Ok(key)
    }
    
    async fn retrieve_asset_collection(&self, key: &StorageKey) -> Result<AssetCollection> {
        let path = self.get_path(key);
        
        // Read and deserialize the asset collection
        let json = fs::read_to_string(&path)
            .await
            .map_err(|e| RodePushError::Storage(StorageError::Io { message: e.to_string() }))?;
        
        let collection: AssetCollection = serde_json::from_str(&json)
            .map_err(|e| RodePushError::Storage(StorageError::Serialization { message: e.to_string() }))?;
        
        Ok(collection)
    }
    
    async fn delete(&self, key: &StorageKey) -> Result<()> {
        let path = self.get_path(key);
        
        fs::remove_file(&path)
            .await
            .map_err(|e| RodePushError::Storage(StorageError::Io { message: e.to_string() }))
    }
    
    async fn exists(&self, key: &StorageKey) -> Result<bool> {
        let path = self.get_path(key);
        Ok(path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::{SemanticVersion, Platform, BundleBuilder};
    use crate::assets::AssetMetadata;
    use crate::CompressionType;  // Import CompressionType
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_filesystem_storage_bundle() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = FilesystemStorage::new(temp_dir.path())?;
        
        // Create a test bundle using the builder pattern
        let mut builder = BundleBuilder::new(
            SemanticVersion::new(1, 0, 0),
            Platform::Android,
            "index.js".to_string()
        ).with_compression(CompressionType::None);  // Use no compression to avoid size issues
        
        // Add a chunk to the bundle with larger data
        let chunk_data = b"console.log('This is a test bundle chunk');".to_vec();
        builder.add_chunk_from_data(&chunk_data, "chunk1".to_string())?;
        
        let bundle = builder.build()?;
        
        // Store the bundle
        let key = storage.store_bundle(&bundle).await?;
        
        // Check that it exists
        assert!(storage.exists(&key).await?);
        
        // Retrieve the bundle
        let retrieved_bundle = storage.retrieve_bundle(&key).await?;
        assert_eq!(bundle.id(), retrieved_bundle.id());
        assert_eq!(bundle.version(), retrieved_bundle.version());
        assert_eq!(bundle.platform(), retrieved_bundle.platform());
        
        // Delete the bundle
        storage.delete(&key).await?;
        
        // Check that it no longer exists
        assert!(!storage.exists(&key).await?);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_filesystem_storage_asset_collection() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let storage = FilesystemStorage::new(temp_dir.path())?;
        
        // Create a test asset collection
        let mut collection = AssetCollection::new();
        collection.assets.insert("image.png".to_string(), AssetMetadata {
            path: "image.png".to_string(),
            size: 100,
            checksum: "abc123".to_string(),
            mime_type: "image/png".to_string(),
        });
        collection.total_size = 100;
        
        // Store the asset collection
        let key = storage.store_asset_collection(&collection).await?;
        
        // Check that it exists
        assert!(storage.exists(&key).await?);
        
        // Retrieve the asset collection
        let retrieved_collection = storage.retrieve_asset_collection(&key).await?;
        assert_eq!(collection.id, retrieved_collection.id);
        assert_eq!(collection.total_size, retrieved_collection.total_size);
        
        // Delete the asset collection
        storage.delete(&key).await?;
        
        // Check that it no longer exists
        assert!(!storage.exists(&key).await?);
        
        Ok(())
    }
}