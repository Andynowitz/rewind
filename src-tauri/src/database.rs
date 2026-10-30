
use chrono::Utc;
use parking_lot::Mutex;
use rusqlite::{
    params,
    Connection,
    OptionalExtension,
    Transaction,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Filesystem error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Record not found: {0}")]
    NotFound(String),

    #[error("Invalid database state: {0}")]
    InvalidState(String),

    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

pub type DatabaseResult<T> = Result<T, DatabaseError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedFolder {
    pub id: String,
    pub root_path: String,
    pub enabled: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedFile {
    pub id: String,
    pub folder_id: String,
    pub relative_path: String,
    pub last_hash: Option<String>,
    pub last_seen_at: Option<String>,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobRecord {
    pub hash: String,
    pub size_bytes: i64,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionRecord {
    pub id: String,
    pub file_id: String,
    pub blob_hash: String,
    pub created_at: String,
    pub size_bytes: i64,
    pub reason: String,
    pub root_path: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreOperation {
    pub id: String,
    pub version_id: String,
    pub file_id: String,
    pub state: String,
    pub previous_hash: Option<String>,
    pub target_hash: String,
    pub temporary_path: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub struct Database {
    connection: Mutex<Connection>,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> DatabaseResult<Self> {
        let path = path.as_ref();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let connection = Connection::open(path)?;

        Self::configure(&connection)?;

        let database = Self {
            connection: Mutex::new(connection),
        };

        database.initialize()?;

        Ok(database)
    }

    pub fn open_in_memory() -> DatabaseResult<Self> {
        let connection = Connection::open_in_memory()?;

        Self::configure(&connection)?;

        let database = Self {
            connection: Mutex::new(connection),
        };

        database.initialize()?;

        Ok(database)
    }

    fn configure(connection: &Connection) -> DatabaseResult<()> {
        connection.busy_timeout(
            std::time::Duration::from_secs(10)
        )?;

        connection.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = FULL;
            PRAGMA temp_store = MEMORY;
            "
        )?;

        Ok(())
    }

    fn initialize(&self) -> DatabaseResult<()> {
        let connection = self.connection.lock();

        connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS protected_folders (
                id TEXT PRIMARY KEY,
                root_path TEXT NOT NULL UNIQUE,
                enabled INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tracked_files (
                id TEXT PRIMARY KEY,
                folder_id TEXT NOT NULL,
                relative_path TEXT NOT NULL,
                last_hash TEXT,
                last_seen_at TEXT,
                state TEXT NOT NULL DEFAULT 'present',

                FOREIGN KEY(folder_id)
                    REFERENCES protected_folders(id)
                    ON DELETE CASCADE,

                UNIQUE(folder_id, relative_path)
            );

            CREATE INDEX IF NOT EXISTS
                idx_tracked_files_folder
            ON tracked_files(folder_id);

            CREATE INDEX IF NOT EXISTS
                idx_tracked_files_state
            ON tracked_files(state);

            CREATE TABLE IF NOT EXISTS blobs (
                hash TEXT PRIMARY KEY,
                size_bytes INTEGER NOT NULL,
                verified INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS versions (
                id TEXT PRIMARY KEY,
                file_id TEXT NOT NULL,
                blob_hash TEXT NOT NULL,
                created_at TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                reason TEXT NOT NULL,

                FOREIGN KEY(file_id)
                    REFERENCES tracked_files(id)
                    ON DELETE CASCADE,

                FOREIGN KEY(blob_hash)
                    REFERENCES blobs(hash)
            );

            CREATE INDEX IF NOT EXISTS
                idx_versions_file_time
            ON versions(file_id, created_at DESC);

            CREATE INDEX IF NOT EXISTS
                idx_versions_blob
            ON versions(blob_hash);

            CREATE TABLE IF NOT EXISTS restore_operations (
                id TEXT PRIMARY KEY,
                version_id TEXT NOT NULL,
                file_id TEXT NOT NULL,
                state TEXT NOT NULL,
                previous_hash TEXT,
                target_hash TEXT NOT NULL,
                temporary_path TEXT,
                error TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,

                FOREIGN KEY(version_id)
                    REFERENCES versions(id),

                FOREIGN KEY(file_id)
                    REFERENCES tracked_files(id)
            );

            CREATE INDEX IF NOT EXISTS
                idx_restore_operations_state
            ON restore_operations(state);

            CREATE INDEX IF NOT EXISTS
                idx_restore_operations_created
            ON restore_operations(created_at DESC);
            "
        )?;

        Ok(())
    }

    pub fn add_protected_folder(
        &self,
        root_path: &Path,
    ) -> DatabaseResult<ProtectedFolder> {
        let canonical = std::fs::canonicalize(root_path)
            .map_err(DatabaseError::Io)?;

        if !canonical.is_dir() {
            return Err(DatabaseError::InvalidPath(
                "Protected path is not a directory".into()
            ));
        }

        let path = canonical.to_string_lossy().to_string();

        let folder = ProtectedFolder {
            id: Uuid::new_v4().to_string(),
            root_path: path,
            enabled: true,
            created_at: Utc::now().to_rfc3339(),
        };

        let connection = self.connection.lock();

        connection.execute(
            "
            INSERT INTO protected_folders
            (id, root_path, enabled, created_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(root_path) DO UPDATE SET
                enabled = 1
            ",
            params![
                folder.id,
                folder.root_path,
                1,
                folder.created_at
            ],
        )?;

        let existing = connection.query_row(
            "
            SELECT id, root_path, enabled, created_at
            FROM protected_folders
            WHERE root_path = ?1
            ",
            params![folder.root_path],
            |row| {
                Ok(ProtectedFolder {
                    id: row.get(0)?,
                    root_path: row.get(1)?,
                    enabled: row.get::<_, i64>(2)? != 0,
                    created_at: row.get(3)?,
                })
            },
        )?;

        Ok(existing)
    }

    pub fn list_protected_folders(
        &self,
    ) -> DatabaseResult<Vec<ProtectedFolder>> {
        let connection = self.connection.lock();

        let mut statement = connection.prepare(
            "
            SELECT id, root_path, enabled, created_at
            FROM protected_folders
            ORDER BY created_at DESC
            "
        )?;

        let rows = statement.query_map([], |row| {
            Ok(ProtectedFolder {
                id: row.get(0)?,
                root_path: row.get(1)?,
                enabled: row.get::<_, i64>(2)? != 0,
                created_at: row.get(3)?,
            })
        })?;

        let mut result = Vec::new();

        for row in rows {
            result.push(row?);
        }

        Ok(result)
    }

    pub fn remove_protected_folder(
        &self,
        folder_id: &str,
    ) -> DatabaseResult<()> {
        let connection = self.connection.lock();

        let changed = connection.execute(
            "DELETE FROM protected_folders WHERE id = ?1",
            params![folder_id],
        )?;

        if changed == 0 {
            return Err(DatabaseError::NotFound(
                format!("Folder {folder_id}")
            ));
        }

        Ok(())
    }

    pub fn get_folder(
        &self,
        folder_id: &str,
    ) -> DatabaseResult<ProtectedFolder> {
        let connection = self.connection.lock();

        connection.query_row(
            "
            SELECT id, root_path, enabled, created_at
            FROM protected_folders
            WHERE id = ?1
            ",
            params![folder_id],
            |row| {
                Ok(ProtectedFolder {
                    id: row.get(0)?,
                    root_path: row.get(1)?,
                    enabled: row.get::<_, i64>(2)? != 0,
                    created_at: row.get(3)?,
                })
            },
        ).optional()?
        .ok_or_else(|| DatabaseError::NotFound(
            format!("Folder {folder_id}")
        ))
    }

    pub fn get_or_create_tracked_file(
        &self,
        folder_id: &str,
        relative_path: &str,
    ) -> DatabaseResult<TrackedFile> {
        let connection = self.connection.lock();

        let existing = connection.query_row(
            "
            SELECT id, folder_id, relative_path,
                   last_hash, last_seen_at, state
            FROM tracked_files
            WHERE folder_id = ?1
              AND relative_path = ?2
            ",
            params![folder_id, relative_path],
            Self::map_tracked_file,
        ).optional()?;

        if let Some(file) = existing {
            return Ok(file);
        }

        let file = TrackedFile {
            id: Uuid::new_v4().to_string(),
            folder_id: folder_id.to_string(),
            relative_path: relative_path.to_string(),
            last_hash: None,
            last_seen_at: None,
            state: "present".into(),
        };

        connection.execute(
            "
            INSERT INTO tracked_files (
                id, folder_id, relative_path, state
            )
            VALUES (?1, ?2, ?3, 'present')
            ",
            params![
                file.id,
                file.folder_id,
                file.relative_path
            ],
        )?;

        Ok(file)
    }

    fn map_tracked_file(
        row: &rusqlite::Row<'_>,
    ) -> rusqlite::Result<TrackedFile> {
        Ok(TrackedFile {
            id: row.get(0)?,
            folder_id: row.get(1)?,
            relative_path: row.get(2)?,
            last_hash: row.get(3)?,
            last_seen_at: row.get(4)?,
            state: row.get(5)?,
        })
    }
    pub fn list_tracked_files(&self) -> DatabaseResult<Vec<TrackedFile>> {
        let connection = self.connection.lock();

        let mut statement = connection.prepare(
            "
            SELECT id, folder_id, relative_path,
                last_hash, last_seen_at, state
            FROM tracked_files
            ORDER BY relative_path COLLATE NOCASE ASC
            "
        )?;

        let rows = statement.query_map([], Self::map_tracked_file)?;

        let mut result = Vec::new();

        for row in rows {
            result.push(row?);
        }

        Ok(result)
    }

    pub fn get_tracked_file(
        &self,
        file_id: &str,
    ) -> DatabaseResult<TrackedFile> {
        let connection = self.connection.lock();

        connection.query_row(
            "
            SELECT id, folder_id, relative_path,
                   last_hash, last_seen_at, state
            FROM tracked_files
            WHERE id = ?1
            ",
            params![file_id],
            Self::map_tracked_file,
        ).optional()?
        .ok_or_else(|| DatabaseError::NotFound(
            format!("File {file_id}")
        ))
    }

    pub fn set_file_state(
        &self,
        file_id: &str,
        hash: Option<&str>,
        state: &str,
    ) -> DatabaseResult<()> {
        let connection = self.connection.lock();

        let changed = connection.execute(
            "
            UPDATE tracked_files
            SET last_hash = ?1,
                state = ?2,
                last_seen_at = ?3
            WHERE id = ?4
            ",
            params![
                hash,
                state,
                Utc::now().to_rfc3339(),
                file_id
            ],
        )?;

        if changed == 0 {
            return Err(DatabaseError::NotFound(
                format!("File {file_id}")
            ));
        }

        Ok(())
    }

    pub fn register_blob(
        &self,
        hash: &str,
        size_bytes: i64,
    ) -> DatabaseResult<()> {
        let connection = self.connection.lock();

        connection.execute(
            "
            INSERT INTO blobs (
                hash, size_bytes, verified, created_at
            )
            VALUES (?1, ?2, 1, ?3)
            ON CONFLICT(hash) DO UPDATE SET
                verified = 1
            ",
            params![
                hash,
                size_bytes,
                Utc::now().to_rfc3339()
            ],
        )?;

        Ok(())
    }

    pub fn get_blob(
        &self,
        hash: &str,
    ) -> DatabaseResult<BlobRecord> {
        let connection = self.connection.lock();

        connection.query_row(
            "
            SELECT hash, size_bytes, verified
            FROM blobs
            WHERE hash = ?1
            ",
            params![hash],
            |row| {
                Ok(BlobRecord {
                    hash: row.get(0)?,
                    size_bytes: row.get(1)?,
                    verified: row.get::<_, i64>(2)? != 0,
                })
            },
        ).optional()?
        .ok_or_else(|| DatabaseError::NotFound(
            format!("Blob {hash}")
        ))
    }

    pub fn register_version(
        &self,
        file_id: &str,
        blob_hash: &str,
        size_bytes: i64,
        reason: &str,
    ) -> DatabaseResult<VersionRecord> {
        let mut connection = self.connection.lock();
        let transaction = connection.transaction()?;

        let blob_exists: bool = transaction.query_row(
            "SELECT EXISTS(
                SELECT 1 FROM blobs WHERE hash = ?1
            )",
            params![blob_hash],
            |row| row.get(0),
        )?;

        if !blob_exists {
            return Err(DatabaseError::NotFound(
                format!("Blob {blob_hash}")
            ));
        }

        let file = transaction.query_row(
            "
            SELECT f.id, f.folder_id, f.relative_path,
                   f.last_hash, f.last_seen_at, f.state,
                   p.root_path
            FROM tracked_files f
            JOIN protected_folders p
                ON p.id = f.folder_id
            WHERE f.id = ?1
            ",
            params![file_id],
            |row| {
                Ok((
                    Self::map_tracked_file_from_join(row)?,
                    row.get::<_, String>(6)?
                ))
            },
        ).optional()?
        .ok_or_else(|| DatabaseError::NotFound(
            format!("File {file_id}")
        ))?;

        let version = VersionRecord {
            id: Uuid::new_v4().to_string(),
            file_id: file_id.to_string(),
            blob_hash: blob_hash.to_string(),
            created_at: Utc::now().to_rfc3339(),
            size_bytes,
            reason: reason.to_string(),
            root_path: file.1,
            relative_path: file.0.relative_path,
        };

        transaction.execute(
            "
            INSERT INTO versions (
                id, file_id, blob_hash,
                created_at, size_bytes, reason
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            ",
            params![
                version.id,
                version.file_id,
                version.blob_hash,
                version.created_at,
                version.size_bytes,
                version.reason
            ],
        )?;

        transaction.commit()?;

        Ok(version)
    }

    fn map_tracked_file_from_join(
        row: &rusqlite::Row<'_>,
    ) -> rusqlite::Result<TrackedFile> {
        Self::map_tracked_file(row)
    }

    pub fn get_version(
        &self,
        version_id: &str,
    ) -> DatabaseResult<VersionRecord> {
        let connection = self.connection.lock();

        connection.query_row(
            "
            SELECT v.id, v.file_id, v.blob_hash,
                   v.created_at, v.size_bytes, v.reason,
                   p.root_path, f.relative_path
            FROM versions v
            JOIN tracked_files f ON f.id = v.file_id
            JOIN protected_folders p ON p.id = f.folder_id
            WHERE v.id = ?1
            ",
            params![version_id],
            |row| {
                Ok(VersionRecord {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    blob_hash: row.get(2)?,
                    created_at: row.get(3)?,
                    size_bytes: row.get(4)?,
                    reason: row.get(5)?,
                    root_path: row.get(6)?,
                    relative_path: row.get(7)?,
                })
            },
        ).optional()?
        .ok_or_else(|| DatabaseError::NotFound(
            format!("Version {version_id}")
        ))
    }

    pub fn list_versions(
        &self,
        file_id: &str,
    ) -> DatabaseResult<Vec<VersionRecord>> {
        let connection = self.connection.lock();

        let mut statement = connection.prepare(
            "
            SELECT v.id, v.file_id, v.blob_hash,
                   v.created_at, v.size_bytes, v.reason,
                   p.root_path, f.relative_path
            FROM versions v
            JOIN tracked_files f ON f.id = v.file_id
            JOIN protected_folders p ON p.id = f.folder_id
            WHERE v.file_id = ?1
            ORDER BY v.created_at DESC, v.rowid DESC
            "
        )?;

        let rows = statement.query_map(
            params![file_id],
            |row| {
                Ok(VersionRecord {
                    id: row.get(0)?,
                    file_id: row.get(1)?,
                    blob_hash: row.get(2)?,
                    created_at: row.get(3)?,
                    size_bytes: row.get(4)?,
                    reason: row.get(5)?,
                    root_path: row.get(6)?,
                    relative_path: row.get(7)?,
                })
            },
        )?;

        let mut result = Vec::new();

        for row in rows {
            result.push(row?);
        }

        Ok(result)
    }

    pub fn create_restore_operation(
        &self,
        version_id: &str,
        file_id: &str,
        previous_hash: Option<&str>,
        target_hash: &str,
        temporary_path: Option<&str>,
    ) -> DatabaseResult<RestoreOperation> {
        let connection = self.connection.lock();

        let operation = RestoreOperation {
            id: Uuid::new_v4().to_string(),
            version_id: version_id.to_string(),
            file_id: file_id.to_string(),
            state: "prepared".into(),
            previous_hash: previous_hash.map(str::to_string),
            target_hash: target_hash.to_string(),
            temporary_path: temporary_path.map(str::to_string),
            error: None,
            created_at: Utc::now().to_rfc3339(),
            updated_at: Utc::now().to_rfc3339(),
        };

        connection.execute(
            "
            INSERT INTO restore_operations (
                id, version_id, file_id, state,
                previous_hash, target_hash,
                temporary_path, error,
                created_at, updated_at
            )
            VALUES (
                ?1, ?2, ?3, ?4, ?5,
                ?6, ?7, ?8, ?9, ?10
            )
            ",
            params![
                operation.id,
                operation.version_id,
                operation.file_id,
                operation.state,
                operation.previous_hash,
                operation.target_hash,
                operation.temporary_path,
                operation.error,
                operation.created_at,
                operation.updated_at
            ],
        )?;

        Ok(operation)
    }

    pub fn update_restore_operation(
        &self,
        operation_id: &str,
        state: &str,
        error: Option<&str>,
    ) -> DatabaseResult<()> {
        let connection = self.connection.lock();

        let changed = connection.execute(
            "
            UPDATE restore_operations
            SET state = ?1,
                error = ?2,
                updated_at = ?3
            WHERE id = ?4
            ",
            params![
                state,
                error,
                Utc::now().to_rfc3339(),
                operation_id
            ],
        )?;

        if changed == 0 {
            return Err(DatabaseError::NotFound(
                format!("Restore operation {operation_id}")
            ));
        }

        Ok(())
    }

    pub fn get_restore_operation(
        &self,
        operation_id: &str,
    ) -> DatabaseResult<RestoreOperation> {
        let connection = self.connection.lock();

        connection.query_row(
            "
            SELECT id, version_id, file_id, state,
                   previous_hash, target_hash,
                   temporary_path, error,
                   created_at, updated_at
            FROM restore_operations
            WHERE id = ?1
            ",
            params![operation_id],
            |row| {
                Ok(RestoreOperation {
                    id: row.get(0)?,
                    version_id: row.get(1)?,
                    file_id: row.get(2)?,
                    state: row.get(3)?,
                    previous_hash: row.get(4)?,
                    target_hash: row.get(5)?,
                    temporary_path: row.get(6)?,
                    error: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            },
        ).optional()?
        .ok_or_else(|| DatabaseError::NotFound(
            format!("Restore operation {operation_id}")
        ))
    }

    pub fn list_pending_restore_operations(
        &self,
    ) -> DatabaseResult<Vec<RestoreOperation>> {
        let connection = self.connection.lock();

        let mut statement = connection.prepare(
            "
            SELECT id, version_id, file_id, state,
                   previous_hash, target_hash,
                   temporary_path, error,
                   created_at, updated_at
            FROM restore_operations
            WHERE state NOT IN ('verified', 'failed')
            ORDER BY created_at ASC
            "
        )?;

        let rows = statement.query_map([], |row| {
            Ok(RestoreOperation {
                id: row.get(0)?,
                version_id: row.get(1)?,
                file_id: row.get(2)?,
                state: row.get(3)?,
                previous_hash: row.get(4)?,
                target_hash: row.get(5)?,
                temporary_path: row.get(6)?,
                error: row.get(7)?,
                created_at: row.get(8)?,
                updated_at: row.get(9)?,
            })
        })?;

        let mut result = Vec::new();

        for row in rows {
            result.push(row?);
        }

        Ok(result)
    }
}
