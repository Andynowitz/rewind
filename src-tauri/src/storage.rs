
use crate::database::Database;
use crate::errors::{AppError, AppResult};
use crate::snapshot::SnapshotStore;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct AppStorage {
    pub root: PathBuf,
    pub database: Arc<Database>,
    pub snapshots: Arc<SnapshotStore>,
}

impl AppStorage {
    pub fn initialize(
        app_data_dir: impl AsRef<Path>,
    ) -> AppResult<Self> {
        let root = app_data_dir.as_ref().join("Rewind");

        fs::create_dir_all(&root)?;

        let root = fs::canonicalize(&root)?;

        let snapshots_dir = root.join("snapshots");
        let logs_dir = root.join("logs");

        fs::create_dir_all(&snapshots_dir)?;
        fs::create_dir_all(&logs_dir)?;

        let database_path = root.join("database.sqlite");

        let database = Database::open(database_path)
            .map_err(|error| AppError::Message(error.to_string()))?;

        let snapshots = SnapshotStore::new(&snapshots_dir)
            .map_err(|error| AppError::Message(error.to_string()))?;

        snapshots
            .remove_stale_staging_files()
            .map_err(|error| AppError::Message(error.to_string()))?;

        Ok(Self {
            root,
            database: Arc::new(database),
            snapshots: Arc::new(snapshots),
        })
    }

    pub fn database(&self) -> Arc<Database> {
        Arc::clone(&self.database)
    }

    pub fn snapshots(&self) -> Arc<SnapshotStore> {
        Arc::clone(&self.snapshots)
    }

    pub fn storage_bytes(&self) -> AppResult<u64> {
        self.snapshots
            .total_blob_storage_bytes()
            .map_err(|error| AppError::Message(error.to_string()))
    }
}
