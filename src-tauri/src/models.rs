
use serde::{Deserialize, Serialize};

use crate::database::{
    ProtectedFolder,
    TrackedFile,
    VersionRecord,
};
use crate::restore::RestoreResultInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderDto {
    pub id: String,
    pub path: String,
    pub enabled: bool,
    pub created_at: String,
}

impl From<ProtectedFolder> for FolderDto {
    fn from(value: ProtectedFolder) -> Self {
        Self {
            id: value.id,
            path: value.root_path,
            enabled: value.enabled,
            created_at: value.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDto {
    pub id: String,
    pub folder_id: String,
    pub relative_path: String,
    pub last_hash: Option<String>,
    pub last_seen_at: Option<String>,
    pub state: String,
}

impl From<TrackedFile> for FileDto {
    fn from(value: TrackedFile) -> Self {
        Self {
            id: value.id,
            folder_id: value.folder_id,
            relative_path: value.relative_path,
            last_hash: value.last_hash,
            last_seen_at: value.last_seen_at,
            state: value.state,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionDto {
    pub id: String,
    pub file_id: String,
    pub hash: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub reason: String,
    pub root_path: String,
    pub relative_path: String,
}

impl From<VersionRecord> for VersionDto {
    fn from(value: VersionRecord) -> Self {
        Self {
            id: value.id,
            file_id: value.file_id,
            hash: value.blob_hash,
            created_at: value.created_at,
            size_bytes: value.size_bytes as u64,
            reason: value.reason,
            root_path: value.root_path,
            relative_path: value.relative_path,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardDto {
    pub protected_folder_count: usize,
    pub tracked_file_count: usize,
    pub snapshot_count: usize,
    pub storage_bytes: u64,
    pub monitoring: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResultDto {
    pub folder_id: String,
    pub files_scanned: usize,
    pub snapshots_created: usize,
    pub unchanged_files: usize,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResultDto {
    pub operation_id: String,
    pub restored_version_id: String,
    pub restored_hash: String,
    pub previous_hash: Option<String>,
    pub target_path: String,
    pub restored_size: u64,
}

impl From<RestoreResultInfo> for RestoreResultDto {
    fn from(value: RestoreResultInfo) -> Self {
        Self {
            operation_id: value.operation_id,
            restored_version_id: value.restored_version_id,
            restored_hash: value.restored_hash,
            previous_hash: value.previous_hash,
            target_path: value.target_path.to_string_lossy().into_owned(),
            restored_size: value.restored_size,
        }
    }
}
