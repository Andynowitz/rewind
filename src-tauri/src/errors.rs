
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    Message(String),

    #[error("Database error: {0}")]
    Database(#[from] crate::database::DatabaseError),

    #[error("Snapshot error: {0}")]
    Snapshot(#[from] crate::snapshot::SnapshotError),

    #[error("Restore error: {0}")]
    Restore(#[from] crate::restore::RestoreError),

    #[error("Filesystem error: {0}")]
    Io(#[from] std::io::Error),
}

impl Serialize for AppError {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
