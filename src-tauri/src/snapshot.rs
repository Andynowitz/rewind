
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("Filesystem error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid SHA-256 hash")]
    InvalidHash,

    #[error("Snapshot integrity verification failed")]
    IntegrityFailure,

    #[error("File exceeds maximum snapshot size: {0} bytes")]
    FileTooLarge(u64),

    #[error("Snapshot staging operation failed: {0}")]
    StagingFailure(String),
}

pub type SnapshotResult<T> = Result<T, SnapshotError>;

#[derive(Debug, Clone)]
pub struct SnapshotBlob {
    pub hash: String,
    pub size_bytes: u64,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct SnapshotStore {
    root: PathBuf,
    staging: PathBuf,
    max_file_size: u64,
}

impl SnapshotStore {
    pub fn new(root: impl AsRef<Path>) -> SnapshotResult<Self> {
        Self::with_max_file_size(
            root,
            512 * 1024 * 1024,
        )
    }

    pub fn with_max_file_size(
        root: impl AsRef<Path>,
        max_file_size: u64,
    ) -> SnapshotResult<Self> {
        let root = root.as_ref().to_path_buf();

        fs::create_dir_all(&root)?;

        let root = fs::canonicalize(&root)?;

        let staging = root.join("staging");

        fs::create_dir_all(&staging)?;

        Ok(Self {
            root,
            staging,
            max_file_size,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn staging(&self) -> &Path {
        &self.staging
    }

    pub fn hash_bytes(bytes: &[u8]) -> String {
        let digest = Sha256::digest(bytes);

        hex::encode(digest)
    }

    pub fn hash_file(
        path: impl AsRef<Path>,
        max_size: u64,
    ) -> SnapshotResult<String> {
        let metadata = fs::metadata(path.as_ref())?;

        if !metadata.is_file() {
            return Err(SnapshotError::StagingFailure(
                "Snapshot source is not a regular file".into()
            ));
        }

        if metadata.len() > max_size {
            return Err(SnapshotError::FileTooLarge(
                metadata.len()
            ));
        }

        let mut file = File::open(path)?;

        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut total = 0u64;

        loop {
            let count = file.read(&mut buffer)?;

            if count == 0 {
                break;
            }

            total += count as u64;

            if total > max_size {
                return Err(SnapshotError::FileTooLarge(total));
            }

            hasher.update(&buffer[..count]);
        }

        Ok(hex::encode(hasher.finalize()))
    }

    pub fn blob_path(&self, hash: &str) -> SnapshotResult<PathBuf> {
        Self::validate_hash(hash)?;

        let directory = self.root.join(&hash[..2]);

        Ok(directory.join(hash))
    }

    fn validate_hash(hash: &str) -> SnapshotResult<()> {
        if hash.len() != 64
            || !hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(SnapshotError::InvalidHash);
        }

        Ok(())
    }

    pub fn put_file(
        &self,
        source: impl AsRef<Path>,
    ) -> SnapshotResult<SnapshotBlob> {
        let source = source.as_ref();

        let metadata = fs::symlink_metadata(source)?;

        if !metadata.file_type().is_file() {
            return Err(SnapshotError::StagingFailure(
                "Source must be a regular, non-symlink file".into()
            ));
        }

        if metadata.len() > self.max_file_size {
            return Err(SnapshotError::FileTooLarge(
                metadata.len()
            ));
        }

        let mut file = File::open(source)?;

        self.put_reader(&mut file)
    }

    pub fn put_bytes(
        &self,
        bytes: &[u8],
    ) -> SnapshotResult<SnapshotBlob> {
        if bytes.len() as u64 > self.max_file_size {
            return Err(SnapshotError::FileTooLarge(
                bytes.len() as u64
            ));
        }

        self.put_reader(&mut &bytes[..])
    }

    fn put_reader<R: Read>(
        &self,
        reader: &mut R,
    ) -> SnapshotResult<SnapshotBlob> {
        let temporary_path = self.staging.join(
            format!("{}.tmp", Uuid::new_v4())
        );

        let mut hasher = Sha256::new();
        let mut total = 0u64;

        let result = (|| {
            let mut output = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary_path)?;

            let mut buffer = [0u8; 64 * 1024];

            loop {
                let count = reader.read(&mut buffer)?;

                if count == 0 {
                    break;
                }

                total += count as u64;

                if total > self.max_file_size {
                    return Err(SnapshotError::FileTooLarge(total));
                }

                hasher.update(&buffer[..count]);

                output.write_all(&buffer[..count])?;
            }

            output.flush()?;
            output.sync_all()?;

            let hash = hex::encode(hasher.finalize());

            let destination = self.blob_path(&hash)?;

            let parent = destination.parent().ok_or_else(|| {
                SnapshotError::StagingFailure(
                    "Invalid blob destination".into()
                )
            })?;

            fs::create_dir_all(parent)?;

            if destination.exists() {
                self.verify_blob_at(&destination, &hash, total)?;

                fs::remove_file(&temporary_path)?;

                return Ok(SnapshotBlob {
                    hash,
                    size_bytes: total,
                    path: destination,
                });
            }

            match fs::rename(&temporary_path, &destination) {
                Ok(()) => {}
                Err(error) if destination.exists() => {
                    fs::remove_file(&temporary_path)?;

                    self.verify_blob_at(
                        &destination,
                        &hash,
                        total,
                    )?;

                    return Ok(SnapshotBlob {
                        hash,
                        size_bytes: total,
                        path: destination,
                    });
                }
                Err(error) => return Err(
                    SnapshotError::Io(error)
                ),
            }

            let final_file = File::open(&destination)?;

            final_file.sync_all()?;

            Ok(SnapshotBlob {
                hash,
                size_bytes: total,
                path: destination,
            })
        })();

        if result.is_err() {
            let _ = fs::remove_file(&temporary_path);
        }

        result
    }

    pub fn read_blob(
        &self,
        hash: &str,
    ) -> SnapshotResult<Vec<u8>> {
        let path = self.blob_path(hash)?;

        let metadata = fs::metadata(&path)?;

        if metadata.len() > self.max_file_size {
            return Err(SnapshotError::FileTooLarge(
                metadata.len()
            ));
        }

        let mut file = File::open(path)?;

        let mut bytes = Vec::with_capacity(
            metadata.len() as usize
        );

        file.read_to_end(&mut bytes)?;

        let actual_hash = Self::hash_bytes(&bytes);

        if actual_hash != hash {
            return Err(SnapshotError::IntegrityFailure);
        }

        Ok(bytes)
    }

    pub fn verify_blob(
        &self,
        hash: &str,
    ) -> SnapshotResult<bool> {
        let path = self.blob_path(hash)?;

        if !path.exists() {
            return Ok(false);
        }

        let metadata = fs::metadata(&path)?;

        if metadata.len() > self.max_file_size {
            return Err(SnapshotError::FileTooLarge(
                metadata.len()
            ));
        }

        self.verify_blob_at(
            &path,
            hash,
            metadata.len(),
        )?;

        Ok(true)
    }

    fn verify_blob_at(
        &self,
        path: &Path,
        expected_hash: &str,
        expected_size: u64,
    ) -> SnapshotResult<()> {
        let metadata = fs::metadata(path)?;

        if metadata.len() != expected_size {
            return Err(SnapshotError::IntegrityFailure);
        }

        let actual_hash = Self::hash_file(
            path,
            self.max_file_size,
        )?;

        if actual_hash != expected_hash {
            return Err(SnapshotError::IntegrityFailure);
        }

        Ok(())
    }

    pub fn remove_stale_staging_files(
        &self,
    ) -> SnapshotResult<usize> {
        let mut removed = 0usize;

        for entry in fs::read_dir(&self.staging)? {
            let entry = entry?;

            let path = entry.path();

            let metadata = fs::symlink_metadata(&path)?;

            if !metadata.file_type().is_file() {
                continue;
            }

            let filename = entry.file_name();

            if filename.to_string_lossy().ends_with(".tmp") {
                fs::remove_file(path)?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    pub fn total_blob_storage_bytes(
        &self,
    ) -> SnapshotResult<u64> {
        let mut total = 0u64;

        for entry in fs::read_dir(&self.root)? {
            let entry = entry?;

            let path = entry.path();

            if path == self.staging {
                continue;
            }

            let metadata = fs::symlink_metadata(&path)?;

            if !metadata.file_type().is_dir() {
                continue;
            }

            for blob in fs::read_dir(path)? {
                let blob = blob?;

                let metadata = fs::symlink_metadata(blob.path())?;

                if metadata.file_type().is_file() {
                    total = total.saturating_add(metadata.len());
                }
            }
        }

        Ok(total)
    }
}
