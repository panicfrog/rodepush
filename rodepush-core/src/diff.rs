use crate::{Bundle, BundleError, BundleId, ChunkMetadata, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Represents the difference between two bundles
#[derive(Debug, Serialize, Deserialize)]
pub struct DiffResult {
    /// Chunks that are new or modified in the new bundle
    pub new_or_modified_chunks: Vec<ChunkMetadata>,
    /// IDs of chunks that were removed from the old bundle
    pub removed_chunk_ids: Vec<String>,
    /// Number of identical chunks between the two bundles
    pub identical_chunk_count: usize,
    /// Total size of the patch (new/modified chunks)
    pub patch_size_bytes: u64,
}

impl DiffResult {
    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        !self.new_or_modified_chunks.is_empty() || !self.removed_chunk_ids.is_empty()
    }

    /// Get a summary of the diff
    pub fn summary(&self) -> String {
        format!(
            "Diff summary: {} new/modified, {} removed, {} identical. Patch size: {} bytes",
            self.new_or_modified_chunks.len(),
            self.removed_chunk_ids.len(),
            self.identical_chunk_count,
            self.patch_size_bytes
        )
    }
}

/// Diff engine for comparing bundles and generating patches
#[derive(Debug, Default)]
pub struct DiffEngine;

impl DiffEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Compare two bundles and generate a diff result
    pub fn compare_bundles(&self, old_bundle: &Bundle, new_bundle: &Bundle) -> Result<DiffResult> {
        if !old_bundle.is_compatible_with(new_bundle) {
            return Err(BundleError::invalid_format(
                "Bundles are not compatible for diffing (different platform or major/minor version)".to_string()
            ).into());
        }

        let old_chunks: HashMap<_, _> = old_bundle
            .chunks
            .iter()
            .map(|c| (c.id().to_string(), c.metadata.clone()))
            .collect();

        let new_chunks: HashMap<_, _> = new_bundle
            .chunks
            .iter()
            .map(|c| (c.id().to_string(), c.metadata.clone()))
            .collect();

        let mut new_or_modified = Vec::new();
        let mut identical_count = 0;

        for (id, new_chunk) in &new_chunks {
            match old_chunks.get(id) {
                Some(old_chunk) if old_chunk.checksum == new_chunk.checksum => {
                    identical_count += 1;
                }
                _ => {
                    new_or_modified.push(new_chunk.clone());
                }
            }
        }

        let old_chunk_ids: HashSet<_> = old_chunks.keys().collect();
        let new_chunk_ids: HashSet<_> = new_chunks.keys().collect();
        let removed_ids: Vec<_> = old_chunk_ids
            .difference(&new_chunk_ids)
            .map(|s| s.to_string())
            .collect();

        let patch_size_bytes = new_or_modified.iter().map(|c| c.size).sum();

        Ok(DiffResult {
            new_or_modified_chunks: new_or_modified,
            removed_chunk_ids: removed_ids,
            identical_chunk_count: identical_count,
            patch_size_bytes,
        })
    }

    /// Create a patch bundle containing only the changed chunks
    pub fn create_patch_bundle(&self, new_bundle: &Bundle, diff: &DiffResult) -> Result<Bundle> {
        // Create new metadata for the patch bundle
        let mut patch_metadata = new_bundle.metadata.clone();
        patch_metadata.id = BundleId::new(); // Generate new ID
        patch_metadata.created_at = chrono::Utc::now();
        patch_metadata.chunks.clear(); // Clear chunk metadata
        patch_metadata.size_bytes = 0; // Reset size

        let mut patch_bundle = Bundle::new(patch_metadata);

        let new_or_modified_ids: HashSet<_> = diff
            .new_or_modified_chunks
            .iter()
            .map(|c| c.id.as_str())
            .collect();

        for chunk in &new_bundle.chunks {
            if new_or_modified_ids.contains(chunk.id()) {
                patch_bundle.add_chunk(chunk.clone())?;
            }
        }

        patch_bundle.validate()?;
        Ok(patch_bundle)
    }

    /// Apply a patch to an old bundle to create a new one
    pub fn apply_patch(&self, old_bundle: &Bundle, patch_bundle: &Bundle) -> Result<Bundle> {
        // Create new metadata for the reconstructed bundle
        let mut new_metadata = old_bundle.metadata.clone();
        new_metadata.id = BundleId::new(); // Generate new ID
        new_metadata.created_at = chrono::Utc::now();
        new_metadata.chunks.clear(); // Clear old chunk metadata
        new_metadata.size_bytes = 0; // Reset size

        let mut new_bundle = Bundle::new(new_metadata);

        let patch_chunks: HashMap<_, _> = patch_bundle
            .chunks
            .iter()
            .map(|c| (c.id().to_string(), c.clone()))
            .collect();

        let patch_chunk_ids: HashSet<_> = patch_chunks.keys().map(|s| s.as_str()).collect();

        // Add all chunks from the old bundle that are not in the patch
        for old_chunk in &old_bundle.chunks {
            if !patch_chunk_ids.contains(old_chunk.id()) {
                new_bundle.add_chunk(old_chunk.clone())?;
            }
        }

        // Add all chunks from the patch
        for (_, patch_chunk) in patch_chunks {
            new_bundle.add_chunk(patch_chunk.clone())?;
        }

        new_bundle.validate()?;
        Ok(new_bundle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CompressionType, Platform, SemanticVersion, bundle::BundleBuilder};

    fn create_test_bundle(
        version: &str,
        chunks: Vec<(&str, &str)>, // (id, data)
    ) -> Bundle {
        let semver = SemanticVersion::parse(version).unwrap();
        let mut builder = BundleBuilder::new(semver, Platform::Ios, "index.js".to_string())
            .with_compression(CompressionType::None);

        for (id, data) in chunks {
            builder
                .add_chunk_from_data(data.as_bytes(), id.to_string())
                .unwrap();
        }

        builder.build().unwrap()
    }

    #[test]
    fn test_bundle_comparison() {
        let old_bundle = create_test_bundle(
            "1.0.0",
            vec![("a", "apple"), ("b", "banana"), ("c", "cherry")],
        );

        let new_bundle = create_test_bundle(
            "1.0.1",
            vec![
                ("a", "apple"),     // Identical
                ("b", "blueberry"), // Modified
                ("d", "date"),      // New
            ],
        );

        let engine = DiffEngine::new();
        let diff = engine.compare_bundles(&old_bundle, &new_bundle).unwrap();

        assert!(diff.has_changes());
        assert_eq!(diff.new_or_modified_chunks.len(), 2);
        assert_eq!(diff.removed_chunk_ids.len(), 1);
        assert_eq!(diff.identical_chunk_count, 1);
        assert_eq!(diff.removed_chunk_ids[0], "c");
    }

    #[test]
    fn test_no_changes_diff() {
        let bundle = create_test_bundle("1.0.0", vec![("a", "data"), ("b", "data2")]);
        let engine = DiffEngine::new();
        let diff = engine.compare_bundles(&bundle, &bundle).unwrap();

        assert!(!diff.has_changes());
        assert_eq!(diff.new_or_modified_chunks.len(), 0);
        assert_eq!(diff.removed_chunk_ids.len(), 0);
        assert_eq!(diff.identical_chunk_count, 2);
    }

    #[test]
    fn test_create_and_apply_patch() {
        let old_bundle = create_test_bundle("1.0.0", vec![("a", "one"), ("b", "two")]);
        let new_bundle =
            create_test_bundle("1.0.1", vec![("a", "one"), ("b", "three"), ("c", "four")]);

        let engine = DiffEngine::new();
        let diff = engine.compare_bundles(&old_bundle, &new_bundle).unwrap();

        let patch_bundle = engine.create_patch_bundle(&new_bundle, &diff).unwrap();
        assert_eq!(patch_bundle.chunk_count(), 2);
        assert!(patch_bundle.find_chunk("b").is_some());
        assert!(patch_bundle.find_chunk("c").is_some());

        let reconstructed_bundle = engine.apply_patch(&old_bundle, &patch_bundle).unwrap();
        assert_eq!(reconstructed_bundle.chunk_count(), new_bundle.chunk_count());

        assert_eq!(
            reconstructed_bundle
                .find_chunk("b")
                .unwrap()
                .metadata
                .checksum,
            new_bundle.find_chunk("b").unwrap().metadata.checksum
        );
    }

    #[test]
    fn test_incompatible_bundle_diff() {
        let bundle1 = create_test_bundle("1.0.0", vec![]);
        let bundle2 = create_test_bundle("2.0.0", vec![]); // Different major version

        let engine = DiffEngine::new();
        let result = engine.compare_bundles(&bundle1, &bundle2);
        assert!(result.is_err());
    }
}
