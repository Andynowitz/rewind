
use crate::database::Database;
use crate::errors::{AppError, AppResult};
use crate::models::ScanResultDto;
use crate::snapshot::SnapshotStore;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct Scanner {
    database: Arc<Database>,
    snapshots: Arc<SnapshotStore>,
}

impl Scanner {
    pub fn new(
        database: Arc<Database>,
        snapshots: Arc<SnapshotStore>,
    ) -> Self {
        Self {
            database,
            snapshots,
        }
    }

    pub fn scan_folder(
        &self,
        folder_id: &str,
    ) -> AppResult<ScanResultDto> {
        let folder = self.database.get_folder(folder_id)?;

        if !folder.enabled {
            return Err(AppError::Message(
                "Protected folder is disabled".into()
            ));
        }

        let root = PathBuf::from(&folder.root_path);

        let canonical_root = fs::canonicalize(&root)?;

        if !canonical_root.is_dir() {
            return Err(AppError::Message(
                "Protected folder is not a directory".into()
            ));
        }

        let mut result = ScanResultDto {
            folder_id: folder_id.to_string(),
            files_scanned: 0,
            snapshots_created: 0,
            unchanged_files: 0,
            errors: Vec::new(),
        };

        self.scan_directory(
            folder_id,
            &canonical_root,
            &canonical_root,
            &mut result,
        )?;

        Ok(result)
    }

    fn scan_directory(
        &self,
        folder_id: &str,
        root: &Path,
        directory: &Path,
        result: &mut ScanResultDto,
    ) -> AppResult<()> {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) => {
                result.errors.push(format!(
                    "{}: {}",
                    directory.display(),
                    error
                ));

                return Ok(());
            }
        };

        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    result.errors.push(error.to_string());
                    continue;
                }
            };

            let path = entry.path();

            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) => {
                    result.errors.push(format!(
                        "{}: {}",
                        path.display(),
                        error
                    ));

                    continue;
                }
            };

            if metadata.file_type().is_symlink() {
                continue;
            }

            if metadata.is_dir() {
                self.scan_directory(
                    folder_id,
                    root,
                    &path,
                    result,
                )?;

                continue;
            }

            if !metadata.is_file() {
                continue;
            }

            if Self::is_internal_temporary_file(&path) {
                continue;
            }

            result.files_scanned += 1;

            let relative_path = match path.strip_prefix(root) {
                Ok(relative) => relative,
                Err(error) => {
                    result.errors.push(error.to_string());
                    continue;
                }
            };

            match self.scan_file(
                folder_id,
                relative_path,
                &path,
            ) {
                Ok(created) => {
                    if created {
                        result.snapshots_created += 1;
                    } else {
                        result.unchanged_files += 1;
                    }
                }
                Err(error) => {
                    result.errors.push(format!(
                        "{}: {}",
                        path.display(),
                        error
                    ));
                }
            }
        }

        Ok(())
    }

    fn scan_file(
        &self,
        folder_id: &str,
        relative_path: &Path,
        absolute_path: &Path,
    ) -> AppResult<bool> {
        let relative = relative_path.to_str().ok_or_else(|| {
            AppError::Message(
                "File path is not valid UTF-8".into()
            )
        })?;

        let metadata_before = fs::symlink_metadata(absolute_path)?;

        if metadata_before.file_type().is_symlink()
            || !metadata_before.is_file()
        {
            return Err(AppError::Message(
                "File is not a regular file".into()
            ));
        }

        let tracked = self.database.get_or_create_tracked_file(
            folder_id,
            relative,
        )?;

        let blob = self.snapshots.put_file(absolute_path)?;

        let metadata_after = fs::symlink_metadata(absolute_path)?;

        if metadata_after.file_type().is_symlink()
            || !metadata_after.is_file()
            || metadata_before.len() != metadata_after.len()
            || metadata_before.modified().ok()
                != metadata_after.modified().ok()
        {
            return Err(AppError::Message(
                "File changed during snapshot creation; retry required".into()
            ));
        }

        if tracked.last_hash.as_deref() == Some(&blob.hash) {
            self.database.set_file_state(
                &tracked.id,
                Some(&blob.hash),
                "present",
            )?;

            return Ok(false);
        }

        self.database.register_blob(
            &blob.hash,
            blob.size_bytes as i64,
        )?;

        self.database.register_version(
            &tracked.id,
            &blob.hash,
            blob.size_bytes as i64,
            "automatic",
        )?;

        self.database.set_file_state(
            &tracked.id,
            Some(&blob.hash),
            "present",
        )?;

        Ok(true)
    }

    fn is_internal_temporary_file(path: &Path) -> bool {
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            return false;
        };

        name.starts_with(".rewind-restore-")
            && name.ends_with(".tmp")
    }

    pub fn scan_changed_path(
        &self,
        folder_id: &str,
        changed_path: &Path,
    ) -> AppResult<bool> {
        let folder = self.database.get_folder(folder_id)?;

        if !folder.enabled {
            return Ok(false);
        }

        let root = fs::canonicalize(&folder.root_path)?;

        let relative = changed_path.strip_prefix(&root)
            .map_err(|_| AppError::Message(
                "Changed path is outside the protected folder".into()
            ))?;

        let metadata = match fs::symlink_metadata(changed_path) {
            Ok(metadata) => metadata,
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound =>
            {
                return Ok(false);
            }
            Err(error) => return Err(AppError::Io(error)),
        };

        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Ok(false);
        }

        self.scan_file(folder_id, relative, changed_path)
    }

    
    pub fn scan_folder_from_root(
        &self,
        root: &Path,
    ) -> AppResult<ScanResultDto> {
        let canonical_root = std::fs::canonicalize(root)?;

        let folders = self.database.list_protected_folders()?;

        let folder = folders
            .into_iter()
            .find(|folder| {
                std::fs::canonicalize(&folder.root_path)
                    .map(|path| path == canonical_root)
                    .unwrap_or(false)
            })
            .ok_or_else(|| AppError::Message(
                "No registered protected folder matches this watcher".into()
            ))?;

        self.scan_folder(&folder.id)
    }

    pub fn scan_changed_path_for_root(
        &self,
        root: &Path,
        path: &Path,
    ) -> AppResult<bool> {
        let canonical_root = std::fs::canonicalize(root)?;

        let folders = self.database.list_protected_folders()?;

        let folder = folders
            .into_iter()
            .find(|folder| {
                std::fs::canonicalize(&folder.root_path)
                    .map(|candidate| candidate == canonical_root)
                    .unwrap_or(false)
            })
            .ok_or_else(|| AppError::Message(
                "Protected folder registration not found".into()
            ))?;

        self.scan_changed_path(&folder.id, path)
    }

}

pub fn scan_folder(
    database: Arc<Database>,
    snapshots: Arc<SnapshotStore>,
    folder_id: &str,
) -> AppResult<ScanResultDto> {
    Scanner::new(database, snapshots).scan_folder(folder_id)
}
