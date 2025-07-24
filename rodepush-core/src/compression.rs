use crate::bundle::CompressionType;
use crate::{BundleError, Result};
use std::io::{Read, Write};

/// Compression statistics for performance monitoring
#[derive(Debug, Clone, PartialEq)]
pub struct CompressionStats {
    /// Original size before compression
    pub original_size: u64,
    /// Size after compression
    pub compressed_size: u64,
    /// Time taken for compression in milliseconds
    pub compression_time_ms: u64,
    /// Compression ratio (compressed_size / original_size)
    pub ratio: f64,
    /// Compression level used
    pub level: i32,
}

impl CompressionStats {
    /// Create new compression stats
    pub fn new(
        original_size: u64,
        compressed_size: u64,
        compression_time_ms: u64,
        level: i32,
    ) -> Self {
        let ratio = if original_size > 0 {
            compressed_size as f64 / original_size as f64
        } else {
            1.0
        };

        Self {
            original_size,
            compressed_size,
            compression_time_ms,
            ratio,
            level,
        }
    }

    /// Calculate space savings as percentage
    pub fn space_savings_percent(&self) -> f64 {
        if self.original_size == 0 {
            0.0
        } else {
            ((self.original_size - self.compressed_size) as f64 / self.original_size as f64) * 100.0
        }
    }
}

/// Compressor trait for compressing and decompressing data
pub trait Compressor: Send + Sync {
    /// Compress data with a given quality/level
    fn compress(&self, data: &[u8], level: i32) -> Result<Vec<u8>>;

    /// Decompress data
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>>;

    /// Compress data with timing statistics
    fn compress_with_stats(&self, data: &[u8], level: i32) -> Result<(Vec<u8>, CompressionStats)> {
        let start = std::time::Instant::now();
        let original_size = data.len() as u64;

        let compressed = self.compress(data, level)?;
        let compression_time_ms = start.elapsed().as_millis() as u64;
        let compressed_size = compressed.len() as u64;

        let stats =
            CompressionStats::new(original_size, compressed_size, compression_time_ms, level);

        Ok((compressed, stats))
    }

    /// Get optimal compression level for this compressor
    fn default_level(&self) -> i32 {
        3
    }

    /// Get maximum compression level for this compressor
    fn max_level(&self) -> i32 {
        22
    }

    /// Get minimum compression level for this compressor
    fn min_level(&self) -> i32 {
        1
    }
}

/// Zstandard compressor implementation
#[derive(Default)]
pub struct ZstdCompressor;

impl ZstdCompressor {
    pub fn new() -> Self {
        Self
    }

    /// Validate compression level for Zstd
    fn validate_level(&self, level: i32) -> Result<i32> {
        if level < self.min_level() || level > self.max_level() {
            return Err(BundleError::compression_failed(format!(
                "Invalid Zstd compression level {}. Valid range: {}-{}",
                level,
                self.min_level(),
                self.max_level()
            ))
            .into());
        }
        Ok(level)
    }
}

impl Compressor for ZstdCompressor {
    fn compress(&self, data: &[u8], level: i32) -> Result<Vec<u8>> {
        let validated_level = self.validate_level(level)?;

        zstd::stream::encode_all(data, validated_level)
            .map_err(|e| BundleError::compression_failed(e.to_string()).into())
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        zstd::stream::decode_all(data).map_err(|e| {
            BundleError::DecompressionFailed {
                message: e.to_string(),
            }
            .into()
        })
    }

    fn default_level(&self) -> i32 {
        3
    }

    fn max_level(&self) -> i32 {
        22
    }

    fn min_level(&self) -> i32 {
        1
    }
}

/// No-op compressor (pass-through)
#[derive(Default)]
pub struct NoneCompressor;

impl NoneCompressor {
    pub fn new() -> Self {
        Self
    }
}

impl Compressor for NoneCompressor {
    fn compress(&self, data: &[u8], _level: i32) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        Ok(data.to_vec())
    }

    fn default_level(&self) -> i32 {
        0
    }

    fn max_level(&self) -> i32 {
        0
    }

    fn min_level(&self) -> i32 {
        0
    }
}

/// Compression utility for easy access to different algorithms
pub struct CompressionUtil {
    compressor: Box<dyn Compressor>,
    compression_type: CompressionType,
}

impl CompressionUtil {
    /// Create a new utility with a specific compression type
    pub fn new(compression_type: CompressionType) -> Self {
        let compressor: Box<dyn Compressor> = match compression_type {
            CompressionType::None => Box::new(NoneCompressor::new()),
            CompressionType::Zstd => Box::new(ZstdCompressor::new()),
            CompressionType::Gzip => return Self::unsupported_compression_type("Gzip"),
            CompressionType::Brotli => return Self::unsupported_compression_type("Brotli"),
        };
        Self {
            compressor,
            compression_type,
        }
    }

    fn unsupported_compression_type(name: &str) -> Self {
        panic!(
            "{} compression is not yet implemented. Currently supported: None, Zstd",
            name
        );
    }

    /// Get the compression type
    pub fn compression_type(&self) -> CompressionType {
        self.compression_type
    }

    /// Compress data with a configured compressor
    pub fn compress(&self, data: &[u8], level: Option<i32>) -> Result<Vec<u8>> {
        let compression_level = level.unwrap_or_else(|| self.compressor.default_level());
        self.compressor.compress(data, compression_level)
    }

    /// Decompress data with a configured compressor
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        self.compressor.decompress(data)
    }

    /// Compress data with timing statistics
    pub fn compress_with_stats(
        &self,
        data: &[u8],
        level: Option<i32>,
    ) -> Result<(Vec<u8>, CompressionStats)> {
        let compression_level = level.unwrap_or_else(|| self.compressor.default_level());
        self.compressor.compress_with_stats(data, compression_level)
    }

    /// Compress data to a writer
    pub fn compress_to_writer<W: Write>(
        &self,
        data: &[u8],
        writer: W,
        level: Option<i32>,
    ) -> Result<u64> {
        let compressed = self.compress(data, level)?;
        let bytes_written = compressed.len() as u64;

        let mut writer = writer;
        writer
            .write_all(&compressed)
            .map_err(|e| BundleError::compression_failed(format!("Write failed: {}", e)))?;

        Ok(bytes_written)
    }

    /// Decompress data from a reader
    pub fn decompress_from_reader<R: Read>(&self, mut reader: R) -> Result<Vec<u8>> {
        let mut compressed_data = Vec::new();
        reader
            .read_to_end(&mut compressed_data)
            .map_err(|e| BundleError::DecompressionFailed {
                message: format!("Read failed: {}", e),
            })?;

        self.decompress(&compressed_data)
    }

    /// Get optimal compression level for the current compressor
    pub fn default_level(&self) -> i32 {
        self.compressor.default_level()
    }

    /// Get compression level range
    pub fn level_range(&self) -> (i32, i32) {
        (self.compressor.min_level(), self.compressor.max_level())
    }

    /// Test compression effectiveness for given data
    pub fn test_compression(&self, data: &[u8], levels: Vec<i32>) -> Result<Vec<CompressionStats>> {
        let mut results = Vec::new();

        for level in levels {
            if level < self.compressor.min_level() || level > self.compressor.max_level() {
                continue;
            }

            match self.compress_with_stats(data, Some(level)) {
                Ok((_, stats)) => results.push(stats),
                Err(_) => continue,
            }
        }

        Ok(results)
    }
}

impl Clone for CompressionUtil {
    fn clone(&self) -> Self {
        Self::new(self.compression_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::CompressionType;

    #[test]
    fn test_compression_stats() {
        let stats = CompressionStats::new(1000, 500, 10, 3);
        assert_eq!(stats.original_size, 1000);
        assert_eq!(stats.compressed_size, 500);
        assert_eq!(stats.ratio, 0.5);
        assert_eq!(stats.space_savings_percent(), 50.0);
    }

    #[test]
    fn test_zstd_compressor() {
        let compressor = ZstdCompressor::new();
        let original_data = b"This is some test data for Zstandard compression. It should be compressible and demonstrate good compression ratios.".to_vec();

        // Test compression and decompression
        let compressed_data = compressor.compress(&original_data, 3).unwrap();
        assert!(!compressed_data.is_empty());
        assert!(compressed_data.len() < original_data.len());

        let decompressed_data = compressor.decompress(&compressed_data).unwrap();
        assert_eq!(original_data, decompressed_data);

        // Test compression with stats
        let (compressed_with_stats, stats) =
            compressor.compress_with_stats(&original_data, 3).unwrap();
        assert_eq!(compressed_data, compressed_with_stats);
        assert_eq!(stats.original_size, original_data.len() as u64);
        assert_eq!(stats.compressed_size, compressed_data.len() as u64);
        assert_eq!(stats.level, 3);
        // Compression time can be 0 for small data on fast machines
    }

    #[test]
    fn test_zstd_validation() {
        let compressor = ZstdCompressor::new();

        // Test valid compression levels
        assert!(compressor.compress(b"test", 1).is_ok());
        assert!(compressor.compress(b"test", 3).is_ok());
        assert!(compressor.compress(b"test", 22).is_ok());

        // Test invalid compression levels
        assert!(compressor.compress(b"test", 0).is_err());
        assert!(compressor.compress(b"test", 23).is_err());
        assert!(compressor.compress(b"test", -1).is_err());
    }

    #[test]
    fn test_zstd_decompress_invalid_data() {
        let compressor = ZstdCompressor::new();
        let invalid_data = b"this is not valid compressed data";

        let result = compressor.decompress(invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_none_compressor() {
        let compressor = NoneCompressor::new();
        let original_data = b"This data should not be compressed";

        let compressed = compressor.compress(original_data, 0).unwrap();
        assert_eq!(original_data, compressed.as_slice());

        let decompressed = compressor.decompress(&compressed).unwrap();
        assert_eq!(original_data, decompressed.as_slice());

        // Test that stats work
        let (compressed_with_stats, stats) =
            compressor.compress_with_stats(original_data, 0).unwrap();
        assert_eq!(compressed, compressed_with_stats);
        assert_eq!(stats.ratio, 1.0);
        assert_eq!(stats.space_savings_percent(), 0.0);
    }

    #[test]
    fn test_compression_util_zstd() {
        let util = CompressionUtil::new(CompressionType::Zstd);
        let original_data = b"Test data for CompressionUtil with Zstandard compression";

        // Test with default compression level
        let compressed = util.compress(original_data, None).unwrap();
        let decompressed = util.decompress(&compressed).unwrap();
        assert_eq!(original_data.to_vec(), decompressed);

        // Test with specific compression level
        let compressed_level_5 = util.compress(original_data, Some(5)).unwrap();
        let decompressed_level_5 = util.decompress(&compressed_level_5).unwrap();
        assert_eq!(original_data.to_vec(), decompressed_level_5);

        // Test compression with stats
        let (compressed_stats, stats) = util.compress_with_stats(original_data, Some(7)).unwrap();
        let decompressed_stats = util.decompress(&compressed_stats).unwrap();
        assert_eq!(original_data.to_vec(), decompressed_stats);
        assert_eq!(stats.level, 7);
    }

    #[test]
    fn test_compression_util_none() {
        let util = CompressionUtil::new(CompressionType::None);
        let original_data = b"Test data with no compression";

        let compressed = util.compress(original_data, None).unwrap();
        assert_eq!(original_data.to_vec(), compressed);

        let decompressed = util.decompress(&compressed).unwrap();
        assert_eq!(original_data.to_vec(), decompressed);
    }

    #[test]
    fn test_compression_util_properties() {
        let util = CompressionUtil::new(CompressionType::Zstd);

        assert_eq!(util.compression_type(), CompressionType::Zstd);
        assert_eq!(util.default_level(), 3);

        let (min, max) = util.level_range();
        assert_eq!(min, 1);
        assert_eq!(max, 22);
    }

    #[test]
    fn test_compression_util_io() {
        let util = CompressionUtil::new(CompressionType::Zstd);
        let original_data = b"Test data for I/O operations with compression";

        // Test compression to writer
        let mut writer = Vec::new();
        let bytes_written = util
            .compress_to_writer(original_data, &mut writer, Some(3))
            .unwrap();
        assert!(bytes_written > 0);
        assert_eq!(writer.len(), bytes_written as usize);

        // Test decompression from reader
        let reader = std::io::Cursor::new(writer);
        let decompressed = util.decompress_from_reader(reader).unwrap();
        assert_eq!(original_data.to_vec(), decompressed);
    }

    #[test]
    fn test_compression_effectiveness_test() {
        let util = CompressionUtil::new(CompressionType::Zstd);
        let test_data = b"This is repetitive test data. This is repetitive test data. This is repetitive test data.".repeat(10);

        let test_levels = vec![1, 3, 6, 9];
        let results = util.test_compression(&test_data, test_levels).unwrap();

        assert_eq!(results.len(), 4);

        // Verify that higher compression levels generally produce better ratios
        for result in results {
            assert!(result.ratio < 1.0); // Should achieve some compression
            assert!(result.space_savings_percent() > 0.0);
        }
    }

    #[test]
    fn test_compression_util_clone() {
        let util1 = CompressionUtil::new(CompressionType::Zstd);
        let util2 = util1.clone();

        assert_eq!(util1.compression_type(), util2.compression_type());
        assert_eq!(util1.default_level(), util2.default_level());

        let test_data = b"Test cloning functionality";
        let compressed1 = util1.compress(test_data, None).unwrap();
        let compressed2 = util2.compress(test_data, None).unwrap();
        assert_eq!(compressed1, compressed2);
    }

    #[test]
    #[should_panic(expected = "Gzip compression is not yet implemented")]
    fn test_unsupported_gzip() {
        CompressionUtil::new(CompressionType::Gzip);
    }

    #[test]
    #[should_panic(expected = "Brotli compression is not yet implemented")]
    fn test_unsupported_brotli() {
        CompressionUtil::new(CompressionType::Brotli);
    }
}
