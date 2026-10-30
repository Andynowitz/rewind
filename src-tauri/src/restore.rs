
use crate::database::{
    Database,
    DatabaseError,
    RestoreOperation,
    VersionRecord,
};
use crate::snapshot::{
    SnapshotError,
    SnapshotStore,
};

use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum RestoreError {
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Snapshot error: {0}")]
    Snapshot(#[from] SnapshotError),

    #[error("Filesystem error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid restore target: {0}")]
    InvalidTarget(String),

    #[error("Restore integrity verification failed")]
    IntegrityFailure,

    #[error("Restore operation failed: {0}")]
    OperationFailed(String),
}

pub type RestoreResult<T> = Result<T, RestoreError>;

#[derive(Debug, Clone)]
pub struct RestoreResultInfo {
    pub operation_id: String,
    pub restored_version_id: String,
    pub restored_hash: String,
    pub previous_hash: Option<String>,
    pub target_path: PathBuf,
    pub restored_size: u64,
}

pub fn validate_relative_path(
    relative_path: &Path,
) -> RestoreResult<()> {
    if relative_path.as_os_str().is_empty() {
        return Err(RestoreError::InvalidTarget(
            "Target path cannot be empty".into()
        ));
    }

    if relative_path.is_absolute() {
        return Err(RestoreError::InvalidTarget(
            "Absolute paths are not permitted".into()
        ));
    }

    for component in relative_path.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {
                return Err(RestoreError::InvalidTarget(
                    "Current-directory components are not permitted".into()
                ));
            }
            Component::ParentDir => {
                return Err(RestoreError::InvalidTarget(
                    "Parent-directory traversal is not permitted".into()
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(RestoreError::InvalidTarget(
                    "Root and prefix components are not permitted".into()
                ));
            }
        }
    }

    Ok(())
}

fn resolve_safe_target(
    root: &Path,
    relative_path: &Path,
) -> RestoreResult<PathBuf> {
    validate_relative_path(relative_path)?;

    let canonical_root = fs::canonicalize(root)?;

    if !canonical_root.is_dir() {
        return Err(RestoreError::InvalidTarget(
            "Protected root is not a directory".into()
        ));
    }

    let target = canonical_root.join(relative_path);

    let mut current = canonical_root.clone();

    for component in relative_path.components() {
        let Component::Normal(part) = component else {
            return Err(RestoreError::InvalidTarget(
                "Unexpected path component".into()
            ));
        };

        current.push(part);

        match fs::symlink_metadata(&current) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(RestoreError::InvalidTarget(
                        "Symbolic links are not permitted in restore paths".into()
                    ));
                }

                let is_final = current == target;

                if !is_final && !metadata.is_dir() {
                    return Err(RestoreError::InvalidTarget(
                        "A parent path component is not a directory".into()
                    ));
                }
            }
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                break;
            }
            Err(error) => return Err(
                RestoreError::Io(error)
            ),
        }
    }

    let parent = target.parent().ok_or_else(|| {
        RestoreError::InvalidTarget(
            "Target has no parent directory".into()
        )
    })?;

    let canonical_parent = fs::canonicalize(parent)?;

    if !canonical_parent.starts_with(&canonical_root) {
        return Err(RestoreError::InvalidTarget(
            "Target parent escapes the protected root".into()
        ));
    }

    if let Ok(metadata) = fs::symlink_metadata(&target) {
        if metadata.file_type().is_symlink() {
            return Err(RestoreError::InvalidTarget(
                "Cannot replace a symbolic link".into()
            ));
        }

        if !metadata.is_file() {
            return Err(RestoreError::InvalidTarget(
                "Restore target must be a regular file".into()
            ));
        }

        let canonical_target = fs::canonicalize(&target)?;

        if !canonical_target.starts_with(&canonical_root) {
            return Err(RestoreError::InvalidTarget(
                "Target escapes the protected root".into()
            ));
        }
    }

    Ok(target)
}

fn hash_bytes(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    hex::encode(digest)
}

fn write_temporary_file(
    parent: &Path,
    bytes: &[u8],
) -> RestoreResult<PathBuf> {
    let temporary_path = parent.join(
        format!(".rewind-restore-{}.tmp", Uuid::new_v4())
    );

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)?;

    file.write_all(bytes)?;
    file.flush()?;
    file.sync_all()?;

    Ok(temporary_path)
}

fn replace_file(
    temporary_path: &Path,
    target_path: &Path,
) -> RestoreResult<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;

        use windows_sys::Win32::Storage::FileSystem::{
            MoveFileExW,
            MOVEFILE_REPLACE_EXISTING,
            MOVEFILE_WRITE_THROUGH,
        };

        let source: Vec<u16> = temporary_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let destination: Vec<u16> = target_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let flags =
            MOVEFILE_REPLACE_EXISTING |
            MOVEFILE_WRITE_THROUGH;

        let result = unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                flags,
            )
        };

        if result == 0 {
            return Err(RestoreError::Io(
                std::io::Error::last_os_error()
            ));
        }
    }

    #[cfg(not(windows))]
    {
        fs::rename(temporary_path, target_path)?;
    }

    Ok(())
}

fn preserve_current_version(
    database: &Database,
    store: &SnapshotStore,
    version: &VersionRecord,
    target_path: &Path,
) -> RestoreResult<Option<String>> {
    let metadata = match fs::symlink_metadata(target_path) {
        Ok(metadata) => metadata,
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound =>
        {
            database.set_file_state(
                &version.file_id,
                None,
                "missing",
            )?;

            return Ok(None);
        }
        Err(error) => return Err(
            RestoreError::Io(error)
        ),
    };

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(RestoreError::InvalidTarget(
            "Current target is not a regular file".into()
        ));
    }

    let blob = store.put_file(target_path)?;

    database.register_blob(
        &blob.hash,
        blob.size_bytes as i64,
    )?;

    if blob.hash == version.blob_hash {
        return Ok(Some(blob.hash));
    }

    database.register_version(
        &version.file_id,
        &blob.hash,
        blob.size_bytes as i64,
        "pre_restore",
    )?;

    Ok(Some(blob.hash))
}

pub fn restore_version(
    database: &Database,
    store: &SnapshotStore,
    version_id: &str,
) -> RestoreResult<RestoreResultInfo> {
    let version = database.get_version(version_id)?;

    let blob = database.get_blob(&version.blob_hash)?;

    if !blob.verified {
        return Err(RestoreError::IntegrityFailure);
    }

    let bytes = store.read_blob(&version.blob_hash)?;

    if bytes.len() as u64 != blob.size_bytes as u64 {
        return Err(RestoreError::IntegrityFailure);
    }

    let root = PathBuf::from(&version.root_path);

    let target_path = resolve_safe_target(
        &root,
        Path::new(&version.relative_path),
    )?;

    let parent = target_path.parent().ok_or_else(|| {
        RestoreError::InvalidTarget(
            "Target parent does not exist".into()
        )
    })?;

    let current_hash = preserve_current_version(
        database,
        store,
        &version,
        &target_path,
    )?;

    let temporary_path = write_temporary_file(
        parent,
        &bytes,
    )?;

    let operation = match database.create_restore_operation(
        &version.id,
        &version.file_id,
        current_hash.as_deref(),
        &version.blob_hash,
        Some(&temporary_path.to_string_lossy()),
    ) {
        Ok(operation) => operation,
        Err(error) => {
            let _ = fs::remove_file(&temporary_path);
            return Err(RestoreError::Database(error));
        }
    };

    let result = execute_restore(
        database,
        &operation,
        &version,
        &temporary_path,
        &target_path,
        &bytes,
    );

    match result {
        Ok(()) => {
            Ok(RestoreResultInfo {
                operation_id: operation.id,
                restored_version_id: version.id,
                restored_hash: version.blob_hash,
                previous_hash: current_hash,
                target_path,
                restored_size: bytes.len() as u64,
            })
        }
        Err(error) => {
            let message = error.to_string();

            let _ = database.update_restore_operation(
                &operation.id,
                "failed",
                Some(&message),
            );

            let _ = fs::remove_file(&temporary_path);

            Err(error)
        }
    }
}

fn execute_restore(
    database: &Database,
    operation: &RestoreOperation,
    version: &VersionRecord,
    temporary_path: &Path,
    target_path: &Path,
    expected_bytes: &[u8],
) -> RestoreResult<()> {
    database.update_restore_operation(
        &operation.id,
        "replacement_ready",
        None,
    )?;

    replace_file(
        temporary_path,
        target_path,
    )?;

    database.update_restore_operation(
        &operation.id,
        "applied",
        None,
    )?;

    let restored_hash = SnapshotStore::hash_file(
        target_path,
        512 * 1024 * 1024,
    )?;

    if restored_hash != version.blob_hash {
        return Err(RestoreError::IntegrityFailure);
    }

    if hash_bytes(expected_bytes) != version.blob_hash {
        return Err(RestoreError::IntegrityFailure);
    }

    database.set_file_state(
        &version.file_id,
        Some(&restored_hash),
        "present",
    )?;

    database.update_restore_operation(
        &operation.id,
        "verified",
        None,
    )?;

    Ok(())
}
