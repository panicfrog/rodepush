# RodePush Design Document

## Overview

RodePush is a Rust-based React Native CodePush solution designed for high performance, security, and scalability. The system uses a modular architecture with three main components that share a common core library. The design emphasizes efficient differential updates, robust error handling, and seamless integration with React Native applications.

### Key Design Principles
- **Modularity**: Shared core functionality with specialized CLI and server components
- **Performance**: Optimized differential algorithms and async operations
- **Security**: End-to-end encryption and integrity verification
- **Reliability**: Comprehensive error handling and rollback capabilities
- **Scalability**: Stateless server design with horizontal scaling support

## Architecture

### High-Level Architecture

```mermaid
graph TB
    RN[React Native App] --> SDK[RodePush SDK]
    DEV[Developer] --> CLI[rodepush-cli]
    CLI --> CORE[rodepush-core]
    CLI --> SERVER[rodepush-server]
    SDK --> SERVER
    SERVER --> CORE
    CORE --> STORAGE[(Bundle Storage)]
    SERVER --> CACHE[(Cache Layer)]
    SERVER --> DB[(Metadata DB)]
    
    subgraph "RodePush System"
        CLI
        SERVER
        CORE
        SDK
    end
```

### Component Architecture

#### 1. rodepush-core
Central library providing shared functionality for bundle operations, compression, and utilities.

**Key Modules:**
- `bundle::Bundle` - Bundle representation and manipulation
- `diff::DiffEngine` - Differential package generation
- `compression::Compressor` - Bundle compression/decompression
- `crypto::Hasher` - Cryptographic operations
- `storage::Storage` - Abstracted storage interface
- `serialization::Metadata` - Bundle metadata handling

#### 2. rodepush-cli
Command-line application for developers to build, package, and upload bundles.

**Key Modules:**
- `commands::build` - React Native bundle building
- `commands::upload` - Server upload functionality
- `config::Config` - Configuration management
- `auth::AuthClient` - Authentication with server
- `progress::ProgressReporter` - Upload progress tracking

#### 3. rodepush-server
HTTP server for bundle management, differential generation, and distribution.

**Key Modules:**
- `api::routes` - REST API endpoints
- `handlers::upload` - Bundle upload handling
- `handlers::download` - Differential package serving
- `services::diff` - Differential package generation
- `auth::middleware` - Authentication middleware
- `storage::manager` - Storage abstraction layer

## Components and Interfaces

### Core Library (rodepush-core)

```rust
// Bundle representation
pub struct Bundle {
    pub id: BundleId,
    pub version: SemanticVersion,
    pub platform: Platform,
    pub metadata: BundleMetadata,
    pub chunks: Vec<BundleChunk>,
}

// Differential engine
pub trait DiffEngine {
    fn generate_diff(&self, source: &Bundle, target: &Bundle) -> Result<DiffPackage>;
    fn apply_diff(&self, base: &Bundle, diff: &DiffPackage) -> Result<Bundle>;
}

// Storage abstraction
pub trait Storage {
    async fn store_bundle(&self, bundle: &Bundle) -> Result<StorageKey>;
    async fn retrieve_bundle(&self, key: &StorageKey) -> Result<Bundle>;
    async fn store_diff(&self, diff: &DiffPackage) -> Result<StorageKey>;
    async fn retrieve_diff(&self, key: &StorageKey) -> Result<DiffPackage>;
}
```

### CLI Interface

```rust
// CLI command structure
#[derive(Parser)]
pub enum Command {
    Build {
        #[arg(long)]
        platform: Platform,
        #[arg(long)]
        entry_file: PathBuf,
        #[arg(long)]
        output_dir: PathBuf,
    },
    Upload {
        #[arg(long)]
        bundle_path: PathBuf,
        #[arg(long)]
        server_url: Url,
        #[arg(long)]
        api_key: String,
    },
    Deploy {
        #[arg(long)]
        app_id: String,
        #[arg(long)]
        environment: Environment,
    },
}
```

### Server API Interface

```rust
// REST API endpoints
POST   /api/v1/bundles                    // Upload bundle
GET    /api/v1/bundles/{id}               // Get bundle metadata
GET    /api/v1/bundles/{id}/download      // Download full bundle
GET    /api/v1/diffs/{from}/{to}          // Get differential package
POST   /api/v1/deployments                // Create deployment
GET    /api/v1/deployments/{id}           // Get deployment status
DELETE /api/v1/deployments/{id}           // Rollback deployment
GET    /api/v1/health                     // Health check
GET    /api/v1/metrics                    // Metrics endpoint
```

### Bundle Format

```rust
// Bundle metadata structure
#[derive(Serialize, Deserialize)]
pub struct BundleMetadata {
    pub id: BundleId,
    pub version: SemanticVersion,
    pub platform: Platform,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub checksum: String,
    pub dependencies: Vec<Dependency>,
    pub chunks: Vec<ChunkMetadata>,
}

// Chunk metadata for bundle splitting
#[derive(Serialize, Deserialize)]
pub struct ChunkMetadata {
    pub id: String,
    pub offset: u64,
    pub size: u64,
    pub checksum: String,
    pub compression: CompressionType,
}
```

## Data Models

### Database Schema (PostgreSQL)

```sql
-- Applications
CREATE TABLE applications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    api_key VARCHAR(255) NOT NULL UNIQUE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Bundles
CREATE TABLE bundles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    application_id UUID REFERENCES applications(id),
    version VARCHAR(50) NOT NULL,
    platform VARCHAR(20) NOT NULL,
    metadata JSONB NOT NULL,
    storage_key VARCHAR(500) NOT NULL,
    size_bytes BIGINT NOT NULL,
    checksum VARCHAR(128) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Deployments
CREATE TABLE deployments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    application_id UUID REFERENCES applications(id),
    bundle_id UUID REFERENCES bundles(id),
    environment VARCHAR(50) NOT NULL,
    status VARCHAR(20) NOT NULL,
    rollout_percentage INTEGER DEFAULT 100,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deployed_at TIMESTAMP WITH TIME ZONE,
    rolled_back_at TIMESTAMP WITH TIME ZONE
);

-- Differential packages cache
CREATE TABLE diff_packages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    source_bundle_id UUID REFERENCES bundles(id),
    target_bundle_id UUID REFERENCES bundles(id),
    storage_key VARCHAR(500) NOT NULL,
    size_bytes BIGINT NOT NULL,
    compression_ratio FLOAT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);
```

### Configuration Models

```toml
# CLI configuration (rodepush.toml)
[server]
url = "https://api.rodepush.com"
timeout_seconds = 30

[build]
platform = "both"  # ios, android, both
entry_file = "index.js"
output_dir = "./build"

[auth]
api_key_file = "~/.rodepush/api_key"

# Server configuration
[server]
host = "0.0.0.0"
port = 8080
workers = 4

[database]
url = "postgresql://user:pass@localhost/rodepush"
max_connections = 10

[storage]
type = "filesystem"  # filesystem, s3, gcs
base_path = "/var/lib/rodepush"

[cache]
type = "redis"
url = "redis://localhost:6379"
ttl_seconds = 3600
```

## Error Handling

### Error Types Hierarchy

```rust
#[derive(Debug, thiserror::Error)]
pub enum RodePushError {
    #[error("Bundle error: {0}")]
    Bundle(#[from] BundleError),
    
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),
    
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),
    
    #[error("Validation error: {0}")]
    Validation(String),
}

#[derive(Debug, thiserror::Error)]
pub enum BundleError {
    #[error("Invalid bundle format")]
    InvalidFormat,
    
    #[error("Checksum mismatch")]
    ChecksumMismatch,
    
    #[error("Compression failed: {0}")]
    CompressionFailed(String),
    
    #[error("Bundle too large: {size} bytes (max: {max})")]
    TooLarge { size: u64, max: u64 },
}
```

### Error Recovery Strategies

1. **Network Failures**: Exponential backoff with jitter for retries
2. **Storage Failures**: Fallback to alternative storage backends
3. **Bundle Corruption**: Automatic re-download from source
4. **Authentication Failures**: Token refresh and re-authentication
5. **Deployment Failures**: Automatic rollback with notification

## Testing Strategy

### Unit Testing
- **Core Library**: Test differential algorithms, compression, and crypto functions
- **CLI**: Test command parsing, configuration loading, and API client
- **Server**: Test handlers, middleware, and business logic

### Integration Testing
- **Database Integration**: Test with real PostgreSQL instance
- **Storage Integration**: Test with filesystem and cloud storage
- **API Integration**: Test complete request/response cycles

### End-to-End Testing
- **CLI to Server**: Complete upload and deployment workflow
- **Client Updates**: Simulated React Native app update scenarios
- **Rollback Scenarios**: Test deployment rollback and recovery

### Performance Testing
- **Bundle Differential**: Test diff generation speed and size optimization
- **Concurrent Uploads**: Test server under load with multiple CLI clients
- **Download Performance**: Test client download speeds and bandwidth usage

### Test Infrastructure

```rust
// Test utilities
pub mod test_utils {
    pub fn create_test_bundle(size: usize) -> Bundle { /* ... */ }
    pub fn setup_test_server() -> TestServer { /* ... */ }
    pub fn create_mock_storage() -> MockStorage { /* ... */ }
}

// Example integration test
#[tokio::test]
async fn test_end_to_end_deployment() {
    let server = setup_test_server().await;
    let cli = RodePushCli::new(server.url());
    
    // Build and upload bundle
    let bundle = cli.build_bundle("./test_app").await.unwrap();
    let upload_result = cli.upload_bundle(bundle).await.unwrap();
    
    // Deploy bundle
    let deployment = cli.deploy(upload_result.bundle_id, "production").await.unwrap();
    assert_eq!(deployment.status, DeploymentStatus::Success);
    
    // Verify differential download
    let diff = server.get_diff(previous_bundle_id, upload_result.bundle_id).await.unwrap();
    assert!(diff.size_bytes < bundle.size_bytes);
}
```

## Technology Stack

### Core Technologies
- **Language**: Rust (2021 edition)
- **Async Runtime**: Tokio
- **HTTP Framework**: Axum (for server)
- **CLI Framework**: Clap v4
- **Serialization**: Serde with JSON/CBOR
- **Database**: PostgreSQL with SQLx
- **Caching**: Redis
- **Compression**: Zstandard (zstd)

### Key Dependencies

```toml
# Core dependencies
[dependencies]
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
uuid = { version = "1.0", features = ["v4", "serde"] }
thiserror = "1.0"
anyhow = "1.0"

# Server dependencies
axum = "0.7"
tower = "0.4"
sqlx = { version = "0.7", features = ["postgres", "uuid", "chrono"] }
redis = { version = "0.24", features = ["tokio-comp"] }

# CLI dependencies
clap = { version = "4.0", features = ["derive"] }
indicatif = "0.17"

# Compression and crypto
zstd = "0.13"
sha2 = "0.10"
blake3 = "1.5"

# HTTP client
reqwest = { version = "0.11", features = ["json", "stream"] }
```

## Security Considerations

### Data Protection
- **TLS/HTTPS**: All network communication encrypted
- **Bundle Signing**: Optional cryptographic signing for bundle verification
- **API Key Management**: Secure token-based authentication
- **Input Validation**: Comprehensive validation and sanitization

### Access Control
- **Application Isolation**: Strict separation between different applications
- **Environment Isolation**: Separate environments (dev, staging, prod)
- **Role-Based Access**: Different permission levels for different operations

### Bundle Integrity
- **Cryptographic Hashing**: SHA-256 for bundle integrity
- **Checksum Verification**: Multiple levels of checksum validation
- **Tamper Detection**: Detect and reject modified bundles

Does the design look good? If so, we can move on to the implementation plan.