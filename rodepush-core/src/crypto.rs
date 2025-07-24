use crate::{Result, BundleError};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Read;

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

/// Generate a checksum for a given file path
pub fn generate_file_checksum(
    file_path: &std::path::Path,
    algorithm: HashAlgorithm,
) -> Result<String> {
    let file = std::fs::File::open(file_path)
        .map_err(|e| BundleError::invalid_format(format!("Failed to open file: {}", e)))?;
    
    let mut reader = std::io::BufReader::new(file);
    
    let mut hasher: Box<dyn Hasher> = match algorithm {
        HashAlgorithm::Sha256 => Box::new(Sha256Hasher::new()),
        HashAlgorithm::Blake3 => Box::new(Blake3Hasher::new()),
    };

    let mut buffer = [0; 8192];
    loop {
        let bytes_read = reader.read(&mut buffer)
            .map_err(|e| BundleError::invalid_format(format!("Failed to read file: {}", e)))?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::NamedTempFile;
    use std::io::Write;

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
}