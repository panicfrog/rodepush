use crate::{BundleError, Result, crypto};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::sync::Mutex;
use uuid::Uuid;

#[allow(dead_code)]
const BUNDLE_MAGIC: &[u8] = b"RDPUSHB";
#[allow(dead_code)]
const BUNDLE_FORMAT_VERSION: u16 = 1;

/// Unique identifier for a bundle
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BundleId(Uuid);

impl BundleId {
    /// Generate a new unique bundle ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from existing UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Create from string representation
    pub fn from_string(s: &str) -> Result<Self> {
        let uuid = Uuid::parse_str(s)
            .map_err(|e| BundleError::invalid_format(format!("Invalid UUID: {}", e)))?;
        Ok(Self(uuid))
    }

    /// Get the underlying UUID
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Get string representation
    pub fn as_str(&self) -> String {
        self.0.to_string()
    }
}

impl Default for BundleId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BundleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for BundleId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<BundleId> for Uuid {
    fn from(id: BundleId) -> Self {
        id.0
    }
}

/// Semantic version following semver.org specification
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
    pub build_metadata: Option<String>,
}

impl SemanticVersion {
    /// Create a new semantic version
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
            pre_release: None,
            build_metadata: None,
        }
    }

    /// Create with pre-release identifier
    pub fn with_pre_release(mut self, pre_release: String) -> Self {
        self.pre_release = Some(pre_release);
        self
    }

    /// Create with build metadata
    pub fn with_build_metadata(mut self, build_metadata: String) -> Self {
        self.build_metadata = Some(build_metadata);
        self
    }

    /// Parse from string (e.g., "1.2.3" or "1.2.3-alpha.1+build.123")
    pub fn parse(s: &str) -> Result<Self> {
        let parts: Vec<&str> = s.split('+').collect();
        let (version_part, build_metadata) = if parts.len() == 2 {
            (parts[0], Some(parts[1].to_string()))
        } else if parts.len() == 1 {
            (parts[0], None)
        } else {
            return Err(BundleError::InvalidVersion {
                version: s.to_string(),
            }
            .into());
        };

        let parts: Vec<&str> = version_part.split('-').collect();
        let (core_version, pre_release) = if parts.len() == 2 {
            (parts[0], Some(parts[1].to_string()))
        } else if parts.len() == 1 {
            (parts[0], None)
        } else {
            return Err(BundleError::InvalidVersion {
                version: s.to_string(),
            }
            .into());
        };

        let version_parts: Vec<&str> = core_version.split('.').collect();
        if version_parts.len() != 3 {
            return Err(BundleError::InvalidVersion {
                version: s.to_string(),
            }
            .into());
        }

        let major = version_parts[0]
            .parse::<u32>()
            .map_err(|_| BundleError::InvalidVersion {
                version: s.to_string(),
            })?;
        let minor = version_parts[1]
            .parse::<u32>()
            .map_err(|_| BundleError::InvalidVersion {
                version: s.to_string(),
            })?;
        let patch = version_parts[2]
            .parse::<u32>()
            .map_err(|_| BundleError::InvalidVersion {
                version: s.to_string(),
            })?;

        Ok(Self {
            major,
            minor,
            patch,
            pre_release,
            build_metadata,
        })
    }

    /// Check if this is a compatible version for updates
    pub fn is_compatible_with(&self, other: &SemanticVersion) -> bool {
        // For React Native CodePush, we typically allow updates within the same minor version
        self.major == other.major && self.minor == other.minor
    }

    /// Check if this version is newer than the other
    pub fn is_newer_than(&self, other: &SemanticVersion) -> bool {
        self > other
    }
}

impl std::fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(ref pre) = self.pre_release {
            write!(f, "-{}", pre)?;
        }
        if let Some(ref build) = self.build_metadata {
            write!(f, "+{}", build)?;
        }
        Ok(())
    }
}

impl std::str::FromStr for SemanticVersion {
    type Err = crate::RodePushError;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

/// Supported platforms for React Native bundles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    /// iOS platform
    Ios,
    /// Android platform
    Android,
    /// Both platforms (universal bundle)
    Both,
}

impl Platform {
    /// Get all supported platforms
    pub fn all() -> Vec<Platform> {
        vec![Platform::Ios, Platform::Android, Platform::Both]
    }

    /// Check if platform is compatible with target
    pub fn is_compatible_with(&self, target: Platform) -> bool {
        match (self, target) {
            (Platform::Both, _) => true,
            (_, Platform::Both) => true,
            (a, b) => a == &b,
        }
    }

    /// Get file extension for bundle files
    pub fn bundle_extension(&self) -> &'static str {
        match self {
            Platform::Ios => "jsbundle",
            Platform::Android => "bundle",
            Platform::Both => "bundle",
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Ios => write!(f, "ios"),
            Platform::Android => write!(f, "android"),
            Platform::Both => write!(f, "both"),
        }
    }
}

impl std::str::FromStr for Platform {
    type Err = crate::RodePushError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "ios" => Ok(Platform::Ios),
            "android" => Ok(Platform::Android),
            "both" => Ok(Platform::Both),
            _ => Err(BundleError::UnsupportedPlatform {
                platform: s.to_string(),
            }
            .into()),
        }
    }
}

/// Compression types supported for bundles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CompressionType {
    /// No compression
    None,
    /// Gzip compression
    Gzip,
    /// Zstandard compression (recommended)
    Zstd,
    /// Brotli compression
    Brotli,
}

impl CompressionType {
    /// Get file extension for compressed files
    pub fn file_extension(&self) -> &'static str {
        match self {
            CompressionType::None => "",
            CompressionType::Gzip => "gz",
            CompressionType::Zstd => "zst",
            CompressionType::Brotli => "br",
        }
    }
}

impl Default for CompressionType {
    fn default() -> Self {
        CompressionType::Zstd
    }
}

/// Bundle dependency information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub resolved: Option<String>,
    pub integrity: Option<String>,
}

/// Metadata for a bundle chunk
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Unique identifier for this chunk
    pub id: String,
    /// Byte offset within the bundle
    pub offset: u64,
    /// Size in bytes
    pub size: u64,
    /// SHA-256 checksum of the chunk
    pub checksum: String,
    /// Compression type used
    pub compression: CompressionType,
    /// Original size before compression
    pub original_size: u64,
    /// Compression level used for this chunk
    #[serde(default)]
    pub compression_level: Option<i32>,
}

impl ChunkMetadata {
    /// Create new chunk metadata
    pub fn new(
        id: String,
        offset: u64,
        size: u64,
        checksum: String,
        compression: CompressionType,
        original_size: u64,
        compression_level: Option<i32>,
    ) -> Self {
        Self {
            id,
            offset,
            size,
            checksum,
            compression,
            original_size,
            compression_level,
        }
    }

    /// Calculate compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            0.0
        } else {
            self.size as f64 / self.original_size as f64
        }
    }

    /// Validate chunk metadata
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() {
            return Err(BundleError::chunk_error("Chunk ID cannot be empty").into());
        }

        if self.size == 0 {
            return Err(BundleError::chunk_error("Chunk size cannot be zero").into());
        }

        if self.checksum.is_empty() {
            return Err(BundleError::chunk_error("Chunk checksum cannot be empty").into());
        }

        if self.original_size < self.size {
            return Err(BundleError::chunk_error(
                "Original size cannot be less than compressed size",
            )
            .into());
        }

        Ok(())
    }
}

/// Complete metadata for a bundle
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleMetadata {
    /// Unique bundle identifier
    pub id: BundleId,
    /// Semantic version
    pub version: SemanticVersion,
    /// Target platform
    pub platform: Platform,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Total bundle size in bytes
    pub size_bytes: u64,
    /// SHA-256 checksum of the complete bundle
    pub checksum: String,
    /// Bundle dependencies
    pub dependencies: Vec<Dependency>,
    /// Chunk information for split bundles
    pub chunks: Vec<ChunkMetadata>,
    /// Entry point file name
    pub entry_point: String,
    /// Bundle format version
    pub format_version: String,
    /// Additional custom metadata
    pub custom_metadata: std::collections::HashMap<String, serde_json::Value>,
    /// Compression type used for all chunks, if uniform
    #[serde(default)]
    pub compression_type: Option<CompressionType>,
    /// Hash algorithm used for all checksums
    #[serde(default)]
    pub hash_algorithm: Option<crypto::HashAlgorithm>,
}

impl BundleMetadata {
    /// Create new bundle metadata
    pub fn new(version: SemanticVersion, platform: Platform, entry_point: String) -> Self {
        Self {
            id: BundleId::new(),
            version,
            platform,
            created_at: Utc::now(),
            size_bytes: 0,
            checksum: String::new(),
            dependencies: Vec::new(),
            chunks: Vec::new(),
            entry_point,
            format_version: "1.0".to_string(),
            custom_metadata: std::collections::HashMap::new(),
            compression_type: None,
            hash_algorithm: None,
        }
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, dependency: Dependency) {
        self.dependencies.push(dependency);
    }

    /// Add a chunk
    pub fn add_chunk(&mut self, chunk: ChunkMetadata) -> Result<()> {
        chunk.validate()?;
        self.chunks.push(chunk);
        self.recalculate_size();
        Ok(())
    }

    /// Recalculate total size from chunks
    fn recalculate_size(&mut self) {
        self.size_bytes = self.chunks.iter().map(|c| c.size).sum();
    }

    /// Validate bundle metadata
    pub fn validate(&self) -> Result<()> {
        if self.entry_point.is_empty() {
            return Err(BundleError::invalid_format("Entry point cannot be empty").into());
        }

        if self.checksum.is_empty() {
            return Err(BundleError::invalid_format("Bundle checksum cannot be empty").into());
        }

        // Validate all chunks
        for chunk in &self.chunks {
            chunk.validate()?;
        }

        // Check for duplicate chunk IDs
        let mut chunk_ids = std::collections::HashSet::new();
        for chunk in &self.chunks {
            if !chunk_ids.insert(chunk.id.clone()) {
                return Err(
                    BundleError::chunk_error(format!("Duplicate chunk ID: {}", chunk.id)).into(),
                );
            }
        }

        Ok(())
    }

    /// Get total number of chunks
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Find chunk by ID
    pub fn find_chunk(&self, chunk_id: &str) -> Option<&ChunkMetadata> {
        self.chunks.iter().find(|c| c.id == chunk_id)
    }

    /// Calculate total compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.chunks.is_empty() {
            return 1.0;
        }

        let total_original: u64 = self.chunks.iter().map(|c| c.original_size).sum();
        let total_compressed: u64 = self.chunks.iter().map(|c| c.size).sum();

        if total_original == 0 {
            1.0
        } else {
            total_compressed as f64 / total_original as f64
        }
    }
}

/// Individual chunk of a bundle
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BundleChunk {
    /// Chunk metadata
    pub metadata: ChunkMetadata,
    /// Actual chunk data
    pub data: Vec<u8>,
}

impl BundleChunk {
    /// Create new bundle chunk
    pub fn new(metadata: ChunkMetadata, data: Vec<u8>) -> Self {
        Self { metadata, data }
    }

    /// Validate chunk data matches metadata
    pub fn validate(&self) -> Result<()> {
        self.metadata.validate()?;

        if self.data.len() != self.metadata.size as usize {
            return Err(BundleError::chunk_error(format!(
                "Chunk data size {} does not match metadata size {}",
                self.data.len(),
                self.metadata.size
            ))
            .into());
        }

        Ok(())
    }

    /// Get chunk data as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get chunk size
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Get chunk ID
    pub fn id(&self) -> &str {
        &self.metadata.id
    }
}

/// Builder for creating bundles
#[derive(Debug)]
pub struct BundleBuilder {
    metadata: BundleMetadata,
    chunks: Vec<BundleChunk>,
    compression_type: CompressionType,
    hash_algorithm: crypto::HashAlgorithm,
}

impl BundleBuilder {
    /// Create a new bundle builder
    pub fn new(version: SemanticVersion, platform: Platform, entry_point: String) -> Self {
        let metadata = BundleMetadata::new(version, platform, entry_point);
        Self {
            metadata,
            chunks: Vec::new(),
            compression_type: CompressionType::default(),
            hash_algorithm: crypto::HashAlgorithm::Sha256,
        }
    }

    /// Set compression type for the bundle
    pub fn with_compression(mut self, compression: CompressionType) -> Self {
        self.compression_type = compression;
        self
    }

    /// Set hash algorithm for the bundle
    pub fn with_hash_algorithm(mut self, algorithm: crypto::HashAlgorithm) -> Self {
        self.hash_algorithm = algorithm;
        self
    }

    /// Add a chunk from data
    pub fn add_chunk_from_data(&mut self, data: &[u8], chunk_id: String) -> Result<()> {
        let original_size = data.len() as u64;

        // Compress the data
        let compressed_data = match self.compression_type {
            CompressionType::None => data.to_vec(),
            CompressionType::Zstd => {
                let mut compressed = Vec::new();
                zstd::stream::copy_encode(data, &mut compressed, 3).map_err(|e| {
                    BundleError::compression_failed(format!("Zstd compression failed: {}", e))
                })?;
                compressed
            }
            CompressionType::Gzip => {
                let mut compressed = Vec::new();
                let mut encoder =
                    flate2::write::GzEncoder::new(&mut compressed, flate2::Compression::default());
                encoder.write_all(data).map_err(|e| {
                    BundleError::compression_failed(format!("Gzip compression failed: {}", e))
                })?;
                encoder.finish().map_err(|e| {
                    BundleError::compression_failed(format!("Gzip compression failed: {}", e))
                })?;
                compressed
            }
            CompressionType::Brotli => {
                let mut compressed = Vec::new();
                {
                    let mut encoder = brotli::CompressorWriter::new(&mut compressed, 4096, 11, 22);
                    encoder.write_all(data).map_err(|e| {
                        BundleError::compression_failed(format!("Brotli compression failed: {}", e))
                    })?;
                    encoder.flush().map_err(|e| {
                        BundleError::compression_failed(format!("Brotli compression failed: {}", e))
                    })?;
                } // encoder is dropped here
                compressed
            }
        };

        // Generate checksum
        let hasher = crypto::BulkHasher::new(self.hash_algorithm);
        let checksum = hasher.hash_data(&compressed_data);

        // Create chunk metadata
        let chunk_metadata = ChunkMetadata::new(
            chunk_id,
            self.metadata.size_bytes, // offset
            compressed_data.len() as u64,
            checksum,
            self.compression_type,
            original_size,
            Some(3), // compression level
        );

        // Create bundle chunk
        let chunk = BundleChunk::new(chunk_metadata, compressed_data);

        // Add to builder
        self.chunks.push(chunk);

        Ok(())
    }

    /// Build the final bundle
    pub fn build(mut self) -> Result<Bundle> {
        // Add all chunks to metadata
        for chunk in &self.chunks {
            self.metadata.add_chunk(chunk.metadata.clone())?;
        }

        // Generate final bundle checksum
        let all_data: Vec<u8> = self
            .chunks
            .iter()
            .flat_map(|chunk| chunk.data.iter())
            .cloned()
            .collect();

        let hasher = crypto::BulkHasher::new(self.hash_algorithm);
        self.metadata.checksum = hasher.hash_data(&all_data);
        self.metadata.compression_type = Some(self.compression_type);
        self.metadata.hash_algorithm = Some(self.hash_algorithm);

        let bundle = Bundle {
            metadata: self.metadata,
            chunks: self.chunks,
        };

        bundle.validate()?;
        Ok(bundle)
    }
}

/// Complete bundle representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Bundle {
    /// Bundle metadata
    pub metadata: BundleMetadata,
    /// Bundle chunks
    pub chunks: Vec<BundleChunk>,
}

impl Bundle {
    /// Create new bundle
    pub fn new(metadata: BundleMetadata) -> Self {
        Self {
            metadata,
            chunks: Vec::new(),
        }
    }

    /// Add a chunk to the bundle
    pub fn add_chunk(&mut self, chunk: BundleChunk) -> Result<()> {
        chunk.validate()?;

        // Add chunk metadata to bundle metadata
        self.metadata.add_chunk(chunk.metadata.clone())?;

        // Add chunk to chunks list
        self.chunks.push(chunk);

        Ok(())
    }

    /// Get bundle ID
    pub fn id(&self) -> &BundleId {
        &self.metadata.id
    }

    /// Get bundle version
    pub fn version(&self) -> &SemanticVersion {
        &self.metadata.version
    }

    /// Get bundle platform
    pub fn platform(&self) -> Platform {
        self.metadata.platform
    }

    /// Get total bundle size
    pub fn size(&self) -> u64 {
        self.metadata.size_bytes
    }

    /// Get chunk count
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Find chunk by ID
    pub fn find_chunk(&self, chunk_id: &str) -> Option<&BundleChunk> {
        self.chunks.iter().find(|c| c.id() == chunk_id)
    }

    /// Validate entire bundle
    pub fn validate(&self) -> Result<()> {
        self.metadata.validate()?;

        if self.chunks.len() != self.metadata.chunks.len() {
            return Err(BundleError::invalid_format(
                "Chunk count mismatch between metadata and actual chunks",
            )
            .into());
        }

        // Validate all chunks
        for chunk in &self.chunks {
            chunk.validate()?;
        }

        // Ensure metadata and chunks are in sync
        for (i, chunk) in self.chunks.iter().enumerate() {
            let metadata_chunk = &self.metadata.chunks[i];
            if chunk.metadata.id != metadata_chunk.id {
                return Err(BundleError::invalid_format(format!(
                    "Chunk metadata mismatch at index {}: {} vs {}",
                    i, chunk.metadata.id, metadata_chunk.id
                ))
                .into());
            }
        }

        Ok(())
    }

    /// Check if this bundle is compatible with another for differential updates
    pub fn is_compatible_with(&self, other: &Bundle) -> bool {
        self.metadata
            .platform
            .is_compatible_with(other.metadata.platform)
            && self
                .metadata
                .version
                .is_compatible_with(&other.metadata.version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_id_creation() {
        let id1 = BundleId::new();
        let id2 = BundleId::new();
        assert_ne!(id1, id2);

        let id_str = id1.as_str();
        let id3 = BundleId::from_string(&id_str).unwrap();
        assert_eq!(id1, id3);
    }

    #[test]
    fn test_semantic_version() {
        let version = SemanticVersion::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");

        let parsed = SemanticVersion::parse("1.2.3-alpha.1+build.123").unwrap();
        assert_eq!(parsed.major, 1);
        assert_eq!(parsed.minor, 2);
        assert_eq!(parsed.patch, 3);
        assert_eq!(parsed.pre_release, Some("alpha.1".to_string()));
        assert_eq!(parsed.build_metadata, Some("build.123".to_string()));

        assert!(SemanticVersion::parse("invalid").is_err());
    }

    #[test]
    fn test_semantic_version_compatibility() {
        let v1_2_3 = SemanticVersion::new(1, 2, 3);
        let v1_2_4 = SemanticVersion::new(1, 2, 4);
        let v1_3_0 = SemanticVersion::new(1, 3, 0);

        assert!(v1_2_3.is_compatible_with(&v1_2_4));
        assert!(!v1_2_3.is_compatible_with(&v1_3_0));
        assert!(v1_2_4.is_newer_than(&v1_2_3));
    }

    #[test]
    fn test_platform() {
        assert_eq!(Platform::Ios.to_string(), "ios");
        assert_eq!(Platform::Android.bundle_extension(), "bundle");
        assert!(Platform::Both.is_compatible_with(Platform::Ios));
        assert!(!Platform::Ios.is_compatible_with(Platform::Android));

        assert_eq!("ios".parse::<Platform>().unwrap(), Platform::Ios);
        assert!("invalid".parse::<Platform>().is_err());
    }

    #[test]
    fn test_chunk_metadata() {
        let chunk = ChunkMetadata::new(
            "chunk1".to_string(),
            0,
            100,
            "checksum123".to_string(),
            CompressionType::Zstd,
            150,
            Some(3),
        );

        assert!(chunk.validate().is_ok());
        assert_eq!(chunk.compression_ratio(), 100.0 / 150.0);

        let invalid_chunk = ChunkMetadata::new(
            "".to_string(), // Invalid empty ID
            0,
            100,
            "checksum".to_string(),
            CompressionType::None,
            150,
            None,
        );
        assert!(invalid_chunk.validate().is_err());
    }

    #[test]
    fn test_bundle_metadata() {
        let version = SemanticVersion::new(1, 0, 0);
        let mut metadata = BundleMetadata::new(version, Platform::Ios, "index.js".to_string());

        let chunk = ChunkMetadata::new(
            "chunk1".to_string(),
            0,
            100,
            "checksum123".to_string(),
            CompressionType::Zstd,
            150,
            Some(3),
        );

        metadata.add_chunk(chunk).unwrap();
        assert_eq!(metadata.chunk_count(), 1);
        assert_eq!(metadata.size_bytes, 100);

        metadata.checksum = "bundle_checksum".to_string();
        assert!(metadata.validate().is_ok());
    }

    #[test]
    fn test_bundle_creation() {
        let version = SemanticVersion::new(1, 0, 0);
        let metadata = BundleMetadata::new(
            version,
            Platform::Android,
            "index.android.bundle".to_string(),
        );

        let bundle = Bundle::new(metadata);
        assert_eq!(bundle.chunk_count(), 0);
        assert_eq!(bundle.platform(), Platform::Android);
    }

    #[test]
    fn test_bundle_compatibility() {
        let version1 = SemanticVersion::new(1, 2, 3);
        let version2 = SemanticVersion::new(1, 2, 4);
        let version3 = SemanticVersion::new(1, 3, 0);

        let bundle1 = Bundle::new(BundleMetadata::new(
            version1,
            Platform::Ios,
            "index.js".to_string(),
        ));

        let bundle2 = Bundle::new(BundleMetadata::new(
            version2,
            Platform::Ios,
            "index.js".to_string(),
        ));

        let bundle3 = Bundle::new(BundleMetadata::new(
            version3,
            Platform::Ios,
            "index.js".to_string(),
        ));

        assert!(bundle1.is_compatible_with(&bundle2));
        assert!(!bundle1.is_compatible_with(&bundle3));
    }
}

/// Bundle cache for in-memory storage of frequently accessed bundles
pub struct BundleCache {
    cache: Mutex<HashMap<BundleId, Bundle>>,
    max_size: usize,
}

impl BundleCache {
    /// Create a new bundle cache with the specified maximum size
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
            max_size,
        }
    }

    /// Get a bundle from cache by ID
    pub fn get(&self, id: &BundleId) -> Option<Bundle> {
        self.cache.lock().unwrap().get(id).cloned()
    }

    /// Store a bundle in cache
    pub fn put(&self, bundle: Bundle) {
        let mut cache = self.cache.lock().unwrap();

        // If cache size is 0, don't store anything
        if self.max_size == 0 {
            return;
        }

        // If cache is full, remove oldest entry (simple LRU-like behavior)
        if cache.len() >= self.max_size {
            if let Some((oldest_id, _)) = cache.iter().next() {
                let oldest_id = oldest_id.clone();
                let _ = cache.remove(&oldest_id);
            }
        }

        cache.insert(bundle.id().clone(), bundle);
    }

    /// Remove a bundle from cache
    pub fn remove(&self, id: &BundleId) -> Option<Bundle> {
        self.cache.lock().unwrap().remove(id)
    }

    /// Clear all entries from cache
    pub fn clear(&self) {
        self.cache.lock().unwrap().clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> BundleCacheStats {
        let cache = self.cache.lock().unwrap();
        BundleCacheStats {
            size: cache.len(),
            max_size: self.max_size,
            utilization: cache.len() as f64 / self.max_size as f64,
        }
    }
}

/// Statistics for bundle cache
#[derive(Debug, Clone)]
pub struct BundleCacheStats {
    pub size: usize,
    pub max_size: usize,
    pub utilization: f64,
}

impl Default for BundleCache {
    fn default() -> Self {
        Self::new(100) // Default cache size of 100 bundles
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    #[test]
    fn test_bundle_cache() {
        let cache = BundleCache::new(2);

        // Create test bundles
        let bundle1 = Bundle::new(BundleMetadata::new(
            SemanticVersion::new(1, 0, 0),
            Platform::Ios,
            "index.js".to_string(),
        ));
        let bundle2 = Bundle::new(BundleMetadata::new(
            SemanticVersion::new(1, 0, 1),
            Platform::Android,
            "index.js".to_string(),
        ));

        // Test put and get
        cache.put(bundle1.clone());
        assert_eq!(cache.get(&bundle1.id()), Some(bundle1.clone()));

        // Test cache stats
        let stats = cache.stats();
        assert_eq!(stats.size, 1);
        assert_eq!(stats.max_size, 2);
        assert_eq!(stats.utilization, 0.5);

        // Test cache eviction
        cache.put(bundle2.clone());
        cache.put(Bundle::new(BundleMetadata::new(
            SemanticVersion::new(1, 0, 2),
            Platform::Both,
            "index.js".to_string(),
        )));

        // One of the old entries should be evicted (HashMap iteration order is not guaranteed)
        let bundle1_exists = cache.get(&bundle1.id()).is_some();
        let bundle2_exists = cache.get(&bundle2.id()).is_some();
        assert!(bundle1_exists != bundle2_exists); // Exactly one should exist
        assert_eq!(cache.stats().size, 2); // Cache should be at max size

        // Test clear
        cache.clear();
        assert_eq!(cache.stats().size, 0);
    }
}
