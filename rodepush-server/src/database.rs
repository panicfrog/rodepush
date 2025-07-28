//! Database models and operations for RodePush Server
//!
//! This module provides database models, connection management, and operations
//! for storing and retrieving bundle metadata, deployments, and differential packages.
//! Supports PostgreSQL as primary database with extensible architecture for MySQL support.

// Declare submodules
pub mod application;
pub mod bundle;
pub mod config;
pub mod connection;
pub mod deployment;
pub mod diff_package;
pub mod error;
pub mod manager;

// Re-export commonly used types for convenience
pub use application::{Application, ApplicationId, ApplicationService};
pub use bundle::{Bundle, BundleService, DatabaseBundleId};
pub use config::{DatabaseConfig, DatabaseType};
pub use connection::{DatabaseConnection, DatabasePool};
pub use deployment::{Deployment, DeploymentId, DeploymentService, DeploymentStatus};
pub use diff_package::{DiffPackage, DiffPackageId, DiffPackageService};
pub use error::DatabaseError;
pub use manager::DatabaseManager;
