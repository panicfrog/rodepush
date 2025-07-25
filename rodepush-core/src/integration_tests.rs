//! Integration tests for RodePush core functionality
//!
//! These tests verify the complete workflow from bundle creation to asset management.

use crate::{
    AssetCollection, AssetCompressor, Bundle, BundleBuilder, BundleCache, BundleMetadata,
    CompressionType, HashAlgorithm, LogContext, Platform, Result, RodePushError, SemanticVersion,
};
use std::time::Instant;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_end_to_end_bundle_creation() -> Result<()> {
        let context = LogContext::new("integration_test", "test");
        context.info("Starting end-to-end bundle creation test");

        // Create a bundle
        let version = SemanticVersion::new(1, 0, 0);
        let mut builder = BundleBuilder::new(version, Platform::Ios, "index.js".to_string())
            .with_compression(CompressionType::Zstd)
            .with_hash_algorithm(HashAlgorithm::Blake3);

        // Add some test data (larger to ensure compression works)
        let test_data = b"console.log('Hello, World!'); console.log('This is a test bundle for RodePush'); console.log('Testing compression and hashing functionality');".repeat(10);
        builder.add_chunk_from_data(&test_data, "main".to_string())?;

        let bundle = builder.build()?;
        context.log_bundle_operation("create", &bundle.id(), &bundle.metadata);

        // Validate the bundle
        bundle.validate()?;

        // Test cache functionality
        let cache = BundleCache::new(10);
        cache.put(bundle.clone());

        let cached_bundle = cache.get(&bundle.id());
        assert!(cached_bundle.is_some());
        assert_eq!(cached_bundle.unwrap(), bundle);

        context.info("End-to-end bundle creation test completed successfully");
        Ok(())
    }

    #[test]
    fn test_large_file_handling() -> Result<()> {
        let context = LogContext::new("large_file_test", "test");
        context.info("Starting large file handling test");

        // Create a large test data (1MB)
        let large_data: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();

        let version = SemanticVersion::new(1, 0, 0);
        let mut builder = BundleBuilder::new(version, Platform::Android, "large.js".to_string())
            .with_compression(CompressionType::Zstd);

        builder.add_chunk_from_data(&large_data, "large_chunk".to_string())?;

        let start_time = Instant::now();
        let bundle = builder.build()?;
        let build_time = start_time.elapsed();

        context.log_performance("large_bundle_build", build_time.as_millis() as u64, &[]);

        // Verify compression worked
        assert!(bundle.size() < large_data.len() as u64);

        context.info("Large file handling test completed successfully");
        Ok(())
    }

    #[test]
    fn test_asset_collection_workflow() -> Result<()> {
        let context = LogContext::new("asset_workflow_test", "test");
        context.info("Starting asset collection workflow test");

        // Create an asset collection
        let collection = AssetCollection::new();

        // Test compression workflow with empty collection
        let compressed = AssetCompressor::compress_collection(&collection)?;

        context.log_asset_operation(
            "compress",
            &collection.id,
            collection.len(),
            collection.total_size,
        );

        // Decompress and verify
        let decompressed = AssetCompressor::decompress_collection(&compressed)?;
        // Note: AssetCollection ID is regenerated during compression/decompression
        // so we verify the structure instead of the ID
        assert_eq!(decompressed.len(), collection.len());
        assert_eq!(decompressed.total_size, collection.total_size);

        context.info("Asset collection workflow test completed successfully");
        Ok(())
    }

    #[test]
    fn test_error_handling_integration() {
        let context = LogContext::new("error_handling_test", "test");

        // Test invalid bundle creation
        let result = BundleBuilder::new(
            SemanticVersion::new(1, 0, 0),
            Platform::Ios,
            "".to_string(), // Invalid empty entry point
        )
        .build();

        assert!(result.is_err());

        if let Err(RodePushError::Bundle(bundle_error)) = result {
            context.log_error(
                &RodePushError::Bundle(bundle_error),
                "Invalid bundle creation",
            );
        }

        // Test cache with zero size
        let cache = BundleCache::new(0); // Zero size cache
        let bundle = Bundle::new(BundleMetadata::new(
            SemanticVersion::new(1, 0, 0),
            Platform::Ios,
            "test.js".to_string(),
        ));

        cache.put(bundle.clone());
        // With zero size cache, nothing should be stored
        assert_eq!(cache.stats().size, 0);
        assert_eq!(cache.get(&bundle.id()), None);
    }

    #[test]
    fn test_performance_metrics() -> Result<()> {
        let context = LogContext::new("performance_test", "test");

        // Test bundle creation performance
        let start_time = Instant::now();

        let version = SemanticVersion::new(1, 0, 0);
        let mut builder = BundleBuilder::new(version, Platform::Both, "perf.js".to_string());

        // Add multiple chunks (larger data to ensure compression works)
        for i in 0..10 {
            let chunk_data = format!(
                "chunk_{}_with_more_data_to_ensure_compression_works_properly_for_testing_purposes",
                i
            )
            .repeat(5)
            .into_bytes();
            builder.add_chunk_from_data(&chunk_data, format!("chunk_{}", i))?;
        }

        let bundle = builder.build()?;
        let build_time = start_time.elapsed();

        context.log_performance(
            "multi_chunk_bundle_build",
            build_time.as_millis() as u64,
            &[
                ("chunk_count", &bundle.chunk_count().to_string()),
                ("total_size", &bundle.size().to_string()),
            ],
        );

        // Test cache performance
        let cache = BundleCache::new(100);
        let cache_start = Instant::now();

        for _ in 0..50 {
            let test_bundle = Bundle::new(BundleMetadata::new(
                SemanticVersion::new(1, 0, 0),
                Platform::Ios,
                "cache_test.js".to_string(),
            ));
            cache.put(test_bundle);
        }

        let cache_time = cache_start.elapsed();
        context.log_performance(
            "cache_operations",
            cache_time.as_millis() as u64,
            &[("operations", "50")],
        );

        Ok(())
    }

    #[test]
    fn test_concurrent_access() -> Result<()> {
        use std::sync::Arc;
        use std::thread;

        let context = LogContext::new("concurrent_test", "test");
        context.info("Starting concurrent access test");

        let cache = Arc::new(BundleCache::new(100));
        let mut handles = vec![];

        // Spawn multiple threads to test concurrent access
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                let bundle = Bundle::new(BundleMetadata::new(
                    SemanticVersion::new(1, 0, i),
                    Platform::Ios,
                    format!("concurrent_{}.js", i),
                ));
                cache_clone.put(bundle);
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all bundles were added
        assert_eq!(cache.stats().size, 10);

        context.info("Concurrent access test completed successfully");
        Ok(())
    }
}
