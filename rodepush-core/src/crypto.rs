use crate::{Result, BundleError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, BufReader};
use std::path::Path;

/// Supported hashing algorithms for bundle integrity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HashAlgorithm {
    /// SHA-256 (standard, secure)
    Sha256,
    /// Blake3 (modern, very fast)
    Blake3,
}

impl Default for HashAlgorithm {
    fn default() -> Self {
        Self::Blake3
    }
}

impl HashAlgorithm {
    /// Get the name of the algorithm as a string
    pub fn name(&self) -> &'static str {
        match self {
            HashAlgorithm::Sha256 => "sha256",
            HashAlgorithm::Blake3 => "blake3",
        }
    }

    /// Get the expected hash length in characters
    pub fn hash_length(&self) -> usize {
        match self {
            HashAlgorithm::Sha256 => 64, // 32 bytes * 2 hex chars
            HashAlgorithm::Blake3 => 64,  // 32 bytes * 2 hex chars
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "sha256" => Ok(HashAlgorithm::Sha256),
            "blake3" => Ok(HashAlgorithm::Blake3),
            _ => Err(BundleError::invalid_format(format!("Unknown hash algorithm: {}", s)).into()),
        }
    }
}

/// Hasher trait for creating content hashes
pub trait Hasher {
    /// Update the hasher with data
    fn update(&mut self, data: &[u8]);

    /// Finalize the hash and return it as a hex string
    fn finalize(&mut self) -> String;
}

/// SHA-256 hasher implementation
#[derive(Default)]
pub struct Sha256Hasher {
    hasher: Sha256,
}

impl Sha256Hasher {
    pub fn new() -> Self {
        Self { hasher: Sha256::new() }
    }
}

impl Hasher for Sha256Hasher {
    fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    fn finalize(&mut self) -> String {
        let result = self.hasher.finalize_reset();
        hex::encode(result)
    }
}

/// Blake3 hasher implementation
#[derive(Default)]
pub struct Blake3Hasher {
    hasher: blake3::Hasher,
}

impl Blake3Hasher {
    pub fn new() -> Self {
        Self { hasher: blake3::Hasher::new() }
    }
}

impl Hasher for Blake3Hasher {
    fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    fn finalize(&mut self) -> String {
        let result = self.hasher.finalize();
        self.hasher.reset();
        result.to_hex().to_string()
    }
}

/// Generic checksum verifier
#[derive(Debug)]
pub struct ChecksumVerifier {
    algorithm: HashAlgorithm,
}

impl ChecksumVerifier {
    /// Create a new verifier with a specific algorithm
    pub fn new(algorithm: HashAlgorithm) -> Self {
        Self { algorithm }
    }

    /// Calculate checksum for given data
    pub fn calculate(&self, data: &[u8]) -> String {
        match self.algorithm {
            HashAlgorithm::Sha256 => {
                let mut hasher = Sha256Hasher::new();
                hasher.update(data);
                hasher.finalize()
            }
            HashAlgorithm::Blake3 => {
                let mut hasher = Blake3Hasher::new();
                hasher.update(data);
                hasher.finalize()
            }
        }
    }

    /// Verify data against an expected checksum (case-insensitive)
    pub fn verify(&self, data: &[u8], expected_checksum: &str) -> Result<()> {
        let actual_checksum = self.calculate(data);
        if actual_checksum.eq_ignore_ascii_case(expected_checksum) {
            Ok(())
        } else {
            Err(BundleError::checksum_mismatch(
                expected_checksum.to_string(),
                actual_checksum,
            ).into())
        }
    }

    /// Verify a stream of data against an expected checksum
    pub fn verify_stream<R: std::io::Read>(
        &self,
        mut reader: R,
        expected_checksum: &str,
    ) -> Result<()> {
        let mut hasher: Box<dyn Hasher> = match self.algorithm {
            HashAlgorithm::Sha256 => Box::new(Sha256Hasher::new()),
            HashAlgorithm::Blake3 => Box::new(Blake3Hasher::new()),
        };

        let mut buffer = [0; 8192];
        loop {
            let bytes_read = reader.read(&mut buffer)
                .map_err(|e| BundleError::invalid_format(format!("Failed to read stream: {}", e)))?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let actual_checksum = hasher.finalize();

        if actual_checksum.eq_ignore_ascii_case(expected_checksum) {
            Ok(())
        } else {
            Err(BundleError::checksum_mismatch(
                expected_checksum.to_string(),
                actual_checksum,
            ).into())
        }
    }
}

/// Securely compare two checksums to prevent timing attacks
pub fn secure_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    
    // Use subtle crate for constant-time comparison
    subtle::ConstantTimeEq::ct_eq(a.as_bytes(), b.as_bytes()).into()
}

/// Progress callback type for hashing operations
pub type ProgressCallback = dyn Fn(u64, u64) + Send + Sync;

/// Generate a checksum for a given file path
pub fn generate_file_checksum(
    file_path: &Path,
    algorithm: HashAlgorithm,
) -> Result<String> {
    generate_file_checksum_with_progress(file_path, algorithm, None)
}

/// Generate a checksum for a given file path with optional progress callback
pub fn generate_file_checksum_with_progress(
    file_path: &Path,
    algorithm: HashAlgorithm,
    progress_callback: Option<&ProgressCallback>,
) -> Result<String> {
    let file = std::fs::File::open(file_path)
        .map_err(|e| BundleError::invalid_format(format!("Failed to open file: {}", e)))?;
    
    let file_size = file.metadata()
        .map_err(|e| BundleError::invalid_format(format!("Failed to get file metadata: {}", e)))?
        .len();
    
    let mut reader = BufReader::new(file);
    
    let mut hasher: Box<dyn Hasher> = match algorithm {
        HashAlgorithm::Sha256 => Box::new(Sha256Hasher::new()),
        HashAlgorithm::Blake3 => Box::new(Blake3Hasher::new()),
    };

    let mut buffer = [0; 32768]; // Larger buffer for better performance
    let mut bytes_processed = 0u64;
    
    loop {
        let bytes_read = reader.read(&mut buffer)
            .map_err(|e| BundleError::invalid_format(format!("Failed to read file: {}", e)))?;
        if bytes_read == 0 {
            break;
        }
        
        hasher.update(&buffer[..bytes_read]);
        bytes_processed += bytes_read as u64;
        
        if let Some(callback) = progress_callback.as_ref() {
            callback(bytes_processed, file_size);
        }
    }

    Ok(hasher.finalize())
}

/// Hash multiple files and return their checksums
pub fn generate_multiple_file_checksums(
    file_paths: &[&Path],
    algorithm: HashAlgorithm,
    progress_callback: Option<&ProgressCallback>,
) -> Result<Vec<(String, String)>> {
    let mut results = Vec::with_capacity(file_paths.len());
    let total_files = file_paths.len() as u64;
    
    for (index, path) in file_paths.iter().enumerate() {
        let checksum = generate_file_checksum(path, algorithm)?;
        let path_str = path.to_string_lossy().to_string();
        results.push((path_str, checksum));
        
        if let Some(callback) = progress_callback.as_ref() {
            callback(index as u64 + 1, total_files);
        }
    }
    
    Ok(results)
}

/// Validate a hash string format for the given algorithm
pub fn validate_hash_format(hash: &str, algorithm: HashAlgorithm) -> Result<()> {
    if hash.len() != algorithm.hash_length() {
        return Err(BundleError::invalid_format(
            format!(
                "Invalid hash length for {}: expected {}, got {}",
                algorithm.name(),
                algorithm.hash_length(),
                hash.len()
            )
        ).into());
    }
    
    if !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(BundleError::invalid_format(
            format!("Hash contains non-hexadecimal characters: {}", hash)
        ).into());
    }
    
    Ok(())
}

/// Utility for bulk checksum operations
pub struct BulkHasher {
    algorithm: HashAlgorithm,
    buffer_size: usize,
}

impl BulkHasher {
    pub fn new(algorithm: HashAlgorithm) -> Self {
        Self {
            algorithm,
            buffer_size: 32768,
        }
    }

    pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size.max(1024); // Minimum buffer size
        self
    }

    /// Hash a single piece of data
    pub fn hash_data(&self, data: &[u8]) -> String {
        let verifier = ChecksumVerifier::new(self.algorithm);
        verifier.calculate(data)
    }

    /// Hash data from a reader
    pub fn hash_reader<R: Read>(&self, mut reader: R) -> Result<String> {
        let mut hasher: Box<dyn Hasher> = match self.algorithm {
            HashAlgorithm::Sha256 => Box::new(Sha256Hasher::new()),
            HashAlgorithm::Blake3 => Box::new(Blake3Hasher::new()),
        };

        let mut buffer = vec![0; self.buffer_size];
        loop {
            let bytes_read = reader.read(&mut buffer)
                .map_err(|e| BundleError::invalid_format(format!("Failed to read: {}", e)))?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(hasher.finalize())
    }

    /// Hash a file with this configuration
    pub fn hash_file(&self, file_path: &Path) -> Result<String> {
        let file = std::fs::File::open(file_path)
            .map_err(|e| BundleError::invalid_format(format!("Failed to open file: {}", e)))?;
        self.hash_reader(BufReader::new(file))
    }

    /// Hash multiple data chunks and return combined result
    pub fn hash_chunks(&self, chunks: &[&[u8]]) -> String {
        let mut hasher: Box<dyn Hasher> = match self.algorithm {
            HashAlgorithm::Sha256 => Box::new(Sha256Hasher::new()),
            HashAlgorithm::Blake3 => Box::new(Blake3Hasher::new()),
        };

        for chunk in chunks {
            hasher.update(chunk);
        }

        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::NamedTempFile;
    use std::io::Write;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_sha256_hasher() {
        let data = b"hello world";
        let expected_hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";

        let mut hasher = Sha256Hasher::new();
        hasher.update(data);
        let actual_hash = hasher.finalize();

        assert_eq!(actual_hash, expected_hash);
    }

    #[test]
    fn test_blake3_hasher() {
        let data = b"hello world";
        let expected_hash = "d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24";

        let mut hasher = Blake3Hasher::new();
        hasher.update(data);
        let actual_hash = hasher.finalize();

        assert_eq!(actual_hash, expected_hash);
    }

    #[test]
    fn test_checksum_verifier() {
        let data = b"test verifier";
        let verifier_sha256 = ChecksumVerifier::new(HashAlgorithm::Sha256);
        let checksum_sha256 = verifier_sha256.calculate(data);

        assert!(verifier_sha256.verify(data, &checksum_sha256).is_ok());
        assert!(verifier_sha256.verify(data, "invalid_checksum").is_err());

        let verifier_blake3 = ChecksumVerifier::new(HashAlgorithm::Blake3);
        let checksum_blake3 = verifier_blake3.calculate(data);

        assert!(verifier_blake3.verify(data, &checksum_blake3).is_ok());
        assert!(verifier_blake3.verify(data, "invalid_checksum").is_err());
    }

    #[test]
    fn test_checksum_verifier_stream() {
        let data = b"test stream verifier";
        let cursor = Cursor::new(data);
        
        let verifier = ChecksumVerifier::new(HashAlgorithm::Sha256);
        let checksum = verifier.calculate(data);

        assert!(verifier.verify_stream(cursor, &checksum).is_ok());
    }

    #[test]
    fn test_secure_compare() {
        let a = "checksum123";
        let b = "checksum123";
        let c = "checksum456";

        assert!(secure_compare(a, b));
        assert!(!secure_compare(a, c));
        assert!(!secure_compare(a, "short")); // Different lengths
    }

    #[test]
    fn test_hash_algorithm_properties() {
        assert_eq!(HashAlgorithm::Sha256.name(), "sha256");
        assert_eq!(HashAlgorithm::Blake3.name(), "blake3");
        
        assert_eq!(HashAlgorithm::Sha256.hash_length(), 64);
        assert_eq!(HashAlgorithm::Blake3.hash_length(), 64);
        
        assert_eq!(HashAlgorithm::from_str("sha256").unwrap(), HashAlgorithm::Sha256);
        assert_eq!(HashAlgorithm::from_str("blake3").unwrap(), HashAlgorithm::Blake3);
        assert!(HashAlgorithm::from_str("unknown").is_err());
    }

    #[test]
    fn test_generate_file_checksum() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test file checksum").unwrap();
        let path = temp_file.path();

        let checksum_sha256 = generate_file_checksum(path, HashAlgorithm::Sha256).unwrap();
        let verifier_sha256 = ChecksumVerifier::new(HashAlgorithm::Sha256);
        assert!(verifier_sha256.verify(b"test file checksum", &checksum_sha256).is_ok());

        let checksum_blake3 = generate_file_checksum(path, HashAlgorithm::Blake3).unwrap();
        let verifier_blake3 = ChecksumVerifier::new(HashAlgorithm::Blake3);
        assert!(verifier_blake3.verify(b"test file checksum", &checksum_blake3).is_ok());
    }

    #[test]
    fn test_generate_file_checksum_with_progress() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"test file checksum with progress tracking";
        temp_file.write_all(test_data).unwrap();
        let path = temp_file.path();

        let progress_calls = Arc::new(AtomicUsize::new(0));
        let progress_calls_clone = progress_calls.clone();
        let progress_callback = move |_bytes_processed: u64, _total_bytes: u64| {
            progress_calls_clone.fetch_add(1, Ordering::SeqCst);
        };

        let checksum = generate_file_checksum_with_progress(
            path, 
            HashAlgorithm::Blake3, 
            Some(&progress_callback)
        ).unwrap();
        
        let verifier = ChecksumVerifier::new(HashAlgorithm::Blake3);
        assert!(verifier.verify(test_data, &checksum).is_ok());
        assert!(progress_calls.load(Ordering::SeqCst) > 0);
    }

    #[test]
    fn test_validate_hash_format() {
        let valid_sha256 = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        let valid_blake3 = "d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24";
        
        assert!(validate_hash_format(valid_sha256, HashAlgorithm::Sha256).is_ok());
        assert!(validate_hash_format(valid_blake3, HashAlgorithm::Blake3).is_ok());
        
        // Test invalid length
        assert!(validate_hash_format("too_short", HashAlgorithm::Sha256).is_err());
        
        // Test non-hex characters
        assert!(validate_hash_format(
            "g94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9", 
            HashAlgorithm::Sha256
        ).is_err());
    }

    #[test]
    fn test_bulk_hasher() {
        let hasher = BulkHasher::new(HashAlgorithm::Blake3)
            .with_buffer_size(4096);
        
        let data1 = b"chunk 1";
        let data2 = b"chunk 2";
        let data3 = b"chunk 3";
        
        // Test individual data hashing
        let hash1 = hasher.hash_data(data1);
        assert_eq!(hash1.len(), 64);
        
        // Test chunk hashing
        let chunks = vec![data1.as_slice(), data2.as_slice(), data3.as_slice()];
        let combined_hash = hasher.hash_chunks(&chunks);
        assert_eq!(combined_hash.len(), 64);
        
        // Test reader hashing
        let mut combined_data = Vec::new();
        combined_data.extend_from_slice(data1);
        combined_data.extend_from_slice(data2);
        combined_data.extend_from_slice(data3);
        
        let reader_hash = hasher.hash_reader(Cursor::new(&combined_data)).unwrap();
        assert_eq!(reader_hash, combined_hash);
    }

    #[test]
    fn test_bulk_hasher_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"bulk hasher test file content").unwrap();
        let path = temp_file.path();

        let hasher = BulkHasher::new(HashAlgorithm::Sha256);
        let file_hash = hasher.hash_file(path).unwrap();
        
        // Verify it matches direct calculation
        let direct_hash = generate_file_checksum(path, HashAlgorithm::Sha256).unwrap();
        assert_eq!(file_hash, direct_hash);
    }

    #[test]
    fn test_generate_multiple_file_checksums() {
        let mut temp_file1 = NamedTempFile::new().unwrap();
        temp_file1.write_all(b"file 1 content").unwrap();
        
        let mut temp_file2 = NamedTempFile::new().unwrap();
        temp_file2.write_all(b"file 2 content").unwrap();
        
        let paths: Vec<&Path> = vec![temp_file1.path(), temp_file2.path()];
        
        let progress_calls = Arc::new(AtomicUsize::new(0));
        let progress_calls_clone = progress_calls.clone();
        let progress_callback = move |_current: u64, _total: u64| {
            progress_calls_clone.fetch_add(1, Ordering::SeqCst);
        };
        
        let results = generate_multiple_file_checksums(
            &paths,
            HashAlgorithm::Blake3,
            Some(&progress_callback),
        ).unwrap();
        
        assert_eq!(results.len(), 2);
        assert_eq!(progress_calls.load(Ordering::SeqCst), 2);
        
        // Verify checksums are correct
        for (path_str, checksum) in results {
            let path = Path::new(&path_str);
            let expected = generate_file_checksum(path, HashAlgorithm::Blake3).unwrap();
            assert_eq!(checksum, expected);
        }
    }
}