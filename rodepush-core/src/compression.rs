use crate::{Result, BundleError};

/// Compressor trait for compressing and decompressing data
pub trait Compressor {
    /// Compress data with a given quality/level
    fn compress(&self, data: &[u8], level: i32) -> Result<Vec<u8>>;
    
    /// Decompress data
    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>>;
}

/// Zstandard compressor implementation
#[derive(Default)]
pub struct ZstdCompressor;

impl ZstdCompressor {
    pub fn new() -> Self {
        Self
    }
}

impl Compressor for ZstdCompressor {
    fn compress(&self, data: &[u8], level: i32) -> Result<Vec<u8>> {
        zstd::stream::encode_all(data, level)
            .map_err(|e| BundleError::compression_failed(e.to_string()).into())
    }

    fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        zstd::stream::decode_all(data)
            .map_err(|e| BundleError::DecompressionFailed { message: e.to_string() }.into())
    }
}

/// Compression utility for easy access to different algorithms
pub struct CompressionUtil {
    compressor: Box<dyn Compressor>,
}

impl CompressionUtil {
    /// Create a new utility with a specific compression type
    pub fn new(compression_type: crate::bundle::CompressionType) -> Self {
        let compressor: Box<dyn Compressor> = match compression_type {
            crate::bundle::CompressionType::Zstd => Box::new(ZstdCompressor::new()),
            // In the future, other compressors like Gzip or Brotli can be added here
            _ => unimplemented!("Only Zstd compression is currently supported"),
        };
        Self { compressor }
    }

    /// Compress data with a configured compressor
    pub fn compress(&self, data: &[u8], level: i32) -> Result<Vec<u8>> {
        self.compressor.compress(data, level)
    }

    /// Decompress data with a configured compressor
    pub fn decompress(&self, data: &[u8]) -> Result<Vec<u8>> {
        self.compressor.decompress(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bundle::CompressionType;

    #[test]
    fn test_zstd_compression() {
        let compressor = ZstdCompressor::new();
        let original_data = b"This is some test data for Zstandard compression. It should be compressible.".to_vec();

        // Default compression level (3)
        let compressed_data = compressor.compress(&original_data, 3).unwrap();
        assert!(!compressed_data.is_empty());
        assert!(compressed_data.len() < original_data.len());

        let decompressed_data = compressor.decompress(&compressed_data).unwrap();
        assert_eq!(original_data, decompressed_data);
    }

    #[test]
    fn test_zstd_decompress_invalid_data() {
        let compressor = ZstdCompressor::new();
        let invalid_data = b"this is not valid compressed data";

        let result = compressor.decompress(invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_compression_util() {
        let util = CompressionUtil::new(CompressionType::Zstd);
        let original_data = b"Test data for CompressionUtil";

        let compressed = util.compress(original_data, 5).unwrap();
        let decompressed = util.decompress(&compressed).unwrap();

        assert_eq!(original_data.to_vec(), decompressed);
    }
}