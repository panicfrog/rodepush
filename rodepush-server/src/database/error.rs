//! Database error types and conversions

use rodepush_core::RodePushError;

/// Database error type
#[derive(Debug, thiserror::Error)]
pub enum DatabaseError {
    #[error("Connection error: {message}")]
    Connection { message: String },

    #[error("Query error: {message}")]
    Query { message: String },

    #[error("Transaction error: {message}")]
    Transaction { message: String },

    #[error("Migration error: {message}")]
    Migration { message: String },

    #[error("Serialization error: {message}")]
    Serialization { message: String },

    #[error("Deserialization error: {message}")]
    Deserialization { message: String },

    #[error("Record not found: {entity} with id {id}")]
    NotFound { entity: String, id: String },

    #[error("Duplicate record: {entity} with key {key}")]
    Duplicate { entity: String, key: String },

    #[error("Constraint violation: {message}")]
    ConstraintViolation { message: String },
}

impl From<DatabaseError> for RodePushError {
    fn from(error: DatabaseError) -> Self {
        RodePushError::Internal {
            message: error.to_string(),
        }
    }
}
